//! Decoders for common FDT properties.

use core::convert::TryInto;
use core::iter::FusedIterator;

use crate::fdt::Error;

/// Iterator over the entries of a `reg` property.
///
/// It yields a `Result` with the tuple `(address, size)` for every entry.
/// After an error, all successive calls will yield `None`.
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

    /// If `done` is true, the `Iterator` has finished.
    done: bool,
}

impl<T: AsRef<[u8]>> Reg<T> {
    /// Creates a [`Reg`] iterator.
    pub fn new(reg: T, address_cells: u32, size_cells: u32) -> Reg<T> {
        Reg {
            address_cells: address_cells as usize,
            size_cells: size_cells as usize,
            bytes: reg,
            idx: 0,
            done: false,
        }
    }

    /// Executes a new iteration. It is called by `Iterator::next`.
    fn iter_next(&mut self) -> Result<Option<(usize, usize)>, Error> {
        let bytes = self.bytes.as_ref();

        if self.idx >= bytes.len() {
            return Ok(None);
        }

        let address_idx = self.idx;
        let size_idx = address_idx + self.address_cells * 4;
        let end_idx = size_idx + self.size_cells * 4;

        let address_bytes =
            bytes.get(address_idx..size_idx).ok_or(Error::OutOfBounds)?;
        let address = usize_from_be_bytes(address_bytes)?;
        let size_bytes =
            bytes.get(size_idx..end_idx).ok_or(Error::OutOfBounds)?;
        let size = usize_from_be_bytes(size_bytes)?;

        self.idx = end_idx;

        Ok(Some((address, size)))
    }
}

impl<T: AsRef<[u8]>> Iterator for Reg<T> {
    type Item = Result<(usize, usize), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let retval = self.iter_next();

        self.done = match retval {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => true,
        };

        retval.transpose()
    }
}

impl<T: AsRef<[u8]>> FusedIterator for Reg<T> {}

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
