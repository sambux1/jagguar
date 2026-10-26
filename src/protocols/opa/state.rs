//! OPA public parameters and state shared by all roles.

use std::sync::mpsc;

use crate::communicator::wire;
use crate::protocols::opa::client::NUM_PARTIES_UPPER_BOUND;
use crate::protocols::opa::messages::FromServer;
use crate::util::packing::unpack_vector;
use serde::{Deserialize, Serialize};

/// Public OPA parameters.
///
/// `security_parameter` is κ in the client encoding and corresponds to a statistical security
//  parameter. `reconstruction_threshold` and `committee_size` are Shamir `t` and `n`.
// `corruption_threshold` is stored but unused in this semi-honest implementation.
#[derive(Copy, Clone)]
pub struct OPASetupParameters {
    pub security_parameter: u64,
    pub corruption_threshold: u64,
    pub reconstruction_threshold: u64,
    pub committee_size: u64,
}

impl OPASetupParameters {
    /// κ, unused corruption threshold, Shamir `t`, and Shamir `n`.
    pub fn new(
        security_parameter: u64,
        corruption_threshold: u64,
        reconstruction_threshold: u64,
        committee_size: u64,
    ) -> Self {
        Self {
            security_parameter,
            corruption_threshold,
            reconstruction_threshold,
            committee_size,
        }
    }
}

/// Public OPA state distributed to clients and committee members.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OPAState {
    /// SHPRG public-parameter seed.
    pub succinct_seed: [u8; 32],
    /// Statistical security parameter κ.
    pub security_parameter: u64,
    /// Stored but unused in this semi-honest implementation.
    pub corruption_threshold: u64,
    /// Shamir reconstruction threshold `t`.
    pub reconstruction_threshold: u64,
    /// Shamir number of shares `n` (committee size).
    pub committee_size: u64,
    /// Committee local-port offsets `1 ..= committee_size`.
    pub committee_port_offsets: Vec<u16>,
    /// Server bind port (set when the communicator starts).
    pub port: u16,
    /// Optional channel for sending decoded output back to the simulator.
    #[serde(skip)]
    pub output_sender: Option<mpsc::Sender<Vec<u32>>>,
    /// Optional file descriptor for sending decoded output to the process manager.
    #[serde(skip)]
    pub output_fd: Option<i32>,
    /// Optional cache of messages used during aggregation.
    #[serde(skip)]
    pub client_messages: Vec<Vec<u8>>,
    /// Raw committee messages, filled before [`crate::protocols::opa::OPAServer::aggregate`].
    #[serde(skip)]
    pub committee_messages: Vec<Vec<u8>>,
}

impl OPAState {
    /// Decode a `FromServer::State` reply.
    pub fn from_reply(bytes: &[u8]) -> std::io::Result<Self> {
        match wire::decode::<FromServer>(bytes) {
            Ok(FromServer::State(state)) => Ok(state),
            Ok(_) => Err(std::io::Error::other("reply was not state")),
            Err(e) => Err(std::io::Error::other(e)),
        }
    }

    /// Decode an unmasked aggregate, treating every slot as payload.
    pub fn decode_output(&self, output: Vec<u128>) -> Vec<u32> {
        let len = output.len();
        self.decode_output_with_len(output, len)
    }

    /// Decode only the first `payload_len` ciphertext slots (excluding zero-padding to OUTPUT_LEN).
    pub fn decode_output_with_len(&self, output: Vec<u128>, payload_len: usize) -> Vec<u32> {
        let kappa: u32 = self.security_parameter as u32;
        let two_to_kappa: u128 = 1u128 << kappa;
        let two_to_kappa_times_n: u128 = two_to_kappa * (NUM_PARTIES_UPPER_BOUND as u128);

        let denom = two_to_kappa_times_n as u128;
        let decoded: Vec<u32> = output[..payload_len.min(output.len())]
            .iter()
            .map(|&x| {
                let q = x / denom;
                let r = x % denom;
                let ceil = q + if r != 0 { 1 } else { 0 };
                (ceil as u32) - 1
            })
            .collect();

        unpack_vector(&decoded)
    }
}
