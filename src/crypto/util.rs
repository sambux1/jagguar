use ark_ff::{Field, PrimeField};
use num_bigint::BigUint;
use num_traits::ToPrimitive;

// Multiply a matrix (rows of field elements) by a vector over the same field.
pub fn matrix_vector_multiplication<F: Field>(matrix: &Vec<Vec<F>>, vector: &Vec<F>) -> Vec<F> {
    let mut out = Vec::with_capacity(matrix.len());
    for row in matrix.iter() {
        assert_eq!(row.len(), vector.len(), "row length must match vector length");
        let mut acc = F::ZERO;
        for (a, b) in row.iter().zip(vector.iter()) {
            acc += *a * *b;
        }
        out.push(acc);
    }
    out
}

pub fn round<F>(vector: Vec<F>, p: u64) -> Vec<F>
where
    F: PrimeField,
    <F as PrimeField>::BigInt: AsRef<[u64]>,
{
    let q = biguint_from_bigint(F::MODULUS);
    let p_big = BigUint::from(p);
    let mut ret = Vec::with_capacity(vector.len());
    for x in vector.iter() {
        let x_big = biguint_from_bigint(x.into_bigint());
        let product = x_big * &p_big; // exact big integer product
        let y = &product / &q;        // floor division
        // map back into field by interpreting y as small integer (y < p <= 2^64)
        let y_u64 = y.to_u64().unwrap_or(u64::MAX);
        ret.push(F::from(y_u64));
    }
    ret
}

// Convert a 2-limb little-endian bigint to u128 (specific to F128).
fn biguint_from_bigint<B>(b: B) -> BigUint
where
    B: AsRef<[u64]>,
{
    let limbs = b.as_ref();
    let mut acc = BigUint::from(0u32);
    for (i, limb) in limbs.iter().enumerate() {
        if *limb != 0 {
            acc += BigUint::from(*limb) << (i * 64);
        }
    }
    acc
}