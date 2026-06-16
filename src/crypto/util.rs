use ark_ff::{BigInteger, PrimeField};

use crate::crypto::{FieldBytes, FIELD_ELEMENT_BYTES};

// Multiply a matrix (rows of u128) by a vector over Z_{2^128}.
// Each multiplication wraps modulo 2^128; the sum also wraps — i.e. arithmetic in Z_{2^128}.
pub fn matrix_vector_multiplication(matrix: &Vec<Vec<u128>>, vector: &Vec<u128>) -> Vec<u128> {
    let mut out = Vec::with_capacity(matrix.len());
    for row in matrix.iter() {
        assert_eq!(row.len(), vector.len(), "row length must match vector length");
        let mut acc = 0u128;
        for (a, b) in row.iter().zip(vector.iter()) {
            acc = acc.wrapping_add(a.wrapping_mul(*b));
        }
        out.push(acc);
    }
    out
}

// Round coefficients from Z_{2^128} down to p = 2^92 by dropping the low bits.
pub fn round(vector: Vec<u128>, shift: u32) -> Vec<u128> {
    vector.into_iter().map(|x| x >> shift).collect()
}

// Least-significant 128 bits of a field element's integer representative.
pub fn field_low_u128<F>(x: F) -> u128
where
    F: PrimeField,
    <F as PrimeField>::BigInt: AsRef<[u64]>,
{
    let limbs = x.into_bigint();
    let limbs = limbs.as_ref();
    let lo = limbs.get(0).copied().unwrap_or(0) as u128;
    let hi = limbs.get(1).copied().unwrap_or(0) as u128;
    lo | (hi << 64)
}

// Serialize a field element to a fixed-width little-endian byte array.
pub fn field_to_bytes<F>(x: F) -> FieldBytes
where
    F: PrimeField,
{
    let limb_bytes = x.into_bigint().to_bytes_le();
    let mut out = [0u8; FIELD_ELEMENT_BYTES];
    let copy_len = limb_bytes.len().min(FIELD_ELEMENT_BYTES);
    out[..copy_len].copy_from_slice(&limb_bytes[..copy_len]);
    out
}

// Deserialize a field element from a fixed-width little-endian byte array.
pub fn field_from_bytes<F>(bytes: &FieldBytes) -> F
where
    F: PrimeField,
{
    F::from_le_bytes_mod_order(bytes)
}
