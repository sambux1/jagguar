//! Public OPA parameters that are safe to hand to clients.

use serde::{Deserialize, Serialize};

/// Public crypto parameters for one OPA run.
///
/// This is the client-visible subset of [`super::server::OPAState`]: transport
/// ports, simulator channels, and queued messages are omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicParams {
    /// SHPRG public-parameter seed.
    pub succinct_seed: [u8; 32],
    /// Statistical security parameter κ.
    pub security_parameter: u64,
    /// Stored but unused in this semi-honest implementation.
    pub corruption_threshold: u64,
    /// Shamir reconstruction threshold `t`.
    pub reconstruction_threshold: u64,
    /// Shamir number of shares `n` (committee size).
    pub committee_size: u16,
    /// Ciphertext length. Matches [`super::client::OUTPUT_LEN`] today.
    pub output_len: u32,
}
