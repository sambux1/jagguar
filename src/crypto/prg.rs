use ark_ff::UniformRand;
use ark_std::rand::Rng;

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

