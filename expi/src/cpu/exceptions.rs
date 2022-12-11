//! Exception handling.

use core::arch::asm;

use crate::Error;

/// Interrupt types.
pub enum Interrupt {
    /// Watchpoint, Breakpoint, and Software Step exceptions.
    Debug,

    /// System error exception.
    SError,

    /// IRQ exception.
    Irq,

    /// FIQ exception.
    Fiq,
}

/// DAIF register mask.
struct DaifMask(u64);

impl From<Interrupt> for DaifMask {
    fn from(int: Interrupt) -> DaifMask {
        match int {
            Interrupt::Debug => DaifMask(1 << 8),
            Interrupt::SError => DaifMask(1 << 8),
            Interrupt::Irq => DaifMask(1 << 7),
            Interrupt::Fiq => DaifMask(1 << 6),
        }
    }
}

/// HCR_EL2 register mask.
struct HcrEl2Mask(u64);

impl TryFrom<Interrupt> for HcrEl2Mask {
    type Error = Error;

    fn try_from(int: Interrupt) -> Result<HcrEl2Mask, Self::Error> {
        match int {
            Interrupt::SError => Ok(HcrEl2Mask(1 << 5)),
            Interrupt::Irq => Ok(HcrEl2Mask(1 << 4)),
            Interrupt::Fiq => Ok(HcrEl2Mask(1 << 3)),
            _ => Err(Error::InvalidArg),
        }
    }
}

/// Mask the provided interrupt.
pub fn mask(int: Interrupt) {
    let mut daif: u64;
    unsafe { asm!("mrs {daif}, daif", daif = out(reg) daif) };

    let daif_mask = DaifMask::from(int);
    daif |= daif_mask.0;

    unsafe { asm!("msr daif, {daif}", daif = in(reg) daif) };
}

/// Unmask the provided interrupt.
pub fn unmask(int: Interrupt) {
    let mut daif: u64;
    unsafe { asm!("mrs {daif}, daif", daif = out(reg) daif) };

    let daif_mask = DaifMask::from(int);
    daif &= !daif_mask.0;

    unsafe { asm!("msr daif, {daif}", daif = in(reg) daif) };
}

/// Sets the exception vector table address.
pub fn set_vector_table(address: usize) {
    unsafe { asm!("msr vbar_el2, {address}", address = in(reg) address) };
}

/// Enables physical routing for the provided interrupt.
pub fn enable_routing(int: Interrupt) -> Result<(), Error> {
    let mut hcr_el2: u64;
    unsafe { asm!("mrs {hcr_el2}, hcr_el2", hcr_el2 = out(reg) hcr_el2) };

    let hcr_el2_mask = HcrEl2Mask::try_from(int)?;
    hcr_el2 |= hcr_el2_mask.0;

    unsafe { asm!("msr hcr_el2, {hcr_el2}", hcr_el2 = in(reg) hcr_el2) };

    Ok(())
}
