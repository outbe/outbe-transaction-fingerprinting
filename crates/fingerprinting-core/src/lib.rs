mod components;
mod protocols;
pub mod secret_sharing;

use crate::components::{DateTimeRaw, ScalarComponent, SqueezeComponent};
use anyhow::{anyhow, Error};
use bytes::{BufMut, Bytes, BytesMut};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use components::{
    AmountComponent, BankIdentifierComponent, CurrencyComponent, DateTimeComponent,
    FingerprintComponent,
};
use fingerprinting_poseidon::{Poseidon, Spec};
use fingerprinting_types::RawTransaction;
use halo2_axiom::halo2curves::bn256::{Fr, G1};
use halo2_axiom::halo2curves::ff::PrimeField as PF;
use halo2_axiom::halo2curves::group::GroupEncoding;
use iso_currency::Currency;
use std::io::Write;
use std::marker::PhantomData;
use std::sync::LazyLock;

pub use crate::protocols::{
    AgentsTopology, CollaborativeProtocol, FingerprintProtocol, NaiveProtocol,
};

// Hash related cashed spec 8 full rounds, 57 partial rounds, with 1 Fr as an input
pub(crate) static SPEC: LazyLock<Spec<Fr, 2, 1>> = LazyLock::new(|| Spec::new(8, 57));

// Hash related cashed spec 8 full rounds, 57 partial rounds, with 4 Fr as an input
pub(crate) static SPEC_BIG: LazyLock<Spec<Fr, 5, 4>> = LazyLock::new(|| Spec::new(8, 57));

// Hash related cashed spec 8 full rounds, 57 partial rounds, with 3 Fr as an input
pub(crate) static SPEC_DC: LazyLock<Spec<Fr, 4, 3>> = LazyLock::new(|| Spec::new(8, 57));

// Base Epoch used for offsetting dates components
pub(crate) static EPOCH: NaiveDateTime = NaiveDateTime::new(
    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
);

pub const HASH_TO_CURVE_PREFIX: &'static str = "CRA_FINGERPRINT";

pub trait HashSqueeze<F: PF> {
    fn squeeze(&self) -> Result<F, Error>;
}

impl HashSqueeze<Fr> for G1 {
    fn squeeze(&self) -> Result<Fr, Error> {
        let bytes = self.to_bytes();
        let bytes_array = bytes.as_ref();

        // Split the 32 bytes of compressed point to the 2 limbs
        // Each limb represent as first 16 bytes in 32 bytes array (to be sure that it will fit into Fr
        // Each of the generated array convert into the Fr
        // Hash the result and squeeze into single Fr

        let frs: Vec<Fr> = bytes_array
            .chunks(16)
            .map(|chunk| {
                let mut buffer_32 = [0u8; 32];
                buffer_32[0..16].copy_from_slice(chunk.as_ref());

                Fr::from_bytes(&buffer_32).unwrap_or(Fr::zero())
            })
            .collect();

        let mut poseidon = Poseidon::new_with_spec(SPEC.clone());

        poseidon.update(frs.as_slice());
        let squeezed_salted_hash = poseidon.squeeze();

        Ok(squeezed_salted_hash)
    }
}

impl HashSqueeze<Fr> for Bytes {
    fn squeeze(&self) -> Result<Fr, Error> {
        // TODO make more generic
        let mut poseidon = Poseidon::new_with_spec(SPEC_BIG.clone());
        let limb_size = self.len() / 4;

        let mut limbs = Vec::with_capacity(4);
        for offset in (0..self.len()).step_by(limb_size) {
            let mut buffer_32 = [0u8; 32];
            buffer_32[0..limb_size].copy_from_slice(&self[offset..offset + limb_size]);

            limbs.push(Fr::from_bytes(&buffer_32).unwrap_or(Fr::zero()));
        }

        poseidon.update(limbs.as_slice());

        Ok(poseidon.squeeze())
    }
}

pub trait Fingerprint<F: PF, P: FingerprintProtocol<F>> {
    /// perform Fingerprint computation
    fn complete_fingerprint(&self, via_protocol: &P) -> impl std::future::Future<Output = Result<F, Error>> + Send;
    fn datetime_fingerprint(&self, via_protocol: &P) -> impl std::future::Future<Output = Result<F, Error>> + Send;

