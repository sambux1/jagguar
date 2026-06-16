use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::crypto::prg::populate_random;
use crate::crypto::util::{matrix_vector_multiplication, round};

// Default parameters obtained via the lattice estimator.
// See /scripts/parameters/shprg_parameters.json for details.
const LAMBDA: usize = 3072;
/// SHPRG outer modulus: arithmetic in Z_{2^128}.
pub const OUTER_MODULUS_BITS: u32 = 128;
const DEFAULT_INNER_MODULUS_BITS: u32 = 92;

#[derive(Debug)]
pub struct SeedHomomorphicPRG {
    public_parameter: Vec<Vec<u128>>,
    seed: Vec<u128>,
    inner_modulus_bits: u32,
}

impl SeedHomomorphicPRG {
    pub fn new() -> Self {
        Self::new_with_modulus(
            Self::sample_public_parameter(4096, LAMBDA),
            Self::sample_seed(LAMBDA),
            DEFAULT_INNER_MODULUS_BITS,
        )
    }

    pub fn new_from_public_seed(seed: [u8; 32]) -> Self {
        Self::new_with_modulus(
            Self::expand_public_parameter(4096, LAMBDA, seed),
            Self::sample_seed(LAMBDA),
            DEFAULT_INNER_MODULUS_BITS,
        )
    }

    pub fn new_from_both_seeds(public_seed: [u8; 32], seed: Vec<u128>) -> Self {
        Self::new_with_modulus(
            Self::expand_public_parameter(4096, LAMBDA, public_seed),
            seed,
            DEFAULT_INNER_MODULUS_BITS,
        )
    }

    pub fn new_with_params(inner_modulus_bits: u32) -> Self {
        assert!(
            inner_modulus_bits < OUTER_MODULUS_BITS,
            "inner modulus must be smaller than outer"
        );
        let pp = Self::sample_public_parameter(4096, LAMBDA);
        let seed = Self::sample_seed(LAMBDA);
        Self::new_with_modulus(pp, seed, inner_modulus_bits)
    }

    fn new_with_modulus(
        public_parameter: Vec<Vec<u128>>,
        seed: Vec<u128>,
        inner_modulus_bits: u32,
    ) -> Self {
        assert!(
            inner_modulus_bits < OUTER_MODULUS_BITS,
            "inner modulus must be smaller than outer"
        );
        Self { public_parameter, seed, inner_modulus_bits }
    }

    pub fn expand(&self) -> Vec<u128> {
        let product = matrix_vector_multiplication(&self.public_parameter, &self.seed);
        round(product, OUTER_MODULUS_BITS - self.inner_modulus_bits)
    }

    fn sample_public_parameter(size0: usize, size1: usize) -> Vec<Vec<u128>> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut public_parameter = vec![vec![0u128; size1]; size0];
        for i in 0..size0 {
            populate_random(&mut public_parameter[i], &mut rng);
        }
        // return the random public parameter matrix
        public_parameter
    }

    fn expand_public_parameter(size0: usize, size1: usize, seed: [u8; 32]) -> Vec<Vec<u128>> {
        let mut rng = ChaCha20Rng::from_seed(seed);
        let mut public_parameter = vec![vec![0u128; size1]; size0];
        for i in 0..size0 {
            populate_random(&mut public_parameter[i], &mut rng);
        }
        // return the random public parameter matrix
        public_parameter
    }

    fn sample_seed(size: usize) -> Vec<u128> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut seed = vec![0u128; size];
        populate_random(&mut seed, &mut rng);
        // return the random seed
        seed
    }

    pub fn get_seed(&self) -> &Vec<u128> {
        &self.seed
    }

    pub fn get_public_parameter(&self) -> &Vec<Vec<u128>> {
        &self.public_parameter
    }

    pub fn outer_modulus_bits(&self) -> u32 {
        OUTER_MODULUS_BITS
    }

    pub fn inner_modulus_bits(&self) -> u32 {
        self.inner_modulus_bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // test that the almost homomorphic property holds with default parameters
    fn test_homomorphic() {
        let prg_0 = SeedHomomorphicPRG::new_from_public_seed([0u8; 32]);
        let prg_1 = SeedHomomorphicPRG::new_from_public_seed([0u8; 32]);
        let output_0 = prg_0.expand();
        let output_1 = prg_1.expand();
        let seed_0 = prg_0.get_seed();
        let seed_1 = prg_1.get_seed();

        // add the seeds together mod 2^128
        let homomorphic_seed: Vec<u128> = seed_0
            .iter()
            .zip(seed_1.iter())
            .map(|(&a, &b)| a.wrapping_add(b))
            .collect();

        let prg_sum = SeedHomomorphicPRG::new_from_both_seeds([0u8; 32], homomorphic_seed);
        let output_sum = prg_sum.expand();

        // output of the sum should equal sum of outputs mod 2^inner (up to rounding error of 1)
        let m = 1u128 << DEFAULT_INNER_MODULUS_BITS;
        for i in 0..output_0.len() {
            let o0 = output_0[i];
            let o1 = output_1[i];
            let o_sum = output_sum[i];
            let sum_mod = o0.wrapping_add(o1) % m;
            let delta = if sum_mod >= o_sum { sum_mod - o_sum } else { o_sum - sum_mod };
            let dist = delta.min(m - delta);
            assert!(dist <= 1);
        }
    }
}
