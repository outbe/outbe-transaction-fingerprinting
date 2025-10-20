use crate::components::{FingerprintComponent, SqueezeComponent};
use crate::{EPOCH, SPEC_DC};
use anyhow::{anyhow, Error};
use bigint::U256;
use chrono::{DateTime, NaiveDate, Utc};
use fingerprinting_poseidon::Poseidon;
use halo2_axiom::halo2curves::bn256::Fr;
use std::io::Write;

pub type Amount = (u64, u64);

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct DateTimeRaw {
    date_time: DateTime<Utc>,
    wwd: NaiveDate,
    amount: Amount,
}

impl DateTimeRaw {
    pub fn new(date_time: DateTime<Utc>, wwd: NaiveDate, amount: Amount) -> Self {
        DateTimeRaw {
            date_time,
            wwd,
            amount,
        }
    }
}

#[derive(Debug)]
pub struct DateTimeComponent {
    raw: DateTimeRaw,
}

#[inline]
fn cantor_pair_function(x: U256, y: U256) -> U256 {
    let top = x * x + U256::from(3) * x + U256::from(2) * x * y + y + y * y;

    top / U256::from(2)
}

impl FingerprintComponent<DateTimeRaw, 32> for DateTimeComponent {
    fn new(original: DateTimeRaw) -> Self {
        Self { raw: original }
    }

    fn serialize<W: Write>(&self, buffer: &mut W) -> Result<(), anyhow::Error> {
        let squeezed = self.squeeze()?;
        let bytes = squeezed.to_bytes();

        let written = buffer.write(&bytes)?;

        debug_assert_eq!(written, Self::size());
        Ok(())
    }

    fn raw(&self) -> &DateTimeRaw {
        &self.raw
    }
}

impl SqueezeComponent<Fr> for DateTimeComponent {
    fn squeeze(&self) -> Result<Fr, Error> {
        let amount_base = U256::from(self.raw.amount.0);
        let amount_atto = U256::from(self.raw.amount.1);
        let full_amount = amount_base * U256::from(10 ^ 18) + amount_atto;

        let date_time = self.raw.date_time;
        let seconds_since_epoch = date_time
            .naive_local()
            .signed_duration_since(EPOCH)
            .num_seconds();

        if seconds_since_epoch < 0 {
            return Err(anyhow!("Date cannot be earlier than Epoch: 01.01.2025"));
        }

        let seconds_since_epoch = U256::from(seconds_since_epoch as u64);
        let days_since_epoch = self.raw.wwd.signed_duration_since(EPOCH.date()).num_days();

        if days_since_epoch < 0 || days_since_epoch > u32::MAX as i64 {
            return Err(anyhow!(
                "World Wide Date cannot be earlier than Epoch: 01.01.2025"
            ));
        }

        let days_since_epoch = U256::from(days_since_epoch as u32);

        // Calculating pair function
        let paired_data = cantor_pair_function(seconds_since_epoch, full_amount / days_since_epoch);

        // Specs for 3 Fr input
        let mut poseidon = Poseidon::new_with_spec(SPEC_DC.clone());

        // According to the docs
        // - seconds since epoch
        // - days since epoch
        // - nonce as pairing function from amount days and seconds
        let seconds_since_epoch = Fr::from(seconds_since_epoch.as_u64());
        let days_since_epoch = Fr::from(days_since_epoch.as_u64());
        let nonce = Fr::from_raw(paired_data.0);

        poseidon.update(&[seconds_since_epoch, days_since_epoch, nonce]);

        Ok(poseidon.squeeze())
    }
}
