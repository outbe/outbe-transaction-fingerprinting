use crate::components::FingerprintComponent;
use bigint::U256;
use std::io::Write;

#[derive(Debug)]
pub struct AmountComponent {
    base: u64,
    atto: u64,
    original: (u64, u64),
}

impl FingerprintComponent<(u64, u64), 32> for AmountComponent {
    fn new(original: (u64, u64)) -> Self {
        Self {
            base: original.0,
            atto: original.1,
            original,
        }
    }

    fn serialize<W: Write>(&self, buffer: &mut W) -> Result<(), anyhow::Error> {
        // 256-bit unsigned integer, big-endian
        // All amounts converted to smallest unit (atto) to eliminate decimal formatting differences

        // build uniform u256 with atto
        let full_amount = U256::from(self.base) * U256::from(10 ^ 18) + U256::from(self.atto);
        let mut full_amount_buffer = [0u8; 32];
        full_amount.to_big_endian(&mut full_amount_buffer);

        let written = buffer.write(&full_amount_buffer)?;

        debug_assert_eq!(written, Self::size());
        Ok(())
    }

    fn raw(&self) -> &(u64, u64) {
        &self.original
    }
}
