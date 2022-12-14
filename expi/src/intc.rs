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

/// Address of the basic pending register.
const INTBASICPEND: usize = INTC_BASE;

/// Base address of the GPU pending registers.
const INTGPUPEND_BASE: usize = INTC_BASE + 0x4;

/// Address of the FIQ control register.
const FIQCTL: usize = INTC_BASE + 0xc;

/// Base address of the interrupt enable registers.
const INTEN_BASE: usize = INTC_BASE + 0x10;

/// Base address of the interrupt disable registers.
const INTDIS_BASE: usize = INTC_BASE + 0x1c;

/// Number of GPU IRQs.
const NGPUIRQS: usize = 64;

/// Represents a GPU IRQ.
#[derive(Debug, Copy, Clone)]
pub struct GpuIrq(usize);

impl TryFrom<usize> for GpuIrq {
    type Error = Error;

    fn try_from(irq: usize) -> Result<GpuIrq> {
        if irq >= NGPUIRQS {
            return Err(Error::InvalidGpuIrq(irq));
        }
        Ok(GpuIrq(irq))
    }
}

impl From<GpuIrq> for usize {
    fn from(irq: GpuIrq) -> usize {
        irq.0
    }
}

/// BCM2837 interrupt source.
#[derive(Debug, Copy, Clone)]
pub enum Source {
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

    /// Illegal Access type 0.
    IllegalAccess0,

    /// Generic GPU interrupt.
    Gpu(GpuIrq),
}

impl TryFrom<Source> for GpuIrq {
    type Error = Error;

    fn try_from(source: Source) -> Result<GpuIrq> {
        let irq = match source {
            Source::Aux => GpuIrq(29),
            Source::I2cSpiSlv => GpuIrq(43),
            Source::Pwa0 => GpuIrq(45),
            Source::Pwa1 => GpuIrq(46),
            Source::Smi => GpuIrq(48),
            Source::GPIO => GpuIrq(52),
            Source::I2c => GpuIrq(53),
            Source::Spi => GpuIrq(54),
            Source::Pcm => GpuIrq(55),
            Source::Uart => GpuIrq(57),
            Source::Gpu(irq) => irq,
            _ => return Err(Error::UnknownGpuIrq),
        };
        Ok(irq)
    }
}

/// Interrupt register.
enum IntReg {
    /// GPU interrupt register 1.
    Gpu1,

    /// GPU interrupt register 2.
    Gpu2,

    /// Basic interrupt register.
    Basic,
}

impl From<IntReg> for usize {
    fn from(reg: IntReg) -> usize {
        match reg {
            IntReg::Gpu1 => 0,
            IntReg::Gpu2 => 1,
            IntReg::Basic => 2,
        }
    }
}

/// Provides the register and bit position required to configure a given
/// source.
struct IntBit(IntReg, usize);

impl From<Source> for IntBit {
    fn from(source: Source) -> IntBit {
        match source {
            Source::Aux => IntBit(IntReg::Gpu1, 29),
            Source::I2cSpiSlv => IntBit(IntReg::Gpu2, 11),
            Source::Pwa0 => IntBit(IntReg::Gpu2, 13),
            Source::Pwa1 => IntBit(IntReg::Gpu2, 14),
            Source::Smi => IntBit(IntReg::Gpu2, 16),
            Source::GPIO => IntBit(IntReg::Gpu2, 20),
            Source::I2c => IntBit(IntReg::Gpu2, 21),
            Source::Spi => IntBit(IntReg::Gpu2, 22),
            Source::Pcm => IntBit(IntReg::Gpu2, 23),
            Source::Uart => IntBit(IntReg::Gpu2, 25),
            Source::ArmTimer => IntBit(IntReg::Basic, 0),
            Source::ArmMailbox => IntBit(IntReg::Basic, 1),
            Source::ArmDoorbell0 => IntBit(IntReg::Basic, 2),
            Source::ArmDoorbell1 => IntBit(IntReg::Basic, 3),
            Source::Gpu0Halted => IntBit(IntReg::Basic, 4),
            Source::Gpu1Halted => IntBit(IntReg::Basic, 5),
            Source::IllegalAccess1 => IntBit(IntReg::Basic, 6),
            Source::IllegalAccess0 => IntBit(IntReg::Basic, 7),
            Source::Gpu(irq) => match irq.0 {
                0..=31 => IntBit(IntReg::Gpu1, irq.0),
                32..=63 => IntBit(IntReg::Gpu2, irq.0 - 32),
                _ => unreachable!(),
            },
        }
    }
}

/// Represents an FIQ source.
struct FiqSource(u32);

impl From<Source> for FiqSource {
    fn from(source: Source) -> FiqSource {
        match source {
            Source::Aux => FiqSource(29),
            Source::I2cSpiSlv => FiqSource(43),
            Source::Pwa0 => FiqSource(45),
            Source::Pwa1 => FiqSource(46),
            Source::Smi => FiqSource(48),
            Source::GPIO => FiqSource(52),
            Source::I2c => FiqSource(53),
            Source::Spi => FiqSource(54),
            Source::Pcm => FiqSource(55),
            Source::Uart => FiqSource(57),
            Source::ArmTimer => FiqSource(64),
            Source::ArmMailbox => FiqSource(65),
            Source::ArmDoorbell0 => FiqSource(66),
            Source::ArmDoorbell1 => FiqSource(67),
            Source::Gpu0Halted => FiqSource(68),
            Source::Gpu1Halted => FiqSource(69),
            Source::IllegalAccess1 => FiqSource(70),
            Source::IllegalAccess0 => FiqSource(71),
            Source::Gpu(irq) => FiqSource(irq.0 as u32),
        }
    }
}