    fn fingerprint(&self, date_time: F, _: PhantomData<P>) -> Result<F, Error>;
}

pub trait Compact
where
    Self: Sized,
{
    fn compact(&self) -> String;

    fn unwrap(compacted: &String) -> Result<Self, Error>;
}

impl<P: FingerprintProtocol<Fr> + Sync> Fingerprint<Fr, P> for TransactionFingerprintData<Fr> {
    async fn complete_fingerprint(&self, via_protocol: &P) -> Result<Fr, Error> {
        let date_time = self.datetime_fingerprint(via_protocol).await?;

        self.fingerprint(date_time, PhantomData::<P>::default())
    }

    async fn datetime_fingerprint(&self, via_protocol: &P) -> Result<Fr, Error> {
        let date_time = &self.date_time;
        let squeezed = date_time.squeeze()?;

        via_protocol.process(squeezed).await
    }

    fn fingerprint(&self, date_time: Fr, _: PhantomData<P>) -> Result<Fr, Error> {
        let fingerprint_size = TransactionFingerprintData::<Fr>::fingerprint_size();
        let buffer = BytesMut::with_capacity(fingerprint_size);
        let mut writer = buffer.writer();
        writer.write(&[0xFF, 0xFE, 0xED, 0xDD, 0xCC, 0x00, 0xDD, 0xEE])?; // Prefix for serialization

        let date_time = ScalarComponent::<Fr, 32>::new(date_time);
        let bic = &self.bic;
        let amount = &self.amount;
        let currency = &self.currency;

        bic.serialize(&mut writer)?;
        amount.serialize(&mut writer)?;
        currency.serialize(&mut writer)?;
        date_time.serialize(&mut writer)?;

        let buffer = writer.into_inner().freeze();
        let fingerprint = buffer.squeeze()?;

        log::info!("Transaction fingerprint generated successfully: {}", fingerprint.compact());

        Ok(fingerprint)
    }
}

impl Compact for Bytes {
    fn compact(&self) -> String {
        bs58::encode(&self).into_string()
    }

    fn unwrap(compacted: &String) -> Result<Bytes, Error> {
        let bytes = bs58::decode(&compacted).into_vec()?;

        Ok(Bytes::copy_from_slice(&bytes))
    }
}

impl Compact for Fr {
    fn compact(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    fn unwrap(compacted: &String) -> Result<Self, Error> {
        let bytes = bs58::decode(&compacted).into_vec()?;
        let fixed_bytes = bytes.first_chunk::<32>()
            .ok_or(anyhow!("failed to decode Fr from compacted string, given array is less than 32 bytes long"))?;

        Fr::from_bytes(fixed_bytes)
            .into_option()
            .ok_or(anyhow!("failed to decode Fr from compacted string, value does not represent Fr"))
    }
}

#[derive(Debug)]
pub struct TransactionFingerprintData<F> {
    bic: BankIdentifierComponent,
    amount: AmountComponent,
    currency: CurrencyComponent,
    date_time: DateTimeComponent,

    _p: PhantomData<F>,
}

impl<F> TransactionFingerprintData<F> {
    pub fn fingerprint_size() -> usize {
        8 + BankIdentifierComponent::size()
            + AmountComponent::size()
            + CurrencyComponent::size()
            + DateTimeComponent::size()
    }
}
impl<F: PF> TransactionFingerprintData<F> {
    pub fn new(
        bic: BankIdentifierComponent,
        amount: AmountComponent,
        currency: CurrencyComponent,
        date_time: DateTimeComponent,
    ) -> Self {
        Self {
            bic,
            amount,
            currency,
            date_time,
            _p: PhantomData::default(),
        }
    }

    pub fn bic(&self) -> &str {
        self.bic.raw()
    }

    pub fn amount(&self) -> (u64, u64) {
        self.amount.raw().clone()
    }

    pub fn currency_code(&self) -> u16 {
        self.currency.raw().clone()
    }

    pub fn currency(&self) -> Option<Currency> {
        Currency::from_numeric(self.currency_code())
    }

    pub fn date_time(&self) -> &DateTime<Utc> {
        unimplemented!()
    }

