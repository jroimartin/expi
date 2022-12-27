//! Memory management.

#![feature(naked_functions, panic_info_message)]
#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::arch::asm;
use core::panic::PanicInfo;

use expi::devicetree;
use expi::mailbox;
use expi::uart;
use expi::{print, println};

use range::{Range, RangeSet};

/// Kernel main function.
#[no_mangle]
extern "C" fn kernel_main(dtb_ptr32: u32) {
    // Initialize the UART.
    if uart::init().is_err() {
        return;
    }

    println!("expi");

    // Print VC memory region.
    let (vc_mem_base, vc_mem_size) = mailbox::get_vc_memory().unwrap();
    println!("VC mem: base={vc_mem_base:#x} size={vc_mem_size:#x}");

    let mut free_mem = RangeSet::new();

    // Add ARM memory to the free memory RangeSet.
    let (arm_mem_base, arm_mem_size) = mailbox::get_arm_memory().unwrap();
    println!("ARM mem: base={arm_mem_base:#x} size={arm_mem_size:#x}");

    let arm_mem_region = Range::new(
        arm_mem_base as u64,
        (arm_mem_base + arm_mem_size - 1) as u64,
    )
    .unwrap();
    free_mem.insert(arm_mem_region).unwrap();

    // Parse DTB.
    let simple_fdt =
        unsafe { devicetree::SimpleFdt::parse(dtb_ptr32 as usize).unwrap() };

    // Reserve the memory region where the DTB is stored.
    let fdt_size = simple_fdt.fdt_size();
    println!("DTB mem: base={dtb_ptr32:#x} size={fdt_size:#x}");

    let dtb_region =
        Range::new(dtb_ptr32 as u64, (dtb_ptr32 + fdt_size - 1) as u64)
            .unwrap();
    free_mem.remove(dtb_region).unwrap();

    // Reserve the regions in the DTB's memory reservation block.
    let mem_rsv_block = simple_fdt.mem_rsv_block();
    for region in mem_rsv_block.regions() {
        let addr = region.address();
        let size = region.size();
        let rsv = Range::new(addr, addr + size - 1).unwrap();
        free_mem.remove(rsv).unwrap();
    }

    // Show free memory.
    println!("free memory: {:#x?}", free_mem.ranges());

    // Initialize global allocator.
    //
    // SAFETY: We are accessing a mutable static variable concurrently. It is
    // fine for our experiment, but it must be handled properly in expi using a
    // mutex.
    unsafe { FREE_MEM = Some(free_mem) };

    let free_mem =
        unsafe { FREE_MEM.as_ref().expect("free mem list is not initialized") };

    // Allocate memory using the global allocator.
    let layout = Layout::from_size_align(42, 0x4000).unwrap();
    let ptr = unsafe { alloc::alloc::alloc(layout) };

    // Make sure the compiler does not optimize away the allocation.
    unsafe { core::ptr::write_volatile(ptr, 1u8) };

    println!("before dealloc: {:#x?}", free_mem.ranges());
    unsafe { alloc::alloc::dealloc(ptr, layout) };
    println!("after dealloc: {:#x?}", free_mem.ranges());

    // Create Vec.
    let mut v = vec![0, 1, 2, 3, 4];
    v.push(5);
    println!("v={v:?}");

    println!("before drop(v): {:#x?}", free_mem.ranges());
    drop(v);
    println!("after drop(v): {:#x?}", free_mem.ranges());

    // Create Vec with capacity.
    let mut vwc: Vec<u8> = Vec::with_capacity(42);
    vwc.push(1u8);
    println!("vwc={vwc:?}");

    println!("before drop(vwc): {:#x?}", free_mem.ranges());
    drop(vwc);
    println!("after drop(vwc): {:#x?}", free_mem.ranges());
}

/// Global allocator.
#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

/// [`RangeSet`] with the free memory regions.
static mut FREE_MEM: Option<RangeSet> = None;

/// Allocation error.
#[derive(Debug)]
enum AllocError {
    /// The global allocator has not been initialized.
    Uninitialized,

    /// Alignment must not be zero and it must be a power of two.
    InvalidAlign,

    /// Could not find a suitable memory region for the allocation.
    NotSatisfiable,

    /// The provided pointer cannot be null.
    NullPtr,

    /// The provided size cannot be zero.
    ZeroSize,

    /// An arithmetic operation caused an integer overflow.
    IntegerOverflow,

    /// Error while performing a [`RangeSet`] or [`Range`] operation.
    RangeError(range::Error),
}

impl From<range::Error> for AllocError {
    fn from(err: range::Error) -> AllocError {
        AllocError::RangeError(err)
    }
}

/// A simple allocator that implements the trait [`GlobalAlloc`].
struct GlobalAllocator;

impl GlobalAllocator {
    /// Tries to allocate memory and returns an [`AllocError`] if anything goes
    /// wrong.
    fn try_alloc(&self, layout: Layout) -> Result<*mut u8, AllocError> {
        if layout.size() == 0 {
            return Ok(core::ptr::null_mut());
        }

        if layout.align().count_ones() != 1 {
            return Err(AllocError::InvalidAlign);
        }

        let free_mem =
            unsafe { FREE_MEM.as_mut().ok_or(AllocError::Uninitialized)? };

        let layout = layout.pad_to_align();
        let align = layout.align() as u64;
        let size = layout.size() as u64;

        let mut reserved = None;
        for region in free_mem.ranges() {
            let start = region
                .start()
                .checked_add(align - 1)
                .ok_or(AllocError::IntegerOverflow)?
                & !(align - 1);
            let end = start
                .checked_add(size - 1)
                .ok_or(AllocError::IntegerOverflow)?;
            if end <= region.end() {
                reserved = Some(Range::new(start, end)?);
                break;
            }
        }

        let reserved = reserved.ok_or(AllocError::NotSatisfiable)?;
        free_mem.remove(reserved)?;
        Ok(reserved.start() as *mut u8)
    }

    /// Tries to deallocate memory and returns an [`AllocError`] if anything
    /// goes wrong.
    fn try_dealloc(
        &self,
        ptr: *mut u8,
        layout: Layout,
    ) -> Result<(), AllocError> {
        if ptr.is_null() {
            return Err(AllocError::NullPtr);
        }

        if layout.size() == 0 {
            return Err(AllocError::ZeroSize);
        }

        if layout.align().count_ones() != 1 {
            return Err(AllocError::InvalidAlign);
        }

        let free_mem =
            unsafe { FREE_MEM.as_mut().ok_or(AllocError::Uninitialized)? };

        let layout = layout.pad_to_align();
        let size = layout.size() as u64;

        let start = ptr as u64;
        let end = start
            .checked_add(size - 1)
            .ok_or(AllocError::IntegerOverflow)?;
        let reserved = Range::new(start, end)?;

        free_mem.insert(reserved)?;

        Ok(())
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.try_alloc(layout)
            .unwrap_or_else(|err| panic!("alloc error: {err:?}"))
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.try_dealloc(ptr, layout)
            .unwrap_or_else(|err| panic!("dealloc error: {err:?}"))
    }
}

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
