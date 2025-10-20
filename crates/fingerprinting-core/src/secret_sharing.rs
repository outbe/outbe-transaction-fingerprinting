use halo2_axiom::halo2curves::ff::PrimeField;
use rand_core::OsRng;
use std::collections::HashMap;

#[cfg(test)]
use halo2_axiom::halo2curves::group;


pub struct SecretSharing<F: PrimeField> {
    pub threshold: usize,
    shares: HashMap<usize, F>,
}

impl<F: PrimeField> SecretSharing<F> {
    pub fn generate(k: F, t: usize, n: usize) -> Self {
        assert!(t <= n, "Threshold must be <= total shares");
        assert!(t > 0, "Threshold must be >= 1");

        let mut rng = OsRng;
        let mut coefficients = vec![k];

        for _ in 1..t {
            coefficients.push(F::random(&mut rng));
        }

        let mut shares = HashMap::new();
        for i in 1..=n {
            let x = F::from(i as u64);
            let mut share = coefficients[0];
            let mut x_power = x;

            for j in 1..t {
                share += coefficients[j] * x_power;
                x_power *= x;
            }
            shares.insert(i, share);
        }

        SecretSharing {
            threshold: t,
            shares,
        }
    }

    pub fn lagrange_coefficient(i: usize, indices: &[usize]) -> F {
        let i_fr = F::from(i as u64);
        let mut result = F::from(1u64);

        for &j in indices {
            if i != j {
                let j_fr = F::from(j as u64);
                let numerator = -j_fr;
                let denominator = i_fr - j_fr;
                result *= numerator * denominator.invert().unwrap();
            }
        }
        result
    }

    #[cfg(test)]
    /// Computing the exponent
    pub(crate) fn compute_exponent<C: group::Group<Scalar = F>>(
        &self,
        i: usize,
        blinded_value: C,
    ) -> (usize, C) {
        let shard = self.shares.get(&i).unwrap();
        let exponent_i = blinded_value * shard.clone();

        (i, exponent_i)
    }

    #[cfg(test)]
    pub(crate) fn get_share(&self, i: usize) -> Option<F> {
        self.shares.get(&i).cloned()
    }

