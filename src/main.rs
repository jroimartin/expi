//! expi is an experimental OS for the Raspberry Pi 3 Model B.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

mod errors;
mod gpio;
mod mailbox;
mod mmio;
mod panic;
mod print;
mod time;
mod uart;

use core::arch::asm;

/// Kernel main function.
#[no_mangle]
extern "C" fn kernel_main() {
    // Initialize the UART.
    if uart::init().is_err() {
        return;
    }

    let x = [0, 1, 2, 3, 4];
    println!("x = {:#x?}", x);

    todo!();
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
