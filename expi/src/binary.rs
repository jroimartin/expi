//! Utilities to work with binary data.

/// Allows to create a value from its representation as a byte array in big or
/// little endian.
pub trait FromBytes {
    /// Creates a value from its representation as a byte array in little
    /// endian.
    fn from_le_bytes<B: AsRef<[u8]>>(buf: B) -> Self;

    /// Creates a value from its representation as a byte array in big endian.
    fn from_be_bytes<B: AsRef<[u8]>>(buf: B) -> Self;
}

/// This macro implements the trait [`FromBytes`] for the provided type. This
/// type must provide the methods `from_le_bytes` and `from_be_bytes` as it is
/// the case for Rust's primitive numeric types.
macro_rules! impl_from_bytes {
    ($Ty:ty) => {
        impl FromBytes for $Ty {
            fn from_le_bytes<B: AsRef<[u8]>>(buf: B) -> Self {
                <$Ty>::from_le_bytes(
                    buf.as_ref().try_into().expect("invalid buffer size"),
                )
            }

            fn from_be_bytes<B: AsRef<[u8]>>(buf: B) -> Self {
                <$Ty>::from_be_bytes(
                    buf.as_ref().try_into().expect("invalid buffer size"),
                )
            }
        }
    };
}

// Implement trait FromBytes for unsigned integers.
impl_from_bytes!(u8);
impl_from_bytes!(u16);
impl_from_bytes!(u32);
impl_from_bytes!(u64);
impl_from_bytes!(u128);

// Implement trait FromBytes for signed integers.
impl_from_bytes!(i8);
impl_from_bytes!(i16);
impl_from_bytes!(i32);
impl_from_bytes!(i64);
impl_from_bytes!(i128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_le_bytes() {
        let buf = [0x11, 0x22, 0x33, 0x44];
        assert_eq!(<u32 as FromBytes>::from_le_bytes(&buf), 0x44332211)
    }

    #[test]
    fn from_be_bytes() {
        let buf = [0x11, 0x22, 0x33, 0x44];
        assert_eq!(<u32 as FromBytes>::from_be_bytes(&buf), 0x11223344)
    }
}
