//! Simple profiling.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

use expi::cpu::{exceptions, pm, time};
use expi::println;
use expi_macros::entrypoint;

/// Kernel main function.
#[entrypoint]
fn kernel_main() {
    println!("expi");

    println!("Current EL: {:x}", exceptions::current_el());

    pm::enable_cycle_counter();

    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    pm::reset_cycle_counter();
    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);

    pm::reset_cycle_counter();
    let start = pm::cycle_counter();
    time::delay(1000);
    let end = pm::cycle_counter();
    println!("start={} end={} cycles={}", start, end, end - start);
}
