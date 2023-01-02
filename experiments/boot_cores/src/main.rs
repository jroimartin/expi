//! Boot all cores.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

use expi::cpu::mp;
use expi::globals;
use expi::{print, println};

use range::Range;

/// Stack size per core.
const STACK_SIZE: usize = 32 * 1024 * 1024;

/// Kernel main function.
#[no_mangle]
extern "C" fn kernel_main(_dtb_ptr32: u32) {
    let core_id = mp::core_id();
    print!("{core_id}");
}

/// Initializes kernel globals and allocates memory for all the stacks at the
/// end of the free memory list. It returns the stack top address.
///
/// # Panics
///
/// If it was not possible to initialize the UART, the function enters an
/// infinite loop. Otherwise, it panics.
#[no_mangle]
#[allow(clippy::empty_loop)]
unsafe extern "C" fn _globals_init(dtb_ptr32: u32) -> u64 {
    match globals::init(dtb_ptr32) {
        Ok(_) => {}
        Err(globals::Error::UartError(_)) => loop {},
        Err(err) => panic!("init error: {}", err),
    }

    let mut free_mem_mg = globals::GLOBALS.free_memory().lock();
    let free_mem = free_mem_mg.as_mut().expect("uninitialized allocator");

    let ranges = free_mem.ranges();
    let last_range = ranges.last().expect("out of memory");

    if last_range.size() < (STACK_SIZE * 4) as u64 {
        panic!("last memory chunk is too small");
    }

    let end = last_range.end();
    let start = end - ((STACK_SIZE * 4) as u64) + 1;
    let stack = Range::new(start, end).expect("error creating stack range");
    free_mem.remove(stack).expect("error allocating stacks");

    end
}

/// Kernel entrypoint.
#[link_section = ".entry"]
#[no_mangle]
#[naked]
unsafe extern "C" fn _start() -> ! {
    asm!(
        r#"
                // Save dtb_ptr32 at 0x1000.
                ldr x5, =0x1000
                str x0, [x5]

                // Allocate an initial stack of approximately 0x80000 bytes for
                // core 0. This is a temporary stack used by `_globals_init`.
                ldr x5, =0x80000
                mov sp, x5

                // Initialize globals and get stack top address.
                bl _globals_init

                // Save stack top address at 0x1008.
                ldr x5, =0x1008
                str x0, [x5]

                // All cores but core 0 are waiting for a wakeup event. Once
                // the event is received, they jump to the address stored at
                // 0xe0 (core 1), 0xe8 (core 2) and 0xf0 (core 3) if not zero.
                // Implementation:
                // https://github.com/raspberrypi/tools/blob/master/armstubs/armstub8.S
                adr x5, _mp_start
                mov x6, #0xe0
                str x5, [x6], #0x8
                str x5, [x6], #0x8
                str x5, [x6], #0x8

                sev

                b _mp_start
        "#,
        options(noreturn),
    )
}

/// Kernel entrypoint for cores 1, 2 and 3. Core 0 also jumps here after kernel
/// initialization.
///
/// It expects `dtb_ptr32` at 0x1000 and the stack top address at 0x1008.
#[no_mangle]
unsafe extern "C" fn _mp_start() -> ! {
    asm!(
        r#"
                // Load dtb_ptr32.
                ldr x5, =0x1000
                ldr x0, [x5]

                // Load stack top address.
                ldr x5, =0x1008
                ldr x1, [x5]

                // Get core ID.
                mrs x5, mpidr_el1
                and x5, x5, #0xff

                // Set stack pointer.
                add x5, x5, #1
                mul x5, x5, x2
                sub x5, x1, x5
                add x5, x5, #1
                mov sp, x5

                bl kernel_main

            2:
                b 2b
        "#,
        in("x2") STACK_SIZE,
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
