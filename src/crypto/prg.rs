use ark_ff::UniformRand;
use ark_std::rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

// Construct a secure, stream-oriented RNG seeded from system entropy.
// Centralizes the project's default RNG choice.
pub fn default_prg() -> ChaCha20Rng {
    ChaCha20Rng::from_entropy()
}

// Generic over any field `F` that implements `UniformRand`.
pub fn populate_random<F, R>(v: &mut [F], rng: &mut R)
where
    F: UniformRand,
    R: Rng + ?Sized,
{
    for x in v.iter_mut() {
        *x = F::rand(rng);
    }
}

// Populate a vector of bytes with random bytes
pub fn populate_random_bytes<R: Rng + ?Sized>(v: &mut [u8], rng: &mut R) {
    rng.fill(v);
}