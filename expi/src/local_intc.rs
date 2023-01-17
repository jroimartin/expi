//! Driver for the ARM-local interrupt controller.

use core::fmt;

use crate::cpu::Core;
use crate::mmio;

/// Base address of the ARM-local interrupt controller.
///
/// [/arch/arm/boot/dts/bcm2837.dtsi] describes it:
///
/// ```text
/// local_intc: local_intc@40000000 {
///     compatible = "brcm,bcm2836-l1-intc";
///     reg = <0x40000000 0x100>;
///     ...
/// };
/// ```
///
/// [/arch/arm/boot/dts/bcm2837.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm2837.dtsi#L13-L19
const INTC_BASE: usize = 0x100_0000;

/// Local timer interrupt routing.
const LOCAL_TIMER_INT_ROUTING: usize = INTC_BASE + 0x24;

/// Local timer control & status.
const LOCAL_TIMER_CONTROL_STATUS: usize = INTC_BASE + 0x34;

/// Base address of the IRQ source registers.
const CORE_IRQ_SOURCE_BASE: usize = INTC_BASE + 0x60;

/// Base address of the FIQ source registers.
const CORE_FIQ_SOURCE_BASE: usize = INTC_BASE + 0x70;

/// Local interrupt controller error.
#[derive(Debug)]
pub enum Error {
    /// Invalid routing configuration.
    InvalidRoute(Core, IntType),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidRoute(core, ty) => {
                write!(f, "cannot route {ty:?} to core {core:?}")
            }
        }
    }
}

/// ARM-local interrupt source.
#[derive(Debug, Copy, Clone)]
pub enum IntSource {
    /// Local timer interrupt.
    LocalTimer,

    /// AXI-outstanding interrupt. For core 0 only, all others are 0.
    Axi,

    /// PMU interrupt.
    Pmu,

    /// GPU interrupt. Can be high in one core only.
    Gpu,

    /// Mailbox 0 interrupt.
    Mailbox0,

    /// Mailbox 1 interrupt.
    Mailbox1,

    /// Mailbox 2 interrupt.
    Mailbox2,

    /// Mailbox 3 interrupt.
    Mailbox3,

    /// CNTVIRQ interrupt.
    CntvIrq,

    /// CNTHPIRQ interrupt.
    CnthpIrq,

    /// CNTPNSIRQ interrupt.
    CntpnsIrq,

    /// CNTPSIRQ interrupt (Physical Timer -1).
    CntpsIrq,
}

/// Interrupt type.
#[derive(Debug)]
pub enum IntType {
    /// Interrupt.
    Irq,

    /// Fast interrupt.
    Fiq,
}

impl IntSource {
    /// Enables a given interrupt.
    pub fn enable(&self) {
        match self {
            IntSource::LocalTimer => LocalTimerInt.enable(),
            IntSource::Axi => todo!(),
            IntSource::Pmu => todo!(),
            IntSource::Gpu => todo!(),
            IntSource::Mailbox0 => todo!(),
            IntSource::Mailbox1 => todo!(),
            IntSource::Mailbox2 => todo!(),
            IntSource::Mailbox3 => todo!(),
            IntSource::CntvIrq => todo!(),
            IntSource::CnthpIrq => todo!(),
            IntSource::CntpnsIrq => todo!(),
            IntSource::CntpsIrq => todo!(),
        }
    }

    /// Disables a given interrupt.
    pub fn disable(&self) {
        match self {
            IntSource::LocalTimer => LocalTimerInt.disable(),
            IntSource::Axi => todo!(),
            IntSource::Pmu => todo!(),
            IntSource::Gpu => todo!(),
            IntSource::Mailbox0 => todo!(),
            IntSource::Mailbox1 => todo!(),
            IntSource::Mailbox2 => todo!(),
            IntSource::Mailbox3 => todo!(),
            IntSource::CntvIrq => todo!(),
            IntSource::CnthpIrq => todo!(),
            IntSource::CntpnsIrq => todo!(),
            IntSource::CntpsIrq => todo!(),
        }
    }

