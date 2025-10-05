use ark_ff::{Fp, MontBackend, MontConfig};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::crypto::prg::populate_random;
use crate::crypto::util::{matrix_vector_multiplication, round};

// create a type for a 128 bit prime field
#[derive(MontConfig)]
#[modulus = "170141183460469231731687303715884105727"] // 2^127 - 1
#[generator = "3"]
pub struct F128Config;
pub type F128 = Fp<MontBackend<F128Config, 2>, 2>;

#[derive(Debug)]
pub struct SeedHomomorphicPRG {
    public_parameter: Vec<Vec<F128>>,
    seed: Vec<F128>
}

impl SeedHomomorphicPRG {
    pub fn new() -> Self {
        Self {
            public_parameter : Self::sample_public_parameter(4096, 2048),
            seed : Self::sample_seed(2048)
        }
    }

    pub fn new_from_public_seed(seed: [u8; 32]) -> Self {
        Self {
            public_parameter : Self::expand_public_parameter(4096, 2048, seed),
            seed : Self::sample_seed(2048)
        }
    }

    pub fn new_from_both_seeds(public_seed: [u8; 32], seed: Vec<F128>) -> Self {
        Self {
            public_parameter : Self::expand_public_parameter(4096, 2048, public_seed),
            seed : seed
        }
    }

    pub fn expand(&self) -> Vec<F128> {
        // multiply the public parameter matrix by the seed
        let product = matrix_vector_multiplication(&self.public_parameter, &self.seed);
        // perform the rounding operation with p = 2^53
        let output = round(product, (1u64) << 53);
        output
    }

    fn sample_public_parameter(size0: usize, size1: usize) -> Vec<Vec<F128>> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut public_parameter = vec![vec![F128::from(0u64); size1]; size0];
        for i in 0..size0 {
            populate_random(&mut public_parameter[i], &mut rng);
        }
        // return the random public parameter matrix
        public_parameter
    }

    fn expand_public_parameter(size0: usize, size1: usize, seed: [u8; 32]) -> Vec<Vec<F128>> {
        let mut rng = ChaCha20Rng::from_seed(seed);
        let mut public_parameter = vec![vec![F128::from(0u64); size1]; size0];
        for i in 0..size0 {
            populate_random(&mut public_parameter[i], &mut rng);
        }
        // return the random public parameter matrix
        public_parameter
    }

    fn sample_seed(size: usize) -> Vec<F128> {
        let mut rng = ChaCha20Rng::from_entropy();
        let mut seed = vec![F128::from(0u64); size];
        populate_random(&mut seed, &mut rng);
        // return the random seed
        seed
    }

    pub fn get_seed(&self) -> &Vec<F128> {
        &self.seed
    }
    
    pub fn get_public_parameter(&self) -> &Vec<Vec<F128>> {
        &self.public_parameter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::util::field_to_64;

    #[test]
    // test that the almost homomorphic property holds
    fn check_homomorphic() {
        // generate two seed homomorphic PRG with the same public parameter matrix
        let prg_0 = SeedHomomorphicPRG::new_from_public_seed([0u8; 32]);
        let prg_1 = SeedHomomorphicPRG::new_from_public_seed([0u8; 32]);
        // expand the PRGs
        let output_0 = prg_0.expand();
        let output_1 = prg_1.expand();
        // get the seeds
        let seed_0 = prg_0.get_seed();
        let seed_1 = prg_1.get_seed();
        
        // add the seeds together in the field
        let mut homomorphic_seed = Vec::with_capacity(seed_0.len());
        for i in 0..seed_0.len() {
            homomorphic_seed.push(seed_0[i] + seed_1[i]);
        }

        // create a new PRG from the homomorphic seed
        let prg_sum = SeedHomomorphicPRG::new_from_both_seeds([0u8; 32], homomorphic_seed);
        let output_sum = prg_sum.expand();

        // check that the output of the sum is approximately the sum of the outputs
        for i in 0..output_0.len() {
            // convert to u64
            let o0 = field_to_64(output_0[i]);
            let o1 = field_to_64(output_1[i]);
            let o_sum = field_to_64(output_sum[i]);
            let diff = ((o0 + o1) % (1i64 << 53)) - o_sum;
            assert!(diff.abs() <= 1);
        }
    }
}