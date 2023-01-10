//! expi simplifies writing kernels for the Raspberry Pi 3 Model B.
//!
//! expi does not aim to be used to build general purpose Operating Systems. We
//! typically run in EL2. Thus, the functionalities exposed by this crate are
//! in many cases (e.g. exception handling) limited to this use case.
//!
//! Peripheral drivers are based on the [BCM2835 ARM Peripherals
//! specification]. The underlying architecture of the BCM2837 is identical to
//! the BCM2835. It is important to note that this specification contains a
//! number of errors. However there is a list of currently known [errata].
//!
//! Although there are other methods, the documentation in this crate expects
//! you to use [flatelf] to generate the kernel image and will provide you with
//! the required linker arguments to make it possible.
//!
//! flatelf does not apply relocations, so we need to configure the relocation
//! model as static. Also, we won't make assumptions about addresses and sizes
//! of sections, meaning that we will configure the code model as large.
//! Finally we will turn off page alignment of sections to remove padding and
//! get smaller kernel images, which can be done with the linker argument
//! `--nmagic`.
//!
//! The following example shows how to do this using a Cargo configuration
//! file.
//!
//! ```text
//! [target.aarch64-unknown-none]
//! rustflags = [
//!     "-Ccode-model=large",
//!     "-Crelocation-model=static",
//!     "-Clink-arg=--nmagic",
//! ]
//! ```
//!
//! [BCM2835 ARM Peripherals specification]: https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf
//! [errata]: https://elinux.org/BCM2835_datasheet_errata
//! [flatelf]: https://github.com/jroimartin/flatelf/

#![feature(panic_info_message)]
#![no_std]

use core::fmt;

pub mod binary;
pub mod cpu;
pub mod devicetree;
pub mod globals;
pub mod gpio;
pub mod intc;
pub mod mailbox;
pub mod mm;
pub mod mmio;
pub mod print;
pub mod ptr;
pub mod systimer;
pub mod uart;

/// expi error.
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

/// Initializes global resources like the MMU, UART, global allocator, etc.
pub fn init(dtb_ptr32: u32) -> Result<(), Error> {
    cpu::mmu::enable_identity_mapping();
    uart::init()?;
    mm::init(dtb_ptr32)?;
    Ok(())
}
