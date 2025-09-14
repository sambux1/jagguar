use aggregation::crypto::SeedHomomorphicPRG;

fn main() {
    println!("Hello, world!");
    let prg = SeedHomomorphicPRG::new();
    println!("prg: {:?}", prg);
}
