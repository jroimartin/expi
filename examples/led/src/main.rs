//! Blinking LED.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use expi::cpu::time;
use expi::gpio;
use expi::println;
use expi_macros::entrypoint;

/// The LED is connected to GPIO26.
const GPIO_LED: u32 = 26;

/// Kernel main function.
#[entrypoint]
extern "C" fn kernel_main() {
    println!("expi");

    gpio::set_function(GPIO_LED, gpio::Function::Output).unwrap();
    loop {
        gpio::set(&[GPIO_LED]).unwrap();
        time::delay(1_000_000);
        gpio::clear(&[GPIO_LED]).unwrap();
        time::delay(1_000_000);
    }
}
