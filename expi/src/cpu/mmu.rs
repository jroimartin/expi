//! MMU configuration.

use core::arch::asm;

/// Represents a page table.
#[repr(C, align(0x1000))]
struct PageTable([u64; 512]);

/// Page table level 1.
static mut PAGE_TABLE_L1: PageTable = PageTable([0; 512]);

/// Page table level 2 for 0x0000_0000-0x3fff_ffff.
static mut PAGE_TABLE_L2_00000000_3FFFFFFF: PageTable = PageTable([0; 512]);

/// Memory attributes.
///
/// Indx=0: 0b0100_0100: Normal memory, Inner and Outer Non-Cacheable.
/// Indx=1: 0b1111_1111: Normal memory, Inner and Outer WB WA RA.
/// Indx=2: 0b0000_0000: Device memory, nGnRnE.
const MAIR: u64 = 0b0100_0100 | (0b1111_1111 << 8);

/// Template for device memory attributes: UXN=1 PXN=1 AF=1 Indx=2.
const TMPL_DEV_NGNRNE: u64 = (1 << 54) | (1 << 53) | (1 << 10) | (2 << 2);

/// Template for normal cacheable memory attributes: AF=1 SH=3 (inner) Indx=1.
const TMPL_NORMAL_WBWARA: u64 = (1 << 10) | (3 << 8) | (1 << 2);

/// Configure the MMU for identity mapping.
pub fn enable_identity_mapping() {
    // Set l1 page table base address.
    let ttbr0_el2 = unsafe { PAGE_TABLE_L1.0.as_ptr() as u64 };
    unsafe { asm!( "msr ttbr0_el2, {}", in(reg) ttbr0_el2) };

    // Set up memory attributes.
    unsafe { asm!("msr mair_el2, {}", in(reg) MAIR) };

    // Configure the translation regime.
    // Limit VA space to 39 bits (64 - 0x19). We set up 4KB granule, thus
    // translation starts at l1.
    let mut tcr_el2: u64 = 0x19;
    // The MMU is configured to store the translation tables in cacheable
    // memory.
    // Inner cacheability: Normal memory, Inner WB WA RA.
    tcr_el2 |= 1 << 8;
    // Outer cacheability: Normal memory, Outer WB WA RA.
    tcr_el2 |= 1 << 10;
    // Inner shareable.
    tcr_el2 |= 3 << 12;
    // TBI = 0: Top byte not ignored.
    // TG0 = 0: 4KB Granule.
    // IPS = 0: 32-bit IPA space.
    unsafe { asm!("msr tcr_el2, {}", in(reg) tcr_el2) };

    // Ensure changes to system registers are visible before MMU is enabled.
    unsafe { asm!("isb") };

    // Invalidate TLBs.
    unsafe {
        asm!(
            r#"
                tlbi alle2
                dsb sy
                isb
            "#
        );
    }

    // Fill page tables.
    unsafe {
        // Level 1: 1GB entries.

        // 0x0000_0000-0x3fff_ffff.
        PAGE_TABLE_L1.0[0] =
            PAGE_TABLE_L2_00000000_3FFFFFFF.0.as_ptr() as u64 | 3;
        // 0x4000_0000-0x7fff_ffff. Peripherals (device memory).
        PAGE_TABLE_L1.0[1] = TMPL_DEV_NGNRNE | 1 | 0x4000_0000;
        // 0x8000_0000-0xbfff_ffff. Peripherals (device memory).
        PAGE_TABLE_L1.0[2] = TMPL_DEV_NGNRNE | 1 | 0x8000_0000;
        // 0xc000_0000-0xffff_ffff. Peripherals (device memory).
        PAGE_TABLE_L1.0[3] = TMPL_DEV_NGNRNE | 1 | 0xc000_0000;

        // Level 2: 2MB entries.

        // 0x0000_0000-0x3fff_ffff. Identity mapping.
        for (i, entry) in
            PAGE_TABLE_L2_00000000_3FFFFFFF.0.iter_mut().enumerate()
        {
            let baddr = (i * 0x20_0000) as u64;
            *entry = match baddr {
                // ARM memory (normal memory, cacheable).
                ..=0x3bff_ffff => TMPL_NORMAL_WBWARA | 1 | baddr,
                // VC memory (not used by expi).
                0x3c00_0000..=0x3eff_ffff => 0,
                // VC memory (device memory).
                0x3f00_0000.. => TMPL_DEV_NGNRNE | 1 | baddr,
            };
        }

        asm!("dsb sy");
    }

    // Enable MMU (M).
    let mut sctlr_el2: u64 = 1;
    // Enable data and unified caches (C).
    sctlr_el2 |= 1 << 2;
    // Enable instruction caches (I).
    sctlr_el2 |= 1 << 12;

    unsafe {
        asm!(
            r#"
                msr sctlr_el2, {}
                isb
            "#,
            in(reg) sctlr_el2,
        );
    }
}

/// Returns the size of the smallest cache line of all the data caches and
/// unified caches.
pub fn data_line_size() -> usize {
    // Get DminLine, which is the Log2 of the number of words in the smallest
    // cache line of all the data caches and unified caches that are controlled
    // by the PE.
    let mut ctr_el0: u64;
    unsafe { asm!("mrs {}, ctr_el0", out(reg) ctr_el0) };
    let dminline = (ctr_el0 >> 16) & 0xf;

    // Calculate the line size.
    (4 << dminline) as usize
}

/// Cleans and invalidates the data cache for a virtual memory region to Point
/// of Coherency.
///
/// # Safety
///
/// This function takes an arbitrary virtual address that might require an
/// address translation from VA to PA, and that translation might fail.
pub unsafe fn dcache_clean_inval_poc(va: usize, size: usize) {
    let linesz = data_line_size();
    let start = va & !(linesz - 1);
    for addr in (start..va + size).step_by(linesz) {
        unsafe { asm!("dc civac, {}", in(reg) addr) };
    }
    unsafe { asm!("dsb sy") };
}
