//! Decoders for common FDT properties.

use core::convert::TryInto;

use crate::fdt::Error;

/// Iterator over the entries of a `reg` property.
///
/// It yields a tupe with the format `(address, size)` for every entry.
#[derive(Debug)]
pub struct Reg<T> {
    /// Number of `<u32>` cells to represent the address in the reg property.
    address_cells: usize,

    /// Number of `<u32>` cells to represent the size in the reg property.
    size_cells: usize,

    /// `reg` bytes.
    bytes: T,

    /// Index of the next entry.
    idx: usize,
}

impl<T> Reg<T> {
    /// Creates a [`Reg`] iterator.
    pub fn decode(reg: T, address_cells: u32, size_cells: u32) -> Reg<T> {
        Reg {
            address_cells: address_cells as usize,
            size_cells: size_cells as usize,
            bytes: reg,
            idx: 0,
        }
    }
}

impl<T: AsRef<[u8]>> Iterator for Reg<T> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let address_idx = self.idx;
        let size_idx = address_idx + self.address_cells * 4;
        let end_idx = size_idx + self.size_cells * 4;

        let bytes = self.bytes.as_ref();

        if end_idx > bytes.len() {
            return None;
        }

        let address =
            usize_from_be_bytes(&bytes[address_idx..size_idx]).ok()?;
        let size = usize_from_be_bytes(&bytes[size_idx..end_idx]).ok()?;

        self.idx = end_idx;

        Some((address, size))
    }
}

/// Creates a native endian integer from its representation as a byte array in
/// big endian and converts it to `usize`.
fn usize_from_be_bytes(bytes: impl AsRef<[u8]>) -> Result<usize, Error> {
    let bytes = bytes.as_ref();

    match bytes.len() {
        1 => Ok(usize::from(u8::from_be_bytes(bytes.try_into()?))),
        2 => Ok(usize::from(u16::from_be_bytes(bytes.try_into()?))),
        4 => Ok(usize::try_from(u32::from_be_bytes(bytes.try_into()?))?),
        8 => Ok(usize::try_from(u64::from_be_bytes(bytes.try_into()?))?),
        _ => Err(Error::ConversionError),
    }
}
