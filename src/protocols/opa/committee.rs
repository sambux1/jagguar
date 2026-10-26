//! OPA committee member.

use crate::communicator::{DefaultTransport, Transport};
use crate::communicator::wire::{CommitteeIndex, decode, encode};
use crate::crypto::{F256, FIELD_ELEMENT_BYTES, FieldBytes, field_from_bytes, field_to_bytes};
use crate::protocols::committee::Committee;
use crate::protocols::opa::messages::{FromServer, ToServer};
use crate::protocols::opa::state::OPAState;

/// Committee member: sum Shamir y-shares over [`crate::crypto::F256`] and return
/// the result.
pub struct OPACommittee<C: Transport = DefaultTransport> {
    port: u16,
    server_state: Option<OPAState>,
    transport: C,
    input_shares: Option<Vec<Vec<FieldBytes>>>,
    output_share: Option<Vec<FieldBytes>>,
}

impl<C> Committee for OPACommittee<C>
where
    C: Transport + Default,
{
    type ServerState = OPAState;

    /// Bind this member to local `port`.
    fn new(port: u16) -> Self {
        Self {
            port,
            server_state: None,
            transport: C::default(),
            input_shares: None,
            output_share: None,
        }
    }

    async fn request_state(&mut self) -> std::io::Result<()> {
        let state_bytes = self.transport.request_state().await?;
        self.server_state = Some(OPAState::from_reply(&state_bytes)?);
        Ok(())
    }

    /// Store the server's [`OPAState`].
    fn set_server_state(&mut self, state: Self::ServerState) {
        self.server_state = Some(state);
    }

    /// Fetch this member's shares with `POST /committee-fetch`.
    ///
    /// The body is [`ToServer::RequestInputs`]. The response is [`FromServer::Inputs`].
    async fn retrieve_inputs(&mut self) {
        let request = ToServer::RequestInputs {
            index: self.committee_index(),
        };
        let request_bytes = encode(&request).expect("encode committee fetch");
        let reply = self
            .transport
            .fetch_committee_inputs(&request_bytes)
            .await
            .expect("Failed to receive inputs from server");
        let FromServer::Inputs(inputs) = decode(&reply).expect("decode committee inputs") else {
            panic!("committee fetch reply was not inputs");
        };

        println!("Parsed {} secret shares from server", inputs.shares.len());
        self.input_shares = Some(inputs.shares);
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

    /// Send the summed share as [`ToServer::CommitteeUpload`].
    ///
    /// # Panics
    ///
    /// Panics if [`Self::aggregate`] has not been called, or if the port
    /// layout is not `server_port + 1 + index`.
    async fn send_output(&mut self) {
        let share = self
            .output_share
            .clone()
            .expect("Must call aggregate before send_output");
        let message = ToServer::CommitteeUpload {
            index: self.committee_index(),
            data: share,
        };
        let data = encode(&message).expect("encode committee output");
        self.transport
            .send_committee_output(&data)
            .await
            .expect("Failed to send output share to server");
    }
}

impl<C: Transport> OPACommittee<C> {
    /// `local_port - server_port - 1`.
    ///
    /// # Panics
    ///
    /// Panics if the port layout is not `server_port + 1 + index`.
    fn committee_index(&self) -> CommitteeIndex {
        let server_port = self.server_state.as_ref().unwrap().port;
        let index = self
            .port
            .checked_sub(server_port + 1)
            .expect("Invalid committee port configuration") as u64;
        CommitteeIndex::new(index)
    }
}
