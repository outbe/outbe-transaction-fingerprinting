use chrono::{DateTime, NaiveDate, Utc};
use derive_builder::Builder;
use fixed_num::Dec19x19;
use fixed_num_helper::FRAC_SCALE_I128;

// Amount with currency representation
#[derive(Default, Builder, Debug, Clone, PartialEq)]
#[builder(setter(into))]
pub struct Money {
    pub amount_base: u64,
    pub amount_atto: u64,
    pub currency: String,
}

// Raw Transaction representation
#[derive(Default, Builder, Debug, Clone, PartialEq)]
#[builder(setter(into))]
pub struct RawTransaction {
    pub bic: String,
    pub amount: Money,
    pub date_time: DateTime<Utc>,
    pub wwd: NaiveDate,
}

impl From<(Dec19x19, &str)> for Money {
    fn from(value: (Dec19x19, &str)) -> Self {
        let amount = value.0;
        let currency = value.1.to_string();
        Money {
            amount_base: (amount.repr / FRAC_SCALE_I128) as u64,
            amount_atto: (amount.repr % FRAC_SCALE_I128) as u64 / 10,
            currency,
        }
    }
}

impl From<(i32, &str)> for Money {
    fn from(value: (i32, &str)) -> Self {
        let currency = value.1.to_string();
        Money {
            amount_base: value.0.abs() as u64,
            amount_atto: 0,
            currency,
        }
    }
}
impl From<(u32, &str)> for Money {
    fn from(value: (u32, &str)) -> Self {
        let currency = value.1.to_string();
        Money {
            amount_base: value.0 as u64,
            amount_atto: 0,
            currency,
        }
    }
}
impl From<(i64, &str)> for Money {
    fn from(value: (i64, &str)) -> Self {
        let currency = value.1.to_string();
        Money {
            amount_base: value.0.abs() as u64,
            amount_atto: 0,
            currency,
        }
    }
}
impl From<(u64, &str)> for Money {
    fn from(value: (u64, &str)) -> Self {
        let currency = value.1.to_string();
        Money {
            amount_base: value.0,
            amount_atto: 0,
            currency,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_money_from() {
        let money_1 = MoneyBuilder::default()
            .amount_base(1000u32)
            .amount_atto(554325 * 10u64.pow(12)) // .5
            .currency("USD")
            .build()
            .unwrap();

        let money_2: Money = (Dec19x19!(1000.554325), "USD").into();

        println!("Builder money:{:?}", money_1);
        println!("Converted money:{:?}", money_2);

        assert_eq!(money_1, money_2);
    }
}
