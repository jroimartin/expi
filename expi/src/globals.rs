//! Global resources.

use core::fmt;

use crate::mm;
use crate::uart;

use range::RangeSet;

/// Globals error.
#[derive(Debug)]
pub enum Error {
    /// UART error.
    UartError(uart::Error),

    /// Allocator error.
    AllocError(mm::AllocError),
}

impl From<uart::Error> for Error {
    fn from(err: uart::Error) -> Error {
        Error::UartError(err)
    }
}

impl From<mm::AllocError> for Error {
    fn from(err: mm::AllocError) -> Error {
        Error::AllocError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::UartError(err) => write!(f, "UART error: {err}"),
            Error::AllocError(err) => write!(f, "allocator error: {err}"),
        }
    }
}

/// Contains the global resources shared between modules.
pub struct GlobalResources {
    /// [`RangeSet`] with the free memory regions.
    free_memory: Option<RangeSet>,
}

/// Global resources shared between modules.
pub static mut GLOBALS: GlobalResources = GlobalResources::new();

impl GlobalResources {
    /// Creates a new [`GlobalResources`] structure.
    const fn new() -> GlobalResources {
        GlobalResources { free_memory: None }
    }

    /// Returns a reference to the list of free memory regions.
    pub fn free_memory_mut(&mut self) -> &mut Option<RangeSet> {
        &mut self.free_memory
    }
}

/// Initialize global resources like UART, global allocator, etc.
pub fn init(dtb_ptr32: u32) -> Result<(), Error> {
    uart::init()?;
    mm::init(dtb_ptr32)?;
    Ok(())
}
