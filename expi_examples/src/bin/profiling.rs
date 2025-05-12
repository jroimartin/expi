//! Simple profiling.

#![no_std]
#![no_main]

use expi::cpu::{exceptions, pmu, time};
use expi::println;
use expi_macros::entrypoint;

/// Kernel main function.
#[entrypoint]
fn kernel_main() {
    println!("expi");

    println!("Current EL: {:x}", exceptions::current_el());

    pmu::enable_cycle_counter();

    let start = pmu::cycle_counter();
    time::delay(1000);
    let end = pmu::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    let start = pmu::cycle_counter();
    time::delay(1000);
    let end = pmu::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    let start = pmu::cycle_counter();
    time::delay(1000);
    let end = pmu::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    pmu::reset_cycle_counter();
    let start = pmu::cycle_counter();
    time::delay(1000);
    let end = pmu::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    pmu::reset_cycle_counter();
    let start = pmu::cycle_counter();
    time::delay(1000);
    let end = pmu::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);
}
