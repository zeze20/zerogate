// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! `zerogate-common` — ABI-stable, `#[repr(C)]` types shared between the
//! eBPF data-plane (`zerogate-ebpf`), userspace agent (`zerogate-agent`),
//! and formal verification model (`zerogate-verus`).
//!
//! Every type is `no_std`-compatible and keeps a fixed, C-layout memory
//! representation so kernel and userspace always agree on offsets.

#![no_std]

pub mod abi;
pub mod constants;
pub mod endian;
pub mod headers;

// Re-export commonly used types at crate root for convenience.
pub use abi::{
    FrameIndex, PacketDecision, PacketMeta, PolicyAction, QueueId, SessionKey, SessionValue,
    UmemAddr, POLICY_DROP, POLICY_PASS, POLICY_REDIRECT,
};
pub use constants::*;
pub use headers::{ChunkHdr, EthHdr, Ipv4Hdr, TcpHdr, UdpHdr, CHUNK_HDR_SIZE};
