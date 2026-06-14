use std::io::Read;
use std::net::TcpStream;
use std::sync::mpsc;

use crate::protocols::server::Server;
use crate::crypto::{SeedHomomorphicPRG, shamir::Shamir, F128};
use crate::crypto::prg::populate_random_bytes;
use crate::crypto::util::field_to_128;
use crate::util::packing::unpack_vector;
use crate::communicator::Communicator;
use crate::protocols::opa::client::{NUM_PARTIES_UPPER_BOUND, OUTPUT_LEN};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

#[derive(Copy, Clone)]
pub struct OPASetupParameters {
    security_parameter: u64,
    corruption_threshold: u64,
    reconstruction_threshold: u64,
    committee_size: u64,
}

impl OPASetupParameters {
    pub fn new(
        security_parameter: u64,
        corruption_threshold: u64,
        reconstruction_threshold: u64,
        committee_size: u64
    ) -> Self {
        Self {
            security_parameter,
            corruption_threshold,
            reconstruction_threshold,
            committee_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OPAState {
    pub succinct_seed: [u8; 32],
    pub security_parameter: u64,
    pub corruption_threshold: u64,
    pub reconstruction_threshold: u64,
    pub committee_size: u64,
    pub committee_port_offsets: Vec<u16>,
    pub port: u16,
    /// Optional channel for sending decoded output back to the simulator.
    pub output_sender: Option<mpsc::Sender<Vec<u32>>>,
    /// Optional cache of messages used during aggregation.
    pub client_messages: Vec<Vec<u8>>,
    pub committee_messages: Vec<Vec<u8>>,
}

pub struct OPAServer {
    setup_parameters: OPASetupParameters,
    state: OPAState,
    public_parameter: Vec<Vec<u128>>,
    communicator: Option<Communicator>,
}

impl OPAState {
    /// Decode a packed, masked aggregate back into a vector of u32s.
    /// This is shared between the server logic and tests/clients.
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
    fn decode_output(state: &OPAState, output: Vec<u128>, payload_len: usize) -> Vec<u32> {
        state.decode_output_with_len(output, payload_len)
    }

    fn send_to_committee(tcp_stream: TcpStream, raw_messages: Vec<Vec<u8>>, port: u16) {
        let committee_index = port - 10001; // TODO: this should be a constant/partyID

        // deserialize each client message: [masked_input u128...][num_shares u32][share...]
        let mut committee_shares = Vec::new();
        
        for raw_msg in &raw_messages {
            let mut cursor = std::io::Cursor::new(raw_msg);
            // skip masked_input (we know how long it is from the expand output length)
            cursor.set_position((4 + OUTPUT_LEN * 16) as u64);
            
            // read num_shares
            let mut buf = [0u8; 4];
            cursor.read_exact(&mut buf).unwrap();
            let num_shares = u32::from_le_bytes(buf) as usize;
            
            // skip to the share for this committee member
            for i in 0..num_shares {
                cursor.read_exact(&mut buf).unwrap();
                let share_len = u32::from_le_bytes(buf) as usize;
                let share_bytes = share_len * 16;
                if i == committee_index as usize {
                    let mut share = Vec::with_capacity(share_len);
                    for _ in 0..share_len {
                        let mut xbuf = [0u8; 16];
                        cursor.read_exact(&mut xbuf).unwrap();
                        share.push(u128::from_le_bytes(xbuf));
                    }
                    committee_shares.push(share);
                } else {
                    cursor.set_position(cursor.position() + share_bytes as u64);
                }
            }
        }
        
        // serialize all shares for this committee member.
        // send_on_stream prefixes each blob with its byte length, so the committee
        // derives the element count as byte_len / 16 — do not add a redundant inner prefix.
        let mut serialized_shares = Vec::new();
        for share in &committee_shares {
            let mut data = Vec::new();
            for &x in share {
                data.extend_from_slice(&x.to_le_bytes());
            }
            serialized_shares.push(data);
        }
        
        // send the combined shares to the committee member
        if let Err(e) = Communicator::send_on_stream(tcp_stream, serialized_shares) {
            eprintln!("Failed to send data to committee member {}: {}", committee_index, e);
        }
    }

    /// Called when all committee outputs have been received. This stores the
    /// messages into the state and runs aggregation.
	fn on_committee_complete(mut state: OPAState, committee_messages: Vec<Vec<u8>>, client_messages: Vec<Vec<u8>>) {
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
                client_messages: Vec::new(),
                committee_messages: Vec::new(),
            },
            public_parameter: Vec::new(),
            communicator: None,
        };
        server.setup(server.setup_parameters);
        server
    }

    fn set_communicator(&mut self, comm: Communicator) {
        self.communicator = Some(comm);
    }

    fn get_communicator(&mut self) -> &mut Communicator {
        self.communicator.as_mut().unwrap()
    }

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
            client_messages,
            committee_messages,
        };

