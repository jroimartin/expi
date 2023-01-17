//! Devicetree parsing.

#![feature(naked_functions)]
#![no_std]
#![no_main]

use expi::globals::GLOBALS;
use expi::mm;
use expi::println;
use expi_macros::entrypoint;

/// Kernel main function.
#[entrypoint]
fn kernel_main() {
    println!("expi");

    let fdt_mg = GLOBALS.fdt().lock();
    let fdt = fdt_mg.as_ref().unwrap();
    println!("device tree: {:x?}", fdt.tree().root_nodes());

    let free_mem_size = mm::free_memory_size().unwrap() as f32;
    println!("free memory: {} MiB", free_mem_size / 1024.0 / 1024.0);

    let free_memory_mg = GLOBALS.free_memory().lock();
    let free_memory = free_memory_mg.as_ref().unwrap();

    let free_memory_ranges = free_memory.ranges();
    println!("free memory: {:#x?}", free_memory_ranges);
    println!("# ranges: {}", free_memory_ranges.len());

    println!("done");
}
