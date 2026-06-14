use ark_ff::UniformRand;
use ark_std::rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rand::distributions::{Distribution, Standard};

// Construct a secure, stream-oriented RNG seeded from system entropy.
// Centralizes the project's default RNG choice.
pub fn default_prg() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}

// Generic over any field `F` that implements `UniformRand`.
pub fn populate_random_field<F, R>(v: &mut [F], rng: &mut R)
where
    F: UniformRand,
    R: Rng + ?Sized,
{
    for x in v.iter_mut() {
        *x = F::rand(rng);
    }
}

// Generic over any native integer type T that `rng.gen()` supports (u8-u128, etc.)
pub fn populate_random<T, R>(v: &mut [T], rng: &mut R)
where
    Standard: Distribution<T>,
    R: Rng + ?Sized,
{
    for x in v.iter_mut() {
        *x = rng.r#gen();
    }
}

// Populate a vector of bytes with random bytes
pub fn populate_random_bytes<R: Rng + ?Sized>(v: &mut [u8], rng: &mut R) {
    rng.fill(v);
}