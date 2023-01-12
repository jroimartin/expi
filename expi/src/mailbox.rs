//! Mailboxes operations.
//!
//! For more information, please see [Mailboxes] and related pages in the
//! Github Raspberry Pi firmware wiki.
//!
//! [Mailboxes]: https://github.com/raspberrypi/firmware/wiki/Mailboxes

use core::fmt;

use crate::cpu::mmu;
use crate::mmio;

/// Base address of the mailbox.
///
/// [/arch/arm/boot/dts/bcm283x.dtsi] describes it:
///
/// ```text
/// mailbox: mailbox@7e00b880 {
///     compatible = "brcm,bcm2835-mbox";
///     reg = <0x7e00b880 0x40>;
///     ...
/// };
/// ```
///
/// [/arch/arm/boot/dts/bcm283x.dtsi]: https://github.com/raspberrypi/linux/blob/770d94882ac145c81af72e9a37180806c3f70bbd/arch/arm/boot/dts/bcm283x.dtsi#L100-L105
const MBOX_BASE: usize = 0xb880;

/// Mailbox0 read/write register. It is used for communication from VC to ARM.
/// From ARM's perspective, it is read-only.
const MBOX_READ: usize = MBOX_BASE;

/// This value is returned as a response code when the request was successful.
const MBOX_REQ_OK: u32 = 0x8000_0000;

/// Mailbox0 status register.
const MBOX_STATUS: usize = MBOX_BASE + 0x18;

/// This bit is set in the status register if there is nothing to read from the
/// mailbox.
const MBOX_STATUS_EMPTY: u32 = 0x4000_0000;

/// This bit is set in the status register if there is no space to write into
/// the mailbox.
const MBOX_STATUS_FULL: u32 = 0x8000_0000;

/// Mailbox1 read/write register. It is used for communication from ARM to VC.
/// From ARM's perspective, it is write-only.
const MBOX_WRITE: usize = MBOX_BASE + 0x20;

/// Property tags channel (ARM -> VC).
const MBOX_CHAN_PROP: u32 = 8;

/// Mailbox error.
#[derive(Debug)]
pub enum Error {
    /// Mailbox request could not be processed.
    RequestFailed,

    /// There is not enough room in the mailbox buffer to allocate the request.
    RequestIsTooBig,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::RequestFailed => {
                write!(f, "mailbox request could not be processed")
            }
            Error::RequestIsTooBig => write!(f, "request it too big"),
        }
    }
}

/// An 16-bytes aligned buffer suitable for mailbox communication.
#[repr(C, align(16))]
struct MboxBuffer([u32; 8192]);

/// Mailbox buffer used for sending and receiving requests.
static mut MBOX_BUFFER: MboxBuffer = MboxBuffer([0u32; 8192]);

/// Sets the UART clock frequency to `freq` Hz.
pub fn set_uartclk_freq(freq: u32) -> Result<(), Error> {
    let tags: [u32; 6] = [
        0x38002, // "Set clock rate" tag id.
        12,      // Value buffer length.
        0,       // Bit 31 is 0 for requests.
        2,       // UART0 clock id.
        freq,    // Clock frequency.
        0,       // Do not skip setting turbo.
    ];

    process_request(&tags)
}

/// Returns the temperature of the SoC in thousandths of a degree C.
pub fn get_temperature() -> Result<u32, Error> {
    let tags: [u32; 5] = [
        0x30006, // "Get temperature" tag id.
        8,       // Value buffer length.
        0,       // Bit 31 is 0 for requests.
        0,       // Temperature id (should be 0).
        0,       // Placeholder for temperature.
    ];

    process_request(&tags)?;

    let temp = unsafe { MBOX_BUFFER.0[6] };

    Ok(temp)
}

/// Returns `(base, size)` of ARM memory.
pub fn get_arm_memory() -> Result<(u32, u32), Error> {
    let tags: [u32; 5] = [
        0x10005, // "Get ARM memory" tag id.
        8,       // Value buffer length.
        0,       // Bit 31 is 0 for requests.
        0,       // Placeholder for base address.
        0,       // Placeholder for size in bytes.
    ];

    process_request(&tags)?;

    let (base, size) = unsafe { (MBOX_BUFFER.0[5], MBOX_BUFFER.0[6]) };

    Ok((base, size))
}

/// Returns `(base, size)` of VideoCore memory.
pub fn get_vc_memory() -> Result<(u32, u32), Error> {
    let tags: [u32; 5] = [
        0x10006, // "Get VC memory" tag id.
        8,       // Value buffer length.
        0,       // Bit 31 is 0 for requests.
        0,       // Placeholder for base address.
        0,       // Placeholder for size in bytes.
    ];

    process_request(&tags)?;

    let (base, size) = unsafe { (MBOX_BUFFER.0[5], MBOX_BUFFER.0[6]) };

    Ok((base, size))
}

/// Issue a new mailbox request with the provided concatenated tags.
pub fn process_request(tags: &[u32]) -> Result<(), Error> {
    unsafe {
        // There must be room for the request, the headers values and the end
        // tag.
        if tags.len() > MBOX_BUFFER.0.len() - 3 {
            return Err(Error::RequestIsTooBig);
        }

        // Clear the buffer.
        MBOX_BUFFER.0.fill(0);

        // Set buffer size in bytes.
        let bufsz = core::mem::size_of_val(&MBOX_BUFFER);
        MBOX_BUFFER.0[0] = bufsz as u32;
        // Set request code to process request.
        MBOX_BUFFER.0[1] = 0;
        // Copy the tags into the buffer used to communicate with the
        // mailbox.
        MBOX_BUFFER.0[2..2 + tags.len()].copy_from_slice(tags);
        // Set end tag.
        MBOX_BUFFER.0[2 + tags.len()] = 0;

        // The mailbox expects the address of the mailbox buffer. The lower 4
        // bits contain the channel.
        let data = (MBOX_BUFFER.0.as_ptr() as u32) & !0xf | MBOX_CHAN_PROP;

        // Wait until there is room for a new request.
        while mmio::read(MBOX_STATUS) & MBOX_STATUS_FULL != 0 {}

        // Send the request.
        mmu::dcache_clean_inval_poc(MBOX_BUFFER.0.as_ptr() as usize, bufsz);
        mmio::write(MBOX_WRITE, data);

        // Wait for the request to be processed.
        while mmio::read(MBOX_STATUS) & MBOX_STATUS_EMPTY != 0 {}

        // The response should return the same data that was sent.
        if mmio::read(MBOX_READ) != data {
            return Err(Error::RequestFailed);
        }

        // Check if the request was processed successfully.
        mmu::dcache_clean_inval_poc(MBOX_BUFFER.0.as_ptr() as usize, bufsz);
        if MBOX_BUFFER.0[1] != MBOX_REQ_OK {
            return Err(Error::RequestFailed);
        }

        Ok(())
    }
}
