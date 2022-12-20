//! PL011 UART driver.
//!
//! For more information, please see:
//!
//! - [BCM2835 ARM Peripherals specification].
//! - [PL011 Technical Reference Manual].
//!
//! [BCM2835 ARM Peripherals specification]: https://datasheets.raspberrypi.com/bcm2835/bcm2835-peripherals.pdf
//! [PL011 Technical Reference Manual]: https://static6.arrow.com/aropdfconversion/32f6a7175ece91477c63bc40811c02e077718861/ddi0183.pdf

use crate::gpio;
use crate::mailbox;
use crate::mmio;

/// Base address of the PL011 UART.
///
/// [/arch/arm/boot/dts/bcm283x.dtsi] describes it:
///
/// ```text
/// uart0: serial@7e201000 {
///     compatible = "arm,pl011", "arm,primecell";
///     reg = <0x7e201000 0x200>;
///     ...
/// };
/// ```
///
/// [/arch/arm/boot/dts/bcm283x.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm283x.dtsi#L304-L312
const UART_BASE: usize = 0x201000;

/// UART data register.
const UARTDR: usize = UART_BASE;

/// UART flag register.
const UARTFR: usize = UART_BASE + 0x18;

/// Receive FIFO empty.
const UARTFR_RXFE: u32 = 0x10;

/// Transmit FIFO full.
const UARTFR_TXFF: u32 = 0x20;

/// UART integer baud rate register.
const UARTIBRD: usize = UART_BASE + 0x24;

/// UART fractional baud rate register.
const UARTFBRD: usize = UART_BASE + 0x28;

/// UART line control register.
const UARTLCR_H: usize = UART_BASE + 0x2c;

/// UART control register.
const UARTCR: usize = UART_BASE + 0x30;

/// UART interrupt mask set/clear register.
const UARTIMSC: usize = UART_BASE + 0x38;

/// UART interrupt clear register.
const UARTICR: usize = UART_BASE + 0x44;

/// UART error.
#[derive(Debug)]
pub enum Error {
    /// GPIO error.
    GpioError(gpio::Error),

    /// Mailbox error.
    MailboxError(mailbox::Error),
}

impl From<gpio::Error> for Error {
    fn from(err: gpio::Error) -> Error {
        Error::GpioError(err)
    }
}

impl From<mailbox::Error> for Error {
    fn from(err: mailbox::Error) -> Error {
        Error::MailboxError(err)
    }
}

/// Initializes the UART.
pub fn init() -> Result<(), Error> {
    unsafe {
        // Mask all UART interrupts. RIMIM, DCDMIM and DSRMIM are unsupported,
        // so we write 0.
        mmio::write(
            UARTIMSC,
            (1 << 1)
                | (1 << 4)
                | (1 << 5)
                | (1 << 6)
                | (1 << 7)
                | (1 << 8)
                | (1 << 9)
                | (1 << 10),
        );

        // Clear all UART interrupts.
        mmio::write(UARTICR, 0x7ff);

        // Disable UART.
        mmio::write(UARTCR, 0);

        // Disable pull-up/down in pins 14 (TX) and 15 (RX).
        let pin_tx = gpio::Pin::try_from(14)?;
        pin_tx.set_pull_state(gpio::PullState::Off);
        let pin_rx = gpio::Pin::try_from(15)?;
        pin_rx.set_pull_state(gpio::PullState::Off);

        // Set UART clock frequency to 3MHz.
        mailbox::set_uartclk_freq(3_000_000)?;

        // Configure the baud rate divisor.
        // BRD = UARTCLK / (16 * Baud rate) = 3000000 / (16 * 115200) = 1.6276
        // UARTIBRD = BRDi = 1
        mmio::write(UARTIBRD, 1);
        // UARTFBRD = int((BRDf * 2**6) + 0.5) = int((0.6276 * 64) + 0.5) = 40
        mmio::write(UARTFBRD, 40);

        // Set UART to 8n1 and enable FIFOs.
        mmio::write(UARTLCR_H, (1 << 4) | (1 << 5) | (1 << 6));

        // Enable UART, transmit and receive.
        mmio::write(UARTCR, (1 << 0) | (1 << 8) | (1 << 9));

        Ok(())
    }
}

/// Transmits a byte.
pub fn send_byte(b: u8) {
    unsafe {
        // Wait while the transmit FIFO is full.
        while mmio::read(UARTFR) & UARTFR_TXFF != 0 {}

        // Write byte.
        mmio::write(UARTDR, b as u32);
    }
}

/// Receives a byte.
pub fn recv_byte() -> u8 {
    unsafe {
        // Wait while the receive FIFO is empty.
        while mmio::read(UARTFR) & UARTFR_RXFE != 0 {}

        // Read byte.
        mmio::read(UARTDR) as u8
    }
}
