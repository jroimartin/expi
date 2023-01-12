//! Global resources.

use core::fmt;
use core::panic::PanicInfo;

use crate::mm;
use crate::print::UartWriter;
use crate::uart;
use crate::{print, println};

use mutex::TicketMutex;
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
    free_memory: TicketMutex<Option<RangeSet>>,

    /// [`UartWriter`] used by the [`print!`] and [`println!`] macros to
    /// provide safe concurrent access to the UART.
    uart_writer: TicketMutex<Option<UartWriter>>,
}

/// Global resources shared between modules.
pub static GLOBALS: GlobalResources = GlobalResources::new();

impl GlobalResources {
    /// Creates a new [`GlobalResources`] structure.
    const fn new() -> GlobalResources {
        GlobalResources {
            free_memory: TicketMutex::new(None),
            uart_writer: TicketMutex::new(None),
        }
    }

    /// Returns a reference to the list of free memory regions.
    pub fn free_memory(&self) -> &TicketMutex<Option<RangeSet>> {
        &self.free_memory
    }

    /// Returns a reference to the UART writer.
    pub fn uart_writer(&self) -> &TicketMutex<Option<UartWriter>> {
        &self.uart_writer
    }
}

/// Global Allocator.
#[global_allocator]
static GLOBAL_ALLOCATOR: mm::GlobalAllocator = mm::GlobalAllocator;

/// Panic handler.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("\n\n!!! PANIC !!!\n\n");

    if let Some(location) = info.location() {
        print!("{}:{}", location.file(), location.line());
    }

    if let Some(message) = info.message() {
        println!(": {}", message);
    } else {
        println!();
    }

    loop {}
}

/// Initializes global resources. E.g. UART, global allocator.
///
/// It is required to configure the MMU before calling this function.
/// Otherwise, atomics won't work.
pub fn init(dtb_ptr32: u32) -> Result<(), Error> {
    uart::init()?;
    mm::init(dtb_ptr32)?;
    Ok(())
}
