//! Driver for the BCM2837 interrupt controller.

use core::fmt;

use crate::mmio;

/// Base address of the interrupt controller.
///
/// [/arch/arm/boot/dts/bcm2837.dtsi] describes it:
///
/// ```text
/// &intc {
///     compatible = "brcm,bcm2836-armctrl-ic";
///     reg = <0x7e00b200 0x200>;
///     ...
/// };
/// ```
///
/// [/arch/arm/boot/dts/bcm2837.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm2837.dtsi#L80-L85
const INTC_BASE: usize = 0xb200;

/// Address of the basic pending register.
const INTBASICPEND: usize = INTC_BASE;

/// Base address of the GPU pending registers.
const INTGPUPEND_BASE: usize = INTC_BASE + 0x4;

/// Address of the FIQ control register.
const FIQCTL: usize = INTC_BASE + 0xc;

/// Base address of the IRQ enable registers.
const INTEN_BASE: usize = INTC_BASE + 0x10;

/// Base address of the IRQ disable registers.
const INTDIS_BASE: usize = INTC_BASE + 0x1c;

/// Number of GPU IRQs.
const NGPUIRQS: usize = 64;

/// Interrupt controller error.
#[derive(Debug)]
pub enum Error {
    /// Invalid GPU IRQ number.
    InvalidGpuIrq(usize),

    /// Not a GPU interrupt.
    NotAGpuIrq,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidGpuIrq(irq) => write!(f, "invalid GPU IRQ: {irq}"),
            Error::NotAGpuIrq => write!(f, "not a GPU interrupt"),
        }
    }
}

/// Represents a GPU IRQ.
#[derive(Debug, Copy, Clone)]
pub struct GpuIrq(usize);

impl TryFrom<usize> for GpuIrq {
    type Error = Error;

    fn try_from(irq: usize) -> Result<GpuIrq, Error> {
        if irq >= NGPUIRQS {
            return Err(Error::InvalidGpuIrq(irq));
        }
        Ok(GpuIrq(irq))
    }
}

/// BCM2837 IRQ source.
#[derive(Debug, Copy, Clone)]
pub enum IrqSource {
    /// System Timer 0.
    SystemTimer0,

    /// System Timer 1.
    SystemTimer1,

    /// System Timer 2.
    SystemTimer2,

    /// System Timer 3.
    SystemTimer3,

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

    /// Generic GPU IRQ.
    Gpu(GpuIrq),
}

impl TryFrom<IrqSource> for GpuIrq {
    type Error = Error;

    fn try_from(src: IrqSource) -> Result<GpuIrq, Error> {
        let irq = match src {
            IrqSource::SystemTimer0 => GpuIrq(0),
            IrqSource::SystemTimer1 => GpuIrq(1),
            IrqSource::SystemTimer2 => GpuIrq(2),
            IrqSource::SystemTimer3 => GpuIrq(3),
            IrqSource::Aux => GpuIrq(29),
            IrqSource::I2cSpiSlv => GpuIrq(43),
            IrqSource::Pwa0 => GpuIrq(45),
            IrqSource::Pwa1 => GpuIrq(46),
            IrqSource::Smi => GpuIrq(48),
            IrqSource::GPIO => GpuIrq(52),
            IrqSource::I2c => GpuIrq(53),
            IrqSource::Spi => GpuIrq(54),
            IrqSource::Pcm => GpuIrq(55),
            IrqSource::Uart => GpuIrq(57),
            IrqSource::Gpu(irq) => irq,
            _ => return Err(Error::NotAGpuIrq),
        };
        Ok(irq)
    }
}

/// IRQ register.
enum IrqReg {
    /// GPU IRQ register 1.
    Gpu1,

    /// GPU IRQ register 2.
    Gpu2,

    /// Basic IRQ register.
    Basic,
}

impl From<IrqReg> for usize {
    fn from(reg: IrqReg) -> usize {
        match reg {
            IrqReg::Gpu1 => 0,
            IrqReg::Gpu2 => 1,
            IrqReg::Basic => 2,
        }
    }
}

/// Provides the register and bit position required to configure a given IRQ
/// source.
struct IrqBit(IrqReg, usize);

