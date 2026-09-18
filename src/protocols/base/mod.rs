//! Role traits for a three-party aggregation protocol.
//!
//! A [`Protocol`] binds compatible [`Client`], [`Server`], and [`Committee`]
//! types that share the server's public state. This is the local-simulation
//! surface, not a network-neutral protocol core.

pub mod client;
pub mod committee;
pub mod messages;
pub mod protocol;
pub mod server;

pub use client::Client;
pub use committee::Committee;
pub use messages::{FromServer, ToServer};
pub use protocol::Protocol;
pub use server::Server;
