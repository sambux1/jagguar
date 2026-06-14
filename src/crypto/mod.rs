use ark_ff::{Fp, MontBackend, MontConfig};

pub mod prg;
pub mod seed_homomorphic_prg;
pub mod shamir;
pub mod util;

// expose structs directly
pub use seed_homomorphic_prg::SeedHomomorphicPRG;
pub use shamir::Shamir;

// 127-bit Mersenne-prime field used by the OPA protocol and Shamir
#[derive(MontConfig)]
#[modulus = "170141183460469231731687303715884105727"] // 2^127 - 1
#[generator = "3"]
pub struct F128Config;
pub type F128 = Fp<MontBackend<F128Config, 2>, 2>;

