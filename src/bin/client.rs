//! Aggregation client.
//!
//! Fetches public state, encrypts an input determined by a seed, and sends
//! it to the server.

use jagguar::crypto::prg::populate_random;
use jagguar::protocols::Client;
use jagguar::protocols::opa::OPAClient;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Parse the seed and length, expand an input, encrypt, and send to the server.
fn main() {
    let args: [String; 2] = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .try_into()
        .expect("usage: client <seed> <length>");
    let seed = parse_hex_seed(&args[0]);
    let length = args[1].parse().expect("invalid length");

    let mut client = OPAClient::<u32>::new();
    if client.request_state().is_err() {
        eprintln!("Error: cannot connect to server");
        std::process::exit(1);
    }
    client.set_input(expand_seed(seed, length));
    client.encrypt_input();
    client.send_input();
}

/// Expand a seed to `length` elements in `[0, 2^20)`.
fn expand_seed(seed: [u8; 32], length: usize) -> Vec<u32> {
    let mut rng = ChaCha20Rng::from_seed(seed);
    let mut input = vec![0u32; length];
    populate_random(&mut input, &mut rng);
    input.iter().map(|x| x % (1 << 20)).collect()
}

/// Decode a 32-byte seed from 64 hex characters.
fn parse_hex_seed(hex: &str) -> [u8; 32] {
    assert_eq!(hex.len(), 64, "seed must be 32 bytes as 64 hex characters");
    let mut seed = [0u8; 32];
    for (i, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("invalid seed");
    }
    seed
}
