use std::io::{Cursor, Read, Write};

use crate::protocols::committee::Committee;
use crate::protocols::opa::server::OPAState;
use crate::communicator::Communicator;
use crate::crypto::{F128, util::field_to_128};

pub struct OPACommittee {
    server_state: Option<OPAState>,
    communicator: Communicator,
    input_shares: Option<Vec<Vec<u128>>>,
    output_share: Option<Vec<u128>>,
}

impl Committee for OPACommittee {
    type ServerState = OPAState;

    fn new(port: u16) -> Self {
        let communicator = Communicator::new(port);

        Self {
            server_state: None,
            communicator,
            input_shares: None,
            output_share: None,
        }
    }

    fn set_server_state(&mut self, state: Self::ServerState) {
        self.server_state = Some(state.clone());
    }

    fn retrieve_inputs(&mut self) {
        let server_port = self.server_state.as_ref().unwrap().port;
        let inputs = self.communicator.receive_from_server(server_port)
            .expect("Failed to receive inputs from server");

        // Format from send_on_stream: [byte_len u32][byte_len bytes of u128 LE]...
        let mut cursor = Cursor::new(&inputs);
        let mut shares = Vec::new();
        
        while cursor.position() < inputs.len() as u64 {
            let mut len_bytes = [0u8; 4];
            cursor.read_exact(&mut len_bytes)
                .expect("Failed to read length prefix");
            let byte_len = u32::from_le_bytes(len_bytes) as usize;
            let share_len = byte_len / 16;
            
            let mut share = Vec::with_capacity(share_len);
            for _ in 0..share_len {
                let mut xbuf = [0u8; 16];
                cursor.read_exact(&mut xbuf)
                    .expect("Failed to read share element");
                share.push(u128::from_le_bytes(xbuf));
            }
            shares.push(share);
        }
        
        println!("Parsed {} secret shares from server", shares.len());
        self.input_shares = Some(shares);
    }

    fn aggregate(&mut self) {
        let shares = self.input_shares.as_ref()
            .expect("Must call retrieve_inputs before aggregate");
        
        if shares.is_empty() {
            return;
        }
        
        let share_len = shares[0].len();
        
        let mut output_share = vec![0u128; share_len];
        
        for share in shares {
            assert_eq!(share.len(), share_len, "All shares must have the same length");
            for (i, &value) in share.iter().enumerate() {
                output_share[i] = field_to_128(F128::from(output_share[i]) + F128::from(value));
            }
        }

        self.output_share = Some(output_share.clone());
        
        println!("Aggregated {} shares into output share of length {}", shares.len(), output_share.len());
    }

    fn send_output(&mut self) {
        let mut data = Vec::new();
        data.extend_from_slice(b"committee");

        let server_port = self.server_state.as_ref().unwrap().port;
        let local_port = self.communicator.port();
        let committee_index: u16 = local_port
            .checked_sub(server_port + 1)
            .expect("Invalid committee port configuration") as u16;
        data.extend_from_slice(&committee_index.to_le_bytes());
        
        let share = self.output_share
            .as_ref()
            .expect("Must call aggregate before send_output");
        // serialize: length (u32) followed by u128 LE elements
        data.write_all(&(share.len() as u32).to_le_bytes()).unwrap();
        for &x in share {
            data.write_all(&x.to_le_bytes()).unwrap();
        }
        
        self.communicator.send_to_server(self.server_state.as_ref().unwrap().port, &data)
            .expect("Failed to send output share to server");
    }
}
