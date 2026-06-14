use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::crypto::prg::populate_random;
use crate::crypto::util::{matrix_vector_multiplication, round};

// Default parameters obtained via the lattice estimator.
// See /scripts/parameters/shprg_parameters.json for details.
const LAMBDA: usize = 3072;
// Outer modulus 2^122: must satisfy (25 clients * 2^outer) < 2^127-1 so F128
// Shamir reconstruction of summed seeds matches SHPRG addition mod 2^outer.
const DEFAULT_OUTER_MODULUS_BITS: u32 = 122;
const DEFAULT_INNER_MODULUS_BITS: u32 = 92;

#[derive(Debug)]
pub struct SeedHomomorphicPRG {
    public_parameter: Vec<Vec<u128>>,
    seed: Vec<u128>,
    // outer modulus is 2^outer_modulus_bits; must be <= 128
    outer_modulus_bits: u32,
    // inner modulus is 2^inner_modulus_bits; must be < outer_modulus_bits
    inner_modulus_bits: u32,
}

impl SeedHomomorphicPRG {
    pub fn new() -> Self {
        Self::new_with_moduli(
            Self::sample_public_parameter(4096, LAMBDA),
            Self::sample_seed(LAMBDA),
            DEFAULT_OUTER_MODULUS_BITS,
            DEFAULT_INNER_MODULUS_BITS,
        )
    }

    pub fn new_from_public_seed(seed: [u8; 32]) -> Self {
        Self::new_with_moduli(
            Self::expand_public_parameter(4096, LAMBDA, seed),
            Self::sample_seed(LAMBDA),
            DEFAULT_OUTER_MODULUS_BITS,
            DEFAULT_INNER_MODULUS_BITS,
        )
    }

    pub fn new_from_both_seeds(public_seed: [u8; 32], seed: Vec<u128>) -> Self {
        Self::new_with_moduli(
            Self::expand_public_parameter(4096, LAMBDA, public_seed),
            seed,
            DEFAULT_OUTER_MODULUS_BITS,
            DEFAULT_INNER_MODULUS_BITS,
        )
    }

    // --- parameterized constructors ---

    pub fn new_with_params(outer_modulus_bits: u32, inner_modulus_bits: u32) -> Self {
        assert!(outer_modulus_bits <= 128, "outer modulus must be <= 2^128");
        assert!(inner_modulus_bits < outer_modulus_bits,
            "inner modulus must be smaller than outer");
        let pp = Self::sample_public_parameter(4096, LAMBDA);
        let seed = Self::sample_seed(LAMBDA);
        Self::new_with_moduli(pp, seed, outer_modulus_bits, inner_modulus_bits)
    }

    fn new_with_moduli(
        mut public_parameter: Vec<Vec<u128>>,
        mut seed: Vec<u128>,
        outer_modulus_bits: u32,
        inner_modulus_bits: u32,
    ) -> Self {
        // When outer_modulus_bits < 128, mask entries down to the outer modulus.
        if outer_modulus_bits < 128 {
            let mask = (1u128 << outer_modulus_bits).wrapping_sub(1);
            for row in public_parameter.iter_mut() {
                for x in row.iter_mut() {
                    *x &= mask;
                }
            }
            for x in seed.iter_mut() {
                *x &= mask;
            }
        }
        Self { public_parameter, seed, outer_modulus_bits, inner_modulus_bits }
    }

    pub fn expand(&self) -> Vec<u128> {
        let product = matrix_vector_multiplication(&self.public_parameter, &self.seed);
        // mask down to the outer modulus if it is smaller than 2^128
        let product = if self.outer_modulus_bits < 128 {
            let mask = (1u128 << self.outer_modulus_bits).wrapping_sub(1);
            product.into_iter().map(|x| x & mask).collect()
        } else {
            product
        };
        let round_shift = self.outer_modulus_bits - self.inner_modulus_bits;
        round(product, round_shift)
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
        self.outer_modulus_bits
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
        // generate two seed homomorphic PRGs with the same public parameter matrix
        let prg_0 = SeedHomomorphicPRG::new_from_public_seed([0u8; 32]);
        let prg_1 = SeedHomomorphicPRG::new_from_public_seed([0u8; 32]);
        // expand the PRGs
        let output_0 = prg_0.expand();
        let output_1 = prg_1.expand();
        // get the seeds
        let seed_0 = prg_0.get_seed();
        let seed_1 = prg_1.get_seed();

        // add the seeds together mod 2^outer
        let mask_outer = (1u128 << DEFAULT_OUTER_MODULUS_BITS).wrapping_sub(1);
        let mut homomorphic_seed = Vec::with_capacity(seed_0.len());
        for i in 0..seed_0.len() {
            homomorphic_seed.push((seed_0[i].wrapping_add(seed_1[i])) & mask_outer);
        }

        // create a new PRG from the homomorphic seed
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
