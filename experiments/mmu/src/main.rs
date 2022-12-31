//! MMU experiments.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]
// This is just an experiment and we want to be explicit with some operations.
// For instance, `let x = y | (0 << n)`.
#![allow(clippy::identity_op)]

use core::arch::asm;
use core::panic::PanicInfo;

use expi::{print, println};

/// Kernel main function.
#[no_mangle]
extern "C" fn kernel_main(_dtb_ptr32: u32) {
    println!("expi");

    // Set up memory attributes.
    // 0: b01000100 -> Normal memory, Inner and Outer Non-Cacheable.
    // 1: b11111111 -> Normal memory, Inner and Outer WB WA RA.
    // 2: b00000000 -> Device memory, nGnRnE.
    let mut mair_el2: u64 = 0b0100_0100;
    mair_el2 |= 0b1111_1111 << 8;
    unsafe { asm!("msr mair_el2, {}", in(reg) mair_el2) };

    // Configure the translation regime.
    // Limit VA space to 39 bits (64 - 0x19). We set up 4KB granule, thus
    // translation starts at l1.
    let mut tcr_el2: u64 = 0x19;
    // The MMU is configured to store the translation tables in cacheable
    // memory.
    // Inner cacheability: Normal memory, Inner WB WA RA.
    tcr_el2 |= 0x1 << 8;
    // Outer cacheability: Normal memory, Outer WB WA RA.
    tcr_el2 |= 0x1 << 10;
    // Inner shareable.
    tcr_el2 |= 0x3 << 12;
    // TBI = 0: Top byte not ignored.
    // TG0 = 0: 4KB Granule.
    // IPS = 0: 32-bit IPA space.
    unsafe { asm!("msr tcr_el2, {}", in(reg) tcr_el2) };

    // Template for device memory attributes: UXN=1 PXN=1 AF=1 Indx=2.
    let tmpl_dev_ngnrne: u64 = (1 << 54) | (1 << 53) | (1 << 10) | (2 << 2);

    // Template for normal cacheable memory attributes: AF=1 Indx=1.
    let tmpl_normal_wbwara: u64 = (1 << 10) | (1 << 2);

    // Fill page tables.
    unsafe {
        // Level 1: 1GB entries.
        // 0x00000000-0x3fffffff.
        PAGE_TABLE_L1.0[0] =
            PAGE_TABLE_L2_00000000_3FFFFFFF.0.as_ptr() as u64 | 3;
        // 0x40000000-0x7fffffff. Peripherals (device memory).
        PAGE_TABLE_L1.0[1] = tmpl_dev_ngnrne | 1 | 0x40000000;
        // 0x80000000-0xbfffffff.
        PAGE_TABLE_L1.0[2] = 0;
        // 0xc0000000-0xffffffff.
        PAGE_TABLE_L1.0[3] = 0;
        // 0x100000000-0x13fffffff.
        PAGE_TABLE_L1.0[4] =
            PAGE_TABLE_L2_100000000_13FFFFFFF.0.as_ptr() as u64 | 3;

        // Level 2 0x00000000-0x3fffffff: 2MB entries.
        for (i, entry) in
            PAGE_TABLE_L2_00000000_3FFFFFFF.0.iter_mut().enumerate()
        {
            let baddr = (i * 0x200000) as u64;
            *entry = match baddr {
                // ARM memory (normal memory, cacheable).
                ..=0x3bffffff => tmpl_normal_wbwara | 1 | baddr,
                // VC memory (device memory).
                0x3c000000.. => tmpl_dev_ngnrne | 1 | baddr,
            };
        }

        // Level 2 0x100000000-0x13fffffff: 2MB entries. Map all entries to
        // 0-0x200000 for testing.
        for entry in PAGE_TABLE_L2_100000000_13FFFFFFF.0.iter_mut() {
            *entry = tmpl_normal_wbwara | 1 | 0;
        }
    }

    // Set l1 page table base address.
    let ttbr0_el2 = unsafe { PAGE_TABLE_L1.0.as_ptr() as u64 };
    unsafe { asm!( "msr ttbr0_el2, {}", in(reg) ttbr0_el2) };

    // Invalidate TLBs.
    unsafe {
        asm!(
            r#"
                tlbi alle2
                dsb sy
                isb
            "#
        )
    }

    // Enable MMU (M).
    let mut sctlr_el2: u64 = 1;
    // Enable data and unified caches (C).
    sctlr_el2 |= 1 << 2;
    // Enable instruction caches (I).
    sctlr_el2 |= 1 << 12;
    unsafe { asm!("msr sctlr_el2, {}", in(reg) sctlr_el2) };

    unsafe {
        println!("writing 0x11223344 to 0x100601100...");
        core::ptr::write_volatile(0x100601100 as *mut u32, 0x11223344);

        println!(
            "read 0x1100 (mmu): {:#x}",
            core::ptr::read_volatile(0x1100 as *mut u32)
        );
        println!(
            "read 0x100201100 (mmu): {:#x}",
            core::ptr::read_volatile(0x100201100 as *mut u32)
        );
        println!(
            "read 0x100401100 (mmu): {:#x}",
            core::ptr::read_volatile(0x100401100 as *mut u32)
        );
    }

    // Disable MMU.
    unsafe { asm!("msr sctlr_el2, {}", in(reg) 0u64) };

    unsafe {
        println!(
            "read 0x1100 (no mmu): {:#x}",
            core::ptr::read_volatile(0x1100 as *mut u32)
        );
    }

    println!("done");
}

/// Represents a page table.
#[repr(C, align(0x1000))]
struct PageTable([u64; 512]);

/// Page table level 1.
static mut PAGE_TABLE_L1: PageTable = PageTable([0; 512]);

/// Page table level 2 for 0x00000000-0x3fffffff.
static mut PAGE_TABLE_L2_00000000_3FFFFFFF: PageTable = PageTable([0; 512]);

/// Page table level 2 for 0x100000000-0x13fffffff.
static mut PAGE_TABLE_L2_100000000_13FFFFFFF: PageTable = PageTable([0; 512]);

/// Kernel entrypoint.
#[link_section = ".entry"]
#[no_mangle]
#[naked]
unsafe extern "C" fn _start() -> ! {
    asm!(
        r#"
                ldr x5, =0x80000
                mov sp, x5
                bl kernel_main
            1:
                b 1b
        "#,
        options(noreturn),
    )
}

/// Panic handler.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("\n\n!!! PANIC !!!\n\n");

    if let Some(location) = info.location() {
        print!("{}:{}", location.file(), location.line());
    }

    if let Some(message) = info.message() {
        println!(": {}", message);
    } else {
        println!();
    }

    loop {}
}
