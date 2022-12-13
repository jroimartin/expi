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
const INTC_BASE: usize = 0xb200;

/// Base address of the interrupt pending registers.
const INTPEND_BASE: usize = INTC_BASE;

/// Address of the FIQ control register.
const FIQCTL: usize = INTC_BASE + 0xc;

/// Base address of the interrupt enable registers.
const INTEN_BASE: usize = INTC_BASE + 0x10;

/// Base address of the interrupt disable registers.
const INTDIS_BASE: usize = INTC_BASE + 0x1c;

/// Interrupt register.
pub enum Reg {
    /// Basic interrupt register.
    Basic,

    /// GPU interrupt register 1.
    Gpu1,

    /// GPU interrupt register 2.
    Gpu2,
}

/// Index of an interrupt control register (i.e. interrupt enable/disable
/// register).
struct RegCtlIdx(usize);

impl From<Reg> for RegCtlIdx {
    fn from(reg: Reg) -> RegCtlIdx {
        match reg {
            Reg::Gpu1 => RegCtlIdx(0),
            Reg::Gpu2 => RegCtlIdx(1),
            Reg::Basic => RegCtlIdx(2),
        }
    }
}

/// Index of a pending register.
struct RegPendingIdx(usize);

impl From<Reg> for RegPendingIdx {
    fn from(reg: Reg) -> RegPendingIdx {
        match reg {
            Reg::Basic => RegPendingIdx(0),
            Reg::Gpu1 => RegPendingIdx(1),
            Reg::Gpu2 => RegPendingIdx(2),
        }
    }
}

/// Provides the register and bit position required to configure a given
/// peripheral.
struct RegBit(Reg, u32);

/// BCM2837 peripherals.
pub enum Peripheral {
    /// GPIO.
    GPIO,
}

impl From<Peripheral> for RegBit {
    fn from(peripheral: Peripheral) -> RegBit {
        match peripheral {
            Peripheral::GPIO => RegBit(Reg::Gpu2, 20),
        }
    }
}

/// Represents an FIQ source.
struct FIQSource(u32);

impl From<Peripheral> for FIQSource {
    fn from(peripheral: Peripheral) -> FIQSource {
        match peripheral {
            Peripheral::GPIO => FIQSource(52),
        }
    }
}

/// Enables interrupts for the provided peripheral.
pub fn enable(peripheral: Peripheral) {
    let reg_bit = RegBit::from(peripheral);
    let reg_idx = RegCtlIdx::from(reg_bit.0);
    let addr = INTEN_BASE + reg_idx.0 * 4;
    unsafe { mmio::write(addr, 1 << reg_bit.1) };
}

/// Disables interrupts for the provided peripheral.
pub fn disable(peripheral: Peripheral) {
    let reg_bit = RegBit::from(peripheral);
    let reg_idx = RegCtlIdx::from(reg_bit.0);
    let addr = INTDIS_BASE + reg_idx.0 * 4;
    unsafe { mmio::write(addr, 1 << reg_bit.1) };
}

/// Returns if a given peripheral has a pending interrupt.
pub fn pending(peripheral: Peripheral) -> bool {
    let reg_bit = RegBit::from(peripheral);
    let reg_idx = RegPendingIdx::from(reg_bit.0);
    let addr = INTPEND_BASE + reg_idx.0 * 4;
    let reg = unsafe { mmio::read(addr) };
    reg & (1 << reg_bit.1) != 0
}

/// Select which interrupt source can generate a FIQ to the ARM. Only a single
/// interrupt can be selected.
pub fn enable_fiq(peripheral: Peripheral) {
    let source = FIQSource::from(peripheral);
    let reg = (1 << 7) | (source.0 & 0b11_1111);
    unsafe { mmio::write(FIQCTL, reg) };
}

/// Disable FIQs.
pub fn disable_fiq() {
    unsafe { mmio::write(FIQCTL, 0) };
}
