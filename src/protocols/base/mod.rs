//! Role traits for a three-party aggregation protocol.
//!
//! A [`Protocol`] binds compatible [`Client`], [`Server`], and [`Committee`]
//! types that share the server's public state. This is the local-simulation
//! surface, not a network-neutral protocol core.

pub mod client;
pub mod committee;
pub mod messages;
#[cfg(feature = "native")]
pub mod protocol;
#[cfg(feature = "native")]
pub mod server;

pub use client::Client;
pub use committee::Committee;
pub use messages::{FromServer, ToServer};
#[cfg(feature = "native")]
pub use protocol::Protocol;
#[cfg(feature = "native")]
pub use server::Server;
