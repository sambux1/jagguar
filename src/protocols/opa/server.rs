//! OPA server: public parameters, share routing, unmask, and decode.

use std::io::Write;
use std::os::fd::FromRawFd;
use std::sync::mpsc;

use crate::communicator::Communicator;
use crate::communicator::wire::{self, CommitteeIndex, decode, encode};
use crate::crypto::prg::populate_random_bytes;
use crate::crypto::{F256, SeedHomomorphicPRG, Shamir, field_from_bytes, field_low_u128};
use crate::protocols::opa::client::{NUM_PARTIES_UPPER_BOUND, OUTPUT_LEN};
use crate::protocols::opa::messages::{FromServer, Inputs, SeedShare, ToServer};
use crate::protocols::server::Server;
use crate::util::packing::unpack_vector;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};

/// Public OPA parameters.
///
/// `security_parameter` is κ in the client encoding and corresponds to a statistical security
//  parameter. `reconstruction_threshold` and `committee_size` are Shamir `t` and `n`.
// `corruption_threshold` is stored but unused in this semi-honest implementation.
#[derive(Copy, Clone)]
pub struct OPASetupParameters {
    security_parameter: u64,
    corruption_threshold: u64,
    reconstruction_threshold: u64,
    committee_size: u64,
}

impl OPASetupParameters {
    /// κ, unused corruption threshold, Shamir `t`, and Shamir `n`.
    pub fn new(
        security_parameter: u64,
        corruption_threshold: u64,
        reconstruction_threshold: u64,
        committee_size: u64,
    ) -> Self {
        Self {
            security_parameter,
            corruption_threshold,
            reconstruction_threshold,
            committee_size,
        }
    }
}

/// Public OPA state distributed to clients and committee members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OPAState {
    /// SHPRG public-parameter seed.
    pub succinct_seed: [u8; 32],
    /// Statistical security parameter κ.
    pub security_parameter: u64,
    /// Stored but unused in this semi-honest implementation.
    pub corruption_threshold: u64,
    /// Shamir reconstruction threshold `t`.
    pub reconstruction_threshold: u64,
    /// Shamir number of shares `n` (committee size).
    pub committee_size: u64,
    /// Committee local-port offsets `1 ..= committee_size`.
    pub committee_port_offsets: Vec<u16>,
    /// Server bind port (set when the communicator starts).
    pub port: u16,
    /// Optional channel for sending decoded output back to the simulator.
    #[serde(skip)]
    pub output_sender: Option<mpsc::Sender<Vec<u32>>>,
    /// Optional file descriptor for sending decoded output to the process manager.
    #[serde(skip)]
    pub output_fd: Option<i32>,
    /// Optional cache of messages used during aggregation.
    #[serde(skip)]
    pub client_messages: Vec<Vec<u8>>,
    /// Raw committee messages, filled before [`OPAServer::aggregate`].
    #[serde(skip)]
    pub committee_messages: Vec<Vec<u8>>,
}

/// OPA aggregation server.
pub struct OPAServer {
    setup_parameters: OPASetupParameters,
    state: OPAState,
    communicator: Option<Communicator>,
}

impl OPAState {
    /// Decode a `FromServer::State` reply.
    pub fn from_reply(bytes: &[u8]) -> std::io::Result<Self> {
        match wire::decode::<FromServer>(bytes) {
            Ok(FromServer::State(state)) => Ok(state),
            Ok(_) => Err(std::io::Error::other("reply was not state")),
            Err(e) => Err(std::io::Error::other(e)),
        }
    }

    /// Decode an unmasked aggregate, treating every slot as payload.
    pub fn decode_output(&self, output: Vec<u128>) -> Vec<u32> {
        let len = output.len();
        self.decode_output_with_len(output, len)
    }

    /// Decode only the first `payload_len` ciphertext slots (excluding zero-padding to OUTPUT_LEN).
    pub fn decode_output_with_len(&self, output: Vec<u128>, payload_len: usize) -> Vec<u32> {
        let kappa: u32 = self.security_parameter as u32;
        let two_to_kappa: u128 = 1u128 << kappa;
        let two_to_kappa_times_n: u128 = two_to_kappa * (NUM_PARTIES_UPPER_BOUND as u128);

        let denom = two_to_kappa_times_n as u128;
        let decoded: Vec<u32> = output[..payload_len.min(output.len())]
            .iter()
            .map(|&x| {
                let q = x / denom;
                let r = x % denom;
                let ceil = q + if r != 0 { 1 } else { 0 };
                (ceil as u32) - 1
            })
            .collect();

        unpack_vector(&decoded)
    }
}

impl OPAServer {
    /// File descriptor that receives the decoded aggregate.
    pub fn set_output_fd(&mut self, fd: i32) {
        self.state.output_fd = Some(fd);
    }

    /// [`OPAState::decode_output_with_len`] for the server's aggregate path.
    fn decode_output(state: &OPAState, output: Vec<u128>, payload_len: usize) -> Vec<u32> {
        state.decode_output_with_len(output, payload_len)
    }

