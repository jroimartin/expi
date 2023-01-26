//! Decoders for common FDT properties.

use core::convert::TryInto;

use crate::fdt::Error;

/// Size of the array used to store the reg property entries.
const REG_SIZE: usize = 32;

/// The devicetree `reg` property.
#[derive(Debug)]
pub struct Reg {
    /// The entries of the `reg` property.
    entries: [(usize, usize); REG_SIZE],

    /// The number of entries of the fixed-size array that are in use.
    in_use: usize,
}

impl Reg {
    /// Decodes a devicetree `reg` property.
    pub fn decode(
        reg: &[u8],
        address_cells: &[u8],
        size_cells: &[u8],
    ) -> Result<Reg, Error> {
        let address_cells =
            u32::from_be_bytes(address_cells.try_into()?) as usize;
        let size_cells = u32::from_be_bytes(size_cells.try_into()?) as usize;

        let mut entries = [(0, 0); REG_SIZE];
        let mut in_use = 0;

        loop {
            let address_idx = in_use * (address_cells + size_cells) * 4;
            let size_idx = address_idx + address_cells * 4;
            let end_idx = size_idx + size_cells * 4;

            if end_idx > reg.len() {
                break;
            }

            if in_use >= REG_SIZE {
                return Err(Error::FullInternalArray);
            }

            let address = usize_from_bytes(&reg[address_idx..size_idx])?;
            let size = usize_from_bytes(&reg[size_idx..end_idx])?;
            entries[in_use] = (address, size);
            in_use += 1;
        }

        Ok(Reg { entries, in_use })
    }

    /// Returns the entries of the `reg` property. An entry has the format
    /// `(address, size)`.
    pub fn entries(&self) -> &[(usize, usize)] {
        &self.entries[..self.in_use]
    }
}

/// Creates a native endian integer from its representation as a byte array in
/// big endian and converts it to `usize`.
fn usize_from_bytes(bytes: impl AsRef<[u8]>) -> Result<usize, Error> {
    let bytes = bytes.as_ref();

    match bytes.len() {
        1 => Ok(usize::from(u8::from_be_bytes(bytes.try_into()?))),
        2 => Ok(usize::from(u16::from_be_bytes(bytes.try_into()?))),
        4 => Ok(usize::try_from(u32::from_be_bytes(bytes.try_into()?))?),
        8 => Ok(usize::try_from(u64::from_be_bytes(bytes.try_into()?))?),
        _ => Err(Error::ConversionError),
    }
}
