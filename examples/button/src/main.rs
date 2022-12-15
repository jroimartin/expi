//! Button controlling a LED.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use expi::cpu::exceptions::{self, Interrupt};
use expi::cpu::time;
use expi::gpio::{Event, Function, Pin, PullState};
use expi::intc::{self, IrqSource};
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

    // Configure LED GPIO pin.
    let pin_led = Pin::try_from(GPIO_LED).unwrap();
    pin_led.set_function(Function::Output);

    // Configure button GPIO pin.
    let pin_button = Pin::try_from(GPIO_BUTTON).unwrap();
    pin_button.set_pull_state(PullState::Up);
    pin_button.set_function(Function::Input);
    pin_button.enable_event(Event::FallingEdge);

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
    intc::enable(IrqSource::GPIO);

    loop {
        time::delay(1_000_000);
    }
}

/// IRQ handler.
#[exception_handler]
fn irq_handler() {
    let basic_status = intc::basic_status();
    if basic_status.pending_reg_2() {
        let gpu_status = intc::gpu_status();
        if gpu_status.pending(IrqSource::GPIO).unwrap() {
            gpio_handler();
        }
    }
}

/// GPIO IRQ handler.
fn gpio_handler() {
    /// Stores if the LED is on.
    static mut LED_ON: bool = false;

    let pin_button = Pin::try_from(GPIO_BUTTON).unwrap();
    pin_button.clear_event();

    let pin_led = Pin::try_from(GPIO_LED).unwrap();
    unsafe {
        if LED_ON {
            pin_led.clear();
        } else {
            pin_led.set();
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