    /// This member's Shamir share from each [`ToServer::ClientUpload`].
    fn member_shares(raw_messages: &[Vec<u8>], committee_index: usize) -> Vec<SeedShare> {
        let mut shares = Vec::new();
        for raw_msg in raw_messages {
            let Ok(ToServer::ClientUpload { data, .. }) = decode(raw_msg) else {
                eprintln!("client queue contained a non-upload");
                continue;
            };
            match data.seed_shares.get(committee_index) {
                Some(share) => shares.push(share.clone()),
                None => eprintln!("client upload missing share for member {committee_index}"),
            }
        }
        shares
    }

    /// [`FromServer::Inputs`] for a `POST /committee-fetch` response.
    fn encode_member_inputs(raw_messages: &[Vec<u8>], committee_index: usize) -> Vec<u8> {
        let shares = Self::member_shares(raw_messages, committee_index);
        println!(
            "Sent {} inputs to committee member {committee_index}",
            shares.len()
        );
        let reply = FromServer::Inputs(Inputs { shares });
        encode(&reply).expect("encode committee inputs")
    }

    /// Called when all committee outputs have been received. This stores the
    /// messages into the state and runs aggregation.
    fn on_committee_complete(
        mut state: OPAState,
        committee_messages: Vec<Vec<u8>>,
        client_messages: Vec<Vec<u8>>,
    ) {
        // Store the messages into the state
        state.committee_messages = committee_messages;
        state.client_messages = client_messages;

        // Run aggregation directly using the state - clean and simple!
        OPAServer::aggregate(&state);
    }
}

impl Server for OPAServer {
    type SetupParameters = OPASetupParameters;
    type State = OPAState;

    /// Construct and [`Self::setup`] from `setup_parameters`.
    fn new(setup_parameters: OPASetupParameters) -> Self {
        // default initialization, delegate real work to setup function
        let mut server = Self {
            setup_parameters,
            state: OPAState {
                succinct_seed: [0u8; 32],
                security_parameter: 0,
                corruption_threshold: 0,
                reconstruction_threshold: 0,
                committee_size: 0,
                committee_port_offsets: Vec::new(),
                port: 0,
                output_sender: None,
                output_fd: None,
                client_messages: Vec::new(),
                committee_messages: Vec::new(),
            },
            communicator: None,
        };
        server.setup(server.setup_parameters);
        server
    }

    /// Install the localhost communicator.
    fn set_communicator(&mut self, comm: Communicator) {
        self.communicator = Some(comm);
    }

    /// The communicator, once [`Self::set_communicator`] has been called.
    ///
    /// # Panics
    ///
    /// Panics if no communicator is set.
    fn get_communicator(&mut self) -> &mut Communicator {
        self.communicator.as_mut().unwrap()
    }

    /// Sample the SHPRG public-parameter seed and committee port offsets
    /// `1 ..= committee_size`. Preserves an existing output channel.
    fn setup(&mut self, args: Self::SetupParameters) {
        self.setup_parameters = args;

        // sample the public parameter seed
        let mut rng = ChaCha20Rng::from_entropy();
        let mut succinct_seed = [0u8; 32];
        populate_random_bytes(&mut succinct_seed, &mut rng);

        // sample the committee ports
        // TODO: currently just uses port offsets 1 through committee_size
        let mut committee_port_offsets = Vec::<u16>::new();
        for i in 1..=self.setup_parameters.committee_size as u16 {
            committee_port_offsets.push(i);
        }

        // preserve any existing output sender when refreshing the public state
        let output_sender = self.state.output_sender.clone();
        let output_fd = self.state.output_fd;
        let client_messages = self.state.client_messages.clone();
        let committee_messages = self.state.committee_messages.clone();

        // set the public state
        self.state = OPAState {
            succinct_seed,
            security_parameter: self.setup_parameters.security_parameter,
            corruption_threshold: self.setup_parameters.corruption_threshold,
            reconstruction_threshold: self.setup_parameters.reconstruction_threshold,
            committee_size: self.setup_parameters.committee_size,
            committee_port_offsets,
            port: 0,
            output_sender,
            output_fd,
            client_messages,
            committee_messages,
        };
    }

    /// Record `port` and register the fetch and aggregation callbacks.
    fn on_communicator_setup(&mut self, port: u16) {
        self.state.port = port;

        let messages = self.get_communicator().get_received_messages();
        self.get_communicator()
            .set_committee_fetch_callback(move |body| {
                let Ok(ToServer::RequestInputs { index }) = decode(body) else {
                    eprintln!("committee fetch body was not RequestInputs");
                    return Vec::new();
                };
                let inputs = messages.lock().unwrap().clone();
                Self::encode_member_inputs(&inputs, index.as_usize())
            });

        // Configure auto-trigger for final aggregation when all committee outputs are received
        let expected = self.state.committee_size as usize;
        self.get_communicator()
            .set_committee_expected_size(expected);
        let state_for_callback = self.state.clone();
        let client_messages_arc = self.get_communicator().get_received_messages();
        self.get_communicator()
            .set_committee_complete_callback(move |msgs| {
                println!(
                    "Auto-triggering final aggregation with {} committee messages",
                    msgs.len()
                );
                let client_messages = client_messages_arc.lock().unwrap().clone();
                Self::on_committee_complete(state_for_callback.clone(), msgs, client_messages);
            });

        let reply = FromServer::State(self.state.clone());
        let bytes = encode(&reply).expect("encode server state");
        self.get_communicator().start_http_communicator(bytes);
    }

