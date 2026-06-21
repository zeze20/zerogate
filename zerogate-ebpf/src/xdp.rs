// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! XDP program entry point and core packet pipeline.
//!
//! No `unsafe` in this file. All unsafe operations are isolated in
//! `parser.rs` — the designated unsafe boundary for this crate.

use aya_ebpf::{bindings::xdp_action, programs::XdpContext};
use zerogate_common::{
    abi::SessionKey,
    constants,
};

use crate::parser;

/// Core packet pipeline: Eth -> IPv4 -> UDP/TCP -> policy decision.
///
/// Returns an XDP action code. All packet reads and map lookups are
/// delegated to `parser.rs`.
#[inline(always)]
pub fn process_packet(ctx: &XdpContext) -> u32 {
    let data: usize = ctx.data();
    let data_end: usize = ctx.data_end();

    // Layer 2 — Ethernet
    let (eth, eth_len) = match parser::read_eth(data, data_end) {
        Some(v) => v,
        None => return xdp_action::XDP_PASS,
    };

    // Only process IPv4.
    if eth.ether_type != constants::ETHERTYPE_IPV4_BE {
        return xdp_action::XDP_PASS;
    }

    // Layer 3 — IPv4
    let ip_offset = eth_len;
    let (ipv4, ip_hdr_len) = match parser::read_ipv4(data, data_end, ip_offset) {
        Some(v) => v,
        None => return xdp_action::XDP_PASS,
    };

    let protocol = ipv4.protocol;

    // Layer 4 — extract ports
    let l4_offset = ip_offset + ip_hdr_len;
    let (src_port, dst_port) = match protocol {
        constants::IPPROTO_UDP => {
            let udp = match parser::read_udp(data, data_end, l4_offset) {
                Some(v) => v,
                None => return xdp_action::XDP_PASS,
            };
            (udp.src_port, udp.dst_port)
        }
        constants::IPPROTO_TCP => {
            let tcp = match parser::read_tcp(data, data_end, l4_offset) {
                Some(v) => v,
                None => return xdp_action::XDP_PASS,
            };
            (tcp.src_port, tcp.dst_port)
        }
        _ => return xdp_action::XDP_PASS,
    };

    // Build compact metadata for policy lookup.
    let meta = zerogate_common::PacketMeta {
        src_addr: ipv4.src_addr,
        dst_addr: ipv4.dst_addr,
        src_port,
        dst_port,
        protocol,
        _pad0: 0,
        _pad1: 0,
    };

    // Policy map lookup.
    if let Some(action) = parser::get_policy(&meta) {
        let act = action.action;
        return match act {
            constants::POLICY_DROP_VAL => xdp_action::XDP_DROP,
            constants::POLICY_REDIRECT_VAL => {
                match parser::redirect_xsk(ctx, 0) {
                    Ok(a) => a,
                    Err(_) => xdp_action::XDP_ABORTED,
                }
            }
            _ => xdp_action::XDP_PASS,
        };
    }

    // No explicit policy — check ZeroGate session protocol on UDP:7443.
    handle_zerogate_session(ctx, data, data_end, l4_offset, protocol, dst_port)
}

/// Handles ZeroGate session validation for UDP traffic on port 7443.
#[inline(always)]
fn handle_zerogate_session(
    ctx: &XdpContext,
    data: usize,
    data_end: usize,
    l4_offset: usize,
    protocol: u8,
    dst_port: u16,
) -> u32 {
    if protocol != constants::IPPROTO_UDP {
        return xdp_action::XDP_PASS;
    }

    // Check destination port (network byte order comparison).
    let zg_port_be = constants::ZEROGATE_PORT.to_be();
    if dst_port != zg_port_be {
        return xdp_action::XDP_PASS;
    }

    // Parse ZeroGate ChunkHdr.
    let chunk_offset = l4_offset + constants::UDP_HDR_LEN;
    let chunk = match parser::read_chunk_hdr(data, data_end, chunk_offset) {
        Some(v) => v,
        None => return xdp_action::XDP_DROP,
    };

    // Validate header invariants.
    if chunk.magic != constants::ZEROGATE_MAGIC
        || chunk.version != constants::ZEROGATE_VERSION_1
        || chunk.payload_len > constants::MAX_PAYLOAD_LEN
        || chunk.flags != 0
    {
        return xdp_action::XDP_DROP;
    }

    // Session lookup — zero-trust enforcement.
    let key = SessionKey::new(chunk.session_id);
    match parser::get_session(&key) {
        None => xdp_action::XDP_DROP,
        Some(val) => {
            match parser::redirect_xsk(ctx, val.xsk_index) {
                Ok(a) => a,
                Err(_) => xdp_action::XDP_ABORTED,
            }
        }
    }
}
