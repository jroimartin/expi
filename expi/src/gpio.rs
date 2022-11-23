//! GPIO operations.
//!
//! For more information, please see [BCM2835 ARM Peripherals specification].
//!
//! [BCM2835 ARM Peripherals specification]: https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf

use crate::errors::Error;
use crate::mmio;
use crate::time;

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

/// GPIO pull-up/down register.
const GPPUD: usize = GPIO_BASE + 0x94;

/// Base address of GPPUDCLKn registers.
const GPPUDCLK_BASE: usize = GPIO_BASE + 0x98;

/// Number of GPIO pins.
const NPIN: u32 = 54;

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

/// Configures the pull state (pull-up/pull-down) of a GPIO pin.
pub fn set_pull_state(pin: u32, state: PullState) -> Result<(), Error> {
    if pin >= NPIN {
        return Err(Error::InvalidGpioPin(pin));
    }

    let nreg = (pin as usize) / 32;
    let reg = GPPUDCLK_BASE + nreg * 4;

    unsafe {
        // Write to GPPUD to set the required control signal.
        mmio::write(GPPUD, state.into());

        // Wait at least 150 cycles. This provides the required set-up time for
        // the control signal.
        time::delay(150);

        // Write to GPPUDCLKn to clock the control signal into the target GPIO
        // pad.
        mmio::write(reg, 1 << (pin % 32));

        // Wait at least 150 cycles. This provides the required hold time for
        // the control signal.
        time::delay(150);

        // Write to GPPUD to remove the control signal.
        mmio::write(GPPUD, 0);

        // Write to GPPUDCLKn to remove the clock.
        mmio::write(reg, 0);
    }

    Ok(())
}

/// Configures the operation of a GPIO pin.
pub fn set_function(pin: u32, fcn: Function) -> Result<(), Error> {
    if pin >= NPIN {
        return Err(Error::InvalidGpioPin(pin));
    }

    let nreg = (pin as usize) / 10;
    let reg = GPFSEL_BASE + nreg * 4;

    let val = unsafe { mmio::read(reg) };
    let shift = (pin % 10) * 3;
    let mask: u32 = 0b111 << shift;
    let fcn: u32 = fcn.into();
    unsafe { mmio::write(reg, (val & !mask) | (fcn << shift)) };

    Ok(())
}

/// Sets a GPIO pin.
pub fn set(pin: u32) -> Result<(), Error> {
    if pin >= NPIN {
        return Err(Error::InvalidGpioPin(pin));
    }

    let nreg = (pin as usize) / 32;
    let reg = GPSET_BASE + nreg * 4;

    unsafe { mmio::write(reg, 1 << (pin % 32)) };

    Ok(())
}

/// Clears a GPIO pin.
pub fn clear(pin: u32) -> Result<(), Error> {
    if pin >= NPIN {
        return Err(Error::InvalidGpioPin(pin));
    }

    let nreg = (pin as usize) / 32;
    let reg = GPCLR_BASE + nreg * 4;

    unsafe { mmio::write(reg, 1 << (pin % 32)) };

    Ok(())
}
