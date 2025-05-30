//! FDT parsing.

#![no_std]
#![no_main]

use expi::fdt;
use expi::fdt::property::Reg;
use expi::fdt::EarlyFdt;
use expi::globals::GLOBALS;
use expi::mm;
use expi::println;
use expi_macros::entrypoint;

/// FDT example error.
#[allow(dead_code)]
#[derive(Debug)]
enum Error {
    /// Uninit global.
    UninitGlobal,

    /// FDT error.
    Fdt(fdt::Error),

    /// Memory management error.
    Mm(mm::Error),
}

impl From<fdt::Error> for Error {
    fn from(err: fdt::Error) -> Error {
        Error::Fdt(err)
    }
}

impl From<mm::Error> for Error {
    fn from(err: mm::Error) -> Error {
        Error::Mm(err)
    }
}

/// Kernel main function.
#[entrypoint]
fn kernel_main() {
    println!("expi");

    println!("\n--- Fdt ---\n");
    fdt_example().expect("error running Fdt example");

    println!("\n--- Fdt iter ---\n");
    fdt_iter_example().expect("error running Fdt iter example");

    println!("\n--- EarlyFdt ---\n");
    early_fdt_example().expect("error running EarlyFdt example");

    println!("\n--- EarlyFdt iter ---\n");
    early_fdt_iter_example().expect("error running EarlyFdt iter example");

    println!("\n--- Free memory ---\n");
    show_free_memory().expect("error getting free memory");

    println!("done");
}

/// Fdt example.
fn fdt_example() -> Result<(), Error> {
    let fdt_mg = GLOBALS.fdt().lock();
    let fdt = fdt_mg.as_ref().ok_or(Error::UninitGlobal)?;

    let root = fdt.structure_block().node("/")?;
    let model = root.property("model")?.to_string()?;
    println!("/ model: {model}");

    let arm_pmu = fdt.structure_block().node("/arm-pmu")?;
    let compatible = arm_pmu.property("compatible")?.to_stringlist()?;
    println!("/arm-pmu compatible: {compatible:?}");

    let cpu = fdt.structure_block().node("/cpus/cpu@0")?;
    let cpu_release_addr = cpu.property("cpu-release-addr")?.to_u64()?;
    println!("/cpus/cpu@0 cpu-release-addr: {cpu_release_addr:#x}");

    let local_intc = fdt.structure_block().node_matches("/soc/local_intc")?;
    let interrupt_cells = local_intc.property("#interrupt-cells")?.to_u32()?;
    println!("/soc/local_intc #interrupt-cells: {interrupt_cells}");

    println!("---");

    let address_cells = root.property("#address-cells")?.to_u32()?;
    let size_cells = root.property("#size-cells")?.to_u32()?;

    let memory = fdt.structure_block().node("/memory@0")?;
    let memory_reg = memory.property("reg")?;
    let memory_reg = Reg::new(memory_reg, address_cells, size_cells);
    for entry in memory_reg.entries() {
        let entry = entry?;
        println!("/memory@0 entry: {entry:x?}");
    }

    println!("---");

    let mem_rsv_block = fdt.mem_rsv_block();
    println!("{mem_rsv_block:x?}");

    Ok(())
}

/// Fdt iter example.
fn fdt_iter_example() -> Result<(), Error> {
    let fdt_mg = GLOBALS.fdt().lock();
    let fdt = fdt_mg.as_ref().ok_or(Error::UninitGlobal)?;

    for node in fdt.structure_block().iter().take(5) {
        println!(
            "path={} properties={:x?}",
            node.path(),
            node.properties().keys()
        );
    }

    println!("---");

    let cpus = fdt.structure_block().node("/cpus")?;
    for node in cpus {
        println!(
            "path={} properties={:x?}",
            node.path(),
            node.properties().keys()
        );
    }

    Ok(())
}

/// EarlyFdt example.
fn early_fdt_example() -> Result<(), Error> {
    let fdt_mg = GLOBALS.fdt().lock();
    let fdt = fdt_mg.as_ref().ok_or(Error::UninitGlobal)?;

    let early_fdt = unsafe { EarlyFdt::parse(fdt.header().ptr())? };

    let root = early_fdt.node("/")?;
    let address_cells = early_fdt.property(root, "#address-cells")?.to_u32()?;
    let size_cells = early_fdt.property(root, "#size-cells")?.to_u32()?;

    println!("/ #address-cells={address_cells:x} #size-cells={size_cells:x}");

    let memory = early_fdt.node("/memory@0")?;
    let memory_reg = early_fdt.property(memory, "reg")?;
    let memory_reg = Reg::new(memory_reg, address_cells, size_cells);
    for entry in memory_reg.entries() {
        let entry = entry?;
        println!("/memory@0 entry: {entry:x?}");
    }

    println!("---");

    for region in early_fdt.mem_rsv_block_regions() {
        let region = region?;
        println!("{region:x?}");
    }

    Ok(())
}

/// EarlyFdt iter example.
fn early_fdt_iter_example() -> Result<(), Error> {
    let fdt_mg = GLOBALS.fdt().lock();
    let fdt = fdt_mg.as_ref().ok_or(Error::UninitGlobal)?;

    let early_fdt = unsafe { EarlyFdt::parse(fdt.header().ptr())? };

    for node_ptr in early_fdt.iter().take(5) {
        let node_ptr = node_ptr?;
        println!("{:x?}", node_ptr);
    }

    Ok(())
}

/// Shows free memory.
fn show_free_memory() -> Result<(), Error> {
    let free_mem_size = mm::free_memory_size()? as f32;
    println!("free memory: {} MiB", free_mem_size / 1024.0 / 1024.0);

    let free_memory_mg = GLOBALS.free_memory().lock();
    let free_memory = free_memory_mg.as_ref().ok_or(Error::UninitGlobal)?;

    let free_memory_ranges = free_memory.ranges();
    println!("free memory: {:#x?}", free_memory_ranges);
    println!("# ranges: {}", free_memory_ranges.len());

    Ok(())
}
