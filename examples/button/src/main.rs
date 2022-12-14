//! Button controlling a LED.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use expi::cpu::exceptions::{self, Interrupt};
use expi::cpu::time;
use expi::gpio::{self, Event, Function, Pin, PullState};
use expi::intc;
use expi::println;
use expi_macros::{entrypoint, exception_handler, exception_vector_table};

/// The LED is connected to GPIO26.
const GPIO_LED: usize = 26;

/// The button is connected to GPIO16.
const GPIO_BUTTON: usize = 16;

/// Kernel main function.
#[entrypoint]
fn kernel_main() {
    println!("expi");

    let pin_led = Pin::try_from(GPIO_LED).unwrap();
    let pin_button = Pin::try_from(GPIO_BUTTON).unwrap();

    // Configure LED GPIO pin.
    gpio::set_function(pin_led, Function::Output);

    // Configure button GPIO pin.
    gpio::set_pull_state(pin_button, PullState::Up);
    gpio::set_function(pin_button, Function::Input);
    gpio::enable_event(pin_button, Event::FallingEdge);

    // Mask all interrupts.
    exceptions::mask(Interrupt::Debug);
    exceptions::mask(Interrupt::SError);
    exceptions::mask(Interrupt::Irq);
    exceptions::mask(Interrupt::Fiq);

    // Enable pysical IRQ routing.
    exceptions::enable_routing(Interrupt::Irq).unwrap();

    // Set vector table address.
    exceptions::set_vector_table(0x90000);

    // Unmask IRQ.
    exceptions::unmask(Interrupt::Irq);

    // Enable GPIO interrupts.
    intc::enable(intc::Peripheral::GPIO).unwrap();

    loop {
        time::delay(1_000_000);
    }
}

/// IRQ handler.
#[exception_handler]
fn irq_handler() {
    /// Stores if the LED is on.
    static mut LED_ON: bool = false;

    let pin_led = Pin::try_from(GPIO_LED).unwrap();
    let pin_button = Pin::try_from(GPIO_BUTTON).unwrap();

    gpio::clear_event(pin_button);

    unsafe {
        if LED_ON {
            gpio::clear(&[pin_led]);
        } else {
            gpio::set(&[pin_led]);
        }

        LED_ON = !LED_ON;
    }
}

/// Unimplemented exception handler.
#[exception_handler]
fn unimplemented_handler() {
    unimplemented!();
}

exception_vector_table! {
    // Exception from the current EL while using SP_EL0.

    // Synchronous.
    unimplemented_handler,
    // IRQ.
    unimplemented_handler,
    // FIQ.
    unimplemented_handler,
    // SError
    unimplemented_handler,

    // Exception from the current EL while using SP_ELx.

    // Synchronous.
    unimplemented_handler,
    // IRQ.
    irq_handler,
    // FIQ.
    unimplemented_handler,
    // SError
    unimplemented_handler,

    // Exception from a lower EL and at least one lower EL is AArch64.

    // Synchronous.
    unimplemented_handler,
    // IRQ.
    unimplemented_handler,
    // FIQ.
    unimplemented_handler,
    // SError
    unimplemented_handler,

    // Exception from a lower EL and all lower ELs are AArch32.

    // Synchronous.
    unimplemented_handler,
    // IRQ.
    unimplemented_handler,
    // FIQ.
    unimplemented_handler,
    // SError
    unimplemented_handler,
}
