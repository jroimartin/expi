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

/// GPIO pull-up/down register.
const GPPUD: usize = GPIO_BASE + 0x94;

/// GPIO pull-up/down clock register 0.
const GPPUDCLK0: usize = GPIO_BASE + 0x98;

/// GPIO pull-up/down clock register 1.
const GPPUDCLK1: usize = GPIO_BASE + 0x9c;

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

/// Set the pull state (pull-up/pull-down) to `state` for the list of GPIO
/// `pins`.
pub fn set_pull_state(state: PullState, pins: &[u32]) -> Result<(), Error> {
    unsafe {
        // Write to GPPUD to set the required control signal.
        mmio::write(GPPUD, state.into());

        // Wait at least 150 cycles. This provides the required set-up time for
        // the control signal.
        time::delay(150);

        // Write to GPPUDCLK0/1 to clock the control signal into the target
        // GPIO pads.
        let mut clk0 = 0u32;
        let mut clk1 = 0u32;
        for &pin in pins {
            match pin {
                0..=31 => clk0 |= 1 << pin,
                32..=53 => clk1 |= 1 << (pin - 32),
                _ => return Err(Error::InvalidGpioPin(pin)),
            }
        }
        mmio::write(GPPUDCLK0, clk0);
        mmio::write(GPPUDCLK1, clk1);

        // Wait at least 150 cycles. This provides the required hold time for
        // the control signal.
        time::delay(150);

        // Write to GPPUD to remove the control signal.
        mmio::write(GPPUD, 0);

        // Write to GPPUDCLK0/1 to remove the clock.
        mmio::write(GPPUDCLK0, 0);
        mmio::write(GPPUDCLK1, 0);

        Ok(())
    }
}