    /// Routes a given interrupt to a specific CPU core.
    pub fn route(&self, core: Core, ty: IntType) -> Result<(), Error> {
        match self {
            IntSource::LocalTimer => LocalTimerInt.route(core, ty),
            IntSource::Axi => todo!(),
            IntSource::Pmu => todo!(),
            IntSource::Gpu => todo!(),
            IntSource::Mailbox0 => todo!(),
            IntSource::Mailbox1 => todo!(),
            IntSource::Mailbox2 => todo!(),
            IntSource::Mailbox3 => todo!(),
            IntSource::CntvIrq => todo!(),
            IntSource::CnthpIrq => todo!(),
            IntSource::CntpnsIrq => todo!(),
            IntSource::CntpsIrq => todo!(),
        }
        Ok(())
    }
}

/// Represents the Local Timer interrupt.
struct LocalTimerInt;

impl LocalTimerInt {
    /// Enables the local timer interrupt.
    fn enable(&self) {
        let mut val = unsafe { mmio::read(LOCAL_TIMER_CONTROL_STATUS) };
        val |= 1 << 29;
        unsafe { mmio::write(LOCAL_TIMER_CONTROL_STATUS, val) };
    }

    /// Disables the local timer interrupt.
    fn disable(&self) {
        let mut val = unsafe { mmio::read(LOCAL_TIMER_CONTROL_STATUS) };
        val &= !(1 << 29);
        unsafe { mmio::write(LOCAL_TIMER_CONTROL_STATUS, val) };
    }

    /// Routes the local timer interrupt to a specific CPU core.
    fn route(&self, core: Core, ty: IntType) {
        let val = match (core.into(), ty) {
            (0, IntType::Irq) => 0b000,
            (1, IntType::Irq) => 0b001,
            (2, IntType::Irq) => 0b010,
            (3, IntType::Irq) => 0b011,
            (0, IntType::Fiq) => 0b100,
            (1, IntType::Fiq) => 0b101,
            (2, IntType::Fiq) => 0b110,
            (3, IntType::Fiq) => 0b111,
            (_, _) => unreachable!(),
        };
        unsafe { mmio::write(LOCAL_TIMER_INT_ROUTING, val) };
    }
}

/// Interrupt status.
#[derive(Debug, Copy, Clone)]
enum IntStatus {
    /// The interrupt is pending.
    Pending,

    /// The interrupt is not pending.
    NotPending,

    /// Unknown status.
    Unknown,
}

impl Default for IntStatus {
    fn default() -> IntStatus {
        IntStatus::Unknown
    }
}

impl From<bool> for IntStatus {
    fn from(status: bool) -> IntStatus {
        if status {
            IntStatus::Pending
        } else {
            IntStatus::NotPending
        }
    }
}

/// Status of the ARM-local interrupts.
#[derive(Debug)]
pub struct Status {
    /// Local Timer interrupt pending.
    local_timer: IntStatus,

    /// AXI-outstanding interrupt pending.
    axi: IntStatus,

    /// PMU interrupt pending.
    pmu: IntStatus,

    /// GPU interrupt pending.
    gpu: IntStatus,

    /// Mailbox 0 interrupt pending.
    mailbox0: IntStatus,

    /// Mailbox 1 interrupt pending.
    mailbox1: IntStatus,

    /// Mailbox 2 interrupt pending.
    mailbox2: IntStatus,

    /// Mailbox 3 interrupt pending.
    mailbox3: IntStatus,

    /// CNTV interrupt pending.
    cntv: IntStatus,

    /// CNTHP interrupt pending.
    cnthp: IntStatus,

    /// CNTPNS interrupt pending.
    cntpns: IntStatus,

    /// CNTPS interrupt pending.
    cntps: IntStatus,
}

impl Status {
    /// Returns true if the Local Timer interrupt is pending.
    pub fn pending_local_timer(&self) -> bool {
        matches!(self.local_timer, IntStatus::Pending)
    }

