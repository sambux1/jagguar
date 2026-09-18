//! Localhost TCP transport for the simulator.
//!
//! Parties bind a port and connect to `127.0.0.1`. Incoming bytes are
//! classified by a prefix: `signal` (6 bytes) opens a callback stream;
//! `committee` is queued until the expected committee size is reached;
//! anything else is a client payload. There is no authentication.
//!
//! [`wire`] holds party IDs and the codec used by typed messages.

pub mod communicator;
pub mod wire;
pub use communicator::Communicator;
