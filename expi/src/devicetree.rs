//! Devicetree parser.

use alloc::collections::BTreeMap;
use alloc::string::{FromUtf8Error, String};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::array::TryFromSliceError;
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

    /// Malformed devicetree structure.
    MalformedStructure,

    /// Malformed devicetree path.
    MalformedPath,

    /// The entity could not be found.
    NotFound,

    /// The path matches more than one node.
    AmbiguousPath,

    /// Type conversion error.
    ConversionError,

    /// Error while dealing with pointers.
    PtrError(ptr::Error),
}

impl From<ptr::Error> for Error {
    fn from(err: ptr::Error) -> Error {
        Error::PtrError(err)
    }
}

impl From<TryFromSliceError> for Error {
    fn from(_err: TryFromSliceError) -> Error {
        Error::ConversionError
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_err: FromUtf8Error) -> Error {
        Error::ConversionError
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
            Error::MalformedStructure => {
                write!(f, "malformed devicetree structure")
            }
            Error::MalformedPath => write!(f, "malformed devicetree path"),
            Error::NotFound => write!(f, "not found"),
            Error::AmbiguousPath => write!(f, "ambiguous path"),
            Error::ConversionError => write!(f, "conversion error"),
            Error::PtrError(err) => write!(f, "memory access error: {err}"),
        }
    }
}

/// Flattened Devicetree header.
#[derive(Debug)]
pub struct FdtHeader {
    /// Pointer to this FDT header.
    ptr: usize,

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
            ptr,
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

    /// Returns the total size of the Flattened Devicetree.
    pub fn totalsize(&self) -> u32 {
        self.totalsize
    }

    /// Returns the physical ID of the system's boot CPU.
    pub fn boot_cpuid_phys(&self) -> u32 {
        self.boot_cpuid_phys
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
    /// Parses the memory reservation block.
    fn parse(header: &FdtHeader) -> Result<FdtMemRsvBlock, Error> {
        let ptr = header.ptr + (header.off_mem_rsvmap as usize);
        let mut mr = MemReader::new(ptr);

        let mut regions = [FdtMemRsvRegion::default(); FDT_MEM_RSVMAP_SIZE];
        let mut in_use = 0;
        loop {
            if in_use >= FDT_MEM_RSVMAP_SIZE {
                return Err(Error::FullRsvRegions);
            }

            let address = unsafe { mr.read_be::<u64>()? };
            let size = unsafe { mr.read_be::<u64>()? };

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

/// Represents a property of a devicetree node.
#[derive(Debug)]
pub struct FdtProperty(Vec<u8>);

impl FdtProperty {
    /// Returns true if the value is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the value as [`u32`].
    pub fn to_u32(&self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.0.as_slice().try_into()?))
    }

    /// Returns the value as [`u64`].
    pub fn to_u64(&self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.0.as_slice().try_into()?))
    }

    /// Returns the value as [`String`].
    pub fn to_string(&self) -> Result<String, Error> {
        let bytes = self.0.strip_suffix(&[0]).ok_or(Error::ConversionError)?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }

    /// Returns the value as [`String`] list.
    pub fn to_stringlist(&self) -> Result<Vec<String>, Error> {
        let bytes = self.0.strip_suffix(&[0]).ok_or(Error::ConversionError)?;
        let stringlist = bytes
            .split(|x| *x == 0)
            .map(|x| String::from_utf8(x.to_vec()))
            .collect::<Result<Vec<String>, FromUtf8Error>>()?;
        Ok(stringlist)
    }
}

/// Represents a node of the devicetree.
#[derive(Debug)]
pub struct FdtNode {
    /// Path of the node in the devicetree structure.
    path: String,

    /// Node's children.
    children: BTreeMap<String, FdtNode>,

    /// Node's properties.
    properties: BTreeMap<String, FdtProperty>,
}

impl FdtNode {
    /// Returns the path of the node in the devicetree structure.
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Returns the node's children.
    pub fn children(&self) -> &BTreeMap<String, FdtNode> {
        &self.children
    }

    /// Returns the node's properties.
    pub fn properties(&self) -> &BTreeMap<String, FdtProperty> {
        &self.properties
    }

    /// Returns an iterator over the nodes of a devicetree structure starting
    /// at this node.
    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }
}

/// Represents the structure block.
#[derive(Debug)]
pub struct FdtStructure(FdtNode);