    /// Returns true if the AXI-outstanding interrupt is pending.
    pub fn pending_axi(&self) -> bool {
        matches!(self.axi, IntStatus::Pending)
    }

    /// Returns true if the PMU interrupt is pending.
    pub fn pending_pmu(&self) -> bool {
        matches!(self.pmu, IntStatus::Pending)
    }

    /// Returns true if the GPU interrupt is pending.
    pub fn pending_gpu(&self) -> bool {
        matches!(self.gpu, IntStatus::Pending)
    }

    /// Returns true if the mailbox 0 interrupt is pending.
    pub fn pending_mailbox0(&self) -> bool {
        matches!(self.mailbox0, IntStatus::Pending)
    }
    /// Returns true if the mailbox 1 interrupt is pending.
    pub fn pending_mailbox1(&self) -> bool {
        matches!(self.mailbox1, IntStatus::Pending)
    }

    /// Returns true if the mailbox 2 interrupt is pending.
    pub fn pending_mailbox2(&self) -> bool {
        matches!(self.mailbox2, IntStatus::Pending)
    }

    /// Returns true if the mailbox 3 interrupt is pending.
    pub fn pending_mailbox3(&self) -> bool {
        matches!(self.mailbox3, IntStatus::Pending)
    }

    /// Returns true if the CNTV interrupt is pending.
    pub fn pending_cntv(&self) -> bool {
        matches!(self.cntv, IntStatus::Pending)
    }

    /// Returns true if the CNTHP interrupt is pending.
    pub fn pending_cnthp(&self) -> bool {
        matches!(self.cnthp, IntStatus::Pending)
    }

    /// Returns true if the CNTPNS interrupt is pending.
    pub fn pending_cntpns(&self) -> bool {
        matches!(self.cntpns, IntStatus::Pending)
    }

    /// Returns true if the CNTPS interrupt is pending.
    pub fn pending_cntps(&self) -> bool {
        matches!(self.cntps, IntStatus::Pending)
    }
}

/// Returns the IRQ status of the ARM-local interrupt sources.
pub fn irq_status(core: Core) -> Status {
    let addr = CORE_IRQ_SOURCE_BASE + usize::from(core) * 4;
    let val = unsafe { mmio::read(addr) };
    Status {
        local_timer: (val & (1 << 11) != 0).into(),
        axi: (val & (1 << 10) != 0).into(),
        pmu: (val & (1 << 9) != 0).into(),
        gpu: (val & (1 << 8) != 0).into(),
        mailbox3: (val & (1 << 7) != 0).into(),
        mailbox2: (val & (1 << 6) != 0).into(),
        mailbox1: (val & (1 << 5) != 0).into(),
        mailbox0: (val & (1 << 4) != 0).into(),
        cntv: (val & (1 << 3) != 0).into(),
        cnthp: (val & (1 << 2) != 0).into(),
        cntpns: (val & (1 << 1) != 0).into(),
        cntps: (val & 1 != 0).into(),
    }
}

/// Returns the FIQ status of the ARM-local interrupt sources.
pub fn fiq_status(core: Core) -> Status {
    let addr = CORE_FIQ_SOURCE_BASE + usize::from(core) * 4;
    let val = unsafe { mmio::read(addr) };
    Status {
        local_timer: (val & (1 << 11) != 0).into(),
        axi: (val & (1 << 10) != 0).into(),
        pmu: (val & (1 << 9) != 0).into(),
        gpu: (val & (1 << 8) != 0).into(),
        mailbox3: (val & (1 << 7) != 0).into(),
        mailbox2: (val & (1 << 6) != 0).into(),
        mailbox1: (val & (1 << 5) != 0).into(),
        mailbox0: (val & (1 << 4) != 0).into(),
        cntv: (val & (1 << 3) != 0).into(),
        cnthp: (val & (1 << 2) != 0).into(),
        cntpns: (val & (1 << 1) != 0).into(),
        cntps: (val & 1 != 0).into(),
    }
}
