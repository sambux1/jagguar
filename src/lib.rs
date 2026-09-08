//! Single-server secure aggregation.
//!
//! Research implementation currently for local simulation. Not a general-purpose
//! cryptography library and not externally audited.
//!
//! - [`crypto`]: SHPRG, Shamir, ChaCha
//! - [`protocols`]: role traits and semi-honest [`protocols::opa`]
//! - [`communicator`]: localhost TCP
//! - [`util`]: integer packing
//!
//! A multi-party localhost run is `cargo simulator` (`--features simulator`).

pub mod crypto;
pub mod util;
pub mod protocols;
pub mod communicator;

#[cfg(feature = "simulator")]
pub mod simulator;