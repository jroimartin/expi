//! System Timer driver.

use core::fmt;

use crate::mmio;

/// Base address of the interrupt controller.
///
/// [/arch/arm/boot/dts/bcm283x.dtsi] describes it:
///
/// ```text
/// system_timer: timer@7e003000 {
///     compatible = "brcm,bcm2835-system-timer";
///     reg = <0x7e003000 0x1000>;
///     ...
///     clock-frequency = <1000000>;
/// };
/// ```
///
/// [/arch/arm/boot/dts/bcm283x.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm283x.dtsi#L69-L78
const TIMER_BASE: usize = 0x3000;

/// System Timer Control/Status register.
const TIMER_CS: usize = TIMER_BASE;

/// System Timer Counter Lower 32 bits.
const TIMER_CLO: usize = TIMER_BASE + 0x4;

/// System Timer Counter Higher 32 bits.
const TIMER_CHI: usize = TIMER_BASE + 0x8;

/// Base address of System Timer Compare registers.
const TIMER_CMP_BASE: usize = TIMER_BASE + 0xc;

/// Number of System Timers.
const NTIMERS: usize = 4;

/// The System Timer runs at 1MHz.
pub const CLOCK_FREQ: u32 = 1_000_000;

/// System Timer error.
#[derive(Debug)]
pub enum Error {
    /// Invalid System Timer.
    InvalidSystemTimer(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidSystemTimer(n) => {
                write!(f, "invalid System Timer: {n}")
            }
        }
    }
}

/// Timer status.
#[derive(Debug, Copy, Clone)]
pub enum TimerStatus {
    /// A timer match has been detected since last cleared.
    Matched,

    /// No timer match has been detected since last cleared.
    NotMatched,

    /// Unknown timer status.
    Unknown,
}

impl From<bool> for TimerStatus {
    fn from(status: bool) -> TimerStatus {
        if status {
            TimerStatus::Matched
        } else {
            TimerStatus::NotMatched
        }
    }
}

/// Status of the system timers.
#[derive(Debug)]
pub struct Status([TimerStatus; NTIMERS]);

impl Status {
    /// Returns true if the status of a timer is "matched".
    pub fn matched(&self, timer: SystemTimer) -> bool {
        let status = self.0[timer.0];
        matches!(status, TimerStatus::Matched)
    }
}

/// Represents a System Timer.
#[derive(Debug, Copy, Clone)]
pub struct SystemTimer(usize);

impl TryFrom<usize> for SystemTimer {
    type Error = Error;

    fn try_from(n: usize) -> Result<SystemTimer, Error> {
        if n >= NTIMERS {
            return Err(Error::InvalidSystemTimer(n));
        }
        Ok(SystemTimer(n))
    }
}

impl SystemTimer {
    /// Returns true if a timer match has been detected since last cleared.
    pub fn matched(&self) -> bool {
        let status = status();
        status.matched(*self)
    }

    /// Clears the timer match.
    pub fn clear(&self) {
        clear(&[*self])
    }

    /// Sets the compare value of the timer.
    pub fn set_cmp(&self, cmp: u32) {
        let addr = TIMER_CMP_BASE + self.0 * 4;
        unsafe { mmio::write(addr, cmp) };
    }

    /// Returns the current compare value of the timer.
    pub fn cmp(&self) -> u32 {
        let addr = TIMER_CMP_BASE + self.0 * 4;
        unsafe { mmio::read(addr) }
    }
}

/// Returns the status of the system timers.
pub fn status() -> Status {
    let cs = unsafe { mmio::read(TIMER_CS) };

    let mut status = [TimerStatus::Unknown; NTIMERS];
    for (i, status) in status.iter_mut().enumerate() {
        *status = (cs & (1 << i) != 0).into()
    }

    Status(status)
}

/// Clears a set of system timer matches.
pub fn clear(timers: &[SystemTimer]) {
    let mut mask = 0;
    for timer in timers {
        mask |= 1 << timer.0
    }
    unsafe { mmio::write(TIMER_CS, mask) };
}

/// Returns the current value of the System Timer free-running counter.
pub fn counter() -> u64 {
    let chi = unsafe { mmio::read(TIMER_CHI) as u64 };
    let clo = unsafe { mmio::read(TIMER_CLO) as u64 };

    (chi << 32) | clo
}