/// Shows which interrupts are pending.
#[derive(Debug)]
pub struct PendingBasic {
    /// ARM Timer IRQ pending.
    pub arm_timer: bool,

    /// ARM Mailbox IRQ pending.
    pub arm_mailbox: bool,

    /// ARM Doorbell 0 IRQ pending.
    pub arm_doorbell_0: bool,

    /// ARM Doorbell 1 IRQ pending.
    pub arm_doorbell_1: bool,

    /// GPU0 halted IRQ pending.
    pub gpu0_halted: bool,

    /// GPU1 halted IRQ pending.
    pub gpu1_halted: bool,

    /// Illegal access type 1 IRQ pending.
    pub illegal_access_1: bool,

    /// Illegal access type 0 IRQ pending.
    pub illegal_access_0: bool,

    /// GPU IRQ pending in the range 0:31, which contains: [Source::Aux].
    pub pending_reg_1: bool,

    /// GPU IRQ pending in the range 32:63, which contains:
    /// [Source::I2cSpiSlv], [Source::Pwa0], [Source::Pwa1], [Source::Smi],
    /// [Source::GPIO], [Source::I2c], [Source::Spi], [Source::Pcm] and
    /// [Source::Uart].
    pub pending_reg_2: bool,

    /// GPU IRQ 7.
    pub gpu_irq_7: bool,

    /// GPU IRQ 9.
    pub gpu_irq_9: bool,

    /// GPU IRQ 10.
    pub gpu_irq_10: bool,

    /// GPU IRQ 18.
    pub gpu_irq_18: bool,

    /// GPU IRQ 19.
    pub gpu_irq_19: bool,

    /// GPU IRQ 53.
    pub gpu_irq_53: bool,

    /// GPU IRQ 54.
    pub gpu_irq_54: bool,

    /// GPU IRQ 55.
    pub gpu_irq_55: bool,

    /// GPU IRQ 56.
    pub gpu_irq_56: bool,

    /// GPU IRQ 57.
    pub gpu_irq_57: bool,

    /// GPU IRQ 62.
    pub gpu_irq_62: bool,
}

/// Enables interrupts for the provided source.
pub fn enable(source: Source) {
    let bit = IntBit::from(source);
    let idx = usize::from(bit.0);
    let addr = INTEN_BASE + idx * 4;
    unsafe { mmio::write(addr, 1 << bit.1) };
}

/// Disables interrupts for the provided source.
pub fn disable(source: Source) {
    let bit = IntBit::from(source);
    let idx = usize::from(bit.0);
    let addr = INTDIS_BASE + idx * 4;
    unsafe { mmio::write(addr, 1 << bit.1) };
}

/// Select which interrupt source can generate a FIQ to the ARM. Only a single
/// interrupt can be selected.
pub fn enable_fiq(source: Source) {
    // Make sure the IRQ is disabled for the source. Otherwise, both the IRQ
    // and the FIQ would be triggered.
    disable(source);

    // Enable FIQ.
    let fiq_source = FiqSource::from(source);
    let reg = (1 << 7) | (fiq_source.0 & 0b11_1111);
    unsafe { mmio::write(FIQCTL, reg) };
}

/// Disable FIQs.
pub fn disable_fiq() {
    unsafe { mmio::write(FIQCTL, 0) };
}

/// Returns the parsed basic pending register.
pub fn pending_basic() -> PendingBasic {
    let reg = unsafe { mmio::read(INTBASICPEND) };

    PendingBasic {
        arm_timer: reg & 1 != 0,
        arm_mailbox: reg & (1 << 1) != 0,
        arm_doorbell_0: reg & (1 << 2) != 0,
        arm_doorbell_1: reg & (1 << 3) != 0,
        gpu0_halted: reg & (1 << 4) != 0,
        gpu1_halted: reg & (1 << 5) != 0,
        illegal_access_1: reg & (1 << 6) != 0,
        illegal_access_0: reg & (1 << 7) != 0,
        pending_reg_1: reg & (1 << 8) != 0,
        pending_reg_2: reg & (1 << 9) != 0,
        gpu_irq_7: reg & (1 << 10) != 0,
        gpu_irq_9: reg & (1 << 11) != 0,
        gpu_irq_10: reg & (1 << 12) != 0,
        gpu_irq_18: reg & (1 << 13) != 0,
        gpu_irq_19: reg & (1 << 14) != 0,
        gpu_irq_53: reg & (1 << 15) != 0,
        gpu_irq_54: reg & (1 << 16) != 0,
        gpu_irq_55: reg & (1 << 17) != 0,
        gpu_irq_56: reg & (1 << 18) != 0,
        gpu_irq_57: reg & (1 << 19) != 0,
        gpu_irq_62: reg & (1 << 20) != 0,
    }
}

/// Returns the pending status of the GPU interrupts.
pub fn pending_gpu() -> [bool; NGPUIRQS] {
    let mut pending = [false; NGPUIRQS];
    for i in 0..2 {
        let addr = INTGPUPEND_BASE + i * 4;
        let reg = unsafe { mmio::read(addr) };
        for j in 0..32 {
            pending[i * 32 + j] = reg & (1 << j) != 0;
        }
    }
    pending
}

/// Returns true if the provided source is pending in `status`. `source` must
/// be a GPU interrupt.
pub fn is_pending(status: &[bool; NGPUIRQS], source: Source) -> Result<bool> {
    let irq = GpuIrq::try_from(source)?;
    Ok(status[irq.0])
}
