//! System Timer.

#![feature(naked_functions)]
#![no_std]
#![no_main]

use core::arch::asm;

use expi::cpu::exceptions::{self, Exception, Interrupt};
use expi::intc::{self, IrqSource};
use expi::println;
use expi::system_timer::{self, SystemTimer};
use expi_macros::{entrypoint, exception_handler, exception_vector_table};

/// Time between interrupts.
const TIME: u32 = 5 * system_timer::CLOCK_FREQ; // 5s

/// Kernel main function.
#[entrypoint]
fn kernel_main(_dtb_ptr32: u32) {
    println!("expi");

    // Mask all interrupts.
    Interrupt::SError.mask();
    Interrupt::Irq.mask();
    Interrupt::Fiq.mask();
    Exception::Debug.mask();

    // Enable pysical IRQ routing.
    Interrupt::Irq.route();

    // Set vector table address.
    exceptions::set_vector_table(0x90000);

    // Unmask IRQ.
    Interrupt::Irq.unmask();

    // Enable System Timer 0 interrupts.
    IrqSource::SystemTimer0.enable();

    // Configure system timer.
    let timer = SystemTimer::try_from(0).unwrap();
    let now = system_timer::counter() as u32;
    timer.set_cmp(now.wrapping_add(TIME));

    loop {
        unsafe { asm!("wfi") };
    }
}

/// IRQ handler.
#[exception_handler]
fn irq_handler() {
    let basic_status = intc::basic_status();
    if basic_status.pending_reg_1() {
        let gpu_status = intc::gpu_status();
        if gpu_status.pending(IrqSource::SystemTimer0).unwrap() {
            system_timer_handler();
        }
    }
}

/// Timer IRQ handler.
fn system_timer_handler() {
    let timer = SystemTimer::try_from(0).unwrap();

    timer.clear();

    let cmp = timer.cmp();
    timer.set_cmp(cmp.wrapping_add(TIME));

    println!("counter={:#x}", system_timer::counter());
}

/// Unimplemented exception handler.
#[exception_handler]
fn unimplemented_handler() {
    unimplemented!();
}

exception_vector_table! {
    // Synchronous.
    unimplemented_handler,
    // IRQ.
    irq_handler,
    // FIQ.
    unimplemented_handler,
    // SError
    unimplemented_handler,
}
