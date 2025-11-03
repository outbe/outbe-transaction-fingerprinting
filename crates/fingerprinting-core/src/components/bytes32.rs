use crate::components::FingerprintComponent;
use anyhow::Error;
use std::io::Write;

/// Bytes32 is a wrapper around a 32-byte array.
pub type Bytes32 = [u8; 32];

/// Bytes32Component implements FingerprintComponent for Bytes32
/// It serializes the full 32-byte address into the fingerprint buffer
#[derive(Debug)]
pub struct Bytes32Component {
    data: [u8; 32],
}

impl FingerprintComponent<Bytes32, 32> for Bytes32Component {
    fn new(original: Bytes32) -> Self {
        Self { data: original }
    }

    fn serialize<W: Write>(&self, buffer: &mut W) -> Result<(), Error> {
        // Write all 32 bytes of the address to the buffer
        let written = buffer.write(self.data.as_ref())?;

        debug_assert_eq!(written, Self::size());
        Ok(())
    }

    fn raw(&self) -> &Bytes32 {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_component_serialize() {
        let address: Bytes32 = [1u8; 32];
        let component = Bytes32Component::new(address);

        let mut buffer = Vec::new();
        component.serialize(&mut buffer).unwrap();

        assert_eq!(buffer.len(), 32);
        assert_eq!(buffer, address.to_vec());
    }

    #[test]
    fn test_address_component_size() {
        assert_eq!(Bytes32Component::size(), 32);
    }

    #[test]
    fn test_address_component_raw() {
        let address: Bytes32 = [2u8; 32];
        let component = Bytes32Component::new(address);

        assert_eq!(component.raw(), &address);
    }
}