impl FdtStructure {
    /// Parses the structure block.
    fn parse(header: &FdtHeader) -> Result<FdtStructure, Error> {
        let ptr = header.ptr + (header.off_dt_struct as usize);
        let mut mr = MemReader::new(ptr);

        // The devicetree structure must begin with a BeginNode token.
        let token = unsafe { mr.read_be::<u32>()?.into() };
        if !matches!(token, FdtToken::BeginNode) {
            return Err(Error::MalformedStructure);
        }

        // Parse nodes.
        let (node_name, node) =
            unsafe { Self::parse_node("", &mut mr, header)? };

        // The devicetree structure must end with an End token.
        let token = unsafe { mr.read_be::<u32>()?.into() };
        if !matches!(token, FdtToken::End) {
            return Err(Error::MalformedStructure);
        }

        // The size of the parsed devicetree structure must match the one in
        // the header.
        let parsed_size = mr.position() - ptr;
        if parsed_size != header.size_dt_struct as usize {
            return Err(Error::MalformedStructure);
        }

        // The name of the root node is an empty string.
        if !node_name.is_empty() {
            return Err(Error::MalformedStructure);
        }

        Ok(FdtStructure(node))
    }

    /// Parses a devicetree node. Returns the tuple `(name, node)`.
    ///
    /// # Safety
    ///
    /// This function accepts a [`MemReader`], allowing it to potentially read
    /// an arbitrary memory address.
    unsafe fn parse_node<P>(
        parent_path: P,
        mr: &mut MemReader,
        header: &FdtHeader,
    ) -> Result<(String, FdtNode), Error>
    where
        P: AsRef<str>,
    {
        let node_name = mr.read_c_string()?;
        // Skip padding.
        mr.set_position((mr.position() + 3) & !3);

        let parent_path = parent_path.as_ref().trim_end_matches('/');
        let mut node = FdtNode {
            path: format!("{parent_path}/{node_name}"),
            children: BTreeMap::new(),
            properties: BTreeMap::new(),
        };

        loop {
            let token = mr.read_be::<u32>()?;
            match token.into() {
                FdtToken::BeginNode => {
                    let (child_name, child) =
                        Self::parse_node(&node.path, mr, header)?;
                    node.children.insert(child_name, child);
                }
                FdtToken::EndNode => break,
                FdtToken::Prop => {
                    let (prop_name, prop) = Self::parse_property(mr, header)?;
                    node.properties.insert(prop_name, prop);
                }
                FdtToken::Nop => {}
                FdtToken::End => return Err(Error::MalformedStructure),
                FdtToken::Unknown => return Err(Error::UnknownToken(token)),
            }
        }

        Ok((node_name, node))
    }

    /// Parses a devicetree property. Returns the tuple `(name, property)`.
    ///
    /// # Safety
    ///
    /// This function accepts a [`MemReader`], allowing it to potentially read
    /// an arbitrary memory address.
    unsafe fn parse_property(
        mr: &mut MemReader,
        header: &FdtHeader,
    ) -> Result<(String, FdtProperty), Error> {
        let strings_ptr = header.ptr + (header.off_dt_strings as usize);
        let strings_size = header.size_dt_strings as usize;

        let len = mr.read_be::<u32>()? as usize;
        let nameoff = mr.read_be::<u32>()? as usize;

        // Check that the string offset is inside the strings block.
        let name_ptr = strings_ptr + nameoff;
        if name_ptr >= strings_ptr + strings_size {
            return Err(Error::MalformedStructure);
        }

        let name = ptr::read_c_string(name_ptr)?;

        let mut value = vec![0u8; len];
        mr.read(&mut value);

        // Skip padding.
        mr.set_position((mr.position() + 3) & !3);

        Ok((name, FdtProperty(value)))
    }

    /// Returns the root node of the [`FdtStructure`].
    pub fn root(&self) -> &FdtNode {
        &self.0
    }

    /// Returns a devicetree node by path. A unit address may be omitted if the
    /// full path to the node is unambiguous.
    pub fn find<P>(&self, path: P) -> Result<&FdtNode, Error>
    where
        P: AsRef<str>,
    {
        let path = path
            .as_ref()
            .strip_prefix('/')
            .ok_or(Error::MalformedPath)?;

        let mut node = &self.0;

        if path.is_empty() {
            return Ok(node);
        }

        for node_name in path.split('/') {
            if node_name.contains('@') {
                // A unit address has been specified and the match must be
                // exact.
                node = node.children().get(node_name).ok_or(Error::NotFound)?;
                continue;
            }

            let matches = node
                .children()
                .iter()
                .filter_map(|(child_name, child_node)| {
                    if node_name == child_name
                        || child_name.starts_with(&format!("{node_name}@"))
                    {
                        Some(child_node)
                    } else {
                        None
                    }
                })
                .collect::<Vec<&FdtNode>>();

            node = match matches.len() {
                0 => return Err(Error::NotFound),
                1 => matches[0],
                _ => return Err(Error::AmbiguousPath),
            };
        }
        Ok(node)
    }

