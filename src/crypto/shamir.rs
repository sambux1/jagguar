use ark_ff::{Field, UniformRand, PrimeField};
use ark_std::rand::Rng;
use core::marker::PhantomData;

// Encapsulates parameters and provides share/reconstruct methods.
pub struct Shamir<F: Field> {
    num_shares: usize,
    threshold: usize,
    // tells the compiler that Shamir depends on the field F
    _marker: PhantomData<F>,
}

#[derive(Debug)]
pub enum ShamirError {
    InvalidThreshold,
    InvalidNumShares,
    InsufficientShares,
    ReconstructionFailed,
}

impl<F: Field> Shamir<F> {
    pub fn new(num_shares: usize, threshold: usize) -> Self {
        Self {
            num_shares,
            threshold,
            _marker: PhantomData,
        }
    }
    
    // Share a single secret value into (x_i, y_i) pairs, using the configured x-coordinates
    pub fn share<R: Rng>(&self, secret: F, rng: &mut R) -> Result<Vec<(F, F)>, ShamirError>
    where
        F: UniformRand + PrimeField,
    {
        if self.threshold < 2 { return Err(ShamirError::InvalidThreshold); }
        if self.num_shares < self.threshold { return Err(ShamirError::InvalidNumShares); }

        let degree = self.threshold - 1;

        // coeffs[0] = secret; coeffs[1..=degree] random
        let mut coeffs = Vec::with_capacity(degree + 1);
        coeffs.push(secret);
        for _ in 0..degree {
            coeffs.push(F::rand(rng));
        }

        // Evaluate at x = 1..=num_shares using Horner's method
        let mut shares = Vec::with_capacity(self.num_shares);
        for i in 1..=self.num_shares {
            let x = F::from(i as u64);
            let mut y = F::ZERO;
            for c in coeffs.iter().rev() {
                y = y * x + *c;
            }
            shares.push((x, y));
        }

        Ok(shares)
    }

    // Reconstruct a secret from any t shares using Lagrange interpolation at x = 0
    pub fn reconstruct(&self, shares: &[(F, F)]) -> Result<F, ShamirError> {
        if shares.len() < self.threshold { return Err(ShamirError::InsufficientShares); }

        // Use exactly t shares (first t provided)
        let k = self.threshold;
        let used = &shares[..k];

        // s = sum_i y_i * l_i(0), where
        // l_i(0) = prod_{j!=i} (-x_j) / (x_i - x_j)
        let mut secret = F::ZERO;
        for i in 0..k {
            let (xi, yi) = (used[i].0, used[i].1);
            let mut numerator = F::ONE;
            let mut denominator = F::ONE;
            for j in 0..k {
                if i == j { continue; }
                let xj = used[j].0;
                numerator *= -xj;
                denominator *= xi - xj;
            }
            let denom_inv = denominator.inverse().ok_or(ShamirError::ReconstructionFailed)?;
            let li_at_zero = numerator * denom_inv;
            secret += yi * li_at_zero;
        }

        Ok(secret)
    }

    // getter functions
    pub fn threshold(&self) -> usize { self.threshold }
    pub fn num_shares(&self) -> usize { self.num_shares }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::F128;
    use crate::crypto::util::field_to_64;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    // test that the shares are non-zero and non-deterministic
    fn test_share_randomization() {
        let shamir = Shamir::<F128>::new(31, 16);
        let mut rng = ChaCha20Rng::from_entropy();

        // create two secret sharings of the same secret
        let shares_0 = shamir.share(F128::from(17), &mut rng).unwrap();
        let shares_1 = shamir.share(F128::from(17), &mut rng).unwrap();

        // check that the shares are non-zero
        for share in shares_0.iter() {
            assert!(field_to_64(share.1) != 0);
        }
        for share in shares_1.iter() {
            assert!(field_to_64(share.1) != 0);
        }

        // check that the two sharings have different shares
        for i in 0..shares_0.len() {
            assert!(shares_0[i].1 != shares_1[i].1);
        }
    }

    #[test]
    // test that the secret can be reconstructed successfully
    fn test_reconstruction() {
        let shamir = Shamir::<F128>::new(31, 16);
        let mut rng = ChaCha20Rng::from_entropy();

        // create a secret sharing of the secret
        let shares = shamir.share(F128::from(17), &mut rng).unwrap();

        // reconstruct the secret from any t shares
        let secret = shamir.reconstruct(&shares).unwrap();
        assert_eq!(secret, F128::from(17));
    }
}