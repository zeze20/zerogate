//! XDP program integration layer.
//!
//! Ties together packet parsing and policy decision into a single XDP flow.
//! This module does not contain unsafe code.

use crate::parser::{ParseResult, XDP_DROP, XDP_PASS, XDP_REDIRECT, parse_packet};
use zerogate_common::abi::PacketMeta;

/// Policy decision stub.
///
/// In the real eBPF program this would perform a BPF map lookup.
/// Currently returns XDP_PASS for all packets (minimal stub).
fn lookup_policy(_meta: &PacketMeta) -> u32 {
    XDP_PASS
}

/// Main XDP processing entry point.
///
/// Flow:
/// 1. Parse packet to extract metadata.
/// 2. If parsing fails or packet is non-IPv4, pass to stack.
/// 3. Lookup policy for the extracted metadata.
/// 4. Return XDP action: DROP, REDIRECT, or PASS.
pub fn xdp_process(data: usize, data_end: usize) -> u32 {
    let meta = match parse_packet(data, data_end) {
        ParseResult::Meta(m) => m,
        ParseResult::Action(action) => return action,
    };

    let decision = lookup_policy(&meta);

    match decision {
        XDP_DROP => XDP_DROP,
        XDP_REDIRECT => XDP_REDIRECT,
        _ => XDP_PASS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdp_process_short_packet_passes() {
        let buf = [0u8; 5];
        let data = buf.as_ptr() as usize;
        let data_end = data + buf.len();
        assert_eq!(xdp_process(data, data_end), XDP_PASS);
    }

    #[test]
    fn xdp_process_valid_ipv4_passes() {
        // Minimal valid ETH + IPv4 + TCP
        let mut pkt = [0u8; 14 + 20 + 20];
        pkt[12] = 0x08;
        pkt[13] = 0x00;
        pkt[14] = 0x45;
        pkt[23] = 6; // TCP
        let data = pkt.as_ptr() as usize;
        let data_end = data + pkt.len();
        // Stub policy always returns PASS
        assert_eq!(xdp_process(data, data_end), XDP_PASS);
    }
}