    /// Returns a devicetree node by path. The match must be exact, thus unit
    /// addresses cannot be omitted.
    pub fn find_exact<P>(&self, path: P) -> Result<&FdtNode, Error>
    where
        P: AsRef<str>,
    {
        let path = path
            .as_ref()
            .strip_prefix('/')
            .ok_or(Error::MalformedPath)?;

        let mut node = &self.0;

        if path.is_empty() {
            return Ok(node);
        }

        for node_name in path.split('/') {
            node = node.children().get(node_name).ok_or(Error::NotFound)?;
        }
        Ok(node)
    }

    /// Returns an iterator over the nodes of the devicetree structure.
    pub fn iter(&self) -> Iter {
        self.0.iter()
    }
}

/// EarlyFdt is a simplified version of the Flattened Devicetree. It is the
/// result of parsing the minimum necessary fields required during the early
/// stages of the kernel initialization.
///
/// It does not require a Global Allocator.
#[derive(Debug)]
pub struct EarlyFdt {
    /// FDT header.
    header: FdtHeader,

    /// Memory reservation block.
    mem_rsv_block: FdtMemRsvBlock,
}

impl EarlyFdt {
    /// Parses enough of a Flattened Devicetree to produce an [`EarlyFdt`].
    ///
    /// `ptr` must point to the beginning of a valid FDT.
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    pub unsafe fn parse(ptr: usize) -> Result<EarlyFdt, Error> {
        // Parse devicetree header.
        let header = FdtHeader::parse(ptr)?;

        // Parse reserved memory regions.
        let mem_rsv_block = FdtMemRsvBlock::parse(&header)?;

        Ok(EarlyFdt {
            header,
            mem_rsv_block,
        })
    }

    /// Returns the FDT header.
    pub fn header(&self) -> &FdtHeader {
        &self.header
    }

    /// Returns the memory reservation block.
    pub fn mem_rsv_block(&self) -> &FdtMemRsvBlock {
        &self.mem_rsv_block
    }
}

/// Represents a Flattened Devicetree.
#[derive(Debug)]
pub struct Fdt {
    /// Flattened Devicetree header.
    header: FdtHeader,

    /// Memory reservation block.
    mem_rsv_block: FdtMemRsvBlock,

    /// Devicetree structure.
    structure: FdtStructure,
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
        let header = FdtHeader::parse(ptr)?;

        // Parse reserved memory regions.
        let mem_rsv_block = FdtMemRsvBlock::parse(&header)?;

        // Parse devicetree structure.
        let structure = FdtStructure::parse(&header)?;

        Ok(Fdt {
            header,
            mem_rsv_block,
            structure,
        })
    }

    /// Returns the FDT header.
    pub fn header(&self) -> &FdtHeader {
        &self.header
    }

    /// Returns the memory reservation block.
    pub fn mem_rsv_block(&self) -> &FdtMemRsvBlock {
        &self.mem_rsv_block
    }

    /// Returns the devicetree structure.
    pub fn structure(&self) -> &FdtStructure {
        &self.structure
    }
}

/// Iterator over the nodes of the subree of an [`FdtNode`].
///
/// It yields a reference to every visited node.
pub struct Iter<'a> {
    /// Contains all the nodes in the subree of a given [`FdtNode`].
    items: Vec<&'a FdtNode>,

    /// Index of the next item that must be returned by the iterator.
    cur: usize,
}

impl Iter<'_> {
    /// Creates an iterator that traverses a devicetree structure starting at
    /// `node`.
    fn new(node: &FdtNode) -> Iter {
        Iter {
            items: Self::traverse(node),
            cur: 0,
        }
    }

    /// Traverses the devicetree starting at `node` and returns the visited
    /// nodes.
    fn traverse(node: &FdtNode) -> Vec<&FdtNode> {
        let mut nodes = vec![node];

        for child in node.children().values() {
            let mut visited = Self::traverse(child);
            nodes.append(&mut visited);
        }

        nodes
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a FdtNode;

    fn next(&mut self) -> Option<Self::Item> {
        self.items
            .get(self.cur)
            .map(|item| {
                self.cur += 1;
                item
            })
            .cloned()
    }
}

impl<'a> IntoIterator for &'a FdtStructure {
    type Item = &'a FdtNode;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a FdtNode {
    type Item = &'a FdtNode;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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
