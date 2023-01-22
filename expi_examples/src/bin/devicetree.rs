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

    // Iterator.
    for (path, node) in fdt.structure().iter() {
        println!("path={path} properties={:x?}", node.properties().keys());
    }

    // Find.
    let node = fdt.structure().find_exact("/").unwrap();
    let prop = node.properties().get("model").unwrap().to_string().unwrap();
    println!("/model: {prop}");

    let node = fdt.structure().find_exact("/arm-pmu").unwrap();
    let prop = node
        .properties()
        .get("compatible")
        .unwrap()
        .to_stringlist()
        .unwrap();
    println!("/arm-pmu/compatible: {prop:?}");

    let node = fdt
        .structure()
        .find_exact("/soc/local_intc@40000000")
        .unwrap();
    let prop = node
        .properties()
        .get("#interrupt-cells")
        .unwrap()
        .to_u32()
        .unwrap();
    println!("/soc/local_intc@40000000/#interrupt-cells: {prop}");

    let node = fdt.structure().find("/soc/local_intc").unwrap();
    let prop = node
        .properties()
        .get("#interrupt-cells")
        .unwrap()
        .to_u32()
        .unwrap();
    println!("/soc/local_intc/#interrupt-cells: {prop}");

    let node = fdt.structure().find_exact("/cpus/cpu@0").unwrap();
    let prop = node
        .properties()
        .get("cpu-release-addr")
        .unwrap()
        .to_u64()
        .unwrap();
    println!("/cpus/cpu@0/cpu-release-addr: {prop:#x}");

    // Free memory.
    let free_mem_size = mm::free_memory_size().unwrap() as f32;
    println!("free memory: {} MiB", free_mem_size / 1024.0 / 1024.0);

    let free_memory_mg = GLOBALS.free_memory().lock();
    let free_memory = free_memory_mg.as_ref().unwrap();

    let free_memory_ranges = free_memory.ranges();
    println!("free memory: {:#x?}", free_memory_ranges);
    println!("# ranges: {}", free_memory_ranges.len());

    println!("done");
}
