// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! BPF map declarations for the ZeroGate XDP program.
//!
//! No `unsafe` in this file. Map declarations use aya-ebpf macros.
//! Safe accessor wrappers that encapsulate the unsafe BPF helper calls
//! live in `parser.rs` (the designated unsafe boundary).

use aya_ebpf::{
    macros::map,
    maps::{HashMap, XskMap},
};
use zerogate_common::{
    abi::{PacketMeta, PolicyAction, SessionKey, SessionValue},
    constants,
};

/// Session allow-list. Populated by `zerogate-agent` in userspace.
#[map]
pub static SESSIONS: HashMap<SessionKey, SessionValue> =
    HashMap::with_max_entries(constants::MAX_SESSIONS, 0);

/// Policy map. Maps packet metadata to policy actions.
#[map]
pub static POLICY: HashMap<PacketMeta, PolicyAction> =
    HashMap::with_max_entries(constants::MAX_SESSIONS, 0);

/// AF_XDP socket array. Userspace registers XSK file descriptors here.
#[map]
pub static XSK_MAP: XskMap = XskMap::with_max_entries(constants::MAX_XSK_SOCKETS, 0);
