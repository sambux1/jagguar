//! Semi-honest one-shot private aggregation.
//!
//! Instantiates the seed-homomorphic PRG and Shamir flavor of
//! [Karthikeyan and Polychroniadou](https://eprint.iacr.org/2024/723.pdf).
//! Clients speak once: they mask with [`crate::crypto::SeedHomomorphicPRG`]
//! and share the seed to a committee. The committee sums shares; the server
//! reconstructs the aggregate seed, unmasks, and decodes.

pub mod client;
pub mod committee;
pub mod messages;
#[cfg(feature = "native")]
pub mod server;
pub mod state;

pub use client::OPAClient;
pub use committee::OPACommittee;
pub use messages::{FromServer, Inputs, SeedShare, ToServer, Upload};
#[cfg(feature = "native")]
pub use server::OPAServer;
pub use state::{OPASetupParameters, OPAState};

// instantiate the OPA protocol
#[cfg(feature = "native")]
use crate::protocols::Protocol;
#[cfg(feature = "native")]
use core::marker::PhantomData;
#[cfg(feature = "native")]
use num_traits::FromPrimitive;

/// Protocol marker binding [`OPAClient`], [`OPAServer`], and [`OPACommittee`].
#[cfg(feature = "native")]
pub struct OPA<T>(pub PhantomData<T>);

#[cfg(feature = "native")]
impl<T> Protocol for OPA<T>
where
    T: Copy + Into<u32> + FromPrimitive,
{
    type Input = T;
    type Server = OPAServer;
    type Client = OPAClient<T>;
    type Committee = OPACommittee;
}
