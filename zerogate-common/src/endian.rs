// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Big-endian newtypes for ABI-safe network byte order fields.
//!
//! These wrappers store values in network byte order (big-endian) and
//! provide explicit conversion to/from host byte order. They are
//! `#[repr(transparent)]` so they occupy the same space and alignment
//! as their inner integer, and are safe to embed in `#[repr(C, packed)]`
//! structs.
//!
//! No std dependency. No allocation.

/// A big-endian `u16` value.
///
/// Stored in network byte order. Use [`BeU16::from_host`] and
/// [`BeU16::to_host`] for conversion.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BeU16(u16);

impl BeU16 {
    /// Creates a `BeU16` from a host-byte-order value.
    #[inline(always)]
    pub const fn from_host(val: u16) -> Self {
        Self(val.to_be())
    }

    /// Creates a `BeU16` from a raw big-endian (network order) value.
    #[inline(always)]
    pub const fn from_network(raw: u16) -> Self {
        Self(raw)
    }

    /// Returns the value in host byte order.
    #[inline(always)]
    pub const fn to_host(self) -> u16 {
        u16::from_be(self.0)
    }

    /// Returns the raw big-endian representation.
    #[inline(always)]
    pub const fn to_network(self) -> u16 {
        self.0
    }
}

/// A big-endian `u32` value.
///
/// Stored in network byte order. Use [`BeU32::from_host`] and
/// [`BeU32::to_host`] for conversion.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BeU32(u32);

impl BeU32 {
    /// Creates a `BeU32` from a host-byte-order value.
    #[inline(always)]
    pub const fn from_host(val: u32) -> Self {
        Self(val.to_be())
    }

    /// Creates a `BeU32` from a raw big-endian (network order) value.
    #[inline(always)]
    pub const fn from_network(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the value in host byte order.
    #[inline(always)]
    pub const fn to_host(self) -> u32 {
        u32::from_be(self.0)
    }

    /// Returns the raw big-endian representation.
    #[inline(always)]
    pub const fn to_network(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn be_u16_roundtrip() {
        let val: u16 = 0x0800;
        let be = BeU16::from_host(val);
        assert_eq!(be.to_host(), val);
        // On little-endian host, network repr should be byte-swapped.
        assert_eq!(be.to_network(), val.to_be());
    }

    #[test]
    fn be_u16_from_network() {
        let raw_be: u16 = 0x0800_u16.to_be();
        let be = BeU16::from_network(raw_be);
        assert_eq!(be.to_host(), 0x0800);
    }

    #[test]
    fn be_u32_roundtrip() {
        let val: u32 = 0xC0A80001; // 192.168.0.1
        let be = BeU32::from_host(val);
        assert_eq!(be.to_host(), val);
        assert_eq!(be.to_network(), val.to_be());
    }

    #[test]
    fn be_u32_from_network() {
        let raw_be: u32 = 0xC0A80001_u32.to_be();
        let be = BeU32::from_network(raw_be);
        assert_eq!(be.to_host(), 0xC0A80001);
    }

    #[test]
    fn be_u16_size() {
        assert_eq!(core::mem::size_of::<BeU16>(), 2);
        assert_eq!(core::mem::align_of::<BeU16>(), 2);
    }

    #[test]
    fn be_u32_size() {
        assert_eq!(core::mem::size_of::<BeU32>(), 4);
        assert_eq!(core::mem::align_of::<BeU32>(), 4);
    }
}
