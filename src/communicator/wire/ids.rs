//! Party identifiers. These replace TCP source ports as identity.
//!
//! A [`CommitteeIndex`] is not a [`ClientId`], so the two cannot be mixed.

use serde::{Deserialize, Serialize};

/// Input party in one aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);

/// Committee member index in `0..n`.
///
/// `index` is private so Shamir x-coordinates go through [`Self::shamir_x`]
/// in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CommitteeIndex {
    index: u64,
}

impl CommitteeIndex {
    /// Index `i` in `0..n`.
    pub fn new(index: u64) -> Self {
        Self { index }
    }

    /// Seat number as `usize` for slice lookup.
    pub fn as_usize(self) -> usize {
        self.index as usize
    }

    /// Shamir x-coordinate. `x = 0` is the secret, so members start at 1.
    pub fn shamir_x(self) -> u64 {
        self.index + 1
    }
}
