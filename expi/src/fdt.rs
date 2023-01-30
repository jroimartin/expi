//! FDT parser.

use alloc::collections::BTreeMap;
use alloc::ffi::{CString, FromVecWithNulError, IntoStringError};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::array::TryFromSliceError;
use core::ffi::{CStr, FromBytesWithNulError};
use core::fmt;
use core::iter::FusedIterator;
use core::num::TryFromIntError;
use core::slice;
use core::str::Utf8Error;

use crate::globals::GLOBALS;
use crate::ptr::{self, MemReader};

pub mod property;

/// FDT parsing error.
#[derive(Debug)]
pub enum Error {
    /// The FDT blob shall be located at an 8-byte-aligned address.
    Unaligned,

    /// The magic field of the FDT header does not match "\xd0\x0d\xfe\xed".
    InvalidMagic,

    /// Only FDT format version 17 is supported.
    UnsupportedVersion(u32),

    /// A DTSpec boot program should provide an FDT in a format which is
    /// backwards compatible with version 16.
    InvalidLastCompVersion(u32),

    /// The internal fixed-size array is full.
    FullInternalArray,

    /// Unknown token found when parsing the FDT.
    UnknownToken(u32),

    /// Malformed FDT structure block.
    MalformedStructureBlock,

    /// Malformed devicetree path.
    MalformedPath,

    /// The entity could not be found.
    NotFound,

    /// The path matches more than one node.
    AmbiguousPath,

    /// Type conversion error.
    ConversionError,

    /// Out-of-bounds access.
    OutOfBounds,

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

impl From<TryFromIntError> for Error {
    fn from(_err: TryFromIntError) -> Error {
        Error::ConversionError
    }
}

impl From<FromVecWithNulError> for Error {
    fn from(_err: FromVecWithNulError) -> Error {
        Error::ConversionError
    }
}

impl From<FromBytesWithNulError> for Error {
    fn from(_err: FromBytesWithNulError) -> Error {
        Error::ConversionError
    }
}

impl From<IntoStringError> for Error {
    fn from(_err: IntoStringError) -> Error {
        Error::ConversionError
    }
}

impl From<Utf8Error> for Error {
    fn from(_err: Utf8Error) -> Error {
        Error::ConversionError
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Unaligned => write!(f, "the FDT blob is not aligned"),
            Error::InvalidMagic => write!(f, "invalid magic"),
            Error::UnsupportedVersion(version) => {
                write!(f, "unsupported version: {version}")
            }
            Error::InvalidLastCompVersion(version) => {
                write!(f, "invalid last compatible version: {version}")
            }
            Error::FullInternalArray => {
                write!(f, "the internal fixed-size array is full")
            }
            Error::UnknownToken(token) => write!(f, "unknown token: {token}"),
            Error::MalformedStructureBlock => {
                write!(f, "malformed FDT structure block")
            }
            Error::MalformedPath => write!(f, "malformed devicetree path"),
            Error::NotFound => write!(f, "not found"),
            Error::AmbiguousPath => write!(f, "ambiguous path"),
            Error::ConversionError => write!(f, "conversion error"),
            Error::OutOfBounds => write!(f, "out-of-bounds access"),
            Error::PtrError(err) => write!(f, "memory access error: {err}"),
        }
    }
}

/// Flattened Devicetree header.
#[derive(Debug)]
pub struct Header {
    /// Pointer to this FDT header.
    ptr: usize,

    /// FDT magic. Must be 0xd00dfeed.
    magic: u32,

    /// Total size in bytes of the FDT.
    totalsize: u32,

    /// Offset in bytes of the structure block.
    off_dt_struct: u32,

    /// Offset in bytes of the strings block.
    off_dt_strings: u32,

    /// Offset in bytes of the memory reservation block.
    off_mem_rsvmap: u32,

    /// FDT format version.
    version: u32,

    /// Lowest FDT format version with which the version used is backwards
    /// compatible. A DTSpec boot program should provide an FDT in a format
    /// which is backwards compatible with version 16, and thus this fields
    /// shall always contain 16.
    last_comp_version: u32,

    /// Physical ID of the system's boot CPU.
    boot_cpuid_phys: u32,

    /// Length in bytes of the strings block.
    _size_dt_strings: u32,

