//! Utilities for dealing with memory through pointers.

use crate::binary::FromBytes;

/// Size of the internal buffer used by [`MemReader`] to store [`FromBytes`]
/// values.
const MEM_READER_BUF_SIZE: usize = 1024;

/// Error while dealing with pointers.
#[derive(Debug)]
pub enum Error {
    /// The fixed size buffer used by [`MemReader`] is full.
    MemReaderBufFull,
}

/// A pointer to an arbitrary memory location.
#[derive(Debug, Copy, Clone)]
pub struct Ptr(usize);

impl From<usize> for Ptr {
    fn from(ptr: usize) -> Ptr {
        Ptr(ptr)
    }
}

impl From<Ptr> for usize {
    fn from(ptr: Ptr) -> usize {
        ptr.0
    }
}

/// Allows to read memory. It maintains an internal cursor that advances with
/// every read. So sucessive reads return consecutive memory locations.
pub struct MemReader {
    /// Current position.
    pos: Ptr,
}

impl MemReader {
    /// Creates a [`MemReader`] and sets its internal memory position to `pos`.
    pub fn new(pos: Ptr) -> MemReader {
        MemReader { pos }
    }

    /// Reads bytes from the current memory position into `buf`. The number of
    /// bytes read is exactly the size of the provided slice.
    ///
    /// # Safety
    ///
    /// The user is free to point the internal reader position to any memory
    /// location, therefore this function is unsafe.
    pub unsafe fn read(&mut self, buf: &mut [u8]) {
        core::ptr::copy(
            self.pos.0 as *const u8,
            buf.as_ptr() as *mut u8,
            buf.len(),
        );
        self.pos.0 += buf.len();
    }

    /// Reads a [`FromBytes`] value from its representation as a byte array in
    /// little endian.
    ///
    /// # Safety
    ///
    /// The user is free to point the internal reader position to any memory
    /// location, therefore this function is unsafe.
    pub unsafe fn read_le<T: FromBytes>(&mut self) -> Result<T, Error> {
        let tsz = core::mem::size_of::<T>();
        if tsz > MEM_READER_BUF_SIZE {
            return Err(Error::MemReaderBufFull);
        }

        let mut buf = [0u8; MEM_READER_BUF_SIZE];
        self.read(&mut buf[..tsz]);
        Ok(T::from_le_bytes(&buf[..tsz]))
    }

    /// Reads a [`FromBytes`] value from its representation as a byte array in
    /// big endian.
    ///
    /// # Safety
    ///
    /// The user is free to point the internal reader position to any memory
    /// location, therefore this function is unsafe.
    pub unsafe fn read_be<T: FromBytes>(&mut self) -> Result<T, Error> {
        let tsz = core::mem::size_of::<T>();
        if tsz > MEM_READER_BUF_SIZE {
            return Err(Error::MemReaderBufFull);
        }

        let mut buf = [0u8; MEM_READER_BUF_SIZE];
        self.read(&mut buf[..tsz]);
        Ok(T::from_be_bytes(&buf[..tsz]))
    }

    /// Sets the memory position of the next read. Setting the internal
    /// position is a safe operation, however reading is unsafe.
    pub fn set_position(&mut self, pos: Ptr) {
        self.pos = pos
    }

    /// Returns the current memory position.
    pub fn position(&self) -> Ptr {
        self.pos
    }

    /// Skips `n` bytes.
    pub fn skip(&mut self, n: usize) {
        self.pos.0 += n
    }
}
