// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! ABI-stable types shared between kernel eBPF, userspace agent, and
//! the formal verification model.
//!
//! Rules:
//! - All types are `#[repr(C, packed)]` or `#[repr(C)]` with explicit padding.
//! - Only fixed-width integers (u8, u16, u32, u64).
//! - No raw pointers, references, usize, Vec, String, Box, trait objects.
//! - Fields must not be accessed by reference in packed structs.

// ---------------------------------------------------------------------------
// Frame identity
// ---------------------------------------------------------------------------

/// A UMEM frame identified by its index within the UMEM region.
///
/// Frame address = index * frame_size.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FrameIndex {
    pub index: u32,
    pub _pad: u32,
}

const _: () = assert!(
    core::mem::size_of::<FrameIndex>() == 8,
    "FrameIndex must be 8 bytes"
);

impl FrameIndex {
    #[inline(always)]
    pub const fn new(index: u32) -> Self {
        Self { index, _pad: 0 }
    }

    /// Computes the UMEM byte offset for this frame given a frame size.
    #[inline(always)]
    pub const fn to_umem_offset(self, frame_size: u32) -> u64 {
        self.index as u64 * frame_size as u64
    }
}

/// A UMEM address (byte offset into the UMEM region).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UmemAddr {
    pub offset: u64,
}

const _: () = assert!(
    core::mem::size_of::<UmemAddr>() == 8,
    "UmemAddr must be 8 bytes"
);

impl UmemAddr {
    #[inline(always)]
    pub const fn new(offset: u64) -> Self {
        Self { offset }
    }

    /// Converts a UMEM address back to a frame index given a frame size.
    #[inline(always)]
    pub const fn to_frame_index(self, frame_size: u32) -> FrameIndex {
        FrameIndex::new((self.offset / frame_size as u64) as u32)
    }
}

// ---------------------------------------------------------------------------
// Queue identity
// ---------------------------------------------------------------------------

/// Identifies a NIC RX/TX queue.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct QueueId {
    pub id: u32,
    pub _pad: u32,
}

const _: () = assert!(
    core::mem::size_of::<QueueId>() == 8,
    "QueueId must be 8 bytes"
);

impl QueueId {
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self { id, _pad: 0 }
    }
}

// ---------------------------------------------------------------------------
// Policy action
// ---------------------------------------------------------------------------

/// Action the XDP program should take for a matched packet.
///
/// Represented as `u8` constants rather than a Rust enum to avoid
/// crossing ABI boundaries with non-integer-repr enums.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PolicyAction {
    pub action: u8,
    pub _pad0: u8,
    pub _pad1: u16,
    pub _pad2: u32,
}

const _: () = assert!(
    core::mem::size_of::<PolicyAction>() == 8,
    "PolicyAction must be 8 bytes"
);

/// Pass the packet to the kernel network stack.
pub const POLICY_PASS: u8 = 0;
/// Drop the packet.
pub const POLICY_DROP: u8 = 1;
/// Redirect to AF_XDP socket.
pub const POLICY_REDIRECT: u8 = 2;