    /// Length in bytes of the structure block.
    size_dt_struct: u32,
}

impl Header {
    /// Parses the FDT header at `ptr`. This function will return an error if
    /// the header is not valid (e.g. wrong magic or version).
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    unsafe fn parse(ptr: usize) -> Result<Header, Error> {
        // The FDT blob must be 8-byte-aligned to be DTSpec compliant.
        if ptr % 8 != 0 {
            return Err(Error::Unaligned);
        }

        let mut mr = MemReader::new(ptr);

        let header = Header {
            ptr,
            magic: mr.read_be::<u32>()?,
            totalsize: mr.read_be::<u32>()?,
            off_dt_struct: mr.read_be::<u32>()?,
            off_dt_strings: mr.read_be::<u32>()?,
            off_mem_rsvmap: mr.read_be::<u32>()?,
            version: mr.read_be::<u32>()?,
            last_comp_version: mr.read_be::<u32>()?,
            boot_cpuid_phys: mr.read_be::<u32>()?,
            _size_dt_strings: mr.read_be::<u32>()?,
            size_dt_struct: mr.read_be::<u32>()?,
        };

        // Check magic.
        if header.magic != 0xd00dfeed {
            return Err(Error::InvalidMagic);
        }

        // Last compatible version must be 16 to be DTSpec compliant.
        if header.last_comp_version != 16 {
            return Err(Error::InvalidLastCompVersion(
                header.last_comp_version,
            ));
        }

        // This parser only supports version 17.
        if header.version != 17 {
            return Err(Error::UnsupportedVersion(header.version));
        }

        Ok(header)
    }

    /// Returns the pointer to the FDT header.
    pub fn ptr(&self) -> usize {
        self.ptr
    }

    /// Returns the total size of the FDT.
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
pub struct MemRsvRegion {
    /// Address of the reserved memory region.
    address: u64,

    /// Size of the reserved memory region.
    size: u64,
}

impl MemRsvRegion {
    /// Returns the address of the reserved memory region.
    pub fn address(&self) -> u64 {
        self.address
    }

    /// Returns the size of the reserved memory region.
    pub fn size(&self) -> u64 {
        self.size
    }
}

/// Iterator over the entries of the memory reservation block.
///
/// It yields a `Result` with a [`MemRsvRegion`] for every entry. After an
/// error, all successive calls will yield `None`.
#[derive(Debug)]
pub struct MemRsvBlockRegions {
    /// Current position.
    entry_ptr: FdtPtr,

    /// If `done` is true, the `Iterator` has finished.
    done: bool,
}

impl MemRsvBlockRegions {
    /// Creates a [`MemRsvBlockRegions`] iterator.
    fn new(header: &Header) -> MemRsvBlockRegions {
        let entry_ptr = FdtPtr(header.ptr + (header.off_mem_rsvmap as usize));

        MemRsvBlockRegions {
            entry_ptr,
            done: false,
        }
    }

    /// Executes a new iteration. It is called by `Iterator::next`.
    fn iter_next(&mut self) -> Result<Option<MemRsvRegion>, Error> {
        let mut mr = MemReader::new(self.entry_ptr.0);

        let address = unsafe { mr.read_be::<u64>()? };
        let size = unsafe { mr.read_be::<u64>()? };

        if address == 0 && size == 0 {
            return Ok(None);
        }

        self.entry_ptr = FdtPtr(mr.position());
        Ok(Some(MemRsvRegion { address, size }))
    }
}

impl Iterator for MemRsvBlockRegions {
    type Item = Result<MemRsvRegion, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let retval = self.iter_next();

        self.done = match retval {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => true,
        };

        retval.transpose()
    }
}

impl FusedIterator for MemRsvBlockRegions {}

/// The structure block is composed of a sequence of pieces, each beginning
/// with one of these tokens.
enum Token {
    /// Marks the beginning of a node's representation.
    BeginNode,

    /// Marks the end of a node's representation.
    EndNode,

    /// Marks the beginning of the representation of one property in the
    /// FDT.
    Prop,

    /// Ignored.
    Nop,

    /// Marks the end of the structure block.
    End,

    /// Unknown token.
    Unknown,
}

