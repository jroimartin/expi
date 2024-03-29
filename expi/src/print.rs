//! UART writer and print macros.

use core::fmt;

use crate::uart;

/// Implements a writer on top of the UART.
pub struct UartWriter;

impl fmt::Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if b == b'\n' {
                uart::send_byte(b'\r');
            }
            uart::send_byte(b);
        }

        Ok(())
    }
}

/// Print to the UART.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            let mut uart_writer_mg =
                $crate::globals::GLOBALS.uart_writer().lock();
            match uart_writer_mg.as_mut() {
                Some(uart_writer) => {
                    // The returned `Result` can be safely ignored because
                    // `UartWriter::write_str` cannot fail.
                    let _ = core::fmt::Write::write_fmt(
                        uart_writer,
                        core::format_args!($($arg)*),
                    );
                }
                None => {}
            }
        }
    };
}

/// Print to the UART, with a newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::println!("");
    };

    ($($arg:tt)*) => {
        {
            let mut uart_writer_mg =
                $crate::globals::GLOBALS.uart_writer().lock();
            match uart_writer_mg.as_mut() {
                Some(uart_writer) => {
                    // The returned `Result` can be safely ignored because
                    // `UartWriter::write_str` cannot fail.
                    let _ = core::fmt::Write::write_fmt(
                        uart_writer,
                        core::format_args!(
                            "{}\n",
                            core::format_args!($($arg)*),
                        ),
                    );
                }
                None => {}
            }
        }
    };
}
