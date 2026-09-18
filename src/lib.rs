//! Single-server secure aggregation.
//!
//! Research implementation currently for local simulation. Not a general-purpose
//! cryptography library and not externally audited.
//!
//! - [`crypto`]: SHPRG, Shamir, ChaCha
//! - [`protocols`]: role traits and semi-honest [`protocols::opa`]
//! - [`communicator`]: localhost TCP and [`communicator::wire`]
//! - [`util`]: integer packing
//!
//! A multi-party localhost run is `cargo simulator` (`--features simulator`).

pub mod communicator;
pub mod crypto;
pub mod protocols;
pub mod util;

#[cfg(feature = "simulator")]
pub mod simulator;
