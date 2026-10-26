//! OPA payloads for [`ToServer`] and [`FromServer`].

use serde::{Deserialize, Serialize};

use crate::crypto::FieldBytes;
use crate::protocols::base::messages::{
    FromServer as GenericFromServer, ToServer as GenericToServer,
};
use crate::protocols::opa::state::OPAState;

/// One Shamir y-share per seed component.
pub type SeedShare = Vec<FieldBytes>;

/// Client ciphertext and per-committee seed shares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upload {
    /// Slots of `masked_input` that are real payload, before zero padding.
    pub payload_len: u32,
    /// Length [`super::client::OUTPUT_LEN`].
    pub masked_input: Vec<u128>,
    /// Indexed by committee member. Length `params.committee_size`.
    pub seed_shares: Vec<SeedShare>,
}

/// Per-client shares for one committee member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inputs {
    pub shares: Vec<SeedShare>,
}

/// Messages OPA parties send to the server.
pub type ToServer = GenericToServer<Upload, SeedShare>;
/// Server state, or one member's shares.
pub type FromServer = GenericFromServer<OPAState, Inputs>;
