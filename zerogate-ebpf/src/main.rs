// SPDX-License-Identifier: GPL-2.0-only OR MIT
//
// zerogate-ebpf — XDP data-plane for ZeroGate zero-trust enforcement.
//
// # Safety Overview
//
// Every `unsafe` block in this module falls into exactly one of three
// categories, each justified below:
//
// 1. **Packet-buffer dereference** — raw pointer reads from the XDP
//    packet range `[ctx.data(), ctx.data_end())`.  Each dereference is
//    guarded by an explicit bounds check of the form:
//
//        if data + offset + size_of::<T>() > data_end { return ...; }
//
//    This pattern is *mandatory* for the Linux eBPF verifier: it proves
//    at load time that every memory access is within the packet.
//
// 2. **BPF map lookup** — `HashMap::get` wraps `bpf_map_lookup_elem`,
//    which the kernel guarantees returns either a valid pointer into
//    the map's pre-allocated value slot, or NULL (surfaced as `None`).
//
// 3. **BPF helper invocation** — `XskMap::redirect` wraps
//    `bpf_redirect_map`, a kernel-provided helper with a stable ABI.
//    The index is validated by the kernel against `max_entries`.

#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, XskMap},
    programs::XdpContext,
};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{Ipv4Hdr, IpProto},
    udp::UdpHdr,
};
use zerogate_common::{
    ChunkHdr, SessionKey, SessionValue, CHUNK_HDR_SIZE, MAX_SESSIONS, MAX_XSK_SOCKETS,
};

// ---------------------------------------------------------------------------
// BPF Maps
// ---------------------------------------------------------------------------

/// Session allow-list.  Populated by `zerogate-agent` in userspace.
///
/// - Key:   `SessionKey`   (u64 session_id, 8 bytes)
/// - Value: `SessionValue` (u32 xsk_index + u32 pad, 8 bytes)
/// - Type:  `BPF_MAP_TYPE_HASH`
#[map]
static SESSIONS: HashMap<SessionKey, SessionValue> =
    HashMap::with_max_entries(MAX_SESSIONS, 0);

/// AF_XDP socket array.  Userspace registers XSK file descriptors here.
/// The XDP program redirects admitted packets into the corresponding socket
/// for zero-copy delivery.
///
/// - Type: `BPF_MAP_TYPE_XSKMAP`
#[map]
static XSK_MAP: XskMap = XskMap::with_max_entries(MAX_XSK_SOCKETS, 0);

// ---------------------------------------------------------------------------
// Protocol constants
// ---------------------------------------------------------------------------

/// ZeroGate UDP destination port (host byte order).
const ZEROGATE_PORT: u16 = 7443;

// ---------------------------------------------------------------------------
// XDP entry point
// ---------------------------------------------------------------------------

/// Top-level XDP hook.  Delegates to [`try_zerogate_xdp`] and maps any
/// internal `Err(())` to `XDP_ABORTED` so the kernel always receives a
/// valid action code.
#[xdp]
pub fn zerogate_xdp(ctx: XdpContext) -> u32 {
    match try_zerogate_xdp(&ctx) {
        Ok(action) => action,
        Err(()) => xdp_action::XDP_ABORTED,
    }
}

// ---------------------------------------------------------------------------
// Core packet pipeline
// ---------------------------------------------------------------------------

