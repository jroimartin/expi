//! Devicetree parser.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use crate::globals::GLOBALS;
use crate::ptr::{self, MemReader};

/// Size of the array used to store the memory reservation block.
const FDT_MEM_RSVMAP_SIZE: usize = 32;

/// Devicetree parsing error.
#[derive(Debug)]
pub enum Error {
    /// The devicetree blob shall be located at an 8-byte-aligned address.
    Unaligned,

    /// The magic field of the Flattened Devicetree header does not match
    /// "\xd0\x0d\xfe\xed".
    InvalidMagic,

    /// Only Devicetree Format version 17 is supported.
    UnsupportedFdtVersion(u32),

    /// A DTSpec boot program should provide a devicetree in a format which is
    /// backwards compatible with version 16.
    InvalidLastCompVersion(u32),

    /// The fixed size array used to store the reserved memory regions is full.
    FullRsvRegions,

    /// Unknown token found when parsing the devicetree.
    UnknownToken(u32),

    /// The size of the parsed devicetree structure does not match the one in
    /// the devicetree header.
    InvalidStructureSize(usize),

    /// Malformed devicetree.
    Malformed,

    /// Error while dealing with pointers.
    PtrError(ptr::Error),
}

impl From<ptr::Error> for Error {
    fn from(err: ptr::Error) -> Error {
        Error::PtrError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Unaligned => write!(f, "the devicetree blob is not aligned"),
            Error::InvalidMagic => write!(f, "invalid FDT magic"),
            Error::UnsupportedFdtVersion(version) => {
                write!(f, "unsupported FDT version: {version}")
            }
            Error::InvalidLastCompVersion(version) => {
                write!(f, "invalid last compatible version: {version}")
            }
            Error::FullRsvRegions => {
                write!(f, "memory reservation internal buffer is full")
            }
            Error::UnknownToken(token) => write!(f, "unknown token: {token}"),
            Error::InvalidStructureSize(size) => {
                write!(f, "invalid structure size: {size}")
            }
            Error::Malformed => write!(f, "malformed devicetree"),
            Error::PtrError(err) => write!(f, "memory access error: {err}"),
        }
    }
}

/// Flattened Devicetree header.
struct FdtHeader {
    /// DTB magic. Must be 0xd00dfeed.
    magic: u32,

    /// Total size in bytes of the devicetree data structure.
    totalsize: u32,

    /// Offset in bytes of the structure block.
    off_dt_struct: u32,

    /// Offset in bytes of the strings block.
    off_dt_strings: u32,

    /// Offset in bytes of the memory reservation block.
    off_mem_rsvmap: u32,

    /// Version of the devicetree data structure.
    version: u32,

    /// Lowest version of the devicetree data structure with which the version
    /// used is backwards compatible. A DTSpec boot program should provide a
    /// devicetree in a format which is backwards compatible with version 16,
    /// and thus this fields shall always contain 16.
    last_comp_version: u32,

    /// Physical ID of the system's boot CPU.
    boot_cpuid_phys: u32,

    /// Length in bytes of the strings block.
    size_dt_strings: u32,

    /// Length in bytes of the structure block.
    size_dt_struct: u32,
}

impl FdtHeader {
    /// Parses the devicetree header at `ptr`. This function will return an
    /// error if the header is not valid (e.g. wrong magic or version).
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    unsafe fn parse(ptr: usize) -> Result<FdtHeader, Error> {
        // The devicetree blob must be 8-byte-aligned to be DTSpec compliant.
        if ptr % 8 != 0 {
            return Err(Error::Unaligned);
        }

        let mut mr = MemReader::new(ptr);

        let fdt = FdtHeader {
            magic: mr.read_be::<u32>()?,
            totalsize: mr.read_be::<u32>()?,
            off_dt_struct: mr.read_be::<u32>()?,
            off_dt_strings: mr.read_be::<u32>()?,
            off_mem_rsvmap: mr.read_be::<u32>()?,
            version: mr.read_be::<u32>()?,
            last_comp_version: mr.read_be::<u32>()?,
            boot_cpuid_phys: mr.read_be::<u32>()?,
            size_dt_strings: mr.read_be::<u32>()?,
            size_dt_struct: mr.read_be::<u32>()?,
        };

        // Check magic.
        if fdt.magic != 0xd00dfeed {
            return Err(Error::InvalidMagic);
        }

        // Last compatible version must be 16 to be DTSpec compliant.
        if fdt.last_comp_version != 16 {
            return Err(Error::InvalidLastCompVersion(fdt.last_comp_version));
        }

        // This parser only supports version 17.
        if fdt.version != 17 {
            return Err(Error::UnsupportedFdtVersion(fdt.version));
        }

        Ok(fdt)
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
    /// Parses the memory reservation block at `ptr`.
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    unsafe fn parse(ptr: usize) -> Result<FdtMemRsvBlock, Error> {
        let mut mr = MemReader::new(ptr);

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

        Ok(FdtMemRsvBlock { regions, in_use })
    }

    /// Returns the reserved memory regions.
    pub fn regions(&self) -> &[FdtMemRsvRegion] {
        &self.regions[..self.in_use]
    }
}

/// The structure block is composed of a sequence of pieces, each beginning
/// with one of these tokens.
enum FdtToken {
    /// Marks the beginning of a node's representation.
    BeginNode,

