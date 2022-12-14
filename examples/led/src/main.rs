//! Blinking LED.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use expi::cpu::time;
use expi::gpio::{Function, Pin};
use expi::println;
use expi_macros::entrypoint;

/// The LED is connected to GPIO26.
const GPIO_LED: usize = 26;

/// Kernel main function.
#[entrypoint]
fn kernel_main() {
    println!("expi");

    let pin_led = Pin::try_from(GPIO_LED).unwrap();
    pin_led.set_function(Function::Output);
    loop {
        pin_led.set();
        time::delay(1_000_000);
        pin_led.clear();
        time::delay(1_000_000);
    }
}
