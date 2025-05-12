//! Local Timer.

#![no_std]
#![no_main]

use core::arch::asm;

use expi::cpu::exceptions::{self, Exception, Interrupt};
use expi::cpu::mp;
use expi::gpio::{Function, Pin};
use expi::local_intc::{self, IntSource, IntType};
use expi::local_timer;
use expi::println;
use expi_macros::{entrypoint, exception_handler, exception_vector_table};

/// The output pin is GPIO26.
const GPIO_OUT: usize = 26;

/// Stores if the output pin is set.
static mut OUT_SET: bool = false;

/// Kernel main function.
#[entrypoint]
fn kernel_main() {
    println!("expi");

    // Mask all interrupts.
    Interrupt::SError.mask();
    Interrupt::Irq.mask();
    Interrupt::Fiq.mask();
    Exception::Debug.mask();

    // Enable pysical IRQ routing.
    Interrupt::Irq.route();

    // Set vector table address.
    exceptions::set_vector_table(0x81000);

    // Unmask IRQ.
    Interrupt::Irq.unmask();

    // Configure GPIO output.
    let out = Pin::try_from(GPIO_OUT).unwrap();
    out.set_function(Function::Output);

    // Configure local timer.
    IntSource::LocalTimer
        .route(mp::core(), IntType::Irq)
        .unwrap();
    IntSource::LocalTimer.enable();
    local_timer::set_reload_value(1000); // 19.2 kHz
    local_timer::enable();

    loop {
        unsafe { asm!("wfi") };
    }
}

/// IRQ handler.
#[exception_handler]
fn irq_handler() {
    let status = local_intc::irq_status(mp::core());
    if status.pending_local_timer() {
        local_timer_handler()
    }
}

/// Local Timer IRQ handler.
fn local_timer_handler() {
    local_timer::clear();

    let out = Pin::try_from(GPIO_OUT).unwrap();
    unsafe {
        if OUT_SET {
            out.clear();
        } else {
            out.set();
        }
        OUT_SET = !OUT_SET;
    }
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
