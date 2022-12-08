//! Errors.

/// Expi error.
#[derive(Debug)]
pub enum Error {
    /// Invalid GPIO pin.
    InvalidGpioPin(u32),

    /// The size of the provided output parameter is not valid.
    InvalidOutputSize,

    /// Mailbox request could not be processed.
    MailboxRequestFailed,

    /// There is not enough room in the mailbox buffer to allocate the request.
    MailboxRequestIsTooBig,
}
