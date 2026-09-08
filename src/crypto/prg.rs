//! Interface to ChaCha20.
//!
//! [`default_prg`] is the crate's entropy-seeded default. [`seeded_rng_at_word`]
//! seeks a ChaCha20 stream so [`super::SeedHomomorphicPRG`] can materialize one
//! public-matrix row at a time without storing the full matrix.

use ark_ff::UniformRand;
use ark_std::rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rand::distributions::{Distribution, Standard};

/// Entropy-seeded ChaCha20 RNG.
pub fn default_prg() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}

/// Fill `v` with uniform field elements from `rng`.
pub fn populate_random_field<F, R>(v: &mut [F], rng: &mut R)
where
    F: UniformRand,
    R: Rng + ?Sized,
{
    for x in v.iter_mut() {
        *x = F::rand(rng);
    }
}

/// Fill `v` with uniform native integers from `rng` (the full range of `T`).
pub fn populate_random<T, R>(v: &mut [T], rng: &mut R)
where
    Standard: Distribution<T>,
    R: Rng + ?Sized,
{
    for x in v.iter_mut() {
        *x = rng.r#gen();
    }
}

/// Fill `v` with uniform bytes from `rng`.
pub fn populate_random_bytes<R: Rng + ?Sized>(v: &mut [u8], rng: &mut R) {
    rng.fill(v);
}

/// ChaCha20 RNG from a 32-byte seed, seeked to a 32-bit word position.
///
/// Used to expand the SHPRG public matrix without materializing it: row `i`
/// starts at `word_pos = i * row_len * words_per_element`.
pub fn seeded_rng_at_word(seed: [u8; 32], word_pos: u128) -> ChaCha20Rng {
    let mut rng = ChaCha20Rng::from_seed(seed);
    rng.set_word_pos(word_pos);
    rng
}