        // sample the public parameter from the succinct seed
        let shprg = SeedHomomorphicPRG::new_from_public_seed(succinct_seed);
        let public_parameter = shprg.get_public_parameter();
        self.public_parameter = public_parameter.clone();
    }

    fn on_communicator_setup(&mut self, port: u16) {
        self.state.port = port;
        
        // Capture the received_messages Arc so the callback can access it
        let messages = self.get_communicator().get_received_messages();
        
        // Set up the callback to gather inputs and send them through the stream
        self.get_communicator().set_signal_callback(move |stream, port| {
            println!("Signal handler called in server");
            let inputs = messages.lock().unwrap().clone();
            Self::send_to_committee(stream, inputs, port);
        });

		// Configure auto-trigger for final aggregation when all committee outputs are received
		let expected = self.state.committee_size as usize;
		self.get_communicator().set_committee_expected_size(expected);
		let state_for_callback = self.state.clone();
		let client_messages_arc = self.get_communicator().get_received_messages();
		self.get_communicator().set_committee_complete_callback(move |msgs| {
			println!("Auto-triggering final aggregation with {} committee messages", msgs.len());
			let client_messages = client_messages_arc.lock().unwrap().clone();
			Self::on_committee_complete(state_for_callback.clone(), msgs, client_messages);
		});
    }

    fn get_state(&self) -> &Self::State {
        &self.state
    }

    fn get_committee_port_offsets(&self) -> Vec<u16> {
        self.state.committee_port_offsets.clone()
    }

    fn set_output_channel(&mut self, sender: mpsc::Sender<Vec<u32>>) {
        self.state.output_sender = Some(sender);
    }

    fn aggregate(state: &OPAState) {
        if state.committee_messages.is_empty() || state.client_messages.is_empty() {
            eprintln!("aggregate() called but messages are not populated; skipping.");
            return;
        }

        let committee_messages = state.committee_messages.clone();
        let client_messages = state.client_messages.clone();

        println!("Performing final aggregation with {} committee messages", committee_messages.len());

        // extract the secret shares from the committee messages, along with their indices
        let mut committee_outputs: Vec<(usize, Vec<u128>)> =
            Vec::with_capacity(committee_messages.len());
        for msg in committee_messages {
            // format: "committee" (9 bytes) || index (u16 LE) || [len u32] [u128 x len]
            if msg.len() < 11 || &msg[..9] != b"committee" {
                eprintln!("Malformed committee message; missing prefix");
                continue;
            }
            let index_bytes = &msg[9..11];
            let committee_index =
                u16::from_le_bytes([index_bytes[0], index_bytes[1]]) as usize;

            let mut cursor = std::io::Cursor::new(&msg[11..]);
            let mut buf = [0u8; 4];
            cursor.read_exact(&mut buf).unwrap();
            let share_len = u32::from_le_bytes(buf) as usize;
            let mut share = Vec::with_capacity(share_len);
            for _ in 0..share_len {
                let mut xbuf = [0u8; 16];
                cursor.read_exact(&mut xbuf).unwrap();
                share.push(u128::from_le_bytes(xbuf));
            }
            committee_outputs.push((committee_index, share));
        }

        if committee_outputs.len() < state.reconstruction_threshold as usize {
            eprintln!("Not enough committee outputs to reconstruct the SHPRG seed");
            return;
        }

        // reconstruct the SHPRG seed from the secret shares (Shamir over F128)
        let shamir = Shamir::<F128>::new(
            state.committee_size as usize,
            state.reconstruction_threshold as usize
        );
        let seed_len = committee_outputs[0].1.len();
        let mut reconstructed_seed: Vec<u128> = Vec::with_capacity(seed_len);
        for j in 0..seed_len {
            let mut pairs: Vec<(F128, F128)> = Vec::with_capacity(committee_outputs.len());
            for (idx, share_vec) in committee_outputs.iter() {
                let x = F128::from((*idx as u64) + 1);
                let y = F128::from(share_vec[j]);
                pairs.push((x, y));
            }
            let s = shamir.reconstruct(&pairs).expect("Shamir reconstruction failed");
            reconstructed_seed.push(field_to_128(s));
        }
        println!("Reconstructed SHPRG seed of length {}", reconstructed_seed.len());

        // expand the SHPRG seed
        let shprg = SeedHomomorphicPRG::new_from_both_seeds(state.succinct_seed, reconstructed_seed);
        let mask = shprg.expand();
        println!("Expanded SHPRG mask of length {}", mask.len());

        // aggregate input ciphertexts in F128
        let mut client_iter = client_messages.into_iter();
        let first_raw = client_iter.next().unwrap();
        let mut cursor = std::io::Cursor::new(&first_raw);
        let mut buf = [0u8; 4];
        cursor.read_exact(&mut buf).unwrap();
        let payload_len = u32::from_le_bytes(buf) as usize;

        let mut aggregated_ciphertext: Vec<u128> = (0..OUTPUT_LEN)
            .map(|_| {
                let mut xbuf = [0u8; 16];
                cursor.read_exact(&mut xbuf).unwrap();
                u128::from_le_bytes(xbuf)
            })
            .collect();
        let mut num_clients = 1usize;
        for raw in client_iter {
            let mut c = std::io::Cursor::new(&raw);
            c.read_exact(&mut buf).unwrap();
            for i in 0..OUTPUT_LEN {
                let mut xbuf = [0u8; 16];
                c.read_exact(&mut xbuf).unwrap();
                aggregated_ciphertext[i] = field_to_128(
                    F128::from(aggregated_ciphertext[i]) + F128::from(u128::from_le_bytes(xbuf))
                );
            }
            num_clients += 1;
        }
        println!("Aggregated masked ciphertext from {} clients", num_clients);

        // unmask the aggregated ciphertext in F128
        let mut unmasked = aggregated_ciphertext.clone();
        for i in 0..OUTPUT_LEN {
            unmasked[i] = field_to_128(F128::from(unmasked[i]) - F128::from(mask[i]));
        }

        let decoded = Self::decode_output(&state, unmasked, payload_len);
        println!("Decoded output length: {}", decoded.len());

        if let Some(ref sender) = state.output_sender {
            if let Err(e) = sender.send(decoded.clone()) {
                eprintln!("Failed to send decoded output over channel: {}", e);
            }
        }
    }
}
