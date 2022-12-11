//! IRQ handling experiment.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

use expi::cpu::time;
use expi::gpio;
use expi::mmio;
use expi::uart;
use expi::{print, println};

/// The button is connected to GPIO16.
const GPIO_BUTTON: u32 = 16;

/// Base address of the BCM2837 interrupt controller.
const IRQ_BASE: usize = 0xb000;

/// IRQ enable 2 register.
const IRQEN2: usize = IRQ_BASE + 0x214;

/// IRQ basic pending register.
const IRQBP: usize = IRQ_BASE + 0x200;

/// IRQ pending 1 register.
const IRQP1: usize = IRQ_BASE + 0x204;

/// IRQ pending 2 register.
const IRQP2: usize = IRQ_BASE + 0x208;

/// Kernel main function.
#[no_mangle]
extern "C" fn kernel_main() {
    // Initialize the UART.
    if uart::init().is_err() {
        return;
    }

    // Print banner.
    println!("expi");

    // Configure GPIO pins.
    gpio::set_pull_state(GPIO_BUTTON, gpio::PullState::Up).unwrap();
    gpio::set_function(GPIO_BUTTON, gpio::Function::Input).unwrap();
    gpio::set_event(GPIO_BUTTON, gpio::Event::FallingEdge).unwrap();

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
        println!("hi!");
        time::delay(1_000_000);
    }
}

/// IRQ handler.
#[no_mangle]
extern "C" fn irq_handler() {
    print_irq_pending_regs();
    gpio::clear_events(&[GPIO_BUTTON]).unwrap();
    print_irq_pending_regs();
}

/// Print IRQ basic pending, IRQ pending 1 and IRQ pending 2 registers.
fn print_irq_pending_regs() {
    let pending_basic = unsafe { mmio::read(IRQBP) };
    let pending1 = unsafe { mmio::read(IRQP1) };
    let pending2 = unsafe { mmio::read(IRQP2) };
    println!(
        "pending_basic={:x} pending1={:x} pending2={:x}",
        pending_basic, pending1, pending2
    );
}

/// Unimplemented exception handler.
#[no_mangle]
extern "C" fn unimplemented_handler() {
    unimplemented!();
}

/// Panic handler.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("\n\n!!! PANIC !!!\n\n");

    if let Some(location) = info.location() {
        print!("{}:{}", location.file(), location.line());
    }

    if let Some(message) = info.message() {
        println!(": {}", message);
    } else {
        println!();
    }

    loop {}
}

/// Kernel entrypoint.
#[link_section = ".entry"]
#[no_mangle]
#[naked]
unsafe extern "C" fn _start() -> ! {
    asm!(
        r#"
                ldr x5, =0x80000
                mov sp, x5
                bl kernel_main
            1:
                b 1b
        "#,
        options(noreturn),
    )
}

/// Vector table.
#[link_section = ".vector_table"]
#[no_mangle]
#[naked]
unsafe extern "C" fn _vector_table() -> ! {
    asm!(
        r#"
            // Exception from the current EL while using SP_EL0.

            // Synchronous.
            b _unimplemented_handler
            // IRQ.
            .balign 0x80
            b _unimplemented_handler
            // FIQ.
            .balign 0x80
            b _unimplemented_handler
            // SError
            .balign 0x80
            b _unimplemented_handler

            // Exception from the current EL while using SP_ELx.

            // Synchronous.
            .balign 0x80
            b _unimplemented_handler
            // IRQ.
            .balign 0x80
            b _irq_handler
            // FIQ.
            .balign 0x80
            b _unimplemented_handler
            // SError
            .balign 0x80
            b _unimplemented_handler

            // Exception from a lower EL and at least one lower EL is
            // AArch64.

            // Synchronous.
            .balign 0x80
            b _unimplemented_handler
            // IRQ.
            .balign 0x80
            b _unimplemented_handler
            // FIQ.
            .balign 0x80
            b _unimplemented_handler
            // SError
            .balign 0x80
            b _unimplemented_handler

            // Exception from a lower EL and all lower ELs are AArch32.

            // Synchronous.
            .balign 0x80
            b _unimplemented_handler
            // IRQ.
            .balign 0x80
            b _unimplemented_handler
            // FIQ.
            .balign 0x80
            b _unimplemented_handler
            // SError
            .balign 0x80
            b _unimplemented_handler
        "#,
        options(noreturn),
    )
}

/// Stub to unimplemented exception handler.
#[no_mangle]
#[naked]
unsafe extern "C" fn _unimplemented_handler() -> ! {
    asm!(
        r#"
            STP X0, X1, [SP, #-16]!
            STP X2, X3, [SP, #-16]!
            STP X4, X5, [SP, #-16]!
            STP X6, X7, [SP, #-16]!
            STP X8, X9, [SP, #-16]!
            STP X10, X11, [SP, #-16]!
            STP X12, X13, [SP, #-16]!
            STP X14, X15, [SP, #-16]!

            bl unimplemented_handler

            LDP X14, X15, [SP], #16
            LDP X12, X13, [SP], #16
            LDP X10, X11, [SP], #16
            LDP X8, X9, [SP], #16
            LDP X6, X7, [SP], #16
            LDP X4, X5, [SP], #16
            LDP X2, X3, [SP], #16
            LDP X0, X1, [SP], #16

            eret
        "#,
        options(noreturn),
    )
}

/// Stub to IRQ handler.
#[no_mangle]
#[naked]
unsafe extern "C" fn _irq_handler() -> ! {
    asm!(
        r#"
            STP X0, X1, [SP, #-16]!
            STP X2, X3, [SP, #-16]!
            STP X4, X5, [SP, #-16]!
            STP X6, X7, [SP, #-16]!
            STP X8, X9, [SP, #-16]!
            STP X10, X11, [SP, #-16]!
            STP X12, X13, [SP, #-16]!
            STP X14, X15, [SP, #-16]!

            bl irq_handler

            LDP X14, X15, [SP], #16
            LDP X12, X13, [SP], #16
            LDP X10, X11, [SP], #16
            LDP X8, X9, [SP], #16
            LDP X6, X7, [SP], #16
            LDP X4, X5, [SP], #16
            LDP X2, X3, [SP], #16
            LDP X0, X1, [SP], #16

            eret
        "#,
        options(noreturn),
    )
}
