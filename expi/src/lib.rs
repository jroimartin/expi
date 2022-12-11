//! expi simplifies writing kernels for the Raspberry Pi 3 Model B.
//!
//! Although there are other methods, the documentation in this crate expects
//! you to use [flatelf] to generate the kernel image and will provide you with
//! the required linker arguments to make it possible.
//!
//! flatelf does not apply relocations, so we need to configure the relocation
//! model as static. Also, we won't make assumptions about addresses and sizes
//! of sections, meaning that we will configure the code-model as large.
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
//! [flatelf]: https://github.com/jroimartin/flatelf/

#![no_std]
#![feature(panic_info_message)]

pub mod cpu;
pub mod gpio;
pub mod intc;
pub mod mailbox;
pub mod mmio;
pub mod print;
pub mod uart;

/// Expi error.
#[derive(Debug)]
pub enum Error {
    /// Invalid GPIO pin.
    InvalidGpioPin(u32),

    /// The size of the provided output parameter is not valid.
    InvalidOutputSize,

    /// Mailbox request could not be processed.
    MailboxRequestFailed,

    /// There is not enough room in the mailbox buffer to allocate the request.
    MailboxRequestIsTooBig,
}
