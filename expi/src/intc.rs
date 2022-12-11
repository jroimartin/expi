//! Driver for the BCM2837 interrupt controller.
//!
//! For more information, please see [BCM2835 ARM Peripherals specification].
//! The underlying architecture of the BCM2837 is identical to the BCM2835.
//!
//! [BCM2835 ARM Peripherals specification]: https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf

use crate::mmio;

/// Base address of the interrupt controller.
///
/// [/arch/arm/boot/dts/bcm2837.dtsi] describes it:
///
/// ```text
/// &intc {
///     compatible = "brcm,bcm2836-armctrl-ic";
///     reg = <0x7e00b200 0x200>;
///     interrupt-parent = <&local_intc>;
///     interrupts = <8 IRQ_TYPE_LEVEL_HIGH>;
/// };
/// ```
///
/// [/arch/arm/boot/dts/bcm2837.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm2837.dtsi
const IRQ_BASE: usize = 0xb200;

/// Base address of the IRQ enable registers.
const IRQEN_BASE: usize = IRQ_BASE + 0x10;

/// Base address of the IRQ disable registers.
const IRQDIS_BASE: usize = IRQ_BASE + 0x1c;

/// Provides the required information to configure a specific peripheral.
struct RegBit {
    /// Register index.
    idx: usize,

    /// Bit position.
    bit: u32,
}

/// BCM2837 peripherals.
pub enum Peripheral {
    /// GPIO.
    GPIO,
}

impl From<Peripheral> for RegBit {
    fn from(peripheral: Peripheral) -> RegBit {
        match peripheral {
            Peripheral::GPIO => RegBit { idx: 1, bit: 20 },
        }
    }
}

/// Enables the interrupts of the provided peripheral.
pub fn enable(peripheral: Peripheral) {
    let reg_bit: RegBit = peripheral.into();
    let addr = IRQEN_BASE + reg_bit.idx * 4;
    unsafe { mmio::write(addr, 1 << reg_bit.bit) };
}

/// Disables the interrupts of the provided peripheral.
pub fn disable(peripheral: Peripheral) {
    let reg_bit: RegBit = peripheral.into();
    let addr = IRQDIS_BASE + reg_bit.idx * 4;
    unsafe { mmio::write(addr, 1 << reg_bit.bit) };
}
