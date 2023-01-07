//! Cycle counting experiment.

#![feature(naked_functions)]
#![no_std]
#![no_main]

use core::arch::asm;

use expi::println;
use expi_macros::entrypoint;

/// Population size used to calculate stats.
const SIZE: usize = 1000;

/// Kernel main function.
#[entrypoint]
fn kernel_main(_dtb_ptr32: u32) {
    println!("expi");

    let mut current_el: u64;
    unsafe { asm!("mrs {:x}, CurrentEL", out(reg) current_el) };
    println!("Current EL: {:x}", (current_el & (0b11 << 2)) >> 2);

    for &n in &[1, 2, 5, 10, 100, 1000, 10000] {
        let mut samples = [0f64; SIZE];
        samples.fill_with(|| count(n) as f64);

        let mean = samples.iter().sum::<f64>() / (SIZE as f64);

        let var = samples
            .iter()
            .fold(0f64, |acc, x| acc + libm::pow(x - mean, 2.0))
            / (SIZE as f64);

        let sd = libm::sqrt(var);

        let cv = sd / mean;

        println!(
            "n: {:10} | mean: {:15.4} | sd: {:15.4} | cv: {:6.4}",
            n, mean, sd, cv,
        );
    }
}

/// Count cycles for `n` iterations.
fn count(n: usize) -> u64 {
    let start: u64;
    let mut end: u64;

    unsafe {
        asm!(
            r#"
                    // Count cycles in EL2.
                    msr PMCCFILTR_EL0, {pmccfiltr_el0:x}

                    // Enable cycle counter.
                    msr PMCNTENSET_EL0, {pmcntenset_el0:x}

                    // Clear cycle counter and start.
                    mrs {pmcr_el0:x}, PMCR_EL0
                    orr {pmcr_el0:x}, {pmcr_el0:x}, {pmcr_el0_mask:x}
                    msr PMCR_EL0, {pmcr_el0:x}

                    // Serialize msr.
                    isb

                    // Read cycle count before loop.
                    mrs {start}, PMCCNTR_EL0

                    // Loop.
                1:
                    subs {n}, {n}, 1
                    bne 1b

                    // Read cycle count after loop.
                    mrs {end}, PMCCNTR_EL0
            "#,
            pmccfiltr_el0 = in(reg) 1 << 27,
            pmcntenset_el0 = in(reg) 1 << 31,
            pmcr_el0 = out(reg) _,
            pmcr_el0_mask = in(reg) (1 << 0) | (1 << 2),
            n = inout(reg) n => _,
            start = out(reg) start,
            end = out(reg) end,
        )
    }

    end - start
}
