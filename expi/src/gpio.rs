//! GPIO operations.
//!
//! For more information, please see [BCM2835 ARM Peripherals specification].
//!
//! [BCM2835 ARM Peripherals specification]: https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf

use core::fmt;

use crate::cpu::time;
use crate::mmio;
use crate::Error;

/// Base address of GPIO.
///
/// [/arch/arm/boot/dts/bcm283x.dtsi] describes it:
///
/// ```text
/// gpio: gpio@7e200000 {
///     compatible = "brcm,bcm2835-gpio";
///     reg = <0x7e200000 0xb4>;
///     ...
/// };
/// ```
///
/// [/arch/arm/boot/dts/bcm283x.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm283x.dtsi#L107-L302
const GPIO_BASE: usize = 0x200000;

/// Base address of GPFSELn registers.
const GPFSEL_BASE: usize = GPIO_BASE;

/// Base address of GPSETn registers.
const GPSET_BASE: usize = GPIO_BASE + 0x1c;

/// Base address of GPCLRn registers.
const GPCLR_BASE: usize = GPIO_BASE + 0x28;

/// Base address of GPLEVn registers.
const GPLEV_BASE: usize = GPIO_BASE + 0x34;

/// Base address of GPEDSn registers.
const GPEDS_BASE: usize = GPIO_BASE + 0x40;

/// Base address of GPRENn registers.
const GPREN_BASE: usize = GPIO_BASE + 0x4c;

/// Base address of GPFENn registers.
const GPFEN_BASE: usize = GPIO_BASE + 0x58;

/// Base address of GPHENn registers.
const GPHEN_BASE: usize = GPIO_BASE + 0x64;

/// Base address of GPLENn registers.
const GPLEN_BASE: usize = GPIO_BASE + 0x70;

/// Base address of GPARENn registers.
const GPAREN_BASE: usize = GPIO_BASE + 0x7c;

/// Base address of GPAFENn registers.
const GPAFEN_BASE: usize = GPIO_BASE + 0x88;

/// GPIO pull-up/down register.
const GPPUD: usize = GPIO_BASE + 0x94;

/// Base address of GPPUDCLKn registers.
const GPPUDCLK_BASE: usize = GPIO_BASE + 0x98;

/// Number of GPIO pins.
const NPINS: u32 = 54;

/// Pull state (pull-up/pull-down) for a GPIO pin.
pub enum PullState {
    /// Disable pull-up/down.
    Off,

    /// Enable pull-down.
    Down,

    /// Enable pull-up.
    Up,
}

impl From<PullState> for u32 {
    fn from(state: PullState) -> u32 {
        match state {
            PullState::Off => 0b00,
            PullState::Down => 0b01,
            PullState::Up => 0b10,
        }
    }
}

/// Pin function.
#[derive(Copy, Clone)]
pub enum Function {
    /// Input pin.
    Input,

    /// Output pin.
    Output,
}

impl From<Function> for u32 {
    fn from(fcn: Function) -> u32 {
        match fcn {
            Function::Input => 0b000,
            Function::Output => 0b001,
        }
    }
}

/// Pin level.
#[derive(Copy, Clone)]
pub enum Level {
    /// Low level.
    Low,

    /// High level.
    High,

    /// Unknown level.
    Unknown,
}

