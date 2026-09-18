//! Turns typed wire values into bytes and back.
//!
//! Uses fixed-width little-endian bincode.

use bincode::config::{self, Configuration, Fixint, LittleEndian};
use thiserror::Error;

const CONFIG: Configuration<LittleEndian, Fixint> = config::standard().with_fixed_int_encoding();

/// Encode or decode failure.
#[derive(Debug, Error)]
pub enum WireError {
    #[error("encode failed: {0}")]
    Encode(#[from] bincode::error::EncodeError),
    #[error("decode failed: {0}")]
    Decode(#[from] bincode::error::DecodeError),
}

/// Serialize `value` to a byte vector.
pub fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, WireError> {
    Ok(bincode::serde::encode_to_vec(value, CONFIG)?)
}

/// Deserialize a `T` from `bytes`.
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, WireError> {
    let (value, _len) = bincode::serde::decode_from_slice(bytes, CONFIG)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{decode, encode};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Toy {
        n: u32,
        words: Vec<u128>,
    }

    #[test]
    fn toy_round_trip() {
        let toy = Toy {
            n: 7,
            words: vec![0, 1, u128::MAX],
        };
        let bytes = encode(&toy).unwrap();
        let back: Toy = decode(&bytes).unwrap();
        assert_eq!(toy, back);
    }

    #[test]
    fn u128_slots_are_fixed_width() {
        // 8-byte length prefix + 8 × 16-byte integers. Varint would be larger
        // for these high-bit values.
        let words = vec![u128::MAX; 8];
        let bytes = encode(&words).unwrap();
        assert_eq!(bytes.len(), 8 + 8 * 16);
    }
}
