use ark_ff::{Fp, MontBackend, MontConfig};

pub mod prg;
pub mod seed_homomorphic_prg;
pub mod shamir;
pub mod util;

pub use seed_homomorphic_prg::{SeedHomomorphicPRG, OUTER_MODULUS_BITS};
pub use shamir::Shamir;
pub use util::{field_from_bytes, field_low_u128, field_to_bytes};

/// Byte length for serializing field elements on the wire.
pub const FIELD_ELEMENT_BYTES: usize = 32;
pub type FieldBytes = [u8; FIELD_ELEMENT_BYTES];

// 255-bit prime field (2^255 - 19) used by Shamir secret sharing in OPA.
#[derive(MontConfig)]
#[modulus = "57896044618658097711785492504343953926634992332820282019728792003956564819949"]
#[generator = "2"]
pub struct F256Config;
pub type F256 = Fp<MontBackend<F256Config, 4>, 4>;
