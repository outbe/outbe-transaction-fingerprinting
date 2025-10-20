use anyhow::anyhow;
use regex::Regex;
use std::io::Write;

use crate::components::FingerprintComponent;

#[derive(Debug)]
pub struct BankIdentifierComponent {
    bic: String,
}

impl FingerprintComponent<String, 6> for BankIdentifierComponent {
    fn new(original: String) -> Self {
        Self { bic: original }
    }

    fn serialize<W: Write>(&self, buffer: &mut W) -> Result<(), anyhow::Error> {
        // First 6 characters of the Bank Identifier Code
        // Truncating to 6 characters removes branch-specific details while maintaining bank identification,
        // normalizing variations from different aggregators

        // BIC Structure:
        // - 4-letter bank code,
        // - a 2-letter country code,
        // - a 2-character location code,
        // - an optional 3-character branch code

        // Firstly check the BIC is valid BIC
        // ([A-Z]{4})([A-Z]{2})([A-Z0-9]{2})([A-Z0-9]{3})?$

        let bic_validation = Regex::new(
            r"(?x)
(?P<bank_code>[A-Z]{4})  # 4-letter bank code
(?P<country_code>[A-Z]{2}) # 2-letter country code
(?P<location_code>[A-Z0-9]{2}) # 2-character location code
(?P<branch_code>[A-Z0-9]{3})? # optional 3-character branch code
$",
        )?;

        let bic = bic_validation
            .captures(&self.bic)
            .ok_or(anyhow!("BIC is invalid format, should be BBBBCCLLBRN"))?;

        let bank_code = &bic["bank_code"];
        let country_code = &bic["country_code"];

        let written = buffer.write(bank_code.as_bytes())?;
        let written = written + buffer.write(country_code.as_bytes())?;

        debug_assert_eq!(written, Self::size());
        Ok(())
    }

    fn raw(&self) -> &String {
        &self.bic
    }
}