impl From<u32> for Token {
    fn from(token: u32) -> Token {
        match token {
            1 => Token::BeginNode,
            2 => Token::EndNode,
            3 => Token::Prop,
            4 => Token::Nop,
            9 => Token::End,
            _ => Token::Unknown,
        }
    }
}

/// Represents a devicetree property.
#[derive(Debug)]
pub struct Property(Vec<u8>);

impl Property {
    /// Returns true if the value is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the value as `u32`.
    pub fn to_u32(&self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.0.as_slice().try_into()?))
    }

    /// Returns the value as `u64`.
    pub fn to_u64(&self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.0.as_slice().try_into()?))
    }

    /// Returns the value as `String`.
    pub fn to_string(&self) -> Result<String, Error> {
        Ok(CString::from_vec_with_nul(self.0.clone())?.into_string()?)
    }

    /// Returns the value as `String` list.
    pub fn to_stringlist(&self) -> Result<Vec<String>, Error> {
        let stringlist = self
            .0
            .split_inclusive(|x| *x == 0)
            .map(|x| Ok(CString::from_vec_with_nul(x.to_vec())?.into_string()?))
            .collect::<Result<Vec<String>, Error>>()?;
        Ok(stringlist)
    }
}

impl AsRef<[u8]> for Property {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Represents a devicetree node.
#[derive(Debug)]
pub struct Node {
    /// Path of the node in the devicetree.
    path: String,

    /// Node's children.
    children: BTreeMap<String, Node>,

    /// Node's properties.
    properties: BTreeMap<String, Property>,
}

impl Node {
    /// Returns the path of the node.
    pub fn path(&self) -> String {
        self.path.clone()
    }

    /// Returns the node's children.
    pub fn children(&self) -> &BTreeMap<String, Node> {
        &self.children
    }

    /// Returns the node's properties.
    pub fn properties(&self) -> &BTreeMap<String, Property> {
        &self.properties
    }

    /// Returns a property of the node.
    pub fn property(&self, name: impl AsRef<str>) -> Result<&Property, Error> {
        self.properties().get(name.as_ref()).ok_or(Error::NotFound)
    }

    /// Returns an iterator over the nodes of a devicetree starting at this
    /// node.
    pub fn iter(&self) -> Nodes {
        Nodes::new(self)
    }
}

/// Represents the structure block.
#[derive(Debug)]
pub struct StructureBlock(Node);

impl StructureBlock {
    /// Parses the structure block.
    fn parse(header: &Header) -> Result<StructureBlock, Error> {
        let ptr = header.ptr + (header.off_dt_struct as usize);
        let mut mr = MemReader::new(ptr);

        // The FDT structure block must begin with a BeginNode token.
        let token = unsafe { mr.read_be::<u32>()?.into() };
        if !matches!(token, Token::BeginNode) {
            return Err(Error::MalformedStructureBlock);
        }

        // Parse nodes.
        let (node_name, node) =
            unsafe { Self::parse_node("", &mut mr, header)? };

        // The FDT structure block must end with an End token.
        let token = unsafe { mr.read_be::<u32>()?.into() };
        if !matches!(token, Token::End) {
            return Err(Error::MalformedStructureBlock);
        }

        // The size of the parsed FDT structure block must be consistent with
        // the header.
        let parsed_size = mr.position() - ptr;
        if parsed_size != header.size_dt_struct as usize {
            return Err(Error::MalformedStructureBlock);
        }

        // The name of the root node is an empty string.
        if !node_name.is_empty() {
            return Err(Error::MalformedStructureBlock);
        }

        Ok(StructureBlock(node))
    }

    /// Parses a devicetree node. Returns the tuple `(name, node)`.
    ///
    /// # Safety
    ///
    /// This function accepts a [`MemReader`], allowing it to potentially read
    /// an arbitrary memory address.
    unsafe fn parse_node(
        parent_path: impl AsRef<str>,
        mr: &mut MemReader,
        header: &Header,
    ) -> Result<(String, Node), Error> {
        let node_name = mr.read_cstr()?;
        // Skip padding.
        mr.set_position((mr.position() + 3) & !3);

        let parent_path = parent_path.as_ref().trim_end_matches('/');
        let mut node = Node {
            path: format!("{parent_path}/{node_name}"),
            children: BTreeMap::new(),
            properties: BTreeMap::new(),
        };

        loop {
            let token = mr.read_be::<u32>()?;
            match token.into() {
                Token::BeginNode => {
                    let (child_name, child) =
                        Self::parse_node(&node.path, mr, header)?;
                    node.children.insert(child_name, child);
                }
                Token::EndNode => break,
                Token::Prop => {
                    let (prop_name, prop) = Self::parse_property(mr, header)?;
                    node.properties.insert(prop_name, prop);
                }
                Token::Nop => {}
                Token::End => return Err(Error::MalformedStructureBlock),
                Token::Unknown => return Err(Error::UnknownToken(token)),
            }
        }

        Ok((node_name.into(), node))
    }

