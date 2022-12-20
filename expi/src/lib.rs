//! expi simplifies writing kernels for the Raspberry Pi 3 Model B.
//!
//! expi does not aim to be used to build general purpose Operating Systems. We
//! typically run in EL2. Thus, the functionalities exposed by this crate are
//! in many cases (e.g. exception handling) limited to this use case.
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
pub mod uart;
