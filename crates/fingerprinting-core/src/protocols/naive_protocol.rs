use anyhow::Error;
use halo2_axiom::halo2curves::bn256::{Fr, G1};
use halo2_axiom::halo2curves::CurveExt;

use crate::protocols::FingerprintProtocol;
use crate::{HashSqueeze, HASH_TO_CURVE_PREFIX};

// Computes the [k] P without split and reconstruct from by cooperating with other agents
pub struct NaiveProtocol {
    secret: Fr,
}

impl NaiveProtocol {
    pub fn new(secret: Fr) -> Self {
        Self { secret }
    }
}

impl FingerprintProtocol<Fr> for NaiveProtocol {
    async fn process(&self, unblinded: Fr) -> Result<Fr, Error> {
        let hasher = G1::hash_to_curve(HASH_TO_CURVE_PREFIX);
        let curve_point = hasher(&unblinded.to_bytes());

        let hash_with_secret = curve_point * self.secret;

        hash_with_secret.squeeze() // Use default compress for G1
    }
}
