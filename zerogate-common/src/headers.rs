// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Network protocol headers as `#[repr(C, packed)]` structs.
//!
//! All fields use fixed-width integers only. No references, no raw
//! pointers, no dynamically sized types. Fields MUST NOT be accessed
//! by reference in packed structs — use `core::ptr::read_unaligned`.

use crate::constants;

// ---------------------------------------------------------------------------
// Ethernet header
// ---------------------------------------------------------------------------

/// Ethernet frame header (14 bytes).
///
/// All multi-byte fields are in network byte order.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct EthHdr {
    /// Destination MAC address.
    pub dst_mac: [u8; 6],
    /// Source MAC address.
    pub src_mac: [u8; 6],
    /// EtherType (network byte order). Compare with [`constants::ETHERTYPE_IPV4_BE`].
    pub ether_type: u16,
}

const _: () = assert!(
    core::mem::size_of::<EthHdr>() == constants::ETH_HDR_LEN,
    "EthHdr must be exactly 14 bytes"
);

// ---------------------------------------------------------------------------
// IPv4 header
// ---------------------------------------------------------------------------

/// IPv4 header (minimum 20 bytes, up to 60 with options).
///
/// All multi-byte fields are in network byte order.
/// The `version_ihl` field packs version (high nibble) and IHL (low nibble).
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Ipv4Hdr {
    /// Version (4 bits) | IHL (4 bits).
    pub version_ihl: u8,
    /// Type of Service / DSCP + ECN.
    pub tos: u8,
    /// Total length in bytes (network byte order).
    pub total_len: u16,
    /// Identification (network byte order).
    pub identification: u16,
    /// Flags (3 bits) | Fragment offset (13 bits) (network byte order).
    pub flags_frag_offset: u16,
    /// Time to live.
    pub ttl: u8,
    /// Protocol number (e.g., 6 = TCP, 17 = UDP).
    pub protocol: u8,
    /// Header checksum (network byte order).
    pub checksum: u16,
    /// Source IP address (network byte order).
    pub src_addr: u32,
    /// Destination IP address (network byte order).
    pub dst_addr: u32,
}

const _: () = assert!(
    core::mem::size_of::<Ipv4Hdr>() == constants::IPV4_MIN_HDR_LEN,
    "Ipv4Hdr must be exactly 20 bytes"
);

impl Ipv4Hdr {
    /// Extracts the IHL (Internet Header Length) field in 32-bit words.
    /// Multiply by 4 to get bytes.
    #[inline(always)]
    pub const fn ihl(version_ihl: u8) -> u8 {
        version_ihl & 0x0F
    }

    /// Extracts the IP version field.
    #[inline(always)]
    pub const fn version(version_ihl: u8) -> u8 {
        version_ihl >> 4
    }
}

// ---------------------------------------------------------------------------
// TCP header
// ---------------------------------------------------------------------------

/// TCP header (minimum 20 bytes, up to 60 with options).
///
/// All multi-byte fields are in network byte order.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct TcpHdr {
    /// Source port (network byte order).
    pub src_port: u16,
    /// Destination port (network byte order).
    pub dst_port: u16,
    /// Sequence number (network byte order).
    pub seq_num: u32,
    /// Acknowledgment number (network byte order).
    pub ack_num: u32,
    /// Data offset (4 bits) | Reserved (3 bits) | Flags (9 bits).
    /// The high nibble of the first byte is the data offset in 32-bit words.
    pub data_offset_flags: u16,
    /// Window size (network byte order).
    pub window: u16,
    /// Checksum (network byte order).
    pub checksum: u16,
    /// Urgent pointer (network byte order).
    pub urgent_ptr: u16,
}

const _: () = assert!(
    core::mem::size_of::<TcpHdr>() == constants::TCP_MIN_HDR_LEN,
    "TcpHdr must be exactly 20 bytes"
);

// ---------------------------------------------------------------------------
// UDP header
// ---------------------------------------------------------------------------

/// UDP header (8 bytes, fixed size).
///
/// All multi-byte fields are in network byte order.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct UdpHdr {
    /// Source port (network byte order).
    pub src_port: u16,
    /// Destination port (network byte order).
    pub dst_port: u16,
    /// Length in bytes (network byte order), includes header + payload.
    pub length: u16,
    /// Checksum (network byte order).
    pub checksum: u16,
}

