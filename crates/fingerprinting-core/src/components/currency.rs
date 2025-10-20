use crate::components::FingerprintComponent;
use std::io::Write;

#[derive(Debug)]
pub struct CurrencyComponent {
    currency_code: u16,
}

impl FingerprintComponent<u16, 2> for CurrencyComponent {
    fn new(original: u16) -> Self {
        Self {
            currency_code: original,
        }
    }

    fn serialize<W: Write>(&self, buffer: &mut W) -> Result<(), anyhow::Error> {
        let written = buffer.write(&self.currency_code.to_be_bytes())?;

        debug_assert_eq!(written, Self::size());
        Ok(())
    }

    fn raw(&self) -> &u16 {
        &self.currency_code
    }
}
