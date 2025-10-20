use crate::components::FingerprintComponent;
use anyhow::Error;
use halo2_axiom::halo2curves::ff::PrimeField;
use std::io::Write;

// Represents the hash of the DateTimeComponent with additional entropy
// Poseidon(Ts|WWD|Nonce) as H
// H^K - hash component with entropy K
// As a result of the computation we will get Fr = Poseidon([K] Poseidon(Ts|WWD|Nonce))
// Current component serialize scalar to buffer
pub struct ScalarComponent<F: PrimeField, const S: usize>(F);

impl<F: PrimeField, const S: usize> FingerprintComponent<F, S> for ScalarComponent<F, S> {
    fn new(original: F) -> Self {
        Self(original)
    }

    fn serialize<W: Write>(&self, buffer: &mut W) -> Result<(), Error> {
        let written = buffer.write(self.0.to_repr().as_ref())?;

        debug_assert_eq!(written, Self::size());
        Ok(())
    }

    fn raw(&self) -> &F {
        &self.0
    }
}
