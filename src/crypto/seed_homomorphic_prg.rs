use ark_ff::{Fp, MontBackend, MontConfig};

// create a type for a 128 bit prime field
#[derive(MontConfig)]
#[modulus = "170141183460469231731687303715884105727"] // 2^127 - 1
#[generator = "3"]
pub struct F128Config;
pub type F128 = Fp<MontBackend<F128Config, 2>, 2>;

#[derive(Debug)]
pub struct SeedHomomorphicPRG {
    seed: Vec<Vec<F128>>,
}

impl SeedHomomorphicPRG {
    pub fn new() -> Self {
        Self { seed: vec![vec![F128::from(0u64), F128::from(1u64)],
                          vec![F128::from(2u64), F128::from(3u64)]] }
    }

    pub fn seed(&self) -> &Vec<Vec<F128>> { &self.seed }
}