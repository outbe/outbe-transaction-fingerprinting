use crate::components::FingerprintComponent;
use anyhow::Error;
use std::io::Write;

/// Address is a wrapper around a 32-byte array.
/// NB: Actually Ethereum EOA has 20 bytes but here we allow more length for compatibility with
/// other possible addresses and derivatives
pub type AddressBytes = [u8; 32];

/// AddressComponent implements FingerprintComponent for Address
/// It serializes the full 32-byte address into the fingerprint buffer
#[derive(Debug)]
pub struct AddressComponent {
    address: [u8; 32],
}

impl FingerprintComponent<AddressBytes, 32> for AddressComponent {
    fn new(original: AddressBytes) -> Self {
        Self { address: original }
    }

    fn serialize<W: Write>(&self, buffer: &mut W) -> Result<(), Error> {
        // Write all 32 bytes of the address to the buffer
        let written = buffer.write(self.address.as_ref())?;

        debug_assert_eq!(written, Self::size());
        Ok(())
    }

    fn raw(&self) -> &AddressBytes {
        &self.address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_component_serialize() {
        let address: AddressBytes = [1u8; 32];
        let component = AddressComponent::new(address);

        let mut buffer = Vec::new();
        component.serialize(&mut buffer).unwrap();

        assert_eq!(buffer.len(), 32);
        assert_eq!(buffer, address.to_vec());
    }

    #[test]
    fn test_address_component_size() {
        assert_eq!(AddressComponent::size(), 32);
    }

    #[test]
    fn test_address_component_raw() {
        let address: AddressBytes = [2u8; 32];
        let component = AddressComponent::new(address);

        assert_eq!(component.raw(), &address);
    }
}
