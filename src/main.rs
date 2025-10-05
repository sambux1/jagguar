use aggregation::crypto::SeedHomomorphicPRG;
use std::time::Instant;

fn main() {
    let start = Instant::now();

    let seed = [0u8; 32];
    
    let prg_0 = SeedHomomorphicPRG::new_from_public_seed(seed);
    let prg_1 = SeedHomomorphicPRG::new_from_public_seed(seed);
    
    let after_sample = Instant::now();
    println!("Sample: {:?}", after_sample.duration_since(start));
    
    prg_0.expand();
    prg_1.expand();
    
    let after_expand = Instant::now();
    println!("Expand: {:?}", after_expand.duration_since(after_sample));
}
