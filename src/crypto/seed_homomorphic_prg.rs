use ark_ff::{Fp, MontBackend, MontConfig};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::crypto::prg::populate_random;

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
            public_parameter : Self::sample_public_parameter(128, 128),
            seed : Self::sample_seed(128)
        }
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