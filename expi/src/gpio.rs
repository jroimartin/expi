//! GPIO operations.
//!
//! For more information, please see [BCM2835 ARM Peripherals specification].
//!
//! [BCM2835 ARM Peripherals specification]: https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf

use crate::cpu::time;
use crate::errors::Error;
use crate::mmio;

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

/// Number of GPFSELn registers.
const NGPFSEL: usize = 6;

/// Base address of GPSETn registers.
const GPSET_BASE: usize = GPIO_BASE + 0x1c;

/// Number of GPSETn registers.
const NGPSET: usize = 2;

/// Base address of GPCLRn registers.
const GPCLR_BASE: usize = GPIO_BASE + 0x28;

/// Number of GPCLRn registers.
const NGPCLR: usize = 2;

/// GPIO pull-up/down register.
const GPPUD: usize = GPIO_BASE + 0x94;

/// Base address of GPPUDCLKn registers.
const GPPUDCLK_BASE: usize = GPIO_BASE + 0x98;

/// Number of GPPUDCLKn registers.
const NGPPUDCLK: usize = 2;

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
#[derive(Clone, Copy)]
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

/// Configures the pull state (pull-up/pull-down) of a set of GPIO pins.
pub fn set_pull_state(state: PullState, pins: &[u32]) -> Result<(), Error> {
    // Precompute the values to be written in regs.
    let mut regs = [0u32; NGPPUDCLK];
    for &pin in pins {
        let n = (pin as usize) / 32;

        if n >= regs.len() {
            return Err(Error::InvalidGpioPin(pin));
        }

        regs[n] |= 1 << (pin % 32);
    }

    // Write to GPPUD to set the required control signal.
    unsafe { mmio::write(GPPUD, state.into()) };

    // Wait at least 150 cycles. This provides the required set-up time for
    // the control signal.
    time::delay(150);

    // Write to GPPUDCLKn to clock the control signal into the target GPIO
    // pad.
    for (i, &reg) in regs.iter().enumerate() {
        let addr = GPPUDCLK_BASE + i * 4;
        unsafe { mmio::write(addr, reg) };
    }

    // Wait at least 150 cycles. This provides the required hold time for
    // the control signal.
    time::delay(150);

    // Write to GPPUD to remove the control signal.
    unsafe { mmio::write(GPPUD, 0) };

    // Write to GPPUDCLKn to remove the clock.
    for (i, _val) in regs.iter().enumerate() {
        let addr = GPPUDCLK_BASE + i * 4;
        unsafe { mmio::write(addr, 0) };
    }

    Ok(())
}

/// Configures the operation of a set of GPIO pins.
pub fn set_function(fcn: Function, pins: &[u32]) -> Result<(), Error> {
    // Read the initial register values.
    let mut regs = [0u32; NGPFSEL];
    for (i, reg) in regs.iter_mut().enumerate() {
        let addr = GPFSEL_BASE + i * 4;
        *reg = unsafe { mmio::read(addr) };
    }

    // Precompute the final register values.
    for &pin in pins {
        let n = (pin as usize) / 10;

        if n >= regs.len() {
            return Err(Error::InvalidGpioPin(pin));
        }

        let shift = (pin % 10) * 3;
        let mask: u32 = 0b111 << shift;
        let fcn: u32 = fcn.into();

        regs[n] = (regs[n] & !mask) | (fcn << shift)
    }

    // Write registers.
    for (i, &reg) in regs.iter().enumerate() {
        let addr = GPFSEL_BASE + i * 4;
        unsafe { mmio::write(addr, reg) };
    }

    Ok(())
}

/// Sets a set of GPIO pins.
pub fn set(pins: &[u32]) -> Result<(), Error> {
    // Precompute the final register values.
    let mut regs = [0u32; NGPSET];
    for &pin in pins {
        let n = (pin as usize) / 32;

        if n >= regs.len() {
            return Err(Error::InvalidGpioPin(pin));
        }

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
    let mut regs = [0u32; NGPCLR];
    for &pin in pins {
        let n = (pin as usize) / 32;

        if n >= regs.len() {
            return Err(Error::InvalidGpioPin(pin));
        }

        regs[n] |= 1 << (pin % 32)
    }

    // Write registers.
    for (i, &reg) in regs.iter().enumerate() {
        let addr = GPCLR_BASE + i * 4;
        unsafe { mmio::write(addr, reg) };
    }

    Ok(())
}