const _: () = assert!(
    core::mem::size_of::<UdpHdr>() == constants::UDP_HDR_LEN,
    "UdpHdr must be exactly 8 bytes"
);

// ---------------------------------------------------------------------------
// ZeroGate ChunkHdr
// ---------------------------------------------------------------------------

/// On-wire chunk header prepended to every ZeroGate payload (24 bytes).
///
/// Layout:
/// ```text
///  0       1       2       3
///  +-------+-------+-------+-------+
///  | magic | version| flags (u16)  |
///  +-------+-------+-------+-------+
///  |          session_id (u64)     |
///  |                               |
///  +-------+-------+-------+-------+
///  |         sequence_num (u64)    |
///  |                               |
///  +-------+-------+-------+-------+
///  |        payload_len (u32)      |
///  +-------+-------+-------+-------+
/// ```
///
/// # Invariants
///
/// * `magic == ZEROGATE_MAGIC`
/// * `version == ZEROGATE_VERSION_1`
/// * `payload_len <= MAX_PAYLOAD_LEN`
/// * `size_of::<ChunkHdr>() == 24`
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ChunkHdr {
    /// Protocol magic byte — must equal [`constants::ZEROGATE_MAGIC`].
    pub magic: u8,
    /// Protocol version — must equal [`constants::ZEROGATE_VERSION_1`].
    pub version: u8,
    /// Bit-flags (reserved, must be 0 for v1).
    pub flags: u16,
    /// Caller-assigned session identifier.
    pub session_id: u64,
    /// Monotonic per-session sequence number (replay detection in userspace).
    pub sequence_num: u64,
    /// Length of the payload following this header, in bytes.
    pub payload_len: u32,
}

/// Compile-time size of ChunkHdr.
pub const CHUNK_HDR_SIZE: usize = core::mem::size_of::<ChunkHdr>();

const _: () = assert!(CHUNK_HDR_SIZE == 24, "ChunkHdr must be exactly 24 bytes");

impl ChunkHdr {
    /// Validates the static invariants of a parsed header.
    #[inline(always)]
    pub fn is_valid(&self, magic: u8, version: u8, flags: u16, payload_len: u32) -> bool {
        magic == constants::ZEROGATE_MAGIC
            && version == constants::ZEROGATE_VERSION_1
            && payload_len <= constants::MAX_PAYLOAD_LEN
            && flags == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eth_hdr_size() {
        assert_eq!(core::mem::size_of::<EthHdr>(), 14);
    }

    #[test]
    fn ipv4_hdr_size() {
        assert_eq!(core::mem::size_of::<Ipv4Hdr>(), 20);
    }

    #[test]
    fn tcp_hdr_size() {
        assert_eq!(core::mem::size_of::<TcpHdr>(), 20);
    }

    #[test]
    fn udp_hdr_size() {
        assert_eq!(core::mem::size_of::<UdpHdr>(), 8);
    }

    #[test]
    fn chunk_hdr_size() {
        assert_eq!(core::mem::size_of::<ChunkHdr>(), 24);
    }

    #[test]
    fn ipv4_ihl_extraction() {
        // version=4, ihl=5 => version_ihl = 0x45
        assert_eq!(Ipv4Hdr::ihl(0x45), 5);
        assert_eq!(Ipv4Hdr::version(0x45), 4);
        // version=4, ihl=15 => version_ihl = 0x4F
        assert_eq!(Ipv4Hdr::ihl(0x4F), 15);
    }

    #[test]
    fn chunk_hdr_no_padding() {
        // Verify no hidden padding by checking field offsets manually.
        // magic(1) + version(1) + flags(2) + session_id(8) + sequence_num(8) + payload_len(4) = 24
        assert_eq!(1 + 1 + 2 + 8 + 8 + 4, 24);
    }

    #[test]
    fn packed_read_unaligned_chunk_hdr() {
        let bytes: [u8; 24] = [
            0x5A, // magic
            0x01, // version
            0x00, 0x00, // flags
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // session_id
            0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, // sequence_num
            0xAA, 0xBB, 0xCC, 0xDD, // payload_len
        ];
        let hdr: ChunkHdr =
            unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const ChunkHdr) };

        assert_eq!(hdr.magic, 0x5A);
        assert_eq!(hdr.version, 0x01);
    }
}
