/// Public key. Safe to put on the wire.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PublicKey(Vec<u8>);

/// Secret key. Not serialized.
pub struct SecretKey(Vec<u8>);

/// Failure from [`super::PublicKeyEncryption::decrypt`].
#[derive(Debug)]
pub enum DecryptError {
    /// Ciphertext is truncated, corrupt, or for a different key.
    InvalidCiphertext,
}

impl PublicKey {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl SecretKey {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl core::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SecretKey")
    }
}
