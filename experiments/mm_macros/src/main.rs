//! Memory management.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;

use expi::globals::GLOBALS;
use expi::println;
use expi_macros::entrypoint;

/// Kernel main function.
#[entrypoint]
fn kernel_main(_dtb_ptr32: u32) {
    println!("expi");

    unsafe {
        println!(
            "start: {:#x?}",
            GLOBALS.free_memory_mut().as_ref().unwrap().ranges()
        );
    }

    let mut v = vec![0, 1, 2, 3, 4];
    println!("{:?}", v);

    unsafe {
        println!(
            "after vec: {:#x?}",
            GLOBALS.free_memory_mut().as_ref().unwrap().ranges()
        );
    }

    v.push(5);
    println!("{:?}", v);

    unsafe {
        println!(
            "after push: {:#x?}",
            GLOBALS.free_memory_mut().as_ref().unwrap().ranges()
        );
    }

    drop(v);

    unsafe {
        println!(
            "after drop: {:#x?}",
            GLOBALS.free_memory_mut().as_ref().unwrap().ranges()
        );
    }
}
