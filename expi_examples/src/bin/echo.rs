//! UART echo.

#![no_std]
#![no_main]

use expi::println;
use expi::uart;
use expi_macros::entrypoint;

#[entrypoint]
fn kernel_main() {
    println!("expi");

    loop {
        uart::send_byte(uart::recv_byte());
    }
}
