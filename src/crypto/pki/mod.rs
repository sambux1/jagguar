//! Public-key encryption and public-key infrastructure.
//!
//! [`PublicKeyEncryption`] is the interface. [`X25519`] is the only
//! implementation so far.

mod keys;
mod pubkey;
mod x25519;

pub use keys::{DecryptError, PublicKey, SecretKey};
pub use pubkey::PublicKeyEncryption;
pub use x25519::X25519;