    pub fn date_time_component(&self) -> &DateTimeComponent {
        &self.date_time
    }
}

impl<F: PF> TryFrom<RawTransaction> for TransactionFingerprintData<F> {
    type Error = Error;

    fn try_from(tx: RawTransaction) -> Result<Self, Self::Error> {
        let money = tx.amount;
        let iso_currency = Currency::from_code(&money.currency)
            .ok_or(anyhow!("Currency is not in the ISO 4217 currency"))?;
        if iso_currency.is_special() {
            return Err(anyhow!("Currency should have numeric value"));
        }
        let iso_currency_code = iso_currency.numeric();

        let bic = BankIdentifierComponent::new(tx.bic.to_string());
        let amount = AmountComponent::new((money.amount_base, money.amount_atto));
        let currency = CurrencyComponent::new(iso_currency_code);

        let dt_raw_data =
            DateTimeRaw::new(tx.date_time, tx.wwd, (money.amount_base, money.amount_atto));

        let date_time = DateTimeComponent::new(dt_raw_data);

        Ok(Self {
            bic,
            amount,
            currency,
            date_time,
            _p: Default::default(),
        })
    }
}

impl<F: PF> TryFrom<&RawTransaction> for TransactionFingerprintData<F> {
    type Error = Error;

    fn try_from(value: &RawTransaction) -> Result<Self, Self::Error> {
        value.clone().try_into()
    }
}

#[cfg(test)]
mod tests {

use super::*;
    use rand::Rng;
    use std::cmp::PartialEq;

    use crate::protocols::NaiveProtocol;
    use chrono::{TimeZone, Utc};
    use rand_core::OsRng;
    use fingerprinting_types::RawTransactionBuilder;
    use halo2_axiom::arithmetic::Field;

    impl PartialEq for &TransactionFingerprintData<Fr> {
        fn eq(&self, other: &Self) -> bool {
            self.bic.raw() == other.bic.raw()
                && self.amount.raw() == other.amount.raw()
                && self.currency.raw() == other.currency.raw()
                && self.date_time.raw() == other.date_time.raw()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fingerprint_construction() -> Result<(), Error> {
        let mut rng = rand::rng();

        // Init naive protocol for testing
        let protocol = NaiveProtocol::new(Fr::from(42));

        let mut tx_fingerprint_set = Vec::new();
        let mut tx_data_set = Vec::new();

        let n = 100usize;
        println!("Phase 1 (Generate Test Data): {}", Utc::now());

        for _i in 0..n {
            let tx_date = Utc
                .with_ymd_and_hms(
                    2025,
                    9,
                    16,
                    rng.random_range(0..=23),
                    rng.random_range(0..=23),
                    rng.random_range(0..=59),
                )
                .unwrap();

            let tx: TransactionFingerprintData<Fr> = RawTransactionBuilder::default()
                .bic("BCEELU21")
                .amount((rng.random_range(1..1000), "EUR"))
                .date_time(tx_date)
                .wwd(tx_date.date_naive())
                .build()?
                .try_into()?;

            tx_data_set.push(tx);
        }

        println!("Phase 2 (Build Fingerprints): {}", Utc::now());

        for i in 0..n {
            let tx = &tx_data_set[i];
            let tx_fingerprint = tx.complete_fingerprint(&protocol).await?;

            tx_fingerprint_set.push(tx_fingerprint);
        }

        println!("Phase 3 (Validate no Collisions): {}", Utc::now());

        for i in 0..(n - 1) {
            for j in i..n {
                let tx_f_i = tx_fingerprint_set[i];
                let tx_f_j = tx_fingerprint_set[j];

                let tx_i = &tx_data_set[i];
                let tx_j = &tx_data_set[i];

                if tx_f_i == tx_f_j && tx_i != tx_j {
                    panic!("Assertion Failed: different transactions have the same fingerprint: {:?} and {:?}", tx_i, tx_j)
                }
            }
        }

        println!("Done: {}", Utc::now());

        Ok(())
    }

    #[test]
    pub fn compact_test() -> Result<(), Error> {
        let mut rng = OsRng;
        let fr = Fr::random(&mut rng);
        let compact_fr = fr.compact();
        let back_to_fr: Fr = Compact::unwrap(&compact_fr)?;

        assert_eq!(fr, back_to_fr);
        Ok(())
    }
}
