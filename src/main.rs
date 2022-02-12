#![feature(naked_functions)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn kernel_main() {
    unsafe {
        core::ptr::write_volatile(0xaabbccdd as *mut u32, 0x55667788);
    }
}

#[link_section = ".entry"]
#[no_mangle]
#[naked]
unsafe extern "C" fn _start() -> ! {
    asm!(
        r#"
                ldr x1, =0x800ff
                mov sp, x1
                bl kernel_main
            1:
                b 1b
        "#,
        options(noreturn),
    )
}
