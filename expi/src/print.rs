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
        let _ = core::fmt::Write::write_fmt(
            &mut $crate::print::UartWriter,
            core::format_args!($($arg)*),
        );
    };
}

/// Print to the UART, with a newline.
#[macro_export]
macro_rules! println {
    () => {
        $crate::println!("");
    };

    ($($arg:tt)*) => {
        let _ = core::fmt::Write::write_fmt(
            &mut $crate::print::UartWriter,
            core::format_args!("{}\n", core::format_args!($($arg)*)),
        );
    };
}