    pub fn get_shares(&self) -> &HashMap<usize, F> {
        &self.shares
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_axiom::halo2curves::bn256::Fr;
    use halo2_axiom::halo2curves::ff::Field;

    #[test]
    fn test_basic_secret_reconstruction() {
        let mut rng = OsRng;
        let secret = Fr::random(&mut rng);

        // Generate 3-of-5 sharing
        let sharing = SecretSharing::generate(secret, 3, 5);

        // Reconstruct using first 3 shares
        let indices = vec![1, 2, 3];
        let mut reconstructed = Fr::zero();

        for &i in &indices {
            let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
            reconstructed += sharing.shares[&i] * lambda_i;
        }

        assert_eq!(secret, reconstructed, "Secret reconstruction failed");
    }

    #[test]
    fn test_any_threshold_subset_reconstructs() {
        let mut rng = OsRng;
        let secret = Fr::random(&mut rng);
        let sharing = SecretSharing::generate(secret, 3, 7);

        // Test different combinations of 3 shares
        let test_combinations = vec![vec![1, 2, 3], vec![2, 4, 6], vec![1, 5, 7], vec![3, 4, 5]];

        for indices in test_combinations {
            let mut reconstructed = Fr::zero();
            for &i in &indices {
                let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
                reconstructed += sharing.shares[&i] * lambda_i;
            }
            assert_eq!(
                secret, reconstructed,
                "Failed to reconstruct with indices {:?}",
                indices
            );
        }
    }

    #[test]
    #[should_panic(expected = "Threshold must be <= total shares")]
    fn test_invalid_threshold_too_large() {
        let secret = Fr::from(42u64);
        SecretSharing::generate(secret, 6, 5);
    }

    #[test]
    #[should_panic(expected = "Threshold must be >= 1")]
    fn test_invalid_threshold_zero() {
        let secret = Fr::from(42u64);
        SecretSharing::generate(secret, 0, 5);
    }

    #[test]
    fn test_threshold_one() {
        let secret = Fr::from(42u64);
        let sharing = SecretSharing::generate(secret, 1, 3);

        // With t=1, any single share should reconstruct the secret
        for i in 1..=3 {
            let indices = vec![i];
            let lambda: Fr = SecretSharing::lagrange_coefficient(i, &indices);
            let reconstructed = sharing.shares[&i] * lambda;

            assert_eq!(secret, reconstructed, "Failed with single share {}", i);
        }
    }

    #[test]
    fn test_threshold_equals_total() {
        let mut rng = OsRng;
        let secret = Fr::random(&mut rng);

        // All shares required
        let sharing = SecretSharing::generate(secret, 5, 5);

        let indices: Vec<usize> = (1..=5).collect();
        let mut reconstructed = Fr::zero();

        for &i in &indices {
            let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
            reconstructed += sharing.shares[&i] * lambda_i;
        }

        assert_eq!(secret, reconstructed);
    }

    #[test]
    fn test_lagrange_coefficient_sum_to_one() {
        // For any set of indices, Lagrange coefficients should sum to 1
        // when evaluating polynomial at x=0
        let indices = vec![1, 3, 5, 7];
        let mut sum = Fr::zero();

        for &i in &indices {
            let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
            sum += lambda_i;
        }

        assert_eq!(sum, Fr::one(), "Lagrange coefficients don't sum to 1");
    }

    #[test]
    fn test_lagrange_coefficient_formula() {
        // Test specific known values
        // For indices [1,2,3], interpolating at x=0:
        // λ_1 = (0-2)(0-3) / (1-2)(1-3) = 6/2 = 3
        // λ_2 = (0-1)(0-3) / (2-1)(2-3) = 3/-1 = -3
        // λ_3 = (0-1)(0-2) / (3-1)(3-2) = 2/2 = 1

        let indices = vec![1, 2, 3];
        let lambda_1: Fr = SecretSharing::lagrange_coefficient(1, &indices);
        let lambda_2: Fr = SecretSharing::lagrange_coefficient(2, &indices);
        let lambda_3: Fr = SecretSharing::lagrange_coefficient(3, &indices);

        assert_eq!(lambda_1, Fr::from(3u64));
        assert_eq!(lambda_2, -Fr::from(3u64));
        assert_eq!(lambda_3, Fr::from(1u64));

        // Verify they sum to 1
        assert_eq!(lambda_1 + lambda_2 + lambda_3, Fr::one());
    }

    #[test]
    fn test_deterministic_shares() {
        // Same secret and random seed should give same shares
        let secret = Fr::from(12345u64);

        // We can't control randomness in generate(), but we can verify
        // that the polynomial P(i) is deterministic given coefficients
        let sharing = SecretSharing::generate(secret, 3, 5);

        // Verify P(0) = secret by reconstructing from any 3 shares
        let indices = vec![1, 2, 3];
        let mut reconstructed = Fr::zero();

        for &i in &indices {
            let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
            reconstructed += sharing.shares[&i] * lambda_i;
        }

        assert_eq!(secret, reconstructed);
    }

    #[test]
    fn test_large_threshold() {
        let mut rng = OsRng;
        let secret = Fr::random(&mut rng);

        // Test with larger numbers
        let sharing = SecretSharing::generate(secret, 20, 30);

        // Reconstruct with exactly t shares
        let indices: Vec<usize> = (1..=20).collect();
        let mut reconstructed = Fr::zero();

        for &i in &indices {
            let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
            reconstructed += sharing.shares[&i] * lambda_i;
        }

        assert_eq!(secret, reconstructed);
    }

    #[test]
    fn test_shares_are_distinct() {
        let secret = Fr::from(999u64);
        let sharing = SecretSharing::generate(secret, 3, 5);

        // All shares should be different (with overwhelming probability)
        let shares: Vec<Fr> = (1..=5).map(|i| sharing.shares[&i]).collect();

        for i in 0..shares.len() {
            for j in (i + 1)..shares.len() {
                assert_ne!(
                    shares[i],
                    shares[j],
                    "Shares {} and {} are identical",
                    i + 1,
                    j + 1
                );
            }
        }
    }

    #[test]
    fn test_share_not_equal_to_secret() {
        let mut rng = OsRng;
        let secret = Fr::random(&mut rng);
        let sharing = SecretSharing::generate(secret, 3, 5);

        // For t > 1, individual shares should not equal the secret
        for i in 1..=5 {
            // With overwhelming probability, share != secret
            // (Could be equal by chance, but probability is 1/r ≈ 2^-254)
            if sharing.threshold > 1 {
                assert_ne!(
                    sharing.shares[&i], secret,
                    "Share {} equals secret (extremely unlikely)",
                    i
                );
            }
        }
    }

    #[test]
    fn test_reconstruction_with_non_sequential_indices() {
        let mut rng = OsRng;
        let secret = Fr::random(&mut rng);
        let sharing = SecretSharing::generate(secret, 4, 10);

        // Use non-sequential indices
        let indices = vec![2, 5, 7, 9];
        let mut reconstructed = Fr::zero();

        for &i in &indices {
            let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
            reconstructed += sharing.shares[&i] * lambda_i;
        }

        assert_eq!(secret, reconstructed);
    }

    #[test]
    fn test_zero_secret() {
        let secret = Fr::zero();
        let sharing = SecretSharing::generate(secret, 3, 5);

        let indices = vec![1, 2, 3];
        let mut reconstructed = Fr::zero();

        for &i in &indices {
            let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
            reconstructed += sharing.shares[&i] * lambda_i;
        }

        assert_eq!(Fr::zero(), reconstructed);
    }

    #[test]
    fn test_polynomial_degree() {
        let secret = Fr::from(100u64);
        let sharing = SecretSharing::generate(secret, 3, 10);

        // Polynomial has degree t-1 = 2
        // This means any 3 points determine it uniquely
        // But 2 points should NOT be enough

        // We verify this by checking that different sets of 3 points
        // all reconstruct the same secret
        let combinations = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];

        let mut results = Vec::new();
        for indices in combinations {
            let mut reconstructed = Fr::zero();
            for &i in &indices {
                let lambda_i: Fr = SecretSharing::lagrange_coefficient(i, &indices);
                reconstructed += sharing.shares[&i] * lambda_i;
            }
            results.push(reconstructed);
        }

        // All should equal the secret
        for result in results {
            assert_eq!(secret, result);
        }
    }
}
