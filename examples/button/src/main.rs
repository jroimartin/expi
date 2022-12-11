//! Button controlling a LED.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use expi::cpu::time;
use expi::gpio;
use expi::println;
use expi_macros::{entrypoint, exception_handler, exception_vector_table};

use core::arch::asm;
use expi::mmio;

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

    // TODO(rm): move interrupt handling into expi.
    /// Base address of the BCM2837 interrupt controller.
    const IRQ_BASE: usize = 0xb000;

    /// IRQ enable 2 register.
    const IRQEN2: usize = IRQ_BASE + 0x214;

    // Mask all exceptions.
    unsafe { asm!("msr daifset, #0b1111") };

    // Enable pysical IRQ routing.
    let mut hcr_el2: u64;
    unsafe { asm!("mrs {hcr_el2}, hcr_el2", hcr_el2 = out(reg) hcr_el2) };
    hcr_el2 |= 1 << 4;
    unsafe { asm!("msr hcr_el2, {hcr_el2}", hcr_el2 = in(reg) hcr_el2) };

    // Set vector table address.
    unsafe { asm!("msr vbar_el2, {addr}", addr = in(reg) 0x90000u64) };

    // Unmask IRQ exceptions.
    unsafe { asm!("msr daifclr, #0b0010") };

    // Enable IRQ 52 (gpio_int[3]) that generates a single interrupt whenever
    // any bit in GPEDSn is set.
    unsafe { mmio::write(IRQEN2, 1 << 20) }

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
