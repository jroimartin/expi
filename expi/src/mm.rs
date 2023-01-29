//! Memory management.

use core::alloc::{GlobalAlloc, Layout};
use core::fmt;

use crate::fdt;
use crate::fdt::property::Reg;
use crate::globals::GLOBALS;

use range::{Range, RangeSet};

/// Base address of the kernel.
const KERNEL_BASE: u64 = 0x80000;

/// Maximum kernel size.
///
/// A memory region of 16 MiB starting at [`KERNEL_BASE`] is reserved for the
/// kernel when the global allocator is initialized.
const KERNEL_MAX_SIZE: u64 = 16 * 1024 * 1024;

/// Memory management error.
#[derive(Debug)]
pub enum Error {
    /// Could not find a memory node in the devicetree.
    MissingMemoryNode,

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

    /// FDT error.
    FdtError(fdt::Error),

    /// Error while dealing with ranges.
    RangeError(range::Error),
}

impl From<fdt::Error> for Error {
    fn from(err: fdt::Error) -> Error {
        Error::FdtError(err)
    }
}

impl From<range::Error> for Error {
    fn from(err: range::Error) -> Error {
        Error::RangeError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::MissingMemoryNode => {
                write!(f, "could not find a memory node in the devicetree")
            }
            Error::Uninitialized => {
                write!(f, "global allocator is not initialized")
            }
            Error::InvalidAlign => write!(f, "invalid alignment"),
            Error::NotSatisfiable => {
                write!(f, "could not find a suitable memory region")
            }
            Error::NullPtr => write!(f, "pointer is null"),
            Error::ZeroSize => write!(f, "size is zero"),
            Error::IntegerOverflow => write!(f, "integer overflow"),
            Error::FdtError(err) => {
                write!(f, "FDT parsing error: {err}")
            }
            Error::RangeError(err) => write!(f, "range error: {err}"),
        }
    }
}

/// A simple allocator that implements the trait [`GlobalAlloc`].
pub struct GlobalAllocator;

impl GlobalAllocator {
    /// Tries to allocate memory.
    fn try_alloc(&self, layout: Layout) -> Result<*mut u8, Error> {
        if layout.size() == 0 {
            return Ok(core::ptr::null_mut());
        }

        if layout.align().count_ones() != 1 {
            return Err(Error::InvalidAlign);
        }

        let mut free_mem_mg = GLOBALS.free_memory().lock();
        let free_mem = free_mem_mg.as_mut().ok_or(Error::Uninitialized)?;

        let size = alloc_size(&layout);
        let align = layout.align() as u64;

        let mut reserved = None;
        for region in free_mem.ranges() {
            let start = region
                .start()
                .checked_add(align - 1)
                .ok_or(Error::IntegerOverflow)?
                & !(align - 1);
            let end =
                start.checked_add(size - 1).ok_or(Error::IntegerOverflow)?;
            if end <= region.end() {
                reserved = Some(Range::new(start, end)?);
                break;
            }
        }

        let reserved = reserved.ok_or(Error::NotSatisfiable)?;
        free_mem.remove(reserved)?;

        Ok(reserved.start() as *mut u8)
    }

    /// Tries to deallocate memory.
    fn try_dealloc(&self, ptr: *mut u8, layout: Layout) -> Result<(), Error> {
        if ptr.is_null() {
            return Err(Error::NullPtr);
        }

        if layout.size() == 0 {
            return Err(Error::ZeroSize);
        }

        if layout.align().count_ones() != 1 {
            return Err(Error::InvalidAlign);
        }

        let mut free_mem_mg = GLOBALS.free_memory().lock();
        let free_mem = free_mem_mg.as_mut().ok_or(Error::Uninitialized)?;

        let size = alloc_size(&layout);

        let start = ptr as u64;
        let end = start.checked_add(size - 1).ok_or(Error::IntegerOverflow)?;
        let reserved = Range::new(start, end)?;

        free_mem.insert(reserved)?;

        Ok(())
    }
}

/// Returns the allocation size for a given layout.
///
/// It reduces fragmentation by grouping small allocations into frequent
/// allocation sizes.
fn alloc_size(layout: &Layout) -> u64 {
    let layout = layout.pad_to_align();
    let size = layout.size();

    if size <= 32 {
        return 32;
    } else if size <= 64 {
        return 64;
    } else if size <= 128 {
        return 128;
    } else if size <= 256 {
        return 256;
    } else if size <= 512 {
        return 512;
    }

    size as u64
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

/// Initializes the global allocator with the list of free memory regions.
pub fn init(dtb_ptr32: u32) -> Result<(), Error> {
    let mut free_mem_mg = GLOBALS.free_memory().lock();
    if free_mem_mg.is_some() {
        // Already initialized.
        return Ok(());
    }

    let mut free_mem = RangeSet::new();

    // Parse DTB.
    let early_fdt = unsafe { fdt::EarlyFdt::parse(dtb_ptr32 as usize)? };

    // Add ARM memory to the free memory RangeSet.
    let root_off = early_fdt.node("/").unwrap();
    let address_cells = early_fdt.property(root_off, "#address-cells")?;
    let size_cells = early_fdt.property(root_off, "#size-cells")?;

    let mut memory_found = false;
    for node_ptr in &early_fdt {
        if let Ok(device_type) = early_fdt.property(node_ptr, "device_type") {
            if device_type != b"memory\x00" {
                continue;
            }

            memory_found = true;

            let memory_reg = early_fdt.property(node_ptr, "reg")?;
            let memory_reg =
                Reg::decode(memory_reg, address_cells, size_cells)?;

            for &(address, size) in memory_reg.entries() {
                let mem_region =
                    Range::new(address as u64, (address + size - 1) as u64)?;
                free_mem.insert(mem_region)?;
            }
        }
    }
    if !memory_found {
        return Err(Error::MissingMemoryNode);
    }

    // Reserve the memory region where the DTB itself is stored.
    let fdt_size = early_fdt.header().totalsize();
    let dtb_region =
        Range::new(dtb_ptr32 as u64, (dtb_ptr32 + fdt_size - 1) as u64)?;
    free_mem.remove(dtb_region)?;

    // Reserve the regions found in the DTB's memory reservation block.
    let mem_rsv_block = early_fdt.mem_rsv_block();
    for region in mem_rsv_block.regions() {
        let addr = region.address();
        let size = region.size();
        let rsv = Range::new(addr, addr + size - 1)?;
        free_mem.remove(rsv)?;
    }

    // Reserve the memory region where the kernel is located. It starts at 0,
    // because the range [0, KERNEL_BASE) is used for global variables (first
    // two pages) and for the stack used during initialization.
    let kernel_region = Range::new(0, KERNEL_BASE + KERNEL_MAX_SIZE - 1)?;
    free_mem.remove(kernel_region)?;

    // Set globals.
    *free_mem_mg = Some(free_mem);

    Ok(())
}

/// Returns the size in bytes of the memory that is currently free.
pub fn free_memory_size() -> Result<u64, Error> {
    let free_mem_mg = GLOBALS.free_memory().lock();
    let free_mem = free_mem_mg.as_ref().ok_or(Error::Uninitialized)?;
    Ok(free_mem.size())
}
