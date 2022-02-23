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

    let temp = mailbox::get_temperature().unwrap() as f64 / 1000_f64;
    println!("SoC temp: {}", temp);

    let (mem_base, mem_size) = mailbox::get_arm_memory().unwrap();
    println!("ARM memory: base={:#x} size={:#x}", mem_base, mem_size);

    loop {
        uart::send_byte(uart::recv_byte());
    }
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
