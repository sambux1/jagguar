//! X25519 hybrid public-key encryption.
//!
//! Ephemeral X25519 Diffie–Hellman and XSalsa20-Poly1305 (`crypto_box_seal`).
//! Ciphertext is `eph_pk (32) || ciphertext_and_tag`.

use super::keys::{DecryptError, PublicKey, SecretKey};
use super::pubkey::PublicKeyEncryption;
use crate::crypto::prg::default_prg;

/// [`PublicKeyEncryption`] using X25519.
pub struct X25519;

impl PublicKeyEncryption for X25519 {
    /// Sample an X25519 keypair.
    fn generate() -> (SecretKey, PublicKey) {
        let secret = crypto_box::SecretKey::generate(&mut default_prg());
        (
            SecretKey::from_bytes(secret.to_bytes()),
            PublicKey::from_bytes(secret.public_key().to_bytes()),
        )
    }

    /// Encrypt to an X25519 public key. Panics if `pk` is not 32 bytes.
    fn encrypt(pk: &PublicKey, plaintext: &[u8]) -> Vec<u8> {
        let pk = crypto_box::PublicKey::from_bytes(
            pk.as_bytes()
                .try_into()
                .expect("X25519 public key must be 32 bytes"),
        );
        pk.seal(&mut default_prg(), plaintext)
            .expect("crypto_box_seal failed")
    }

    /// Decrypt.
    /// Wrong key, truncated ciphertext, or a bad tag returns [`DecryptError::InvalidCiphertext`].
    fn decrypt(sk: &SecretKey, ciphertext: &[u8]) -> Result<Vec<u8>, DecryptError> {
        let sk = crypto_box::SecretKey::from_bytes(
            sk.as_bytes()
                .try_into()
                .map_err(|_| DecryptError::InvalidCiphertext)?,
        );
        sk.unseal(ciphertext)
            .map_err(|_| DecryptError::InvalidCiphertext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bytes added by `encrypt` around the plaintext: ephemeral public key (32) +
    /// Poly1305 tag (16). A ciphertext is `OVERHEAD + plaintext.len()`.
    const OVERHEAD: usize = crypto_box::SEALBYTES;

    #[test]
    fn encrypt_decrypt() {
        let (sk, pk) = X25519::generate();
        let plaintext = b"test message";
        let ciphertext = X25519::encrypt(&pk, plaintext);
        assert_ne!(ciphertext.as_slice(), plaintext);
        assert!(ciphertext.len() >= OVERHEAD + plaintext.len());
        assert_eq!(X25519::decrypt(&sk, &ciphertext).unwrap(), plaintext);
    }

    #[test]
    fn invalid_key_fails() {
        let (_sk, pk) = X25519::generate();
        let (other_sk, _) = X25519::generate();
        let ciphertext = X25519::encrypt(&pk, b"secret");
        assert!(X25519::decrypt(&other_sk, &ciphertext).is_err());
    }

    #[test]
    fn truncated_ciphertext_fails() {
        let (sk, pk) = X25519::generate();
        let ciphertext = X25519::encrypt(&pk, b"secret");
        assert!(X25519::decrypt(&sk, &ciphertext[..ciphertext.len() - 1]).is_err());
    }
}
