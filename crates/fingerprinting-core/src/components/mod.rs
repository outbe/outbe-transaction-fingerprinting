use halo2_axiom::halo2curves::ff::PrimeField;
use std::io::Write;

mod bytes32;
mod amount;
mod bank_identifier;
mod currency;
mod date_time_raw;
mod scalar;

pub trait SqueezeComponent<F: PrimeField> {
    /// Squeeze original data into prime field
    fn squeeze(&self) -> Result<F, anyhow::Error>;
}

pub trait FingerprintComponent<O, const S: usize> {
    /// constructor
    fn new(original: O) -> Self;

    /// normalization and serialization function to fill up the buffer
    fn serialize<W: Write>(&self, buffer: &mut W) -> Result<(), anyhow::Error>;

    fn raw(&self) -> &O;

    /// size of the component contribution to target hash
    fn size() -> usize {
        S
    }
}

#[allow(unused_imports)]
pub use bytes32::{Bytes32, Bytes32Component};
pub use amount::AmountComponent;
pub use bank_identifier::BankIdentifierComponent;
pub use currency::CurrencyComponent;
pub use date_time_raw::DateTimeComponent;
pub use date_time_raw::DateTimeRaw;
pub use scalar::ScalarComponent;