impl Default for Level {
    fn default() -> Level {
        Level::Unknown
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Level::Low => write!(f, "Low"),
            Level::High => write!(f, "High"),
            Level::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Pin event.
pub enum Event {
    /// Rising edge transition using synchronous edge detection. The input
    /// signal is sampled using the system clock and then it is looking for a
    /// "011" pattern on the sampled signal. This has the effect of suppressing
    /// glitches.
    RisingEdge,

    /// Falling edge transition using synchronous edge detection. The input
    /// signal is sampled using the system clock and then it is looking for a
    /// "100" pattern on the sampled signal. This has the effect of suppressing
    /// glitches.
    FallingEdge,

    /// Rising edge transition using asynchronous edge detection. The incoming
    /// signal is not sampled by the system clock. As such rising edges of very
    /// short duration can be detected.
    AsyncRisingEdge,

    /// Falling edge transition using asynchronous edge detection. The incoming
    /// signal is not sampled by the system clock. As such falling edges of
    /// very short duration can be detected.
    AsyncFallingEdge,

    /// High level.
    PinHigh,

    /// Low level.
    PinLow,
}

/// Configures the pull state (pull-up/pull-down) of a GPIO pin.
pub fn set_pull_state(pin: u32, state: PullState) -> Result<(), Error> {
    if pin >= NPINS {
        return Err(Error::InvalidGpioPin(pin));
    }

    // Write to GPPUD to set the required control signal.
    unsafe { mmio::write(GPPUD, state.into()) };

    // Wait at least 150 cycles. This provides the required set-up time for
    // the control signal.
    time::delay(150);

    // Write to GPPUDCLKn to clock the control signal into the target GPIO
    // pad.
    let n = (pin as usize) / 32;
    let addr = GPPUDCLK_BASE + n * 4;
    let reg = 1 << (pin % 32);
    unsafe { mmio::write(addr, reg) };

    // Wait at least 150 cycles. This provides the required hold time for
    // the control signal.
    time::delay(150);

    // Write to GPPUD to remove the control signal.
    unsafe { mmio::write(GPPUD, 0) };

    // Write to GPPUDCLKn to remove the clock.
    unsafe { mmio::write(addr, 0) };

    Ok(())
}

/// Configures the operation of a GPIO pin.
pub fn set_function(pin: u32, fcn: Function) -> Result<(), Error> {
    if pin >= NPINS {
        return Err(Error::InvalidGpioPin(pin));
    }

    // Read the initial register value.
    let n = (pin as usize) / 10;
    let addr = GPFSEL_BASE + n * 4;
    let reg = unsafe { mmio::read(addr) };

    // Write register.
    let shift = (pin % 10) * 3;
    let mask: u32 = 0b111 << shift;
    let fcn: u32 = fcn.into();
    unsafe { mmio::write(addr, (reg & !mask) | (fcn << shift)) };

    Ok(())
}

/// Sets a set of GPIO pins.
pub fn set(pins: &[u32]) -> Result<(), Error> {
    // Precompute the final register values.
    let mut regs = [0u32; 2];
    for &pin in pins {
        if pin >= NPINS {
            return Err(Error::InvalidGpioPin(pin));
        }

        let n = (pin as usize) / 32;
        regs[n] |= 1 << (pin % 32)
    }

    // Write registers.
    for (i, &reg) in regs.iter().enumerate() {
        let addr = GPSET_BASE + i * 4;
        unsafe { mmio::write(addr, reg) };
    }

    Ok(())
}

/// Clears a set of GPIO pins.
pub fn clear(pins: &[u32]) -> Result<(), Error> {
    // Precompute the final register values.
    let mut regs = [0u32; 2];
    for &pin in pins {
        if pin >= NPINS {
            return Err(Error::InvalidGpioPin(pin));
        }

        let n = (pin as usize) / 32;
        regs[n] |= 1 << (pin % 32)
    }

    // Write registers.
    for (i, &reg) in regs.iter().enumerate() {
        let addr = GPCLR_BASE + i * 4;
        unsafe { mmio::write(addr, reg) };
    }

    Ok(())
}

/// Returns the value of a set of GPIO pins in `levels`.
pub fn read_levels(pins: &[u32], levels: &mut [Level]) -> Result<(), Error> {
    if pins.len() != levels.len() {
        return Err(Error::InvalidOutputSize);
    }

    // Read the initial register values.
    let mut regs = [0u32; 2];
    for (i, reg) in regs.iter_mut().enumerate() {
        let addr = GPLEV_BASE + i * 4;
        *reg = unsafe { mmio::read(addr) };
    }

    // Get levels.
    for (i, &pin) in pins.iter().enumerate() {
        if pin >= NPINS {
            return Err(Error::InvalidGpioPin(pin));
        }

        let n = (pin as usize) / 32;
        levels[i] = if (regs[n] & (1 << (pin % 32))) == 0 {
            Level::Low
        } else {
            Level::High
        };
    }

    Ok(())
}

/// Returns the value of a GPIO pin.
pub fn read_level(pin: u32) -> Result<Level, Error> {
    let mut levels = [Level::default(); 1];
    read_levels(&[pin], &mut levels)?;
    Ok(levels[0])
}

/// Enables an event type for a pin.
pub fn enable_event(pin: u32, event: Event) -> Result<(), Error> {
    if pin >= NPINS {
        return Err(Error::InvalidGpioPin(pin));
    }

    // Read the intial enable register value.
    let n = (pin as usize) / 32;
    let addr = match event {
        Event::RisingEdge => GPREN_BASE + n * 4,
        Event::FallingEdge => GPFEN_BASE + n * 4,
        Event::AsyncRisingEdge => GPAREN_BASE + n * 4,
        Event::AsyncFallingEdge => GPAFEN_BASE + n * 4,
        Event::PinHigh => GPHEN_BASE + n * 4,
        Event::PinLow => GPLEN_BASE + n * 4,
    };
    let reg = unsafe { mmio::read(addr) };

    // Enable pin event.
    let mask = 1 << (pin % 32);
    unsafe { mmio::write(addr, reg | mask) };

    Ok(())
}

/// Disables an event type for a pin.
pub fn disable_event(pin: u32, event: Event) -> Result<(), Error> {
    if pin >= NPINS {
        return Err(Error::InvalidGpioPin(pin));
    }

    // Read the intial enable register value.
    let n = (pin as usize) / 32;
    let addr = match event {
        Event::RisingEdge => GPREN_BASE + n * 4,
        Event::FallingEdge => GPFEN_BASE + n * 4,
        Event::AsyncRisingEdge => GPAREN_BASE + n * 4,
        Event::AsyncFallingEdge => GPAFEN_BASE + n * 4,
        Event::PinHigh => GPHEN_BASE + n * 4,
        Event::PinLow => GPLEN_BASE + n * 4,
    };
    let reg = unsafe { mmio::read(addr) };

    // Enable pin event.
    let mask = 1 << (pin % 32);
    unsafe { mmio::write(addr, reg & !mask) };

    Ok(())
}

/// Clear the event status of a GPIO pin.
pub fn clear_event(pin: u32) -> Result<(), Error> {
    if pin >= NPINS {
        return Err(Error::InvalidGpioPin(pin));
    }

    let n = (pin as usize) / 32;
    let addr = GPEDS_BASE + n * 4;
    let reg = 1 << (pin % 32);
    unsafe { mmio::write(addr, reg) };

    Ok(())
}

/// Returns the event status of a set of GPIO pins in `events`. If `true`, the
/// programmed event type has been detected.
pub fn read_events(pins: &[u32], events: &mut [bool]) -> Result<(), Error> {
    if pins.len() != events.len() {
        return Err(Error::InvalidOutputSize);
    }

    // Read the initial register values.
    let mut regs = [0u32; 2];
    for (i, reg) in regs.iter_mut().enumerate() {
        let addr = GPEDS_BASE + i * 4;
        *reg = unsafe { mmio::read(addr) };
    }

    // Get event status.
    for (i, &pin) in pins.iter().enumerate() {
        if pin >= NPINS {
            return Err(Error::InvalidGpioPin(pin));
        }

        let n = (pin as usize) / 32;
        events[i] = regs[n] & (1 << (pin % 32)) != 0;
    }

    Ok(())
}

/// Returns the event status of a GPIO pin. If `true`, the programmed event
/// type has been detected.
pub fn read_event(pin: u32) -> Result<bool, Error> {
    let mut events = [false; 1];
    read_events(&[pin], &mut events)?;
    Ok(events[0])
}
