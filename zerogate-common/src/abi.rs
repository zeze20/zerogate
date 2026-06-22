//! ABI-stable types shared across eBPF, userspace agent, BPF maps, and Verus models.
//!
//! All types use `#[repr(C, packed)]` with fixed-width integers only.
//! No platform-dependent sizes, no implicit padding, no raw pointers.

/// Compact packet metadata produced by the eBPF parser and consumed by policy logic.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PacketMeta {
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub _reserved: [u8; 3],
}

/// Index into the UMEM frame table.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameIndex {
    pub index: u32,
}

/// Byte offset into UMEM memory region.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UmemAddr {
    pub addr: u64,
}

/// Identifies a NIC queue for AF_XDP binding.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueueId {
    pub id: u32,
}

/// Policy action communicated via BPF maps.
/// Stored as a single byte for ABI compactness.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PolicyAction {
    pub action: u8,
}

impl PolicyAction {
    pub const PASS: u8 = 0;
    pub const DROP: u8 = 1;
    pub const REDIRECT: u8 = 2;
}

/// Session key for the SESSIONS BPF map.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SessionKey {
    pub session_id: u64,
}

/// Session value for the SESSIONS BPF map.
/// Maps a session to its assigned XSK queue index.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SessionValue {
    pub xsk_index: u32,
}
