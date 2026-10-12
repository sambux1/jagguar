//! Localhost HTTP transport for the simulator.
//!
//! HTTP carries `GET /state`, `POST /client-upload`, `POST /committee-upload`,
//! and `POST /committee-fetch`. Payloads are opaque bytes. There is no
//! authentication.
//!
//! [`wire`] holds party IDs and the codec used by typed messages.

pub mod communicator;
pub mod http;
pub mod wire;
pub use communicator::Communicator;
