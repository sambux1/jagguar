//! Aggregation protocol roles and instantiations.
//!
//! [`Client`], [`Server`], [`Committee`], and [`Protocol`] are the localhost
//! simulation API: methods take TCP ports and a
//! [`crate::communicator::Communicator`]. [`opa`] is the current instantiation
//! ([OPA](https://eprint.iacr.org/2024/723.pdf), semi-honest).

pub mod opa;

pub mod base;

pub use base::client as client;
pub use base::server as server;
pub use base::committee as committee;

pub use base::client::Client;
pub use base::server::Server;
pub use base::committee::Committee;

pub use base::protocol::Protocol;
