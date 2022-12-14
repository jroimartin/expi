//! Driver for the BCM2837 interrupt controller.
//!
//! For more information, please see [BCM2835 ARM Peripherals specification].
//! The underlying architecture of the BCM2837 is identical to the BCM2835.
//!
//! [BCM2835 ARM Peripherals specification]: https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf

use crate::mmio;
use crate::{Error, Result};

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
enum Reg {
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
#[derive(Debug, Copy, Clone)]
pub enum Peripheral {
    /// Aux.
    Aux,

    /// I2C SPI Slave.
    I2cSpiSlv,

    /// PWA0.
    Pwa0,

    /// PWA1.
    Pwa1,

    /// SMI.
    Smi,

    /// GPIO.
    GPIO,

    /// I2C.
    I2c,

    /// SPI.
    Spi,

    /// PCM.
    Pcm,

    /// UART.
    Uart,

    /// ARM Timer.
    ArmTimer,

    /// ARM Mailbox.
    ArmMailbox,

    /// ARM Doorbell 0.
    ArmDoorbell0,

    /// ARM Doorbell 1.
    ArmDoorbell1,

    /// GPU0 halted.
    Gpu0Halted,

    /// GPU1 halted.
    Gpu1Halted,

    /// Illegal Access type 1.
    IllegalAccess1,

    /// Illegal Access type 2.
    IllegalAccess2,

    /// Generic GPU interrupt.
    Gpu(u32),
}

impl TryFrom<Peripheral> for RegBit {
    type Error = Error;

    fn try_from(peripheral: Peripheral) -> Result<RegBit> {
        let reg_bit = match peripheral {
            Peripheral::Aux => RegBit(Reg::Gpu1, 29),
            Peripheral::I2cSpiSlv => RegBit(Reg::Gpu2, 11),
            Peripheral::Pwa0 => RegBit(Reg::Gpu2, 13),
            Peripheral::Pwa1 => RegBit(Reg::Gpu2, 14),
            Peripheral::Smi => RegBit(Reg::Gpu2, 16),
            Peripheral::GPIO => RegBit(Reg::Gpu2, 20),
            Peripheral::I2c => RegBit(Reg::Gpu2, 21),
            Peripheral::Spi => RegBit(Reg::Gpu2, 22),
            Peripheral::Pcm => RegBit(Reg::Gpu2, 23),
            Peripheral::Uart => RegBit(Reg::Gpu2, 25),
            Peripheral::ArmTimer => RegBit(Reg::Basic, 0),
            Peripheral::ArmMailbox => RegBit(Reg::Basic, 1),
            Peripheral::ArmDoorbell0 => RegBit(Reg::Basic, 2),
            Peripheral::ArmDoorbell1 => RegBit(Reg::Basic, 3),
            Peripheral::Gpu0Halted => RegBit(Reg::Basic, 4),
            Peripheral::Gpu1Halted => RegBit(Reg::Basic, 5),
            Peripheral::IllegalAccess1 => RegBit(Reg::Basic, 6),
            Peripheral::IllegalAccess2 => RegBit(Reg::Basic, 7),
            Peripheral::Gpu(n) => match n {
                0..=31 => RegBit(Reg::Gpu1, n),
                32..=63 => RegBit(Reg::Gpu2, n - 32),
                _ => return Err(Error::InvalidGpuInterrupt(n)),
            },
        };
        Ok(reg_bit)
    }
}

/// Represents an FIQ source.
struct FIQSource(u32);

impl TryFrom<Peripheral> for FIQSource {
    type Error = Error;

    fn try_from(peripheral: Peripheral) -> Result<FIQSource> {
        let fiq_source = match peripheral {
            Peripheral::Aux => FIQSource(29),
            Peripheral::I2cSpiSlv => FIQSource(43),
            Peripheral::Pwa0 => FIQSource(45),
            Peripheral::Pwa1 => FIQSource(46),
            Peripheral::Smi => FIQSource(48),
            Peripheral::GPIO => FIQSource(52),
            Peripheral::I2c => FIQSource(53),
            Peripheral::Spi => FIQSource(54),
            Peripheral::Pcm => FIQSource(55),
            Peripheral::Uart => FIQSource(57),
            Peripheral::ArmTimer => FIQSource(64),
            Peripheral::ArmMailbox => FIQSource(65),
            Peripheral::ArmDoorbell0 => FIQSource(66),
            Peripheral::ArmDoorbell1 => FIQSource(67),
            Peripheral::Gpu0Halted => FIQSource(68),
            Peripheral::Gpu1Halted => FIQSource(69),
            Peripheral::IllegalAccess1 => FIQSource(70),
            Peripheral::IllegalAccess2 => FIQSource(71),
            Peripheral::Gpu(n) => match n {
                0..=63 => FIQSource(n),
                _ => return Err(Error::InvalidGpuInterrupt(n)),
            },
        };
        Ok(fiq_source)
    }
}

/// Enables interrupts for the provided peripheral.
pub fn enable(peripheral: Peripheral) -> Result<()> {
    let reg_bit = RegBit::try_from(peripheral)?;
    let reg_idx = RegCtlIdx::from(reg_bit.0);
    let addr = INTEN_BASE + reg_idx.0 * 4;
    unsafe { mmio::write(addr, 1 << reg_bit.1) };
    Ok(())
}

/// Disables interrupts for the provided peripheral.
pub fn disable(peripheral: Peripheral) -> Result<()> {
    let reg_bit = RegBit::try_from(peripheral)?;
    let reg_idx = RegCtlIdx::from(reg_bit.0);
    let addr = INTDIS_BASE + reg_idx.0 * 4;
    unsafe { mmio::write(addr, 1 << reg_bit.1) };
    Ok(())
}

/// Returns if a given peripheral has a pending interrupt.
pub fn pending(peripheral: Peripheral) -> Result<bool> {
    let reg_bit = RegBit::try_from(peripheral)?;
    let reg_idx = RegPendingIdx::from(reg_bit.0);
    let addr = INTPEND_BASE + reg_idx.0 * 4;
    let reg = unsafe { mmio::read(addr) };
    Ok(reg & (1 << reg_bit.1) != 0)
}

/// Select which interrupt source can generate a FIQ to the ARM. Only a single
/// interrupt can be selected.
pub fn enable_fiq(peripheral: Peripheral) -> Result<()> {
    // Make sure the IRQ is disabled for the peripheral. Otherwise, both IRQ
    // and FIQ would be triggered.
    disable(peripheral)?;

    // Enable FIQ.
    let fiq_source = FIQSource::try_from(peripheral)?;
    let reg = (1 << 7) | (fiq_source.0 & 0b11_1111);
    unsafe { mmio::write(FIQCTL, reg) };
    Ok(())
}

/// Disable FIQs.
pub fn disable_fiq() {
    unsafe { mmio::write(FIQCTL, 0) };
}