    /// Parses a devicetree property. Returns the tuple `(name, property)`.
    ///
    /// # Safety
    ///
    /// This function accepts a [`MemReader`], allowing it to potentially read
    /// an arbitrary memory address.
    unsafe fn parse_property(
        mr: &mut MemReader,
        header: &Header,
    ) -> Result<(String, Property), Error> {
        let len = mr.read_be::<u32>()? as usize;
        let nameoff = mr.read_be::<u32>()? as usize;

        let strings_ptr = header.ptr + (header.off_dt_strings as usize);
        let name = ptr::read_cstr(strings_ptr + nameoff)?;

        let mut value = vec![0u8; len];
        mr.read(&mut value);

        // Skip padding.
        mr.set_position((mr.position() + 3) & !3);

        Ok((name.into(), Property(value)))
    }

    /// Returns the root node of the [`StructureBlock`].
    pub fn root(&self) -> &Node {
        &self.0
    }

    /// Returns a devicetree node by path. Unit addresses may be omitted if the
    /// full path to the node is unambiguous.
    pub fn node_matches(&self, path: impl AsRef<str>) -> Result<&Node, Error> {
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
                .collect::<Vec<&Node>>();

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
    pub fn node(&self, path: impl AsRef<str>) -> Result<&Node, Error> {
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

    /// Returns an iterator over the nodes of the devicetree.
    pub fn iter(&self) -> Nodes {
        self.0.iter()
    }
}

/// Represents a reference to a devicetree property.
#[derive(Debug)]
pub struct RefProperty<'a>(&'a [u8]);

impl RefProperty<'_> {
    /// Returns true if the value is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the value as `u32`.
    pub fn to_u32(&self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.0.try_into()?))
    }

    /// Returns the value as `u64`.
    pub fn to_u64(&self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.0.try_into()?))
    }

    /// Returns the value as `&str`.
    pub fn to_str(&self) -> Result<&str, Error> {
        Ok(CStr::from_bytes_with_nul(self.0)?.to_str()?)
    }
}

impl AsRef<[u8]> for RefProperty<'_> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

/// EarlyFdt is a simplified version of [`Fdt`]. It is the result of parsing
/// the minimum necessary fields required during the early stages of the kernel
/// initialization.
///
/// It allows to scan for specific properties without parsing the whole FDT.
/// So, it does not require a Global Allocator.
#[derive(Debug)]
pub struct EarlyFdt {
    /// FDT header.
    header: Header,
}

/// A pointer within the FDT boundaries.
#[derive(Debug, Copy, Clone)]
pub struct FdtPtr(usize);

impl EarlyFdt {
    /// Parses enough of an FDT to produce an [`EarlyFdt`].
    ///
    /// `ptr` must point to the beginning of a valid FDT.
    ///
    /// # Safety
    ///
    /// This function accepts an arbitrary memory address, therefore it is
    /// unsafe.
    pub unsafe fn parse(ptr: usize) -> Result<EarlyFdt, Error> {
        // Parse FDT header.
        let header = Header::parse(ptr)?;

        Ok(EarlyFdt { header })
    }

    /// Returns the FDT header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Returns an iterator over the entries of the memory reservation block.
    pub fn mem_rsv_block_regions(&self) -> MemRsvBlockRegions {
        MemRsvBlockRegions::new(&self.header)
    }

