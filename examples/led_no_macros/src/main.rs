//! Blinking LED.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

use expi::cpu::time;
use expi::gpio::{Function, Pin};
use expi::uart;
use expi::{print, println};

/// The LED is connected to GPIO26.
const GPIO_LED: usize = 26;

/// Kernel main function.
#[no_mangle]
extern "C" fn kernel_main() {
    // Initialize the UART.
    if uart::init().is_err() {
        return;
    }

    println!("expi");

    let pin_led = Pin::try_from(GPIO_LED).unwrap();
    pin_led.set_function(Function::Output);
    loop {
        pin_led.set();
        time::delay(1_000_000);
        pin_led.clear();
        time::delay(1_000_000);
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
