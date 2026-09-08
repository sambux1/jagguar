//! Example of using Jagguar internals.
//!
//! For a multi-party OPA run, use `cargo simulator`.

use jagguar::crypto::SeedHomomorphicPRG;

fn main() {
    println!("This is an example of how to use jagguar internals.");

    let prg = SeedHomomorphicPRG::new();
    let output = prg.expand(8);
    println!("SHPRG expand(8): {output:?}");
}
