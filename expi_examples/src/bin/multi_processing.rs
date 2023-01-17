//! Boot all cores.

#![feature(naked_functions)]
#![no_std]
#![no_main]

use expi::cpu::mp;
use expi::println;
use expi_macros::entrypoint_mp;

/// Kernel main function.
#[entrypoint_mp]
fn kernel_main() {
    println!("Hello, core {}!", mp::core_id());
    println!("Bye, core {}!", mp::core_id());
}