    /// Marks the end of a node's representation.
    EndNode,

    /// Marks the beginning of the representation of one property in the
    /// devicetree.
    Prop,

    /// Ignored.
    Nop,

    /// Marks the end of the structure block.
    End,

    /// Unknown token.
    Unknown,
}

impl From<u32> for FdtToken {
    fn from(token: u32) -> FdtToken {
        match token {
            1 => FdtToken::BeginNode,
            2 => FdtToken::EndNode,
            3 => FdtToken::Prop,
            4 => FdtToken::Nop,
            9 => FdtToken::End,
            _ => FdtToken::Unknown,
        }
    }
}

/// Represents a property of a node of the devicetree.
#[derive(Debug)]
pub struct FdtProp {
    /// Property node.
    name: String,

    /// Property value.
    value: Vec<u8>,
}

impl FdtProp {
    /// Returns the name of the property.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value of the property
    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

/// Represents a node of the devicetree.
#[derive(Debug)]
pub struct FdtNode {
    /// Node name.
    name: String,

    /// Node's children.
    children: Vec<FdtNode>,

    /// Node's properties.
    props: Vec<FdtProp>,
}

impl FdtNode {
    /// Returns the name of the node.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the node's children.
    pub fn children(&self) -> &[FdtNode] {
        &self.children
    }

    /// returns the node's properties.
    pub fn prop(&self) -> &[FdtProp] {
        &self.props
    }
}

/// Represents the structure block. Contains the root nodes of the devicetree.
#[derive(Debug)]
pub struct FdtStructure(Vec<FdtNode>);

impl FdtStructure {
    /// Parses the structure block at `ptr`.
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    unsafe fn parse(
        struct_ptr: usize,
        struct_size: usize,
        strings_ptr: usize,
        strings_size: usize,
    ) -> Result<FdtStructure, Error> {
        let mut mr = MemReader::new(struct_ptr);
        let mut root_nodes = Vec::new();
        while let Some(node) =
            FdtStructure::parse_node(&mut mr, strings_ptr, strings_size, None)?
        {
            root_nodes.push(node);
        }

        // The size of the parsed devicetree structure must match the one in
        // the header.
        let parsed_size = mr.position() - struct_ptr;
        if parsed_size != struct_size {
            return Err(Error::InvalidStructureSize(parsed_size));
        }

        Ok(FdtStructure(root_nodes))
    }

    /// Parses a devicetree node and its subtree.
    ///
    /// # Safety
    ///
    /// This function accepts a [`MemReader`], allowing it to potentially read
    /// an arbitrary memory address.
    unsafe fn parse_node(
        mr: &mut MemReader,
        strings_ptr: usize,
        strings_size: usize,
        mut parent: Option<FdtNode>,
    ) -> Result<Option<FdtNode>, Error> {
        loop {
            let token = mr.read_be::<u32>()?;
            match token.into() {
                FdtToken::BeginNode => {
                    let name = mr.read_c_string()?;
                    let node = FdtNode {
                        name,
                        children: Vec::new(),
                        props: Vec::new(),
                    };

                    // Skip padding.
                    mr.set_position((mr.position() + 3) & !3);

                    let node = FdtStructure::parse_node(
                        mr,
                        strings_ptr,
                        strings_size,
                        Some(node),
                    )?
                    .ok_or(Error::Malformed)?;

                    if let Some(mut parent_node) = parent {
                        parent_node.children.push(node);
                        parent = Some(parent_node);
                    } else {
                        break Ok(Some(node));
                    }
                }
                FdtToken::EndNode => {
                    if let Some(parent_node) = parent.take() {
                        break Ok(Some(parent_node));
                    } else {
                        break Err(Error::Malformed);
                    }
                }
                FdtToken::Prop => {
                    let len = mr.read_be::<u32>()? as usize;
                    let nameoff = mr.read_be::<u32>()? as usize;

                    // Check that the string offset is inside the strings
                    // block.
                    let name_ptr = strings_ptr + nameoff;
                    if name_ptr >= strings_ptr + strings_size {
                        break Err(Error::Malformed);
                    }

                    let name = ptr::read_c_string(name_ptr)?;

                    let mut value = vec![0u8; len];
                    mr.read(&mut value);

                    // Skip padding.
                    mr.set_position((mr.position() + 3) & !3);

                    if let Some(mut tmp) = parent.take() {
                        tmp.props.push(FdtProp { name, value });
                        parent = Some(tmp);
                    } else {
                        break Err(Error::Malformed);
                    }
                }
                FdtToken::Nop => {}
                FdtToken::End => break Ok(None),
                FdtToken::Unknown => break Err(Error::UnknownToken(token)),
            }
        }
    }

