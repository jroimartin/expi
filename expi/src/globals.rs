//! Global resources.

use core::panic::PanicInfo;

use crate::mm;
use crate::{print, println};

use mutex::TicketMutex;
use range::RangeSet;

/// Contains the global resources shared between modules.
pub struct GlobalResources {
    /// [`RangeSet`] with the free memory regions.
    free_memory: TicketMutex<Option<RangeSet>>,
}

/// Global resources shared between modules.
pub static GLOBALS: GlobalResources = GlobalResources::new();

impl GlobalResources {
    /// Creates a new [`GlobalResources`] structure.
    const fn new() -> GlobalResources {
        GlobalResources {
            free_memory: TicketMutex::new(None),
        }
    }

    /// Returns a reference to the list of free memory regions.
    pub fn free_memory(&self) -> &TicketMutex<Option<RangeSet>> {
        &self.free_memory
    }
}

/// Global Allocator.
#[global_allocator]
static GLOBAL_ALLOCATOR: mm::GlobalAllocator = mm::GlobalAllocator;

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
