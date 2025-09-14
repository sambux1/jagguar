#[derive(Debug)]
pub struct SeedHomomorphicPRG {
    seed: Vec<u8>,
}

impl SeedHomomorphicPRG {
    pub fn new() -> Self {
        Self { seed: vec![0, 32] }
    }
}