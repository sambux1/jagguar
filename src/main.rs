use aggregation::crypto::SeedHomomorphicPRG;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    
    let prg = SeedHomomorphicPRG::new();
    
    let after_sample = Instant::now();
    println!("Sample: {:?}", after_sample.duration_since(start));
    
    prg.expand();
    
    let after_expand = Instant::now();
    println!("Expand: {:?}", after_expand.duration_since(after_sample));
}
