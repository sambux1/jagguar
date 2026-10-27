//! Aggregation protocol roles and instantiations.
//!
//! [`Client`], [`Server`], [`Committee`], and [`Protocol`] are the localhost
//! simulation API: methods take a port and a
//! [`crate::communicator::Communicator`]. [`opa`] is the current instantiation
//! ([OPA](https://eprint.iacr.org/2024/723.pdf), semi-honest).

pub mod opa;

pub mod base;

pub use base::client;
pub use base::committee;
#[cfg(feature = "native")]
pub use base::server;

pub use base::client::Client;
pub use base::committee::Committee;
#[cfg(feature = "native")]
pub use base::server::Server;

#[cfg(feature = "native")]
pub use base::protocol::Protocol;
