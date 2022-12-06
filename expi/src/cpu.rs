//! CPU specific operations.

use core::arch::asm;

pub mod pm;
pub mod time;

/// Returns the current Exception Level.
pub fn current_el() -> u64 {
    unsafe {
        let mut current_el: u64;
        asm!(
            "mrs {current_el}, CurrentEL",
            current_el = out(reg) current_el,
        );
        (current_el & (0b11 << 2)) >> 2
    }
}
