//! Utilities to interact with the Performance Monitor Unit.

use core::arch::asm;

/// Enables the cycle counter.
pub fn enable_cycle_counter() {
    unsafe {
        // Count cycles in EL2.
        asm!(
            "msr pmccfiltr_el0, {pmccfiltr_el0}",
            pmccfiltr_el0 = in(reg) 1u64 << 27,
        );

        // Enable cycle counter.
        asm!(
            "msr pmcntenset_el0, {pmcntenset_el0}",
            pmcntenset_el0 = in(reg) 1u64 << 31,
        );

        // Get PMCR_EL0.
        let mut pmcr_el0: u64;
        asm!("mrs {pmcr_el0}, pmcr_el0", pmcr_el0 = out(reg) pmcr_el0);

        // Enable long cycle counter.
        pmcr_el0 |= 1 << 6;

        // Count every clock cycle.
        pmcr_el0 &= !(1 << 3);

        // Reset cycle counter.
        pmcr_el0 |= 1 << 2;

        // Enable counters.
        pmcr_el0 |= 1 << 0;

        // Set PMCR_EL0.
        asm!("msr pmcr_el0, {pmcr_el0}", pmcr_el0 = in(reg) pmcr_el0);

        // Instruction Synchronization Barrier.
        asm!("isb");
    }
}

/// Resets the cycle counter.
#[inline(always)]
pub fn reset_cycle_counter() {
    unsafe {
        // Get PMCR_EL0.
        let mut pmcr_el0: u64;
        asm!("mrs {pmcr_el0}, pmcr_el0", pmcr_el0 = out(reg) pmcr_el0);

        // Reset cycle counter.
        pmcr_el0 |= 1 << 2;

        // Set PMCR_EL0.
        asm!("msr pmcr_el0, {pmcr_el0}", pmcr_el0 = in(reg) pmcr_el0);

        // Instruction Synchronization Barrier.
        asm!("isb");
    }
}

/// Returns the current cycle count.
#[inline(always)]
pub fn cycle_counter() -> u64 {
    unsafe {
        let mut cycles: u64;
        asm!("mrs {cycles}, pmccntr_el0", cycles = out(reg) cycles);
        cycles
    }
}