    /// Scans the FDT for a given path and returns a pointer to the node.
    ///
    /// This function requires to parse the FDT until it finds the node. It
    /// does it in-place without allocating memory, so it is suitable for the
    /// early stages of boot when the global allocator is not available.
    pub fn node(&self, path: impl AsRef<str>) -> Result<FdtPtr, Error> {
        let path = path.as_ref();

        if !path.starts_with('/') {
            return Err(Error::MalformedPath);
        }

        let path = if path == "/" { "" } else { path };

        let struct_ptr = self.header.ptr + (self.header.off_dt_struct as usize);
        let mut mr = MemReader::new(struct_ptr);

        let mut node_ptr = None;

        for node_name in path.split('/') {
            let mut level = 0;

            node_ptr = loop {
                let token = unsafe { mr.read_be::<u32>()? };
                match token.into() {
                    Token::BeginNode => {
                        level += 1;

                        let name = unsafe { mr.read_cstr()? };
                        // Skip padding.
                        mr.set_position((mr.position() + 3) & !3);

                        if level != 1 {
                            // Wrong level.
                            continue;
                        }

                        if name != node_name {
                            // Wrong name.
                            continue;
                        }

                        break Some(FdtPtr(mr.position()));
                    }
                    Token::EndNode => {
                        level -= 1;

                        if level < 0 {
                            // The current node ended and we have not found the
                            // child.
                            return Err(Error::NotFound);
                        }
                    }
                    Token::Prop => {
                        let len = unsafe { mr.read_be::<u32>()? as usize };

                        // Skip name offset (4), property value (len) and
                        // padding.
                        mr.skip((4 + len + 3) & !3);
                    }
                    Token::Nop => {}
                    Token::End => return Err(Error::NotFound),
                    Token::Unknown => return Err(Error::UnknownToken(token)),
                }
            };
        }

        node_ptr.ok_or(Error::NotFound)
    }

    /// Scans the FDT for a given property under the provided node.
    ///
    /// This function requires to parse the FDT until it finds the property.
    /// It does it in-place without allocating memory, so it is suitable for
    /// the early stages of boot when the global allocator is not available.
    pub fn property(
        &self,
        node_ptr: FdtPtr,
        property_name: impl AsRef<str>,
    ) -> Result<RefProperty, Error> {
        let mut mr = MemReader::new(node_ptr.0);

        let strings_ptr =
            self.header.ptr + (self.header.off_dt_strings as usize);

        let mut level = 0;

        loop {
            let token = unsafe { mr.read_be::<u32>()? };
            match token.into() {
                Token::BeginNode => {
                    level += 1;

                    let _name = unsafe { mr.read_cstr()? };
                    // Skip padding.
                    mr.set_position((mr.position() + 3) & !3);
                }
                Token::EndNode => {
                    level -= 1;

                    if level < 0 {
                        // The provided node ended and we have not found the
                        // property.
                        break Err(Error::NotFound);
                    }
                }
                Token::Prop => {
                    let len = unsafe { mr.read_be::<u32>()? as usize };

                    if level != 0 {
                        // This is not the provided node.
                        mr.skip((4 + len + 3) & !3);
                        continue;
                    }

                    let nameoff = unsafe { mr.read_be::<u32>()? as usize };

                    let name =
                        unsafe { ptr::read_cstr(strings_ptr + nameoff)? };

                    if name != property_name.as_ref() {
                        // Wrong name. So, skip value and padding.
                        mr.skip((len + 3) & !3);
                        continue;
                    }

                    // We found the requested property at the requested path.
                    let value = unsafe {
                        slice::from_raw_parts(mr.position() as *const u8, len)
                    };
                    break Ok(RefProperty(value));
                }
                Token::Nop => {}
                Token::End => break Err(Error::MalformedStructureBlock),
                Token::Unknown => break Err(Error::UnknownToken(token)),
            }
        }
    }

    /// Returns an iterator over the nodes of the devicetree.
    pub fn iter(&self) -> NodePtrs {
        let root_ptr = self.header.ptr + (self.header.off_dt_struct as usize);
        NodePtrs::new(FdtPtr(root_ptr))
    }
}

/// Represents a Flattened Devicetree.
#[derive(Debug)]
pub struct Fdt {
    /// FDT header.
    header: Header,

    /// Memory reservation block.
    mem_rsv_block: Vec<MemRsvRegion>,

    /// FDT structure block.
    structure_block: StructureBlock,
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
        // Parse FDT header.
        let header = Header::parse(ptr)?;