/// Parses Eth → IPv4 → UDP → ChunkHdr, validates the session, and either
/// drops or redirects the packet.
///
/// # Verifier Compliance
///
/// A monotonically increasing `offset` accumulator tracks the parse
/// position.  Before every pointer dereference at offset `o`, an explicit
/// guard ensures:
///
/// ```text
///     data + o + size_of::<T>() <= data_end
/// ```
///
/// The verifier sees each guard as a range restriction on the packet
/// pointer, proving all subsequent reads are in-bounds.
///
/// # Verus Invariant Sketch
///
/// ```text
/// requires
///     ctx.data() <= ctx.data_end(),
///     ctx.data_end() - ctx.data() <= u16::MAX as usize,
/// ensures |result: Result<u32, ()>|
///     result.is_ok() ==> match result.unwrap() {
///         XDP_PASS | XDP_DROP | XDP_REDIRECT => true,
///         _ => false,
///     }
/// ```
#[inline(always)]
fn try_zerogate_xdp(ctx: &XdpContext) -> Result<u32, ()> {
    let data: usize = ctx.data();
    let data_end: usize = ctx.data_end();

    // ---------------------------------------------------------------
    // Layer 2 — Ethernet
    // ---------------------------------------------------------------
    // Invariant: [data .. data + EthHdr::LEN) is within packet.
    if data + EthHdr::LEN > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    // SAFETY [category 1]: bounds check immediately above guarantees
    // EthHdr::LEN (14) bytes are accessible from `data`.
    // EthHdr is #[repr(C, packed)] with all byte-array fields except
    // ether_type at offset 12, which is 2-byte aligned here.
    let eth: &EthHdr = unsafe { &*(data as *const EthHdr) };

    if eth.ether_type != EtherType::Ipv4 {
        // Not IPv4 — pass through to the kernel network stack.
        return Ok(xdp_action::XDP_PASS);
    }

    // ---------------------------------------------------------------
    // Layer 3 — IPv4
    // ---------------------------------------------------------------
    let ip_offset: usize = EthHdr::LEN;

    // Invariant: [data + ip_offset .. data + ip_offset + Ipv4Hdr::LEN)
    // is within packet.
    if data + ip_offset + Ipv4Hdr::LEN > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    // SAFETY [category 1]: bounds check above covers the minimum IPv4
    // header (20 bytes).  Ipv4Hdr is #[repr(C)] with byte-array and
    // u8 fields, so alignment requirement is 1.
    let ipv4: &Ipv4Hdr = unsafe { &*((data + ip_offset) as *const Ipv4Hdr) };

    if ipv4.proto != IpProto::Udp {
        return Ok(xdp_action::XDP_PASS);
    }

    // Compute actual IP header length from the IHL field (includes options).
    let ihl: usize = ipv4.ihl() as usize;
    let ip_hdr_len: usize = ihl * 4;

    // IHL < 5 is malformed (header must be >= 20 bytes).
    if ip_hdr_len < Ipv4Hdr::LEN {
        return Ok(xdp_action::XDP_DROP);
    }

    // ---------------------------------------------------------------
    // Layer 4 — UDP
    // ---------------------------------------------------------------
    let udp_offset: usize = ip_offset + ip_hdr_len;

    // Invariant: [data + udp_offset .. data + udp_offset + UdpHdr::LEN)
    // is within packet.
    if data + udp_offset + UdpHdr::LEN > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    // SAFETY [category 1]: bounds check above covers UdpHdr::LEN (8)
    // bytes.  UdpHdr is #[repr(C)] with [u8; 2] fields — alignment 1.
    let udp: &UdpHdr = unsafe { &*((data + udp_offset) as *const UdpHdr) };

    // UdpHdr::dest() returns the port in host byte order.
    if udp.dest() != ZEROGATE_PORT {
        return Ok(xdp_action::XDP_PASS);
    }

    // ---------------------------------------------------------------
    // Layer 7 — ZeroGate ChunkHdr
    // ---------------------------------------------------------------
    let chunk_offset: usize = udp_offset + UdpHdr::LEN;

    // Invariant: [data + chunk_offset .. data + chunk_offset + CHUNK_HDR_SIZE)
    // is within packet.
    if data + chunk_offset + CHUNK_HDR_SIZE > data_end {
        // Truncated ZeroGate frame — drop, not pass, since the port
        // matched but the payload is malformed.
        return Ok(xdp_action::XDP_DROP);
    }

    // SAFETY [category 1]: bounds check above ensures 24 bytes are
    // accessible.  We use `read_unaligned` because ChunkHdr is
    // `#[repr(C, packed)]` and the packet buffer offers no alignment
    // guarantees beyond byte alignment.
    let chunk: ChunkHdr =
        unsafe { core::ptr::read_unaligned((data + chunk_offset) as *const ChunkHdr) };

    // ---------------------------------------------------------------
    // Header validation — provable invariants
    // ---------------------------------------------------------------
    //
    // Verus assertion:
    //   chunk.is_valid() <==>
    //       chunk.magic      == ZEROGATE_MAGIC      &&
    //       chunk.version    == ZEROGATE_VERSION_1   &&
    //       chunk.payload_len <= MAX_PAYLOAD_LEN
    if !chunk.is_valid() {
        return Ok(xdp_action::XDP_DROP);
    }

    // ---------------------------------------------------------------
    // Session lookup — zero-trust enforcement point
    // ---------------------------------------------------------------
    let key = SessionKey::new(chunk.session_id);

    // SAFETY [category 2]: `bpf_map_lookup_elem` returns either a
    // kernel-validated pointer into the map's value slot or NULL.
    // The aya `HashMap::get` wrapper surfaces NULL as `None`.
    let session_val = unsafe { SESSIONS.get(&key) };

    match session_val {
        None => {
            // Session not in allow-list — zero-trust policy: DROP.
            Ok(xdp_action::XDP_DROP)
        }
        Some(val) => {
            // -------------------------------------------------------
            // AF_XDP redirect — zero-copy fast path
            // -------------------------------------------------------
            //
            // `bpf_redirect_map` sets the redirect target.  The kernel
            // validates `xsk_idx` against `XSK_MAP.max_entries` at
            // runtime and falls back to the flags-encoded action on
            // failure (flags=0 → XDP_ABORTED).
            let xsk_idx: u32 = val.xsk_index;
            XSK_MAP.redirect(xsk_idx, 0).map_err(|_| ())
        }
    }
}

// ---------------------------------------------------------------------------
// Panic handler
// ---------------------------------------------------------------------------

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
