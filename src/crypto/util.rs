use ark_ff::PrimeField;

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

// Convert a field element into a native u64 by taking the least-significant limb.
// This is correct when the integer representative fits in 64 bits (e.g. outputs of round with p <= 2^64).
pub fn field_to_64<F>(x: F) -> i64
where
    F: PrimeField,
    <F as PrimeField>::BigInt: AsRef<[u64]>,
{
    let limbs = x.into_bigint();
    let limbs = limbs.as_ref();
    if limbs.is_empty() { 0 } else { limbs[0] as i64 }
}

// Convert a field element into a native u128 by computing two 64-bit limbs.
pub fn field_to_128<F>(x: F) -> u128
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

