//! Localhost multi-party simulator.
//!
//! Spawns a [`Protocol`](crate::protocols::Protocol) server, clients, and
//! committee on localhost (ports from 10000). Behind the `simulator`
//! feature. Run with `cargo simulator`.

pub mod port_pool;
pub mod simulator;

pub use port_pool::PortPool;
pub use simulator::Simulator;
