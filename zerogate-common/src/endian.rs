//! Big-endian wrapper types for ABI-stable network byte order fields.
//!
//! These wrappers have deterministic size and alignment (packed).
//! They do not contain host-dependent fields or hidden padding.

/// Big-endian unsigned 16-bit integer.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeU16 {
    bytes: [u8; 2],
}

impl BeU16 {
    /// Create from a native-endian u16.
    pub fn from_native(val: u16) -> Self {
        Self {
            bytes: val.to_be_bytes(),
        }
    }

    /// Convert to native-endian u16.
    pub fn to_native(self) -> u16 {
        u16::from_be_bytes(self.bytes)
    }

    /// Create from raw big-endian bytes.
    pub fn from_be_bytes(bytes: [u8; 2]) -> Self {
        Self { bytes }
    }

    /// Get the raw big-endian bytes.
    pub fn to_be_bytes(self) -> [u8; 2] {
        self.bytes
    }
}

/// Big-endian unsigned 32-bit integer.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeU32 {
    bytes: [u8; 4],
}

impl BeU32 {
    /// Create from a native-endian u32.
    pub fn from_native(val: u32) -> Self {
        Self {
            bytes: val.to_be_bytes(),
        }
    }

    /// Convert to native-endian u32.
    pub fn to_native(self) -> u32 {
        u32::from_be_bytes(self.bytes)
    }

    /// Create from raw big-endian bytes.
    pub fn from_be_bytes(bytes: [u8; 4]) -> Self {
        Self { bytes }
    }

    /// Get the raw big-endian bytes.
    pub fn to_be_bytes(self) -> [u8; 4] {
        self.bytes
    }
}
