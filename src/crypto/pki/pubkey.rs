use super::keys::{DecryptError, PublicKey, SecretKey};

/// Public-key encryption: generate a keypair, encrypt to a public key, decrypt
/// with the matching secret key.
pub trait PublicKeyEncryption {
    fn generate() -> (SecretKey, PublicKey);
    fn encrypt(pk: &PublicKey, plaintext: &[u8]) -> Vec<u8>;
    fn decrypt(sk: &SecretKey, ciphertext: &[u8]) -> Result<Vec<u8>, DecryptError>;
}
