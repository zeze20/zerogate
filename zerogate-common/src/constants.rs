// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Protocol and system constants shared across all ZeroGate crates.

// ---------------------------------------------------------------------------
// Network header sizes
// ---------------------------------------------------------------------------

/// Ethernet header length in bytes (dst[6] + src[6] + ethertype[2]).
pub const ETH_HDR_LEN: usize = 14;

/// Minimum IPv4 header length in bytes (IHL = 5, no options).
pub const IPV4_MIN_HDR_LEN: usize = 20;

/// Minimum TCP header length in bytes (data offset = 5, no options).
pub const TCP_MIN_HDR_LEN: usize = 20;

/// UDP header length in bytes (fixed).
pub const UDP_HDR_LEN: usize = 8;

// ---------------------------------------------------------------------------
// EtherType constants (network byte order)
// ---------------------------------------------------------------------------

/// EtherType for IPv4 in network byte order (big-endian).
pub const ETHERTYPE_IPV4_BE: u16 = 0x0008; // 0x0800 byte-swapped

/// EtherType for IPv4 in host byte order.
pub const ETHERTYPE_IPV4: u16 = 0x0800;

// ---------------------------------------------------------------------------
// IP protocol numbers
// ---------------------------------------------------------------------------

/// IP protocol number for TCP.
pub const IPPROTO_TCP: u8 = 6;

/// IP protocol number for UDP.
pub const IPPROTO_UDP: u8 = 17;

// ---------------------------------------------------------------------------
// ZeroGate protocol constants
// ---------------------------------------------------------------------------

/// Magic byte for ZeroGate on-wire headers (`0x5A` = ASCII 'Z').
pub const ZEROGATE_MAGIC: u8 = 0x5A;

/// Current protocol version.
pub const ZEROGATE_VERSION_1: u8 = 1;

/// ZeroGate UDP destination port.
pub const ZEROGATE_PORT: u16 = 7443;

// ---------------------------------------------------------------------------
// AF_XDP / UMEM defaults
// ---------------------------------------------------------------------------

/// Default UMEM frame size in bytes (4 KiB, matching PAGE_SIZE).
pub const DEFAULT_UMEM_FRAME_SIZE: u32 = 4096;

/// Default number of UMEM frames.
pub const DEFAULT_UMEM_FRAME_COUNT: u32 = 4096;

/// Maximum number of NIC queues supported.
pub const MAX_QUEUES: u32 = 64;

// ---------------------------------------------------------------------------
// BPF map limits
// ---------------------------------------------------------------------------

/// Maximum concurrent sessions in the BPF hash map.
pub const MAX_SESSIONS: u32 = 65536;

/// Maximum AF_XDP sockets (one per queue).
pub const MAX_XSK_SOCKETS: u32 = 64;

/// Maximum payload after ChunkHdr (64 KiB minus header).
pub const MAX_PAYLOAD_LEN: u32 = 65536 - 24;

// ---------------------------------------------------------------------------
// Policy action values (integer constants for ABI safety)
// ---------------------------------------------------------------------------

/// Pass the packet to the kernel network stack.
pub const POLICY_PASS_VAL: u8 = 0;
/// Drop the packet.
pub const POLICY_DROP_VAL: u8 = 1;
/// Redirect to AF_XDP socket.
pub const POLICY_REDIRECT_VAL: u8 = 2;