impl From<IrqSource> for IrqBit {
    fn from(src: IrqSource) -> IrqBit {
        match src {
            IrqSource::SystemTimer0 => IrqBit(IrqReg::Gpu1, 0),
            IrqSource::SystemTimer1 => IrqBit(IrqReg::Gpu1, 1),
            IrqSource::SystemTimer2 => IrqBit(IrqReg::Gpu1, 2),
            IrqSource::SystemTimer3 => IrqBit(IrqReg::Gpu1, 3),
            IrqSource::Aux => IrqBit(IrqReg::Gpu1, 29),
            IrqSource::I2cSpiSlv => IrqBit(IrqReg::Gpu2, 11),
            IrqSource::Pwa0 => IrqBit(IrqReg::Gpu2, 13),
            IrqSource::Pwa1 => IrqBit(IrqReg::Gpu2, 14),
            IrqSource::Smi => IrqBit(IrqReg::Gpu2, 16),
            IrqSource::GPIO => IrqBit(IrqReg::Gpu2, 20),
            IrqSource::I2c => IrqBit(IrqReg::Gpu2, 21),
            IrqSource::Spi => IrqBit(IrqReg::Gpu2, 22),
            IrqSource::Pcm => IrqBit(IrqReg::Gpu2, 23),
            IrqSource::Uart => IrqBit(IrqReg::Gpu2, 25),
            IrqSource::ArmTimer => IrqBit(IrqReg::Basic, 0),
            IrqSource::ArmMailbox => IrqBit(IrqReg::Basic, 1),
            IrqSource::ArmDoorbell0 => IrqBit(IrqReg::Basic, 2),
            IrqSource::ArmDoorbell1 => IrqBit(IrqReg::Basic, 3),
            IrqSource::Gpu0Halted => IrqBit(IrqReg::Basic, 4),
            IrqSource::Gpu1Halted => IrqBit(IrqReg::Basic, 5),
            IrqSource::IllegalAccess1 => IrqBit(IrqReg::Basic, 6),
            IrqSource::IllegalAccess0 => IrqBit(IrqReg::Basic, 7),
            IrqSource::Gpu(irq) => match irq.0 {
                0..=31 => IrqBit(IrqReg::Gpu1, irq.0),
                32..=63 => IrqBit(IrqReg::Gpu2, irq.0 - 32),
                _ => unreachable!(),
            },
        }
    }
}

/// Represents an FIQ source.
struct FiqSource(usize);

impl From<IrqSource> for FiqSource {
    fn from(src: IrqSource) -> FiqSource {
        match src {
            IrqSource::SystemTimer0 => FiqSource(0),
            IrqSource::SystemTimer1 => FiqSource(1),
            IrqSource::SystemTimer2 => FiqSource(2),
            IrqSource::SystemTimer3 => FiqSource(3),
            IrqSource::Aux => FiqSource(29),
            IrqSource::I2cSpiSlv => FiqSource(43),
            IrqSource::Pwa0 => FiqSource(45),
            IrqSource::Pwa1 => FiqSource(46),
            IrqSource::Smi => FiqSource(48),
            IrqSource::GPIO => FiqSource(52),
            IrqSource::I2c => FiqSource(53),
            IrqSource::Spi => FiqSource(54),
            IrqSource::Pcm => FiqSource(55),
            IrqSource::Uart => FiqSource(57),
            IrqSource::ArmTimer => FiqSource(64),
            IrqSource::ArmMailbox => FiqSource(65),
            IrqSource::ArmDoorbell0 => FiqSource(66),
            IrqSource::ArmDoorbell1 => FiqSource(67),
            IrqSource::Gpu0Halted => FiqSource(68),
            IrqSource::Gpu1Halted => FiqSource(69),
            IrqSource::IllegalAccess1 => FiqSource(70),
            IrqSource::IllegalAccess0 => FiqSource(71),
            IrqSource::Gpu(irq) => FiqSource(irq.0),
        }
    }
}

/// IRQ status.
#[derive(Debug, Copy, Clone)]
enum IrqStatus {
    /// The IRQ is pending.
    Pending,

    /// The IRQ is not pending.
    NotPending,

    /// Unknown status.
    Unknown,
}

impl From<bool> for IrqStatus {
    fn from(status: bool) -> IrqStatus {
        if status {
            IrqStatus::Pending
        } else {
            IrqStatus::NotPending
        }
    }
}

/// IRQ status of the basic sources.
#[derive(Debug)]
pub struct BasicStatus {
    /// ARM Timer IRQ pending.
    arm_timer: IrqStatus,

    /// ARM Mailbox IRQ pending.
    arm_mailbox: IrqStatus,

    /// ARM Doorbell 0 IRQ pending.
    arm_doorbell_0: IrqStatus,

    /// ARM Doorbell 1 IRQ pending.
    arm_doorbell_1: IrqStatus,

    /// GPU0 halted IRQ pending.
    gpu0_halted: IrqStatus,

    /// GPU1 halted IRQ pending.
    gpu1_halted: IrqStatus,

    /// Illegal access type 1 IRQ pending.
    illegal_access_1: IrqStatus,

    /// Illegal access type 0 IRQ pending.
    illegal_access_0: IrqStatus,

    /// GPU IRQ pending in the range 0:31, which contains:
    /// [IrqSource::SystemTimer0], [IrqSource::SystemTimer1],
    /// [IrqSource::SystemTimer2], [IrqSource::SystemTimer3] and
    /// [IrqSource::Aux].
    pending_reg_1: IrqStatus,

