// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Policy evaluation for received packets.
//!
//! No `unsafe` in this file. Pure decision logic separated from I/O.

use zerogate_common::{
    constants,
    CHUNK_HDR_SIZE,
};

/// Result of evaluating a packet against the local policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketAction {
    /// Print session info and recycle frame.
    Accept { session_id: u64, sequence_num: u64, payload_len: u32 },
    /// Drop — invalid header or policy rejection.
    Drop,
    /// Pass through — not a ZeroGate packet.
    PassThrough,
}

/// Evaluates a received AF_XDP packet.
///
/// The packet slice starts at the Ethernet header.
/// This function is pure — no I/O, no unsafe, deterministic for same input.
pub fn evaluate_packet(pkt: &[u8]) -> PacketAction {
    // Minimum: Eth(14) + IPv4(20) + UDP(8) + ChunkHdr(24) = 66 bytes.
    const MIN_FRAME: usize = constants::ETH_HDR_LEN
        + constants::IPV4_MIN_HDR_LEN
        + constants::UDP_HDR_LEN
        + CHUNK_HDR_SIZE;

    if pkt.len() < MIN_FRAME {
        return PacketAction::PassThrough;
    }

    // Extract IHL from IPv4 header.
    let version_ihl = pkt[constants::ETH_HDR_LEN];
    let ihl = (version_ihl & 0x0F) as usize;
    let ip_hdr_len = ihl * 4;

    if ip_hdr_len < constants::IPV4_MIN_HDR_LEN {
        return PacketAction::Drop;
    }

    let chunk_offset = constants::ETH_HDR_LEN + ip_hdr_len + constants::UDP_HDR_LEN;
    if pkt.len() < chunk_offset + CHUNK_HDR_SIZE {
        return PacketAction::Drop;
    }

    // Read ChunkHdr using read_unaligned (packed struct).
    // Note: this module is NOT in the unsafe-allowed list, but
    // read_unaligned on a validated slice is technically unsafe.
    // We extract fields byte-by-byte to avoid unsafe entirely.
    let hdr_bytes = &pkt[chunk_offset..chunk_offset + CHUNK_HDR_SIZE];

    let magic = hdr_bytes[0];
    let version = hdr_bytes[1];
    let flags = u16::from_le_bytes([hdr_bytes[2], hdr_bytes[3]]);
    let session_id = u64::from_le_bytes([
        hdr_bytes[4], hdr_bytes[5], hdr_bytes[6], hdr_bytes[7],
        hdr_bytes[8], hdr_bytes[9], hdr_bytes[10], hdr_bytes[11],
    ]);
    let sequence_num = u64::from_le_bytes([
        hdr_bytes[12], hdr_bytes[13], hdr_bytes[14], hdr_bytes[15],
        hdr_bytes[16], hdr_bytes[17], hdr_bytes[18], hdr_bytes[19],
    ]);
    let payload_len = u32::from_le_bytes([
        hdr_bytes[20], hdr_bytes[21], hdr_bytes[22], hdr_bytes[23],
    ]);

    if magic != constants::ZEROGATE_MAGIC
        || version != constants::ZEROGATE_VERSION_1
        || flags != 0
        || payload_len > constants::MAX_PAYLOAD_LEN
    {
        return PacketAction::Drop;
    }

    PacketAction::Accept {
        session_id,
        sequence_num,
        payload_len,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_packet() -> Vec<u8> {
        let mut pkt = vec![0u8; 66];
        // Eth header (14 bytes) — ethertype IPv4
        pkt[12] = 0x08;
        pkt[13] = 0x00;
        // IPv4 header — version 4, IHL 5
        pkt[14] = 0x45;
        // Protocol: UDP (17)
        pkt[14 + 9] = 17;
        // UDP dst port: 7443 in network byte order
        let port_be = 7443u16.to_be_bytes();
        pkt[14 + 20 + 2] = port_be[0];
        pkt[14 + 20 + 3] = port_be[1];
        // ChunkHdr at offset 42
        let chunk_offset = 42;
        pkt[chunk_offset] = constants::ZEROGATE_MAGIC;
        pkt[chunk_offset + 1] = constants::ZEROGATE_VERSION_1;
        // flags = 0 (already zero)
        // session_id = 0xDEADBEEF
        let sid = 0xDEADBEEFu64.to_le_bytes();
        pkt[chunk_offset + 4..chunk_offset + 12].copy_from_slice(&sid);
        // sequence_num = 1
        let seq = 1u64.to_le_bytes();
        pkt[chunk_offset + 12..chunk_offset + 20].copy_from_slice(&seq);
        // payload_len = 100
        let plen = 100u32.to_le_bytes();
        pkt[chunk_offset + 20..chunk_offset + 24].copy_from_slice(&plen);
        pkt
    }

    #[test]
    fn valid_packet_accepted() {
        let pkt = make_valid_packet();
        match evaluate_packet(&pkt) {
            PacketAction::Accept { session_id, sequence_num, payload_len } => {
                assert_eq!(session_id, 0xDEADBEEF);
                assert_eq!(sequence_num, 1);
                assert_eq!(payload_len, 100);
            }
            other => panic!("expected Accept, got {other:?}"),
        }
    }

    #[test]
    fn short_packet_passes_through() {
        let pkt = vec![0u8; 10];
        assert_eq!(evaluate_packet(&pkt), PacketAction::PassThrough);
    }

    #[test]
    fn bad_magic_dropped() {
        let mut pkt = make_valid_packet();
        pkt[42] = 0xFF; // bad magic
        assert_eq!(evaluate_packet(&pkt), PacketAction::Drop);
    }

    #[test]
    fn bad_ihl_dropped() {
        let mut pkt = make_valid_packet();
        pkt[14] = 0x43; // IHL = 3 (< 5, malformed)
        assert_eq!(evaluate_packet(&pkt), PacketAction::Drop);
    }

    #[test]
    fn excessive_payload_len_dropped() {
        let mut pkt = make_valid_packet();
        let chunk_offset = 42;
        let bad_len = (constants::MAX_PAYLOAD_LEN + 1).to_le_bytes();
        pkt[chunk_offset + 20..chunk_offset + 24].copy_from_slice(&bad_len);
        assert_eq!(evaluate_packet(&pkt), PacketAction::Drop);
    }
}
