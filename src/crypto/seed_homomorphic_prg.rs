use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::crypto::prg::{populate_random, populate_random_bytes, seeded_rng_at_word};
use crate::crypto::util::dot_product;

// Default parameters obtained via the lattice estimator.
// See /scripts/parameters/shprg_parameters.json for details.
const LAMBDA: usize = 3072;
/// SHPRG outer modulus: arithmetic in Z_{2^128}.
pub const OUTER_MODULUS_BITS: u32 = 128;
const DEFAULT_INNER_MODULUS_BITS: u32 = 92;
// Each u128 occupies 4 × 32-bit ChaCha words.
const WORDS_PER_U128: u128 = 4;

#[derive(Debug)]
pub struct SeedHomomorphicPRG {
    public_parameter_seed: [u8; 32],
    seed: Vec<u128>,
    inner_modulus_bits: u32,
}

impl SeedHomomorphicPRG {
    pub fn new() -> Self {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut public_parameter_seed = [0u8; 32];
        populate_random_bytes(&mut public_parameter_seed, &mut rng);
        Self {
            public_parameter_seed,
            seed: Self::sample_seed(LAMBDA),
            inner_modulus_bits: DEFAULT_INNER_MODULUS_BITS,
        }
    }

    pub fn new_from_public_seed(public_parameter_seed: [u8; 32]) -> Self {
        Self {
            public_parameter_seed,
            seed: Self::sample_seed(LAMBDA),
            inner_modulus_bits: DEFAULT_INNER_MODULUS_BITS,
        }
    }

    pub fn new_from_both_seeds(public_parameter_seed: [u8; 32], seed: Vec<u128>) -> Self {
        Self {
            public_parameter_seed,
            seed,
            inner_modulus_bits: DEFAULT_INNER_MODULUS_BITS,
        }
    }

    pub fn new_with_params(inner_modulus_bits: u32) -> Self {
        assert!(
            inner_modulus_bits < OUTER_MODULUS_BITS,
            "inner modulus must be smaller than outer"
        );
        let mut rng = ChaCha20Rng::from_entropy();
        let mut public_parameter_seed = [0u8; 32];
        populate_random_bytes(&mut public_parameter_seed, &mut rng);
        Self {
            public_parameter_seed,
            seed: Self::sample_seed(LAMBDA),
            inner_modulus_bits,
        }
    }

    /// Expand to an arbitrary number of output elements.
    /// Materializes one row of the public matrix at a time, keeping memory at O(λ).
    pub fn expand(&self, n: usize) -> Vec<u128> {
        let shift = OUTER_MODULUS_BITS - self.inner_modulus_bits;
        let mut output = Vec::with_capacity(n);
        let mut row = vec![0u128; LAMBDA];
        for i in 0..n {
            let word_pos = (i as u128) * (LAMBDA as u128) * WORDS_PER_U128;
            let mut row_rng = seeded_rng_at_word(self.public_parameter_seed, word_pos);
            populate_random(&mut row, &mut row_rng);
            output.push(dot_product(&row, &self.seed) >> shift);
        }
        output
    }

    fn sample_seed(size: usize) -> Vec<u128> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut seed = vec![0u128; size];
        populate_random(&mut seed, &mut rng);
        seed
    }

    pub fn get_seed(&self) -> &Vec<u128> {
        &self.seed
    }

    pub fn get_public_parameter_seed(&self) -> [u8; 32] {
        self.public_parameter_seed
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
        let output_0 = prg_0.expand(4096);
        let output_1 = prg_1.expand(4096);
        let seed_0 = prg_0.get_seed();
        let seed_1 = prg_1.get_seed();

        // add the seeds together mod 2^128
        let homomorphic_seed: Vec<u128> = seed_0
            .iter()
            .zip(seed_1.iter())
            .map(|(&a, &b)| a.wrapping_add(b))
            .collect();

        let prg_sum = SeedHomomorphicPRG::new_from_both_seeds([0u8; 32], homomorphic_seed);
        let output_sum = prg_sum.expand(4096);

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
