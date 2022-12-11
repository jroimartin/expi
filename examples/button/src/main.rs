//! Button controlling a LED.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use expi::cpu::{exceptions, time};
use expi::gpio;
use expi::intc;
use expi::println;
use expi_macros::{entrypoint, exception_handler, exception_vector_table};

/// The LED is connected to GPIO26.
const GPIO_LED: u32 = 26;

/// The button is connected to GPIO16.
const GPIO_BUTTON: u32 = 16;

/// Kernel main function.
#[entrypoint]
fn kernel_main() {
    println!("expi");

    // Configure LED GPIO pin.
    gpio::set_function(GPIO_LED, gpio::Function::Output).unwrap();

    // Configure button GPIO pin.
    gpio::set_pull_state(GPIO_BUTTON, gpio::PullState::Up).unwrap();
    gpio::set_function(GPIO_BUTTON, gpio::Function::Input).unwrap();
    gpio::set_event(GPIO_BUTTON, gpio::Event::FallingEdge).unwrap();

    // Mask all interrupts.
    exceptions::mask(exceptions::Interrupt::Debug);
    exceptions::mask(exceptions::Interrupt::SError);
    exceptions::mask(exceptions::Interrupt::Irq);
    exceptions::mask(exceptions::Interrupt::Fiq);

    // Enable pysical IRQ routing.
    exceptions::enable_routing(exceptions::Interrupt::Irq).unwrap();

    // Set vector table address.
    exceptions::set_vector_table(0x90000);

    // Unmask IRQ.
    exceptions::unmask(exceptions::Interrupt::Irq);

    // Enable GPIO interrupts.
    intc::enable(intc::Peripheral::GPIO);

    loop {
        time::delay(1_000_000);
    }
}

/// IRQ handler.
#[exception_handler]
fn irq_handler() {
    /// Stores if the LED is on.
    static mut LED_ON: bool = false;

    gpio::clear_events(&[GPIO_BUTTON]).unwrap();

    unsafe {
        if LED_ON {
            gpio::clear(&[GPIO_LED]).unwrap();
        } else {
            gpio::set(&[GPIO_LED]).unwrap();
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
