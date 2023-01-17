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
//! Local peripheral drivers are based on the [BCM2836 ARM-local Peripherals
//! specification], which also applies to the BCM2837.
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
//! [BCM2836 ARM-local Peripherals specification]: https://datasheets.raspberrypi.com/bcm2836/bcm2836-peripherals.pdf
//! [flatelf]: https://github.com/jroimartin/flatelf/

#![feature(panic_info_message)]
#![no_std]

extern crate alloc;

pub mod binary;
pub mod cpu;
pub mod devicetree;
pub mod globals;
pub mod gpio;
pub mod intc;
pub mod local_intc;
pub mod local_timer;
pub mod mailbox;
pub mod mm;
pub mod mmio;
pub mod print;
pub mod ptr;
pub mod system_timer;
pub mod uart;
