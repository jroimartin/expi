//! Memory mapped I/O operations.
//!
//! For more information, please see [BCM2835 ARM Peripherals specification].
//!
//! [BCM2835 ARM Peripherals specification]: https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf

use core::ptr::{read_volatile, write_volatile};

/// MMIO base address.
///
/// [/arch/arm/boot/dts/bcm2837.dtsi] defines the following mapping:
///
/// ```text
/// <0x7e000000 0x3f000000 0x1000000>
/// ```
///
/// [/arch/arm/boot/dts/bcm2837.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm2837.dtsi#L9-L10
const MMIO_BASE: usize = 0x3f000000;

/// Read register. `reg` is the offset of the register from the MMIO base
/// address.
///
/// # Safety
///
/// This function reads an arbitrary memory address, thus it is unsafe.
pub unsafe fn read(reg: usize) -> u32 {
    read_volatile((MMIO_BASE + reg) as *const u32)
}

/// Write value into register. `reg` is the offset of the register from the
/// MMIO base address.
///
/// # Safety
///
/// This function writes to an arbitrary memory address, thus it is unsafe.
pub unsafe fn write(reg: usize, val: u32) {
    write_volatile((MMIO_BASE + reg) as *mut u32, val)
}
