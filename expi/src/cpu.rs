//! CPU specific operations.

pub mod exceptions;
pub mod mmu;
pub mod mp;
pub mod pmu;
pub mod time;

/// Number of CPU cores.
const NCORES: usize = 4;

/// CPU related error.
#[derive(Debug)]
pub enum Error {
    /// Invalid CPU core.
    InvalidCore(usize),
}

/// Represents a CPU core.
#[derive(Debug, Copy, Clone)]
pub struct Core(usize);

impl TryFrom<usize> for Core {
    type Error = Error;

    fn try_from(core: usize) -> Result<Core, Error> {
        if core >= NCORES {
            return Err(Error::InvalidCore(core));
        }
        Ok(Core(core))
    }
}

impl TryFrom<u8> for Core {
    type Error = Error;

    fn try_from(core: u8) -> Result<Core, Error> {
        Core::try_from(core as usize)
    }
}

impl From<Core> for usize {
    fn from(core: Core) -> usize {
        core.0
    }
}
