//! OPA committee member.

use std::io::{Cursor, Read, Write};

use crate::communicator::Communicator;
use crate::crypto::{F256, FIELD_ELEMENT_BYTES, FieldBytes, field_from_bytes, field_to_bytes};
use crate::protocols::committee::Committee;
use crate::protocols::opa::server::OPAState;

/// Committee member: sum Shamir y-shares over [`crate::crypto::F256`] and return
/// the result.
pub struct OPACommittee {
    server_state: Option<OPAState>,
    communicator: Communicator,
    input_shares: Option<Vec<Vec<FieldBytes>>>,
    output_share: Option<Vec<FieldBytes>>,
}

impl Committee for OPACommittee {
    type ServerState = OPAState;

    /// Bind this member to local `port`.
    fn new(port: u16) -> Self {
        let communicator = Communicator::new(port);

        Self {
            server_state: None,
            communicator,
            input_shares: None,
            output_share: None,
        }
    }

    fn request_state(&mut self) -> std::io::Result<()> {
        self.server_state = Some(OPAState::from_reply(&self.communicator.request_state()?)?);
        Ok(())
    }

    /// Store the server's [`OPAState`].
    fn set_server_state(&mut self, state: Self::ServerState) {
        self.server_state = Some(state);
    }

    /// Fetch this member's shares with `POST /committee-fetch` and parse them.
    ///
    /// The body is this member's index. The response is length-prefixed shares:
    /// `[byte_len u32][byte_len bytes of field elements LE]...`.
    fn retrieve_inputs(&mut self) {
        let committee_index = self.committee_index();
        let inputs = self
            .communicator
            .fetch_committee_inputs(&committee_index.to_le_bytes())
            .expect("Failed to receive inputs from server");

        let mut cursor = Cursor::new(&inputs);
        let mut shares = Vec::new();

        while cursor.position() < inputs.len() as u64 {
            let mut len_bytes = [0u8; 4];
            cursor
                .read_exact(&mut len_bytes)
                .expect("Failed to read length prefix");
            let byte_len = u32::from_le_bytes(len_bytes) as usize;
            let share_len = byte_len / FIELD_ELEMENT_BYTES;

            let mut share = Vec::with_capacity(share_len);
            for _ in 0..share_len {
                let mut xbuf = [0u8; FIELD_ELEMENT_BYTES];
                cursor
                    .read_exact(&mut xbuf)
                    .expect("Failed to read share element");
                share.push(xbuf);
            }
            shares.push(share);
        }

        println!("Parsed {} secret shares from server", shares.len());
        self.input_shares = Some(shares);
    }

    /// Sum shares componentwise in [`crate::crypto::F256`].
    ///
    /// # Panics
    ///
    /// Panics if [`Self::retrieve_inputs`] has not been called, or if share
    /// lengths differ.
    fn aggregate(&mut self) {
        let shares = self
            .input_shares
            .as_ref()
            .expect("Must call retrieve_inputs before aggregate");

        if shares.is_empty() {
            return;
        }

        let share_len = shares[0].len();

        let mut output_share = vec![[0u8; FIELD_ELEMENT_BYTES]; share_len];

        for share in shares {
            assert_eq!(
                share.len(),
                share_len,
                "All shares must have the same length"
            );
            for (i, value) in share.iter().enumerate() {
                let sum =
                    field_from_bytes::<F256>(&output_share[i]) + field_from_bytes::<F256>(value);
                output_share[i] = field_to_bytes(sum);
            }
        }

        self.output_share = Some(output_share.clone());

        println!(
            "Aggregated {} shares into output share of length {}",
            shares.len(),
            output_share.len()
        );
    }

    /// Send the summed share. Committee index is `local_port - server_port - 1`.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::aggregate`] has not been called, or if the port
    /// layout is not `server_port + 1 + index`.
    fn send_output(&mut self) {
        let mut data = Vec::new();
        data.extend_from_slice(b"committee");

        let committee_index = self.committee_index();
        data.extend_from_slice(&committee_index.to_le_bytes());

        let share = self
            .output_share
            .as_ref()
            .expect("Must call aggregate before send_output");
        // serialize: length (u32) followed by 32-byte field elements
        data.write_all(&(share.len() as u32).to_le_bytes()).unwrap();
        for x in share {
            data.write_all(x).unwrap();
        }

        self.communicator
            .send_committee_output(&data)
            .expect("Failed to send output share to server");
    }
}

impl OPACommittee {
    /// `local_port - server_port - 1`.
    ///
    /// # Panics
    ///
    /// Panics if the port layout is not `server_port + 1 + index`.
    fn committee_index(&self) -> u16 {
        let server_port = self.server_state.as_ref().unwrap().port;
        let local_port = self.communicator.port();
        local_port
            .checked_sub(server_port + 1)
            .expect("Invalid committee port configuration") as u16
    }
}
