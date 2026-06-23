//! Verifier-safe packet parser for eBPF/XDP.
//!
//! All reads use explicit bounds checks before pointer formation.
//! Unsafe is confined to this file only.

use zerogate_common::abi::PacketMeta;
use zerogate_common::headers::{EthHdr, Ipv4Hdr, TcpHdr, UdpHdr};

/// Ethernet EtherType for IPv4.
const ETH_P_IPV4: u16 = 0x0800;

/// IP protocol number for TCP.
const IPPROTO_TCP: u8 = 6;

/// IP protocol number for UDP.
const IPPROTO_UDP: u8 = 17;

/// Ethernet header length in bytes.
const ETH_HDR_LEN: usize = 14;

/// Read a value of type T from a packet buffer at a given offset.
///
/// Returns `None` if the read would exceed `data_end`.
/// This pattern mirrors the exact bounds-check sequence required by the eBPF verifier.
///
/// # Safety contract (internal)
///
/// The unsafe block is guarded by the bounds check above it.
/// The pointer is formed only after proving `data + offset + size_of::<T>() <= data_end`.
/// `read_unaligned` is used because packed network headers are not aligned.
pub fn read_at<T: Copy>(data: usize, data_end: usize, offset: usize) -> Option<T> {
    let size = core::mem::size_of::<T>();

    if data + offset + size > data_end {
        return None;
    }

    let ptr = (data + offset) as *const T;

    // SAFETY: bounds checked above — data + offset + size_of::<T>() <= data_end.
    // Pointer provenance: ptr is within [data, data_end).
    // Alignment: using read_unaligned for packed network headers.
    // Lifetime: the packet buffer is valid for the duration of XDP program execution.
    // Aliasing: read-only access, no mutable aliases.
    Some(unsafe { core::ptr::read_unaligned(ptr) })
}

/// XDP action: pass packet to normal network stack.
pub const XDP_PASS: u32 = 2;

/// XDP action: drop packet.
pub const XDP_DROP: u32 = 1;

/// XDP action: redirect packet (e.g., to AF_XDP socket).
pub const XDP_REDIRECT: u32 = 4;

/// Result of packet parsing: either a valid PacketMeta or an XDP action to take.
pub enum ParseResult {
    /// Successfully parsed packet metadata.
    Meta(PacketMeta),
    /// Parsing determined an immediate XDP action (PASS for non-IPv4, etc.).
    Action(u32),
}