    /// GPU IRQ pending in the range 32:63, which contains:
    /// [IrqSource::I2cSpiSlv], [IrqSource::Pwa0], [IrqSource::Pwa1],
    /// [IrqSource::Smi], [IrqSource::GPIO], [IrqSource::I2c],
    /// [IrqSource::Spi], [IrqSource::Pcm] and [IrqSource::Uart].
    pending_reg_2: IrqStatus,

    /// GPU IRQ 7 pending.
    gpu_irq_7: IrqStatus,

    /// GPU IRQ 9 pending.
    gpu_irq_9: IrqStatus,

    /// GPU IRQ 10 pending.
    gpu_irq_10: IrqStatus,

    /// GPU IRQ 18 pending.
    gpu_irq_18: IrqStatus,

    /// GPU IRQ 19 pending.
    gpu_irq_19: IrqStatus,

    /// GPU IRQ 53 pending.
    gpu_irq_53: IrqStatus,

    /// GPU IRQ 54 pending.
    gpu_irq_54: IrqStatus,

    /// GPU IRQ 55 pending.
    gpu_irq_55: IrqStatus,

    /// GPU IRQ 56 pending.
    gpu_irq_56: IrqStatus,

    /// GPU IRQ 57 pending.
    gpu_irq_57: IrqStatus,

    /// GPU IRQ 62 pending.
    gpu_irq_62: IrqStatus,
}

impl BasicStatus {
    /// Returns true if the ARM Timer IRQ is pending.
    pub fn pending_arm_timer(&self) -> bool {
        matches!(self.arm_timer, IrqStatus::Pending)
    }

    /// Returns true if the ARM Mailbox IRQ is pending.
    pub fn pending_arm_mailbox(&self) -> bool {
        matches!(self.arm_mailbox, IrqStatus::Pending)
    }

    /// Returns true if the ARM Doorbell 0 IRQ is pending.
    pub fn pending_arm_doorbell_0(&self) -> bool {
        matches!(self.arm_doorbell_0, IrqStatus::Pending)
    }

    /// Returns true if the ARM Doorbell 1 IRQ is pending.
    pub fn pending_arm_doorbell_1(&self) -> bool {
        matches!(self.arm_doorbell_1, IrqStatus::Pending)
    }

    /// Returns true if the GPU0 halted IRQ is pending.
    pub fn pending_gpu0_halted(&self) -> bool {
        matches!(self.gpu0_halted, IrqStatus::Pending)
    }

    /// Returns true if the GPU1 halted IRQ is pending.
    pub fn pending_gpu1_halted(&self) -> bool {
        matches!(self.gpu1_halted, IrqStatus::Pending)
    }

    /// Returns true if the Illegal access type 1 IRQ is pending.
    pub fn pending_illegal_access_1(&self) -> bool {
        matches!(self.illegal_access_1, IrqStatus::Pending)
    }

    /// Returns true if the Illegal access type 0 IRQ is pending.
    pub fn pending_illegal_access_0(&self) -> bool {
        matches!(self.illegal_access_0, IrqStatus::Pending)
    }

    /// Returns true if a GPU IRQ in the range 0:31 is pending. This includes:
    /// [IrqSource::SystemTimer0], [IrqSource::SystemTimer1],
    /// [IrqSource::SystemTimer2], [IrqSource::SystemTimer3] and
    /// [IrqSource::Aux].
    pub fn pending_reg_1(&self) -> bool {
        matches!(self.pending_reg_1, IrqStatus::Pending)
    }

