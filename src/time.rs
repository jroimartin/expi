//! Time operations.

use core::arch::asm;

/// Wait at least `cycles`.
pub fn delay(cycles: u64) {
    unsafe {
        asm!(
            r#"
                1:
                    subs {cycles}, {cycles}, #1
                    bne 1b
            "#,
            cycles = inout(reg) cycles => _
        )
    }
}
