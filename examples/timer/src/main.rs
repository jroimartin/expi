//! System Timer.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use core::arch::asm;

use expi::cpu::exceptions::{self, Exception, Interrupt};
use expi::intc::{self, IrqSource};
use expi::println;
use expi::systimer::{self, SysTimer};
use expi_macros::{entrypoint, exception_handler, exception_vector_table};

/// Time between interrupts.
const TIME: u32 = 5 * systimer::CLOCK_FREQ; // 5s

/// Kernel main function.
#[entrypoint]
fn kernel_main(_dtb_ptr32: u32) {
    println!("expi");

    // Configure system timer.
    let timer = SysTimer::try_from(0).unwrap();
    timer.set_cmp(TIME);

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
    IrqSource::SysTimer0.enable();

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
        if gpu_status.pending(IrqSource::SysTimer0).unwrap() {
            systimer_handler();
        }
    }
}

/// Timer IRQ handler.
fn systimer_handler() {
    let timer = SysTimer::try_from(0).unwrap();

    timer.clear();

    let cmp = timer.cmp();
    timer.set_cmp(cmp.wrapping_add(TIME));

    println!("counter={:#x}", systimer::counter());
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