    /// Returns true if a GPU IRQ in the range 32:63 is pending. This includes:
    /// [IrqSource::I2cSpiSlv], [IrqSource::Pwa0], [IrqSource::Pwa1],
    /// [IrqSource::Smi], [IrqSource::GPIO], [IrqSource::I2c],
    /// [IrqSource::Spi], [IrqSource::Pcm] and [IrqSource::Uart].
    pub fn pending_reg_2(&self) -> bool {
        matches!(self.pending_reg_2, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 7 is pending.
    pub fn pending_gpu_irq_7(&self) -> bool {
        matches!(self.gpu_irq_7, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 9 is pending.
    pub fn pending_gpu_irq_9(&self) -> bool {
        matches!(self.gpu_irq_9, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 10 is pending.
    pub fn pending_gpu_irq_10(&self) -> bool {
        matches!(self.gpu_irq_10, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 18 is pending.
    pub fn pending_gpu_irq_18(&self) -> bool {
        matches!(self.gpu_irq_18, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 19 is pending.
    pub fn pending_gpu_irq_19(&self) -> bool {
        matches!(self.gpu_irq_19, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 53 is pending.
    pub fn pending_gpu_irq_53(&self) -> bool {
        matches!(self.gpu_irq_53, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 54 is pending.
    pub fn pending_gpu_irq_54(&self) -> bool {
        matches!(self.gpu_irq_54, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 55 is pending.
    pub fn pending_gpu_irq_55(&self) -> bool {
        matches!(self.gpu_irq_55, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 56 is pending.
    pub fn pending_gpu_irq_56(&self) -> bool {
        matches!(self.gpu_irq_56, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 57 is pending.
    pub fn pending_gpu_irq_57(&self) -> bool {
        matches!(self.gpu_irq_57, IrqStatus::Pending)
    }

    /// Returns true if the GPU IRQ 62 is pending.
    pub fn pending_gpu_irq_62(&self) -> bool {
        matches!(self.gpu_irq_62, IrqStatus::Pending)
    }
}

/// IRQ status of the GPU.
#[derive(Debug)]
pub struct GpuStatus([IrqStatus; NGPUIRQS]);

impl GpuStatus {
    /// Returns true if the provided IRQ source is pending. `src` must be a
    /// GPU IRQ.
    pub fn pending(&self, src: IrqSource) -> Result<bool, Error> {
        let irq = GpuIrq::try_from(src)?;
        let status = self.0[irq.0];
        Ok(matches!(status, IrqStatus::Pending))
    }
}

impl IrqSource {
    /// Enables the IRQ source.
    pub fn enable(&self) {
        let bit = IrqBit::from(*self);
        let idx = usize::from(bit.0);
        let addr = INTEN_BASE + idx * 4;
        unsafe { mmio::write(addr, 1 << bit.1) };
    }

    /// Disables the IRQ source.
    pub fn disable(&self) {
        let bit = IrqBit::from(*self);
        let idx = usize::from(bit.0);
        let addr = INTDIS_BASE + idx * 4;
        unsafe { mmio::write(addr, 1 << bit.1) };
    }

    /// Enables FIQ for the source. Only a single interrupt can be selected.
    pub fn enable_fiq(&self) {
        // Make sure the IRQ is disabled for the source. Otherwise, both the
        // IRQ and the FIQ would be triggered.
        self.disable();

        // Enable FIQ.
        let fiq_src = FiqSource::from(*self);
        let fiq_mask = (fiq_src.0 as u32) & 0b11_1111;
        let reg = (1 << 7) | fiq_mask;
        unsafe { mmio::write(FIQCTL, reg) };
    }
}

/// Disables FIQ.
pub fn disable_fiq() {
    unsafe { mmio::write(FIQCTL, 0) };
}

/// Returns the IRQ status of the basic sources.
pub fn basic_status() -> BasicStatus {
    let reg = unsafe { mmio::read(INTBASICPEND) };

    BasicStatus {
        arm_timer: (reg & 1 != 0).into(),
        arm_mailbox: (reg & (1 << 1) != 0).into(),
        arm_doorbell_0: (reg & (1 << 2) != 0).into(),
        arm_doorbell_1: (reg & (1 << 3) != 0).into(),
        gpu0_halted: (reg & (1 << 4) != 0).into(),
        gpu1_halted: (reg & (1 << 5) != 0).into(),
        illegal_access_1: (reg & (1 << 6) != 0).into(),
        illegal_access_0: (reg & (1 << 7) != 0).into(),
        pending_reg_1: (reg & (1 << 8) != 0).into(),
        pending_reg_2: (reg & (1 << 9) != 0).into(),
        gpu_irq_7: (reg & (1 << 10) != 0).into(),
        gpu_irq_9: (reg & (1 << 11) != 0).into(),
        gpu_irq_10: (reg & (1 << 12) != 0).into(),
        gpu_irq_18: (reg & (1 << 13) != 0).into(),
        gpu_irq_19: (reg & (1 << 14) != 0).into(),
        gpu_irq_53: (reg & (1 << 15) != 0).into(),
        gpu_irq_54: (reg & (1 << 16) != 0).into(),
        gpu_irq_55: (reg & (1 << 17) != 0).into(),
        gpu_irq_56: (reg & (1 << 18) != 0).into(),
        gpu_irq_57: (reg & (1 << 19) != 0).into(),
        gpu_irq_62: (reg & (1 << 20) != 0).into(),
    }
}

/// Returns the IRQ status of the GPU.
pub fn gpu_status() -> GpuStatus {
    let mut pending = [IrqStatus::Unknown; NGPUIRQS];
    for i in 0..2 {
        let addr = INTGPUPEND_BASE + i * 4;
        let reg = unsafe { mmio::read(addr) };
        for j in 0..32 {
            pending[i * 32 + j] = (reg & (1 << j) != 0).into();
        }
    }
    GpuStatus(pending)
}
