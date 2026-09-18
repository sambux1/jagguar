//! Messages every single-server aggregation protocol sends.
//!
//! Parties always initiate. The variants are shared; the payload types are not.

use serde::{Deserialize, Serialize};

use crate::communicator::wire::{ClientId, CommitteeIndex};

/// Anything a party sends to the aggregation server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToServer<Upload, Report> {
    RequestState,
    ClientUpload { from: ClientId, data: Upload },
    RequestInputs { index: CommitteeIndex },
    CommitteeUpload { index: CommitteeIndex, data: Report },
}

/// Replies to [`ToServer::RequestState`] and [`ToServer::RequestInputs`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FromServer<State, Inputs> {
    State(State),
    Inputs(Inputs),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::communicator::wire::{decode, encode};

    #[test]
    fn to_server_round_trip() {
        let msg: ToServer<u32, u32> = ToServer::ClientUpload {
            from: ClientId(1),
            data: 7,
        };
        let back = decode(&encode(&msg).unwrap()).unwrap();
        assert_eq!(msg, back);
    }
}
