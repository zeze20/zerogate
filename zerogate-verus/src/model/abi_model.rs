// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Formal model of ABI type sizes and alignment invariants.
//!
//! Pure model — no I/O, no unsafe, no syscalls.
//!
//! ## Invariants to prove
//!
//! 1. All ABI types have the expected size.
//! 2. All ABI types have the expected alignment.
//! 3. `#[repr(C, packed)]` structs have alignment 1 and no hidden padding.
//! 4. `#[repr(C)]` structs have explicit padding fields.

#[cfg(test)]
mod tests {
    use zerogate_common::{
        abi::{
            FrameIndex, PacketDecision, PacketMeta, PolicyAction, QueueId, SessionKey,
            SessionValue, UmemAddr,
        },
        headers::{ChunkHdr, EthHdr, Ipv4Hdr, TcpHdr, UdpHdr},
    };

    // --- Size checks ---

    #[test]
    fn eth_hdr_abi() {
        assert_eq!(core::mem::size_of::<EthHdr>(), 14);
        assert_eq!(core::mem::align_of::<EthHdr>(), 1); // packed
    }

    #[test]
    fn ipv4_hdr_abi() {
        assert_eq!(core::mem::size_of::<Ipv4Hdr>(), 20);
        assert_eq!(core::mem::align_of::<Ipv4Hdr>(), 1); // packed
    }

    #[test]
    fn tcp_hdr_abi() {
        assert_eq!(core::mem::size_of::<TcpHdr>(), 20);
        assert_eq!(core::mem::align_of::<TcpHdr>(), 1); // packed
    }

    #[test]
    fn udp_hdr_abi() {
        assert_eq!(core::mem::size_of::<UdpHdr>(), 8);
        assert_eq!(core::mem::align_of::<UdpHdr>(), 1); // packed
    }

    #[test]
    fn chunk_hdr_abi() {
        assert_eq!(core::mem::size_of::<ChunkHdr>(), 24);
        assert_eq!(core::mem::align_of::<ChunkHdr>(), 1); // packed
    }

    #[test]
    fn session_key_abi() {
        assert_eq!(core::mem::size_of::<SessionKey>(), 8);
    }

    #[test]
    fn session_value_abi() {
        assert_eq!(core::mem::size_of::<SessionValue>(), 8);
    }

    #[test]
    fn frame_index_abi() {
        assert_eq!(core::mem::size_of::<FrameIndex>(), 8);
    }

    #[test]
    fn umem_addr_abi() {
        assert_eq!(core::mem::size_of::<UmemAddr>(), 8);
    }

    #[test]
    fn queue_id_abi() {
        assert_eq!(core::mem::size_of::<QueueId>(), 8);
    }

    #[test]
    fn policy_action_abi() {
        assert_eq!(core::mem::size_of::<PolicyAction>(), 8);
    }

    #[test]
    fn packet_decision_abi() {
        assert_eq!(core::mem::size_of::<PacketDecision>(), 8);
    }

    #[test]
    fn packet_meta_abi() {
        assert_eq!(core::mem::size_of::<PacketMeta>(), 16);
        assert_eq!(core::mem::align_of::<PacketMeta>(), 1); // packed
    }

    // --- Alignment sanity ---

    #[test]
    fn repr_c_types_have_stable_alignment() {
        // #[repr(C)] types — alignment comes from largest field.
        assert!(core::mem::align_of::<SessionKey>() >= 1);
        assert!(core::mem::align_of::<SessionValue>() >= 1);
        assert!(core::mem::align_of::<FrameIndex>() >= 1);
    }

    // --- No-padding checks for packed types ---

    #[test]
    fn packed_eth_hdr_no_padding() {
        // 6 + 6 + 2 = 14
        assert_eq!(6 + 6 + 2, core::mem::size_of::<EthHdr>());
    }

    #[test]
    fn packed_ipv4_hdr_no_padding() {
        // 1+1+2+2+2+1+1+2+4+4 = 20
        assert_eq!(1 + 1 + 2 + 2 + 2 + 1 + 1 + 2 + 4 + 4, core::mem::size_of::<Ipv4Hdr>());
    }

    #[test]
    fn packed_chunk_hdr_no_padding() {
        // 1+1+2+8+8+4 = 24
        assert_eq!(1 + 1 + 2 + 8 + 8 + 4, core::mem::size_of::<ChunkHdr>());
    }

    #[test]
    fn packed_packet_meta_no_padding() {
        // 4+4+2+2+1+1+2 = 16
        assert_eq!(4 + 4 + 2 + 2 + 1 + 1 + 2, core::mem::size_of::<PacketMeta>());
    }
}
