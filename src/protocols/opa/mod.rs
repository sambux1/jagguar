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
pub mod params;
pub mod server;

pub use client::OPAClient;
pub use committee::OPACommittee;
pub use messages::{FromServer, Inputs, SeedShare, ToServer, Upload};
pub use params::PublicParams;
pub use server::{OPAServer, OPASetupParameters};

// instantiate the OPA protocol
use crate::protocols::Protocol;
use core::marker::PhantomData;
use num_traits::FromPrimitive;

/// Protocol marker binding [`OPAClient`], [`OPAServer`], and [`OPACommittee`].
pub struct OPA<T>(pub PhantomData<T>);

impl<T> Protocol for OPA<T>
where
    T: Copy + Into<u32> + FromPrimitive,
{
    type Input = T;
    type Server = OPAServer;
    type Client = OPAClient<T>;
    type Committee = OPACommittee;
}
