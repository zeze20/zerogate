//! `zerogate-common` — ABI-stable, `#[repr(C)]` types shared between the
//! eBPF data-plane (`zerogate-ebpf`) and the userspace agent (`zerogate-agent`).
//!
//! Every type here is `no_std`-compatible and **must** keep a fixed, C-layout
//! memory representation so the kernel and userspace always agree on offsets.

#![no_std]

// ---------------------------------------------------------------------------
// ChunkHdr — the ZeroGate on-wire encapsulation header
// ---------------------------------------------------------------------------

/// On-wire chunk header prepended to every ZeroGate payload.
///
/// Layout (network byte order, 24 bytes total):
///
/// ```text
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |     magic     |    version    |             flags             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                          session_id                          |
/// |                         (8 bytes)                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        sequence_num                          |
/// |                         (8 bytes)                            |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                        payload_len                           |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
///
/// # Invariants (Verus-ready)
///
/// * `magic == ZEROGATE_MAGIC`  — discriminator for fast-path rejection.
/// * `version` ∈ {`ZEROGATE_VERSION_1`} — forward-compatible versioning.
/// * `payload_len <= MAX_PAYLOAD_LEN` — upper bound prevents map-of-maps DoS.
/// * `size_of::<ChunkHdr>() == 24` — compile-time enforced below.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ChunkHdr {
    /// Protocol magic byte — must equal [`ZEROGATE_MAGIC`].
    pub magic: u8,
    /// Protocol version — must equal [`ZEROGATE_VERSION_1`].
    pub version: u8,
    /// Bit-flags (reserved, must be 0 for v1).
    pub flags: u16,
    /// Caller-assigned session identifier validated against the session map.
    pub session_id: u64,
    /// Monotonic per-session sequence number (replay detection in userspace).
    pub sequence_num: u64,
    /// Length of the payload following this header, in bytes.
    pub payload_len: u32,
}

/// Magic byte — `0x5A` (ASCII `Z` for ZeroGate).
/// Chosen for easy identification in hex dumps.
pub const ZEROGATE_MAGIC: u8 = 0x5A;

/// Current protocol version.
pub const ZEROGATE_VERSION_1: u8 = 1;

/// Hard ceiling on `payload_len` to bound per-packet processing.
/// 64 KiB minus header is generous for MTU ≤ 9000 paths.
pub const MAX_PAYLOAD_LEN: u32 = 65536 - (core::mem::size_of::<ChunkHdr>() as u32);

/// Total header size in bytes — compile-time assertion anchor.
pub const CHUNK_HDR_SIZE: usize = core::mem::size_of::<ChunkHdr>();

// Compile-time layout assertion.
const _: () = assert!(CHUNK_HDR_SIZE == 24, "ChunkHdr layout drift detected");

// ---------------------------------------------------------------------------
// BPF map key / value types
// ---------------------------------------------------------------------------

/// Key for the `SESSIONS` `BPF_MAP_TYPE_HASH`.
///
/// Using a dedicated struct (rather than a bare `u64`) leaves room to
/// extend the key with a namespace or tenant discriminator later without
/// breaking the map ABI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SessionKey {
    pub session_id: u64,
}

/// Value stored alongside each [`SessionKey`] in the sessions map.
///
/// `xsk_index` indicates which AF_XDP socket queue should receive traffic
/// for this session.  The agent populates this when a session is admitted.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SessionValue {
    /// Index into the `XSKMAP` that this session's traffic is redirected to.
    pub xsk_index: u32,
    /// Reserved padding to keep the struct 8-byte aligned for future fields.
    pub _pad: u32,
}

// Compile-time layout assertions for map types.
const _: () = assert!(
    core::mem::size_of::<SessionKey>() == 8,
    "SessionKey must be 8 bytes"
);
const _: () = assert!(
    core::mem::size_of::<SessionValue>() == 8,
    "SessionValue must be 8 bytes"
);

// ---------------------------------------------------------------------------
// Map configuration constants
// ---------------------------------------------------------------------------

/// Maximum number of concurrent sessions the BPF hash map will hold.
pub const MAX_SESSIONS: u32 = 65536;

/// Maximum number of AF_XDP sockets (RX queues) we support.
pub const MAX_XSK_SOCKETS: u32 = 64;

// ---------------------------------------------------------------------------
// Helpers (available in both kernel and userspace)
// ---------------------------------------------------------------------------

impl ChunkHdr {
    /// Validates the static invariants of a parsed header.
    ///
    /// # Verus-style specification (pseudo)
    ///
    /// ```text
    /// ensures |result: bool|
    ///     result ==> (
    ///         self.magic   == ZEROGATE_MAGIC &&
    ///         self.version == ZEROGATE_VERSION_1 &&
    ///         self.payload_len <= MAX_PAYLOAD_LEN
    ///     )
    /// ```
    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.magic == ZEROGATE_MAGIC
            && self.version == ZEROGATE_VERSION_1
            && self.payload_len <= MAX_PAYLOAD_LEN
    }
}

impl SessionKey {
    #[inline(always)]
    pub fn new(session_id: u64) -> Self {
        Self { session_id }
    }
}

impl SessionValue {
    #[inline(always)]
    pub fn new(xsk_index: u32) -> Self {
        Self {
            xsk_index,
            _pad: 0,
        }
    }
}