    /// Returns the root nodes of the [`FdtStructure`].
    pub fn root_nodes(&self) -> &[FdtNode] {
        &self.0
    }
}

/// SimpleFdt is a simplified version of the Flattened Devicetree. It is the
/// result of parsing the minimum necessary fields required during the early
/// stages of the kernel initialization. It does not requires a Global
/// Allocator.
#[derive(Debug)]
pub struct SimpleFdt {
    /// Total size of the Flattened Devicetree.
    fdt_size: u32,

    /// Memory reservation block.
    mem_rsv_block: FdtMemRsvBlock,
}

impl SimpleFdt {
    /// Parses enough of a Flattened Devicetree to produce a [`SimpleFdt`].
    ///
    /// `ptr` must point to the beginning of a valid FDT.
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    pub unsafe fn parse(ptr: usize) -> Result<SimpleFdt, Error> {
        // Parse devicetree header.
        let hdr = FdtHeader::parse(ptr)?;

        // Parse reserved memory regions.
        let mem_rsv_block =
            FdtMemRsvBlock::parse(ptr + (hdr.off_mem_rsvmap as usize))?;

        Ok(SimpleFdt {
            fdt_size: hdr.totalsize,
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

/// Represents a Flattened Devicetree.
#[derive(Debug)]
pub struct Fdt {
    /// Total size of the Flattened Devicetree.
    totalsize: u32,

    /// Version of the devicetree data structure.
    version: u32,

    /// Lowest version of the devicetree data structure with which the version
    /// used is backwards compatible.
    last_comp_version: u32,

    /// Physical ID of the system's boot CPU.
    boot_cpuid_phys: u32,

    /// Memory reservation block.
    mem_rsv_block: FdtMemRsvBlock,

    /// Devicetree structure.
    tree: FdtStructure,
}

impl Fdt {
    /// Parses a Flattened Devicetree to produce an [`Fdt`].
    ///
    /// `ptr` must point to the beginning of a valid FDT.
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    pub unsafe fn parse(ptr: usize) -> Result<Fdt, Error> {
        // Parse devicetree header.
        let hdr = FdtHeader::parse(ptr)?;

        // Parse reserved memory regions.
        let mem_rsv_block =
            FdtMemRsvBlock::parse(ptr + (hdr.off_mem_rsvmap as usize))?;

        // Parse devicetree structure.
        let tree = FdtStructure::parse(
            ptr + (hdr.off_dt_struct as usize),
            hdr.size_dt_struct as usize,
            ptr + (hdr.off_dt_strings as usize),
            hdr.size_dt_strings as usize,
        )?;

        Ok(Fdt {
            totalsize: hdr.totalsize,
            version: hdr.version,
            last_comp_version: hdr.last_comp_version,
            boot_cpuid_phys: hdr.boot_cpuid_phys,
            mem_rsv_block,
            tree,
        })
    }

    /// Returns the total size of the Flattened Devicetree.
    pub fn totalsize(&self) -> u32 {
        self.totalsize
    }

    /// Returns the version of the devicetree data structure.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Returns the lowest version of the devicetree data structure with which
    /// the version used is backwards compatible.
    pub fn last_comp_version(&self) -> u32 {
        self.last_comp_version
    }

    /// Returns the physical ID of the system's boot CPU.
    pub fn boot_cpuid_phys(&self) -> u32 {
        self.boot_cpuid_phys
    }

    /// Returns the memory reservation block.
    pub fn mem_rsv_block(&self) -> &FdtMemRsvBlock {
        &self.mem_rsv_block
    }

    /// Returns the devicetree structure.
    pub fn tree(&self) -> &FdtStructure {
        &self.tree
    }
}

/// Initializes the global devicetree.
pub fn init(dtb_ptr32: u32) -> Result<(), Error> {
    let mut fdt_mg = GLOBALS.fdt().lock();
    if fdt_mg.is_some() {
        // Already initialized.
        return Ok(());
    }

    let fdt = unsafe { Fdt::parse(dtb_ptr32 as usize)? };
    *fdt_mg = Some(fdt);

    Ok(())
}
