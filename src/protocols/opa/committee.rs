use crate::protocols::committee::Committee;
use crate::protocols::opa::server::OPAState;
use crate::communicator::Communicator;
use crate::crypto::F128;
use ark_serialize::CanonicalDeserialize;
use std::io::{Cursor, Read};

pub struct OPACommittee {
    server_state: Option<OPAState>,
    communicator: Communicator,
    input_shares: Option<Vec<Vec<F128>>>,
}

impl Committee for OPACommittee {
    type ServerState = OPAState;

    fn new(port: u16) -> Self {
        // create a communicator on the passed port
        let communicator = Communicator::new(port);

        Self {
            server_state: None,
            communicator,
            input_shares: None,
        }
    }

    fn set_server_state(&mut self, state: Self::ServerState) {
        self.server_state = Some(state.clone());
    }

    fn retrieve_inputs(&mut self) {
        // signal the server that we're ready to retrieve inputs
        let server_port = self.server_state.as_ref().unwrap().port;
        let inputs = self.communicator.receive_from_server(server_port)
            .expect("Failed to receive inputs from server");

        // Parse the received data into secret shares.
        // The data format is length-prefixed: each share is preceded by a 4-byte little-endian
        // u32 indicating the length of that share's serialized data. This allows us to read
        // variable-length shares sequentially. Each share is a Vec<F128> representing one
        // client's secret share for this committee member.
        // Format: [len1 (4 bytes)][share1 data][len2 (4 bytes)][share2 data]...
        let mut cursor = Cursor::new(&inputs);
        let mut shares = Vec::new();
        
        while cursor.position() < inputs.len() as u64 {
            // read the length prefix (u32, little-endian)
            let mut len_bytes = [0u8; 4];
            cursor.read_exact(&mut len_bytes)
                .expect("Failed to read length prefix");
            let len = u32::from_le_bytes(len_bytes) as usize;
            
            // read the serialized share
            let mut share_data = vec![0u8; len];
            cursor.read_exact(&mut share_data)
                .expect("Failed to read share data");
            
            // deserialize the share (Vec<F128>)
            let mut share_cursor = Cursor::new(&share_data);
            let share: Vec<F128> = Vec::<F128>::deserialize_compressed(&mut share_cursor)
                .expect("Failed to deserialize share");
            
            shares.push(share);
        }
        
        println!("Parsed {} secret shares from server", shares.len());
        self.input_shares = Some(shares);
    }

    fn aggregate(&mut self) {
        // get the input shares
        let shares = self.input_shares.as_ref()
            .expect("Must call retrieve_inputs before aggregate");
        
        if shares.is_empty() {
            return;
        }
        
        // determine the length of each share (all should be the same)
        let share_len = shares[0].len();
        
        // create the output share by summing corresponding elements
        let mut output_share = vec![F128::from(0u64); share_len];
        
        // iterate over each input share and add corresponding elements
        for share in shares {
            assert_eq!(share.len(), share_len, "All shares must have the same length");
            for (i, &value) in share.iter().enumerate() {
                output_share[i] = output_share[i] + value;
            }
        }
        
        println!("Aggregated {} shares into output share of length {}", shares.len(), output_share.len());
    }
}
