//! Multi-processing operations.

use core::arch::asm;

/// Returns the ID of the current core.
pub fn core_id() -> u8 {
    let mut mpidr_el1: u64;
    unsafe { asm!("mrs {}, mpidr_el1", out(reg) mpidr_el1) };
    (mpidr_el1 & 0xff) as u8
}
