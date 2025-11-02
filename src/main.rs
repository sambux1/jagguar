use aggregation::crypto::{SeedHomomorphicPRG, shamir::Shamir};
use std::time::Instant;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

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

    let shamir = Shamir::<aggregation::crypto::F128>::new(31, 16);
    println!("Shamir {} / {}", shamir.threshold(), shamir.num_shares());
    let mut rng = ChaCha20Rng::from_entropy();
    let shares = shamir.share(aggregation::crypto::F128::from(17), &mut rng).unwrap();
    // print the shares
    for share in shares.iter() {
        println!("Share: {:?}", share);
    }
    // reconstruct the secret
    let secret = shamir.reconstruct(&shares).unwrap();
    println!("Secret: {:?}", secret);
    println!("Opened secret: {:?}", secret);
}
