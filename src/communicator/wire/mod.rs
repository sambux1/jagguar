//! Party IDs and the codec for typed messages.
//!
//! Payload types live with their protocol (for example [`crate::protocols::opa`]).

pub mod codec;
pub mod ids;

pub use codec::{WireError, decode, encode};
pub use ids::{ClientId, CommitteeIndex};
