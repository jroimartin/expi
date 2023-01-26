//! Utilities for dealing with memory through pointers.

use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;
use core::fmt;

use crate::binary::FromBytes;

/// Size of the internal buffer used by [`MemReader`] to store [`FromBytes`]
/// values.
const MEM_READER_BUF_SIZE: usize = 1024;

/// Error while dealing with memory through pointers.
#[derive(Debug)]
pub enum Error {
    /// The requested type is too big to fit in the fixed size buffer used by
    /// [`MemReader`].
    TypeSizeIsTooBig,

    /// The read bytes cannot be converted into an UTF-8 string.
    InvalidUtf8String(FromUtf8Error),
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Error {
        Error::InvalidUtf8String(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::TypeSizeIsTooBig => write!(f, "requested type is to big"),
            Error::InvalidUtf8String(err) => {
                write!(f, "invalid UTF-8 string: {err}")
            }
        }
    }
}

/// Allows to read memory. It maintains an internal cursor that advances with
/// every read. So sucessive reads return consecutive memory locations.
pub struct MemReader {
    /// Current position.
    pos: usize,
}

impl MemReader {
    /// Creates a [`MemReader`] and sets its internal memory position to `pos`.
    pub fn new(pos: usize) -> MemReader {
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
            self.pos as *const u8,
            buf.as_ptr() as *mut u8,
            buf.len(),
        );
        self.pos += buf.len();
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
            return Err(Error::TypeSizeIsTooBig);
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
            return Err(Error::TypeSizeIsTooBig);
        }

        let mut buf = [0u8; MEM_READER_BUF_SIZE];
        self.read(&mut buf[..tsz]);
        Ok(T::from_be_bytes(&buf[..tsz]))
    }

    /// Reads a null-terminated string.
    ///
    /// # Safety
    ///
    /// The user is free to point the internal reader position to any memory
    /// location, therefore this function is unsafe.
    pub unsafe fn read_cstr(&mut self) -> Result<String, Error> {
        let s = read_cstr(self.pos)?;
        self.pos += s.len() + 1;
        Ok(s)
    }

    /// Creates a raw slice from a null-terminated string.
    ///
    /// # Safety
    ///
    /// The user is free to point the internal reader position to any memory
    /// location, therefore this function is unsafe.
    pub unsafe fn slice_from_cstr(&mut self) -> *const [u8] {
        let s = slice_from_cstr(self.pos);
        self.pos += (*s).len() + 1;
        s
    }

    /// Sets the memory position of the next read. Setting the internal
    /// position is a safe operation, however reading is unsafe.
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos
    }

    /// Returns the current memory position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Skips `n` bytes.
    pub fn skip(&mut self, n: usize) {
        self.pos += n
    }
}

/// Reads a null-terminated string at `ptr`.
///
/// # Safety
///
/// This function accepts an arbitrary memory address, therefore it is unsafe.
pub unsafe fn read_cstr(ptr: usize) -> Result<String, Error> {
    let mut ptr = ptr as *const u8;
    let mut bytes = Vec::new();
    while *ptr != 0 {
        bytes.push(*ptr);
        ptr = ptr.add(1);
    }
    Ok(String::from_utf8(bytes)?)
}

/// Creates a raw slice from a pointer to a null-terminated string.
///
/// # Safety
///
/// This function accepts an arbitrary memory address, therefore it is unsafe.
pub unsafe fn slice_from_cstr(ptr: usize) -> *const [u8] {
    let mut cur = ptr as *const u8;
    while *cur != 0 {
        cur = cur.add(1);
    }
    let len = (cur as usize) - ptr;
    core::ptr::slice_from_raw_parts(ptr as *const u8, len)
}
