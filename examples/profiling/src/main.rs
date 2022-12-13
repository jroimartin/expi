//! Simple profiling.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

use expi::cpu::{exceptions, pm, time};
use expi::uart;
use expi::{print, println};

/// Kernel main function.
#[no_mangle]
extern "C" fn kernel_main() {
    // Initialize the UART.
    if uart::init().is_err() {
        return;
    }

    println!("expi");

    println!("Current EL: {:x}", exceptions::current_el());

    pm::enable_cycle_counter();

    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    pm::reset_cycle_counter();
    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    pm::reset_cycle_counter();
    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);
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