/// Parse a packet buffer and extract metadata.
///
/// Performs strict bounds checking at every layer before reading.
/// Returns `ParseResult::Action(XDP_PASS)` for non-IPv4 or malformed packets.
/// Returns `ParseResult::Meta(...)` for successfully parsed IPv4/TCP/UDP packets.
pub fn parse_packet(data: usize, data_end: usize) -> ParseResult {
    // Step 1: Read Ethernet header
    let eth: EthHdr = match read_at(data, data_end, 0) {
        Some(hdr) => hdr,
        None => return ParseResult::Action(XDP_PASS),
    };

    // Step 2: Check EtherType — only process IPv4
    // EthHdr.ether_type is stored in network byte order (big-endian)
    let ether_type = u16::from_be(eth.ether_type);
    if ether_type != ETH_P_IPV4 {
        return ParseResult::Action(XDP_PASS);
    }

    // Step 3: Read IPv4 header
    let ipv4: Ipv4Hdr = match read_at(data, data_end, ETH_HDR_LEN) {
        Some(hdr) => hdr,
        None => return ParseResult::Action(XDP_PASS),
    };

    // Step 4: Validate IPv4 IHL (minimum 5, meaning 20 bytes)
    let ihl = (ipv4.version_ihl & 0x0F) as usize;
    if ihl < 5 {
        return ParseResult::Action(XDP_PASS);
    }

    let ipv4_header_size = ihl * 4;

    // Verify the full IPv4 header (including options) fits in the packet
    if data + ETH_HDR_LEN + ipv4_header_size > data_end {
        return ParseResult::Action(XDP_PASS);
    }

    // Step 5: Parse transport layer
    let transport_offset = ETH_HDR_LEN + ipv4_header_size;
    let mut src_port: u16 = 0;
    let mut dst_port: u16 = 0;

    let protocol = ipv4.protocol;

    if protocol == IPPROTO_TCP {
        let tcp: TcpHdr = match read_at(data, data_end, transport_offset) {
            Some(hdr) => hdr,
            None => return ParseResult::Action(XDP_PASS),
        };
        src_port = u16::from_be(tcp.src_port);
        dst_port = u16::from_be(tcp.dst_port);
    } else if protocol == IPPROTO_UDP {
        let udp: UdpHdr = match read_at(data, data_end, transport_offset) {
            Some(hdr) => hdr,
            None => return ParseResult::Action(XDP_PASS),
        };
        src_port = u16::from_be(udp.src_port);
        dst_port = u16::from_be(udp.dst_port);
    }

    // Step 6: Build PacketMeta
    let meta = PacketMeta {
        src_ip: ipv4.src_addr,
        dst_ip: ipv4.dst_addr,
        src_port,
        dst_port,
        protocol,
        _reserved: [0u8; 3],
    };

    ParseResult::Meta(meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a packet buffer and return (data, data_end).
    fn buf_bounds(buf: &[u8]) -> (usize, usize) {
        let data = buf.as_ptr() as usize;
        let data_end = data + buf.len();
        (data, data_end)
    }

    /// Build a minimal valid Ethernet + IPv4 + TCP packet.
    fn build_tcp_packet() -> Vec<u8> {
        let mut pkt = vec![0u8; 14 + 20 + 20]; // ETH + IPv4 + TCP

        // Ethernet: dst_mac(6) + src_mac(6) + ethertype(2)
        pkt[12] = 0x08; // ETH_P_IPV4 = 0x0800
        pkt[13] = 0x00;

        // IPv4: version_ihl
        pkt[14] = 0x45; // version=4, ihl=5
        // protocol at offset 14+9 = 23
        pkt[23] = IPPROTO_TCP;
        // src_addr at offset 14+12..14+16
        pkt[26] = 10;
        pkt[27] = 0;
        pkt[28] = 0;
        pkt[29] = 1;
        // dst_addr at offset 14+16..14+20
        pkt[30] = 10;
        pkt[31] = 0;
        pkt[32] = 0;
        pkt[33] = 2;

        // TCP: src_port at offset 34..36
        pkt[34] = 0x1F; // 8080 = 0x1F90
        pkt[35] = 0x90;
        // TCP: dst_port at offset 36..38
        pkt[36] = 0x00; // 80 = 0x0050
        pkt[37] = 0x50;

        pkt
    }

    /// Build a minimal valid Ethernet + IPv4 + UDP packet.
    fn build_udp_packet() -> Vec<u8> {
        let mut pkt = vec![0u8; 14 + 20 + 8]; // ETH + IPv4 + UDP

        // Ethernet
        pkt[12] = 0x08;
        pkt[13] = 0x00;

        // IPv4
        pkt[14] = 0x45; // version=4, ihl=5
        pkt[23] = IPPROTO_UDP;
        pkt[26] = 192;
        pkt[27] = 168;
        pkt[28] = 1;
        pkt[29] = 100;
        pkt[30] = 192;
        pkt[31] = 168;
        pkt[32] = 1;
        pkt[33] = 1;

        // UDP: src_port
        pkt[34] = 0x13; // 5000 = 0x1388
        pkt[35] = 0x88;
        // UDP: dst_port
        pkt[36] = 0x00; // 53 = 0x0035
        pkt[37] = 0x35;

        pkt
    }

    #[test]
    fn short_ethernet_packet() {
        // Packet too short for Ethernet header
        let buf = [0u8; 10];
        let (data, data_end) = buf_bounds(&buf);
        match parse_packet(data, data_end) {
            ParseResult::Action(XDP_PASS) => {}
            _ => panic!("expected XDP_PASS for short packet"),
        }
    }

    #[test]
    fn non_ipv4_packet() {
        // Valid Ethernet header but not IPv4 (ARP = 0x0806)
        let mut buf = [0u8; 64];
        buf[12] = 0x08;
        buf[13] = 0x06; // ARP
        let (data, data_end) = buf_bounds(&buf);
        match parse_packet(data, data_end) {
            ParseResult::Action(XDP_PASS) => {}
            _ => panic!("expected XDP_PASS for non-IPv4"),
        }
    }

    #[test]
    fn malformed_ihl() {
        // IPv4 with IHL < 5 (invalid)
        let mut buf = [0u8; 64];
        buf[12] = 0x08;
        buf[13] = 0x00;
        buf[14] = 0x42; // version=4, ihl=2 (invalid, minimum is 5)
        let (data, data_end) = buf_bounds(&buf);
        match parse_packet(data, data_end) {
            ParseResult::Action(XDP_PASS) => {}
            _ => panic!("expected XDP_PASS for malformed IHL"),
        }
    }

    #[test]
    fn valid_tcp_packet() {
        let pkt = build_tcp_packet();
        let (data, data_end) = buf_bounds(&pkt);
        match parse_packet(data, data_end) {
            ParseResult::Meta(meta) => {
                // Copy fields from packed struct to locals before comparison
                let src_ip = { meta.src_ip };
                let dst_ip = { meta.dst_ip };
                let src_port = { meta.src_port };
                let dst_port = { meta.dst_port };
                let protocol = { meta.protocol };
                assert_eq!(src_ip.to_ne_bytes(), [10, 0, 0, 1]);
                assert_eq!(dst_ip.to_ne_bytes(), [10, 0, 0, 2]);
                assert_eq!(src_port, 8080);
                assert_eq!(dst_port, 80);
                assert_eq!(protocol, IPPROTO_TCP);
            }
            _ => panic!("expected Meta for valid TCP packet"),
        }
    }

    #[test]
    fn valid_udp_packet() {
        let pkt = build_udp_packet();
        let (data, data_end) = buf_bounds(&pkt);
        match parse_packet(data, data_end) {
            ParseResult::Meta(meta) => {
                let src_ip = { meta.src_ip };
                let dst_ip = { meta.dst_ip };
                let src_port = { meta.src_port };
                let dst_port = { meta.dst_port };
                let protocol = { meta.protocol };
                assert_eq!(src_ip.to_ne_bytes(), [192, 168, 1, 100]);
                assert_eq!(dst_ip.to_ne_bytes(), [192, 168, 1, 1]);
                assert_eq!(src_port, 5000);
                assert_eq!(dst_port, 53);
                assert_eq!(protocol, IPPROTO_UDP);
            }
            _ => panic!("expected Meta for valid UDP packet"),
        }
    }

    #[test]
    fn ipv4_header_truncated() {
        // Ethernet is valid but IPv4 header is truncated
        let mut buf = [0u8; 20]; // Only room for ETH(14) + 6 bytes of IPv4
        buf[12] = 0x08;
        buf[13] = 0x00;
        buf[14] = 0x45; // version=4, ihl=5 (needs 20 bytes)
        let (data, data_end) = buf_bounds(&buf);
        match parse_packet(data, data_end) {
            ParseResult::Action(XDP_PASS) => {}
            _ => panic!("expected XDP_PASS for truncated IPv4"),
        }
    }
}