impl PolicyAction {
    #[inline(always)]
    pub const fn pass() -> Self {
        Self {
            action: POLICY_PASS,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }

    #[inline(always)]
    pub const fn drop_pkt() -> Self {
        Self {
            action: POLICY_DROP,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }

    #[inline(always)]
    pub const fn redirect() -> Self {
        Self {
            action: POLICY_REDIRECT,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Packet decision
// ---------------------------------------------------------------------------

/// The result of policy evaluation for a single packet.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PacketDecision {
    /// Action to take (POLICY_PASS / POLICY_DROP / POLICY_REDIRECT).
    pub action: u8,
    pub _pad0: u8,
    /// Target queue for redirect (only meaningful when action == POLICY_REDIRECT).
    pub queue_id: u16,
    pub _pad1: u32,
}

const _: () = assert!(
    core::mem::size_of::<PacketDecision>() == 8,
    "PacketDecision must be 8 bytes"
);

impl PacketDecision {
    #[inline(always)]
    pub const fn pass() -> Self {
        Self {
            action: POLICY_PASS,
            _pad0: 0,
            queue_id: 0,
            _pad1: 0,
        }
    }

    #[inline(always)]
    pub const fn drop_pkt() -> Self {
        Self {
            action: POLICY_DROP,
            _pad0: 0,
            queue_id: 0,
            _pad1: 0,
        }
    }

    #[inline(always)]
    pub const fn redirect(queue_id: u16) -> Self {
        Self {
            action: POLICY_REDIRECT,
            _pad0: 0,
            queue_id,
            _pad1: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Packet metadata
// ---------------------------------------------------------------------------

/// Compact packet metadata extracted by the XDP parser.
///
/// Used as BPF map lookup key or passed to the policy engine.
#[repr(C, packed)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PacketMeta {
    /// Source IPv4 address (network byte order).
    pub src_addr: u32,
    /// Destination IPv4 address (network byte order).
    pub dst_addr: u32,
    /// Source port (network byte order).
    pub src_port: u16,
    /// Destination port (network byte order).
    pub dst_port: u16,
    /// IP protocol number (6 = TCP, 17 = UDP).
    pub protocol: u8,
    /// Explicit padding to reach 16 bytes.
    pub _pad0: u8,
    pub _pad1: u16,
}

const _: () = assert!(
    core::mem::size_of::<PacketMeta>() == 16,
    "PacketMeta must be 16 bytes"
);

impl PacketMeta {
    /// Creates a zeroed PacketMeta.
    #[inline(always)]
    pub const fn zeroed() -> Self {
        Self {
            src_addr: 0,
            dst_addr: 0,
            src_port: 0,
            dst_port: 0,
            protocol: 0,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// BPF map key/value types
// ---------------------------------------------------------------------------

/// Key for the SESSIONS BPF HashMap.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SessionKey {
    pub session_id: u64,
}

const _: () = assert!(
    core::mem::size_of::<SessionKey>() == 8,
    "SessionKey must be 8 bytes"
);

impl SessionKey {
    #[inline(always)]
    pub const fn new(session_id: u64) -> Self {
        Self { session_id }
    }
}

/// Value for the SESSIONS BPF HashMap.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SessionValue {
    /// Index into XSK_MAP for AF_XDP redirect.
    pub xsk_index: u32,
    /// Explicit padding for 8-byte alignment.
    pub _pad: u32,
}

const _: () = assert!(
    core::mem::size_of::<SessionValue>() == 8,
    "SessionValue must be 8 bytes"
);

impl SessionValue {
    #[inline(always)]
    pub const fn new(xsk_index: u32) -> Self {
        Self { xsk_index, _pad: 0 }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_index_size_and_alignment() {
        assert_eq!(core::mem::size_of::<FrameIndex>(), 8);
        assert_eq!(core::mem::align_of::<FrameIndex>(), 4);
    }

    #[test]
    fn frame_index_to_offset() {
        let fi = FrameIndex::new(10);
        assert_eq!(fi.to_umem_offset(4096), 10 * 4096);
    }

    #[test]
    fn umem_addr_roundtrip() {
        let addr = UmemAddr::new(8192);
        let fi = addr.to_frame_index(4096);
        assert_eq!(fi.index, 2);
    }

    #[test]
    fn queue_id_size() {
        assert_eq!(core::mem::size_of::<QueueId>(), 8);
    }

    #[test]
    fn policy_action_size() {
        assert_eq!(core::mem::size_of::<PolicyAction>(), 8);
    }

    #[test]
    fn packet_decision_size() {
        assert_eq!(core::mem::size_of::<PacketDecision>(), 8);
    }

    #[test]
    fn packet_meta_size() {
        assert_eq!(core::mem::size_of::<PacketMeta>(), 16);
    }

    #[test]
    fn session_key_size() {
        assert_eq!(core::mem::size_of::<SessionKey>(), 8);
    }

    #[test]
    fn session_value_size() {
        assert_eq!(core::mem::size_of::<SessionValue>(), 8);
    }

    #[test]
    fn packet_decision_variants() {
        let pass = PacketDecision::pass();
        assert_eq!(pass.action, POLICY_PASS);

        let drop = PacketDecision::drop_pkt();
        assert_eq!(drop.action, POLICY_DROP);

        let redir = PacketDecision::redirect(3);
        assert_eq!(redir.action, POLICY_REDIRECT);
        assert_eq!(redir.queue_id, 3);
    }

    #[test]
    fn no_raw_pointers_in_abi() {
        // Compile-time guarantees: all fields are fixed-width integers.
        // This test just documents the invariant — the type definitions
        // themselves enforce it since only u8/u16/u32/u64 are used.
        assert_eq!(core::mem::size_of::<PacketMeta>(), 16);
        assert_eq!(core::mem::size_of::<PacketDecision>(), 8);
        assert_eq!(core::mem::size_of::<SessionKey>(), 8);
        assert_eq!(core::mem::size_of::<SessionValue>(), 8);
    }
}
