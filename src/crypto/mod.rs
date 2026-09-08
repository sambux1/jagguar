//! Cryptographic primitives used by Jagguar protocols.
//!
//! This module a set of cryptographic primitives. It is not a general-purpose cryptography
//! library and has not been externally audited.
//!
//! # Contents
//!
//! - [`SeedHomomorphicPRG`]: almost seed-homomorphic PRG
//!   ([Boneh, Lewi, Montgomery, and Raghunathan](https://eprint.iacr.org/2015/220.pdf)).
//!   Arithmetic is in `Z_{2^128}`; default inner modulus is `2^92` with seed
//!   length λ = 3072, chosen via the lattice estimator (`scripts/parameters/`).
//!   Expansion is almost homomorphic in the seed: `Expand(s0 + s1)` is within
//!   rounding error 1 of `Expand(s0) + Expand(s1)` in the inner modulus.
//! - [`Shamir`]: Shamir secret sharing over a prime field. OPA uses [`F256`].
//! - [`prg`]: ChaCha20 RNG helpers, including seekable expansion of the SHPRG
//!   public matrix.
//! - Field helpers: [`field_to_bytes`], [`field_from_bytes`], [`field_low_u128`].

use ark_ff::{Fp, MontBackend, MontConfig};

pub mod prg;
pub mod seed_homomorphic_prg;
pub mod shamir;
pub mod util;

pub use seed_homomorphic_prg::{OUTER_MODULUS_BITS, SeedHomomorphicPRG};
pub use shamir::Shamir;
pub use util::{field_from_bytes, field_low_u128, field_to_bytes};

/// Byte length for serializing field elements on the wire.
///
/// Matches the little-endian encoding of [`F256`] (four 64-bit limbs).
pub const FIELD_ELEMENT_BYTES: usize = 32;

/// Fixed-width little-endian encoding of a field element.
pub type FieldBytes = [u8; FIELD_ELEMENT_BYTES];

/// Montgomery configuration for [`F256`].
#[derive(MontConfig)]
#[modulus = "57896044618658097711785492504343953926634992332820282019728792003956564819949"]
#[generator = "2"]
pub struct F256Config;

/// 255-bit prime field `2^255 - 19`, used by Shamir secret sharing in OPA.
pub type F256 = Fp<MontBackend<F256Config, 4>, 4>;
