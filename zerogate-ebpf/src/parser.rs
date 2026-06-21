// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Verifier-safe packet parsing helpers for the XDP program.
//!
//! This is the ONLY module in `zerogate-ebpf` that may contain `unsafe`.
//! Every unsafe block follows the mandatory pattern:
//!
//! ```text
//! if data + offset + size_of::<T>() > data_end {
//!     return None;
//! }
//! let ptr = (data + offset) as *const T;
//! let value = unsafe { core::ptr::read_unaligned(ptr) };
//! ```
//!
//! # Safety Justification Categories
//!
//! 1. **Packet-buffer dereference**: bounds check immediately before the
//!    read guarantees the eBPF verifier can prove the access is in-range.
//!    `read_unaligned` is used because packet buffers have no alignment
//!    guarantees and all header structs are `#[repr(C, packed)]`.

use aya_ebpf::programs::XdpContext;
use zerogate_common::{
    abi::{PacketMeta, PolicyAction, SessionKey, SessionValue},
    headers::{ChunkHdr, EthHdr, Ipv4Hdr, TcpHdr, UdpHdr},
    constants,
};

/// Reads a value of type `T` from the packet buffer at the given offset.
///
/// Returns `None` if the read would exceed `data_end` (verifier-safe).
///
/// # Safety contract (internal)
///
/// The bounds check `data + offset + size_of::<T>() > data_end` is
/// performed before the dereference. The eBPF verifier traces this
/// guard as a range restriction, proving the subsequent `read_unaligned`
/// accesses only valid packet memory.
///
/// **Pointer provenance**: `data` comes from `XdpContext::data()`, which
/// is the kernel-provided packet buffer start pointer.
///
/// **Alignment**: `read_unaligned` — no alignment requirement.
///
/// **Lifetime**: the value is copied out; no reference is held.
///
/// **Aliasing**: read-only; no mutable aliases exist.
#[inline(always)]
fn read_at<T: Copy>(data: usize, data_end: usize, offset: usize) -> Option<T> {
    let size = core::mem::size_of::<T>();
    if data + offset + size > data_end {
        return None;
    }
    // SAFETY: bounds check above guarantees [data+offset .. data+offset+size)
    // is within the packet buffer [data .. data_end).
    // - Provenance: data is the XDP packet buffer pointer from the kernel.
    // - Alignment: read_unaligned handles any alignment.
    // - Lifetime: T is Copy, value is moved out, no dangling reference.
    // - Aliasing: read-only access, no mutable aliases.
    let ptr = (data + offset) as *const T;
    Some(unsafe { core::ptr::read_unaligned(ptr) })
}

/// Reads an Ethernet header from offset 0.
///
/// Returns `(EthHdr, ETH_HDR_LEN)` or `None` if packet is too short.
#[inline(always)]
pub fn read_eth(data: usize, data_end: usize) -> Option<(EthHdr, usize)> {
    let eth = read_at::<EthHdr>(data, data_end, 0)?;
    Some((eth, constants::ETH_HDR_LEN))
}

/// Reads an IPv4 header at the given offset.
///
/// Returns `(Ipv4Hdr, ip_header_length_bytes)` or `None` if:
/// - Packet is too short for the minimum IPv4 header.
/// - IHL < 5 (malformed).
/// - Packet is too short for the full IP header (with options).
#[inline(always)]
pub fn read_ipv4(data: usize, data_end: usize, offset: usize) -> Option<(Ipv4Hdr, usize)> {
    let ipv4 = read_at::<Ipv4Hdr>(data, data_end, offset)?;

    // Extract IHL and compute actual header length.
    // SAFETY: version_ihl is a u8, no alignment concern after read_unaligned copy.
    let ihl = Ipv4Hdr::ihl(ipv4.version_ihl) as usize;
    let ip_hdr_len = ihl * 4;

    // IHL < 5 is malformed (minimum header is 20 bytes).
    if ip_hdr_len < constants::IPV4_MIN_HDR_LEN {
        return None;
    }

    // Verify the full IP header (including options) fits in the packet.
    if data + offset + ip_hdr_len > data_end {
        return None;
    }

    Some((ipv4, ip_hdr_len))
}

/// Reads a UDP header at the given offset.
#[inline(always)]
pub fn read_udp(data: usize, data_end: usize, offset: usize) -> Option<UdpHdr> {
    read_at::<UdpHdr>(data, data_end, offset)
}

/// Reads a TCP header at the given offset.
#[inline(always)]
pub fn read_tcp(data: usize, data_end: usize, offset: usize) -> Option<TcpHdr> {
    read_at::<TcpHdr>(data, data_end, offset)
}

/// Reads a ZeroGate ChunkHdr at the given offset.
#[inline(always)]
pub fn read_chunk_hdr(data: usize, data_end: usize, offset: usize) -> Option<ChunkHdr> {
    read_at::<ChunkHdr>(data, data_end, offset)
}

// ---------------------------------------------------------------------------
// BPF map accessor wrappers
// ---------------------------------------------------------------------------
// These wrap the unsafe aya-ebpf BPF helper calls in safe functions.
// They are placed here because parser.rs is the designated unsafe boundary
// for the entire zerogate-ebpf crate.

/// Looks up a session key in the SESSIONS hash map.
///
/// Returns a reference to the value if found, or `None`.
#[inline(always)]
pub fn get_session(key: &SessionKey) -> Option<&SessionValue> {
    // SAFETY: bpf_map_lookup_elem returns either a kernel-validated
    // pointer into the map's pre-allocated value slot, or NULL.
    // The aya HashMap::get wrapper surfaces NULL as None.
    // - Provenance: pointer comes from kernel map memory.
    // - Alignment: SessionValue is #[repr(C)], 4-byte aligned, map values
    //   are properly aligned by the kernel.
    // - Lifetime: valid until map is modified (single-threaded XDP context).
    // - Aliasing: read-only; XDP program is single-threaded per packet.
    unsafe { crate::maps::SESSIONS.get(key) }
}

/// Looks up packet metadata in the POLICY hash map.
///
/// Returns a reference to the policy action if found, or `None`.
#[inline(always)]
pub fn get_policy(meta: &PacketMeta) -> Option<&PolicyAction> {
    // SAFETY: same justification as get_session above.
    // bpf_map_lookup_elem returns valid pointer or NULL.
    unsafe { crate::maps::POLICY.get(meta) }
}

/// Redirects a packet to an AF_XDP socket via XSK_MAP.
///
/// Returns the XDP action code on success, or an error.
#[inline(always)]
pub fn redirect_xsk(ctx: &XdpContext, xsk_idx: u32) -> Result<u32, u32> {
    // SAFETY: bpf_redirect_map is a kernel-provided helper with stable ABI.
    // The kernel validates xsk_idx against XSK_MAP.max_entries at runtime.
    // flags=0 means XDP_ABORTED on failure.
    // - Provenance: XSK_MAP is a kernel-managed map.
    // - Bounds: kernel checks index < max_entries.
    crate::maps::XSK_MAP.redirect(xsk_idx, 0)
}
