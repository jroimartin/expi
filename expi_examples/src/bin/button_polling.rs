//! Button controlling a LED.

#![feature(naked_functions)]
#![no_std]
#![no_main]

use expi::gpio::{Event, Function, Pin, PullState};
use expi::println;
use expi_macros::entrypoint;

/// The LED is connected to GPIO26.
const GPIO_LED: usize = 26;

/// The button is connected to GPIO16.
const GPIO_BUTTON: usize = 16;

/// Kernel main function.
#[entrypoint]
fn kernel_main(_dtb_ptr32: u32) {
    println!("expi");

    // Configure LED GPIO pin.
    let pin_led = Pin::try_from(GPIO_LED).unwrap();
    pin_led.set_function(Function::Output);

    // Configure button GPIO pin.
    let pin_button = Pin::try_from(GPIO_BUTTON).unwrap();
    pin_button.set_pull_state(PullState::Up);
    pin_button.set_function(Function::Input);
    pin_button.enable_event(Event::FallingEdge);

    let mut led_on = false;
    loop {
        if pin_button.detected() {
            pin_button.clear_event();

            if led_on {
                pin_led.clear();
            } else {
                pin_led.set();
            }
            led_on = !led_on;
        }
    }
}
