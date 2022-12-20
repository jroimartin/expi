//! Devicetree parser.

use crate::ptr::{self, MemReader, Ptr};

/// Size of the array used to store the memory reservation block.
const FDT_MEM_RSVMAP_SIZE: usize = 32;

/// Devicetree error.
#[derive(Debug)]
pub enum Error {
    /// The magic field of the Flattened Devicetree header does not match
    /// "\xd0\x0d\xfe\xed".
    InvalidFdtMagic,

    /// Only Devicetree Format version 17 is supported.
    UnsupportedFdtVersion,

    /// The fixed size array used to store the reserved memory regions is full.
    FullRsvRegions,

    /// Error while dealing with pointers.
    PtrError(ptr::Error),
}

impl From<ptr::Error> for Error {
    fn from(err: ptr::Error) -> Error {
        Error::PtrError(err)
    }
}

/// Reserved memory region.
#[derive(Debug, Default, Copy, Clone)]
pub struct FdtMemRsvRegion {
    /// Address of the reserved memory region.
    address: u64,

    /// Size of the reserved memory region.
    size: u64,
}

impl FdtMemRsvRegion {
    /// Returns the address of the reserved memory region.
    pub fn address(&self) -> u64 {
        self.address
    }

    /// Returns the size of the reserved memory region.
    pub fn size(&self) -> u64 {
        self.size
    }
}

/// Memory reservation block.
#[derive(Debug)]
pub struct FdtMemRsvBlock {
    /// Internal fixed size array to store the reserved memory regions.
    regions: [FdtMemRsvRegion; FDT_MEM_RSVMAP_SIZE],

    /// Number of elements in the fixed size array that are being used.
    in_use: usize,
}

impl FdtMemRsvBlock {
    /// Returns the reserved memory regions.
    pub fn regions(&self) -> &[FdtMemRsvRegion] {
        &self.regions[..self.in_use]
    }
}

/// SimpleFdt is a simplified version of the Flattened Devicetree. It is the
/// result of parsing the minimum necessary fields required during the early
/// stages of the kernel initialization. It does not requires a Global
/// Allocator.
pub struct SimpleFdt {
    /// Total size of the Flattened Devicetree.
    fdt_size: u32,

    /// Memory reservation block.
    mem_rsv_block: FdtMemRsvBlock,
}

impl SimpleFdt {
    /// Parses enough of a Flattened Devicetree to produce an [`SimpleFdt`].
    ///
    /// `ptr` must point to the beginning of a valid DTB.
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    pub unsafe fn parse(ptr: usize) -> Result<SimpleFdt, Error> {
        let mut mr = MemReader::new(ptr.into());

        // Check magic.
        let magic = mr.read_be::<u32>()?;
        if magic != 0xd00dfeed {
            return Err(Error::InvalidFdtMagic);
        }

        // Get DTB's totalsize.
        let totalsize = mr.read_be::<u32>()?;

        // Skip the fields: off_dt_struct: u32 and off_dt_strings: u32.
        mr.skip(2 * 4);

        // Get the offset of the memory reservation block.
        let off_mem_rsvmap = mr.read_be::<u32>()?;

        // Check Flattened Devicetree Format version.
        let version = mr.read_be::<u32>()?;
        if version != 17 {
            return Err(Error::UnsupportedFdtVersion);
        }

        // Parse reserved memory regions.
        mr.set_position(Ptr::from(ptr + (off_mem_rsvmap as usize)));
        let mut regions = [FdtMemRsvRegion::default(); FDT_MEM_RSVMAP_SIZE];
        let mut in_use = 0;
        loop {
            if in_use >= FDT_MEM_RSVMAP_SIZE {
                return Err(Error::FullRsvRegions);
            }

            let address = mr.read_be::<u64>()?;
            let size = mr.read_be::<u64>()?;

            if address == 0 && size == 0 {
                break;
            }

            regions[in_use] = FdtMemRsvRegion { address, size };

            in_use += 1;
        }

        let mem_rsv_block = FdtMemRsvBlock { regions, in_use };

        Ok(SimpleFdt {
            fdt_size: totalsize,
            mem_rsv_block,
        })
    }

    /// Returns the total size of the Flattened Devicetree.
    pub fn fdt_size(&self) -> u32 {
        self.fdt_size
    }

    /// Returns the memory reservation block.
    pub fn mem_rsv_block(&self) -> &FdtMemRsvBlock {
        &self.mem_rsv_block
    }
}
