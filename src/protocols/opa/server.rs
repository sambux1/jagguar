use crate::protocols::server::Server;
use crate::crypto::{SeedHomomorphicPRG, shamir::Shamir};
use crate::crypto::prg::populate_random_bytes;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use crate::communicator::Communicator;
use crate::crypto::F128;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::net::TcpStream;
use std::io::Cursor;
use crate::crypto::util::field_to_128;
use crate::util::packing::unpack_vector;
use crate::protocols::opa::client::NUM_PARTIES_UPPER_BOUND;
use std::sync::mpsc;

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
    public_parameter: Vec<Vec<crate::crypto::F128>>,
    communicator: Option<Communicator>,
}

impl OPAState {
    /// Decode a packed, masked aggregate back into a vector of u32s.
    /// This is shared between the server logic and tests/clients.
    pub fn decode_output(&self, output: Vec<u128>) -> Vec<u32> {
        // compute 2^kappa and (2^kappa * n)
        let kappa: u32 = self.security_parameter as u32;
        let two_to_kappa: u128 = 1u128 << kappa;
        let two_to_kappa_times_n: u128 = two_to_kappa * (NUM_PARTIES_UPPER_BOUND as u128);

        // decoded = ceil(encoded / (2^kappa * n)) - 1
        let denom = two_to_kappa_times_n as u128;
        let decoded: Vec<u32> = output.iter()
            .map(|x| {
                let q = x / denom;
                let r = x % denom;
                let ceil = q + if r != 0 { 1 } else { 0 };
                (ceil as u32) - 1
            })
            .collect();
 
        // unpack the elements of the decoded vector into the target type
        let result: Vec<u32> = unpack_vector(&decoded);
        
        // return the decoded output
        result
    }

}

impl OPAServer {
    fn decode_output(state: &OPAState, output: Vec<u128>) -> Vec<u32> {
        state.decode_output(output)
    }

    fn send_to_committee(tcp_stream: TcpStream, raw_messages: Vec<Vec<u8>>, port: u16) {
        let committee_index = port - 10001; // TODO: this should be a constant/partyID

        // deserialize each client message to extract the portion for this committee member
        let mut committee_shares = Vec::new();
        
        for raw_msg in &raw_messages {
            // deserialize the client message: (masked_input, shares)
			let mut cursor = Cursor::new(raw_msg);
            let _masked_input = Vec::<F128>::deserialize_compressed(&mut cursor)
                .expect("Failed to deserialize masked_input");
            let shares = Vec::<Vec<F128>>::deserialize_compressed(&mut cursor)
                .expect("Failed to deserialize shares");
            
            // extract the share for this committee member (committee_index is 0-based)
            if (committee_index as usize) < shares.len() {
                committee_shares.push(shares[committee_index as usize].clone());
            }
        }
        
        // serialize all shares for this committee member
        let mut serialized_shares = Vec::new();
        for share in &committee_shares {
            let mut data = Vec::new();
            share.serialize_compressed(&mut data).expect("Failed to serialize share");
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
        // Aggregate using the provided state. This assumes that
        // `client_messages` and `committee_messages` have been populated already.
        if state.committee_messages.is_empty() || state.client_messages.is_empty() {
            eprintln!("aggregate() called but messages are not populated; skipping.");
            return;
        }

        let committee_messages = state.committee_messages.clone();
        let client_messages = state.client_messages.clone();

        println!("Performing final aggregation with {} committee messages", committee_messages.len());

        // extract the secret shares from the committee messages, along with their indices
        // so we can use the correct x-coordinates in Shamir reconstruction.
        let mut committee_outputs: Vec<(usize, Vec<F128>)> =
            Vec::with_capacity(committee_messages.len());
        for msg in committee_messages {
            // format: "committee" (9 bytes) || index (u16 LE) || compressed Vec<F128>
            if msg.len() < 11 || &msg[..9] != b"committee" {
                eprintln!("Malformed committee message; missing prefix");
                continue;
            }
            let index_bytes = &msg[9..11];
            let committee_index =
                u16::from_le_bytes([index_bytes[0], index_bytes[1]]) as usize;

            let mut cursor = Cursor::new(&msg[11..]);
            match Vec::<F128>::deserialize_compressed(&mut cursor) {
                Ok(vec) => committee_outputs.push((committee_index, vec)),
                Err(e) => eprintln!("Failed to deserialize committee output share: {}", e),
            }
        }

        if committee_outputs.len() < state.reconstruction_threshold as usize {
            eprintln!("Not enough committee outputs to reconstruct the SHPRG seed");
            return;
        }

        // reconstruct the SHPRG seed from the secret shares
        let shamir = Shamir::<F128>::new(
            state.committee_size as usize,
            state.reconstruction_threshold as usize
        );
        let seed_len = committee_outputs[0].1.len();
        let mut reconstructed_seed: Vec<F128> = Vec::with_capacity(seed_len);
        for j in 0..seed_len {
            // build (x_i, y_{i,j}) pairs across parties i for seed component j
            let mut pairs: Vec<(F128, F128)> = Vec::with_capacity(committee_outputs.len());
            for (idx, share_vec) in committee_outputs.iter() {
                // x-coordinate is fixed by how Shamir sharing was created: x = index + 1
                let x = F128::from((*idx as u64) + 1);
                let y = share_vec[j];
                pairs.push((x, y));
            }
            let s = shamir.reconstruct(&pairs).expect("Shamir reconstruction failed");
            reconstructed_seed.push(s);
        }
        println!("Reconstructed SHPRG seed of length {}", reconstructed_seed.len());

        // expand the SHPRG seed
        let shprg = SeedHomomorphicPRG::new_from_both_seeds(state.succinct_seed, reconstructed_seed);
        let mask = shprg.expand();
        println!("Expanded SHPRG mask of length {}", mask.len());

        // aggregate input ciphertexts
        let mut client_iter = client_messages.into_iter();
        let first_raw = client_iter.next().unwrap();
        let mut cursor = Cursor::new(&first_raw);
        let mut aggregated_ciphertext = Vec::<F128>::deserialize_compressed(&mut cursor).unwrap();
        let mut num_clients = 1usize;
        for raw in client_iter {
            let mut c = Cursor::new(&raw);
            let masked_input = Vec::<F128>::deserialize_compressed(&mut c).unwrap();
            for (i, v) in masked_input.iter().enumerate() {
                aggregated_ciphertext[i] = aggregated_ciphertext[i] + *v;
            }
            num_clients += 1;
        }
        println!("Aggregated masked ciphertext from {} clients", num_clients);
        println!("Aggregated masked ciphertext length: {}", aggregated_ciphertext.len());

        // unmask the aggregated ciphertext
        let mut unmasked = aggregated_ciphertext.clone();
        for i in 0..1024 {
            unmasked[i] = unmasked[i] - mask[i];
        }
        println!("Unmasked aggregated ciphertext length: {}", unmasked.len());

        // decode the unmasked aggregated ciphertext to obtain the output
        let unmasked_u128: Vec<u128> = unmasked.into_iter().map(|x| field_to_128(x)).collect();
        let decoded = Self::decode_output(&state, unmasked_u128);
        println!("Decoded output length: {}", decoded.len());

        // If an output channel was configured in the server state, send the
        // decoded output back to the simulator.
        if let Some(ref sender) = state.output_sender {
            if let Err(e) = sender.send(decoded.clone()) {
                eprintln!("Failed to send decoded output over channel: {}", e);
            }
        }
    }
}
