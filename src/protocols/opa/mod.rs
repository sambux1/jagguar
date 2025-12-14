pub mod client;
pub mod server;
pub mod committee;

pub use client::OPAClient;
pub use server::{OPAServer, OPASetupParameters};
pub use committee::OPACommittee;


// instantiate the OPA protocol
use crate::protocols::Protocol;
use num_traits::FromPrimitive;
use core::marker::PhantomData;

// marker tying together the OPA server, client, and committee under a single Protocol
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