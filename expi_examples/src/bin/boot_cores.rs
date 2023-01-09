//! Boot all cores.

#![feature(naked_functions)]
#![no_std]
#![no_main]

use expi::cpu::mp;
use expi::print;
use expi_macros::entrypoint_mp;

/// Kernel main function.
#[entrypoint_mp]
fn kernel_main(_dtb_ptr32: u32) {
    print!("{}", mp::core_id());
}
