//! Exception handling.

use core::arch::asm;

/// Interrupt types.
#[derive(Debug, Copy, Clone)]
pub enum Interrupt {
    /// System error exception.
    SError,

    /// IRQ exception.
    Irq,

    /// FIQ exception.
    Fiq,
}

/// Exception types.
#[derive(Debug, Copy, Clone)]
pub enum Exception {
    /// Interrupt.
    Interrupt(Interrupt),

    /// Debug Exception.
    Debug,
}

impl From<Interrupt> for Exception {
    fn from(int: Interrupt) -> Exception {
        Exception::Interrupt(int)
    }
}

/// DAIF register mask.
#[derive(Debug, Copy, Clone)]
struct DaifMask(u64);

impl From<Exception> for DaifMask {
    fn from(exc: Exception) -> DaifMask {
        match exc {
            Exception::Interrupt(int) => match int {
                Interrupt::SError => DaifMask(1 << 8),
                Interrupt::Irq => DaifMask(1 << 7),
                Interrupt::Fiq => DaifMask(1 << 6),
            },
            Exception::Debug => DaifMask(1 << 9),
        }
    }
}

impl Exception {
    /// Mask the exception.
    pub fn mask(&self) {
        let mut daif: u64;
        unsafe { asm!("mrs {daif}, daif", daif = out(reg) daif) };

        let daif_mask = DaifMask::from(*self);
        daif |= daif_mask.0;

        unsafe { asm!("msr daif, {daif}", daif = in(reg) daif) };
    }

    /// Unmask the exception.
    pub fn unmask(&self) {
        let mut daif: u64;
        unsafe { asm!("mrs {daif}, daif", daif = out(reg) daif) };

        let daif_mask = DaifMask::from(*self);
        daif &= !daif_mask.0;

        unsafe { asm!("msr daif, {daif}", daif = in(reg) daif) };
    }
}

impl Interrupt {
    /// Mask the interrupt.
    pub fn mask(&self) {
        Exception::from(*self).mask()
    }

    /// Unmask the interrupt.
    pub fn unmask(&self) {
        Exception::from(*self).unmask()
    }
}

/// HCR_EL2 register mask.
struct HcrEl2Mask(u64);

impl From<Interrupt> for HcrEl2Mask {
    fn from(int: Interrupt) -> HcrEl2Mask {
        match int {
            Interrupt::SError => HcrEl2Mask(1 << 5),
            Interrupt::Irq => HcrEl2Mask(1 << 4),
            Interrupt::Fiq => HcrEl2Mask(1 << 3),
        }
    }
}

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

/// Sets the exception vector table address.
pub fn set_vector_table(address: usize) {
    unsafe { asm!("msr vbar_el2, {address}", address = in(reg) address) };
}

/// Enables physical routing for the provided interrupt.
pub fn enable_routing(int: Interrupt) {
    let mut hcr_el2: u64;
    unsafe { asm!("mrs {hcr_el2}, hcr_el2", hcr_el2 = out(reg) hcr_el2) };

    let hcr_el2_mask = HcrEl2Mask::from(int);
    hcr_el2 |= hcr_el2_mask.0;

    unsafe { asm!("msr hcr_el2, {hcr_el2}", hcr_el2 = in(reg) hcr_el2) };
}