    /// Current public [`OPAState`].
    fn get_state(&self) -> &Self::State {
        &self.state
    }

    /// Committee port offsets (`1 ..= committee_size`).
    fn get_committee_port_offsets(&self) -> Vec<u16> {
        self.state.committee_port_offsets.clone()
    }

    /// Simulator channel for the decoded aggregate.
    fn set_output_channel(&mut self, sender: mpsc::Sender<Vec<u32>>) {
        self.state.output_sender = Some(sender);
    }

    /// Reconstruct the summed SHPRG seed, expand, unmask, decode, and send
    /// on the simulator channel if set.
    fn aggregate(state: &OPAState) {
        if state.committee_messages.is_empty() || state.client_messages.is_empty() {
            eprintln!("aggregate() called but messages are not populated; skipping.");
            return;
        }

        let committee_messages = state.committee_messages.clone();
        let client_messages = state.client_messages.clone();

        println!(
            "Performing final aggregation with {} committee messages",
            committee_messages.len()
        );

        // extract the secret shares from the committee messages, along with their indices
        let mut committee_outputs: Vec<(CommitteeIndex, SeedShare)> =
            Vec::with_capacity(committee_messages.len());
        for msg in committee_messages {
            let Ok(ToServer::CommitteeUpload { index, data }) = decode(&msg) else {
                eprintln!("Malformed committee message");
                continue;
            };
            committee_outputs.push((index, data));
        }

        if committee_outputs.len() < state.reconstruction_threshold as usize {
            eprintln!("Not enough committee outputs to reconstruct the SHPRG seed");
            return;
        }

        // reconstruct the SHPRG seed from the secret shares (Shamir over F256)
        let shamir = Shamir::<F256>::new(
            state.committee_size as usize,
            state.reconstruction_threshold as usize,
        );
        let seed_len = committee_outputs[0].1.len();
        let mut reconstructed_seed: Vec<u128> = Vec::with_capacity(seed_len);
        for j in 0..seed_len {
            let mut pairs: Vec<(F256, F256)> = Vec::with_capacity(committee_outputs.len());
            for (idx, share_vec) in committee_outputs.iter() {
                let x = F256::from(idx.shamir_x());
                let y = field_from_bytes(&share_vec[j]);
                pairs.push((x, y));
            }
            let s = shamir
                .reconstruct(&pairs)
                .expect("Shamir reconstruction failed");
            // Sum of client seed components can exceed 2^128; SHPRG uses Z_{2^128}.
            reconstructed_seed.push(field_low_u128(s));
        }
        println!(
            "Reconstructed SHPRG seed of length {}",
            reconstructed_seed.len()
        );

        // expand the SHPRG seed
        let shprg =
            SeedHomomorphicPRG::new_from_both_seeds(state.succinct_seed, reconstructed_seed);
        let mask = shprg.expand(OUTPUT_LEN);
        println!("Expanded SHPRG mask of length {}", mask.len());

        // aggregate input ciphertexts in Z_{2^128}
        let mut uploads = Vec::with_capacity(client_messages.len());
        for raw in client_messages {
            let Ok(ToServer::ClientUpload { data, .. }) = decode(&raw) else {
                eprintln!("client queue contained a non-upload");
                continue;
            };
            uploads.push(data);
        }
        if uploads.is_empty() {
            eprintln!("no client uploads to aggregate");
            return;
        }
        let payload_len = uploads[0].payload_len as usize;
        let mut aggregated_ciphertext = uploads[0].masked_input.clone();
        for upload in uploads.iter().skip(1) {
            assert_eq!(
                upload.masked_input.len(),
                aggregated_ciphertext.len(),
                "client uploads have different ciphertext lengths"
            );
            for (slot, value) in aggregated_ciphertext.iter_mut().zip(&upload.masked_input) {
                *slot = slot.wrapping_add(*value);
            }
        }
        let num_clients = uploads.len();
        println!("Aggregated masked ciphertext from {} clients", num_clients);

        // unmask the aggregated ciphertext in Z_{2^128}
        let unmasked: Vec<u128> = aggregated_ciphertext
            .iter()
            .zip(mask.iter())
            .map(|(&c, &m)| c.wrapping_sub(m))
            .collect();

        let decoded = Self::decode_output(&state, unmasked, payload_len);
        println!("Decoded output length: {}", decoded.len());

        if let Some(fd) = state.output_fd {
            let mut out = unsafe { std::fs::File::from_raw_fd(fd) };
            let bytes = encode(&decoded).expect("encode decoded output");
            out.write_all(&bytes).expect("write decoded output");
        }

        if let Some(ref sender) = state.output_sender {
            if let Err(e) = sender.send(decoded.clone()) {
                eprintln!("Failed to send decoded output over channel: {}", e);
            }
        }
    }
}
