//! FDT parsing.

#![feature(naked_functions)]
#![no_std]
#![no_main]

use expi::fdt::property::Reg;
use expi::fdt::EarlyFdt;
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

    println!("\n--- ITERATOR ---\n");

    for node in fdt.structure_block() {
        println!(
            "path={} properties={:x?}",
            node.path(),
            node.properties().keys()
        );
    }

    // Find and Iterator.

    println!("\n--- FIND + ITERATOR ---\n");

    let cpus = fdt.structure_block().node("/cpus").unwrap();
    for node in cpus {
        println!(
            "path={} properties={:x?}",
            node.path(),
            node.properties().keys()
        );
    }

    // Find.

    println!("\n--- FIND ---\n");

    let node = fdt.structure_block().node("/").unwrap();
    let prop = node.properties().get("model").unwrap().to_string().unwrap();
    println!("/ model: {prop}");

    let node = fdt.structure_block().node("/arm-pmu").unwrap();
    let prop = node
        .properties()
        .get("compatible")
        .unwrap()
        .to_stringlist()
        .unwrap();
    println!("/arm-pmu compatible: {prop:?}");

    let node = fdt
        .structure_block()
        .node("/soc/local_intc@40000000")
        .unwrap();
    let prop = node
        .properties()
        .get("#interrupt-cells")
        .unwrap()
        .to_u32()
        .unwrap();
    println!("/soc/local_intc@40000000 #interrupt-cells: {prop}");

    let node = fdt
        .structure_block()
        .node_matches("/soc/local_intc")
        .unwrap();
    let prop = node
        .properties()
        .get("#interrupt-cells")
        .unwrap()
        .to_u32()
        .unwrap();
    println!("/soc/local_intc #interrupt-cells: {prop}");

    let node = fdt.structure_block().node("/cpus/cpu@0").unwrap();
    let prop = node
        .properties()
        .get("cpu-release-addr")
        .unwrap()
        .to_u64()
        .unwrap();
    println!("/cpus/cpu@0 cpu-release-addr: {prop:#x}");

    // Scan.

    println!("\n--- SCAN ---\n");

    let early_fdt = unsafe { EarlyFdt::parse(fdt.header().ptr()).unwrap() };

    let node = early_fdt.node("/").unwrap();
    let address_cells = early_fdt.property(node, "#address-cells").unwrap();
    let size_cells = early_fdt.property(node, "#size-cells").unwrap();

    println!("/ #address-cells={address_cells:x?} #size-cells={size_cells:x?}");

    let node = early_fdt.node("/memory@0").unwrap();
    let reg = early_fdt.property(node, "reg").unwrap();

    println!("/memory@0 reg: {reg:x?}");

    let reg = Reg::decode(reg, address_cells, size_cells).unwrap();
    let reg_entries = reg.entries();

    println!("/memory@0 entries: {reg_entries:x?}");

    // Free memory.

    println!("\n--- FREE MEMORY ---\n");

    let free_mem_size = mm::free_memory_size().unwrap() as f32;
    println!("free memory: {} MiB", free_mem_size / 1024.0 / 1024.0);

    let free_memory_mg = GLOBALS.free_memory().lock();
    let free_memory = free_memory_mg.as_ref().unwrap();

    let free_memory_ranges = free_memory.ranges();
    println!("free memory: {:#x?}", free_memory_ranges);
    println!("# ranges: {}", free_memory_ranges.len());

    println!("done");
}