        // Parse reserved memory regions.
        let mem_rsv_block = MemRsvBlockRegions::new(&header)
            .collect::<Result<Vec<MemRsvRegion>, Error>>()?;

        // Parse FDT structure block.
        let structure_block = StructureBlock::parse(&header)?;

        Ok(Fdt {
            header,
            mem_rsv_block,
            structure_block,
        })
    }

    /// Returns the FDT header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Returns the memory reservation block.
    pub fn mem_rsv_block(&self) -> &[MemRsvRegion] {
        &self.mem_rsv_block
    }

    /// Returns the FDT structure block.
    pub fn structure_block(&self) -> &StructureBlock {
        &self.structure_block
    }
}

/// Iterator over the nodes of the subree of a [`Node`].
///
/// It yields a reference to every visited node.
pub struct Nodes<'a> {
    /// Contains all the nodes in the subree of a given [`Node`].
    items: Vec<&'a Node>,

    /// Index of the next item that must be returned by the iterator.
    cur: usize,
}

impl Nodes<'_> {
    /// Creates an iterator that traverses a devicetree starting at `node`.
    fn new(node: &Node) -> Nodes {
        Nodes {
            items: Self::traverse(node),
            cur: 0,
        }
    }

    /// Traverses the devicetree starting at `node` and returns the visited
    /// nodes.
    fn traverse(node: &Node) -> Vec<&Node> {
        let mut nodes = vec![node];

        for child in node.children().values() {
            let mut visited = Self::traverse(child);
            nodes.append(&mut visited);
        }

        nodes
    }
}

impl<'a> Iterator for Nodes<'a> {
    type Item = &'a Node;

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

impl<'a> IntoIterator for &'a StructureBlock {
    type Item = &'a Node;
    type IntoIter = Nodes<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a Node {
    type Item = &'a Node;
    type IntoIter = Nodes<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the nodes of an [`EarlyFdt`].
///
/// It yields a `Result` with a pointer to every visited node. After an error,
/// all successive calls will yield `None`.
pub struct NodePtrs {
    /// Current position.
    node_ptr: FdtPtr,

    /// If `done` is true, the `Iterator` has finished.
    done: bool,
}

impl NodePtrs {
    /// Creates an iterator that traverses the nodes of an [`EarlyFdt`].
    fn new(node_ptr: FdtPtr) -> NodePtrs {
        NodePtrs {
            node_ptr,
            done: false,
        }
    }

    /// Executes a new iteration. It is called by `Iterator::next`.
    fn iter_next(&mut self) -> Result<Option<FdtPtr>, Error> {
        let mut mr = MemReader::new(self.node_ptr.0);

        loop {
            let token = unsafe { mr.read_be::<u32>()? };
            match token.into() {
                Token::BeginNode => {
                    let _name = unsafe { mr.read_cstr()? };
                    // Skip padding.
                    mr.set_position((mr.position() + 3) & !3);

                    self.node_ptr = FdtPtr(mr.position());
                    break Ok(Some(self.node_ptr));
                }
                Token::EndNode => {}
                Token::Prop => {
                    let len = unsafe { mr.read_be::<u32>()? as usize };
                    // Skip name offset (4), property value (len) and
                    // padding.
                    mr.skip((4 + len + 3) & !3);
                }
                Token::Nop => {}
                Token::End => break Ok(None),
                Token::Unknown => break Ok(None),
            }
        }
    }
}

impl Iterator for NodePtrs {
    type Item = Result<FdtPtr, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let retval = self.iter_next();

        self.done = match retval {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => true,
        };

        retval.transpose()
    }
}

impl FusedIterator for NodePtrs {}

impl IntoIterator for &EarlyFdt {
    type Item = Result<FdtPtr, Error>;
    type IntoIter = NodePtrs;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Initializes the global FDT.
pub fn init(fdt_ptr32: u32) -> Result<(), Error> {
    let mut fdt_mg = GLOBALS.fdt().lock();
    if fdt_mg.is_some() {
        // Already initialized.
        return Ok(());
    }

    let fdt = unsafe { Fdt::parse(fdt_ptr32 as usize)? };
    *fdt_mg = Some(fdt);

    Ok(())
}
