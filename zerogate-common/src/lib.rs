#![no_std]

pub mod abi;
pub mod endian;
pub mod headers;

#[cfg(test)]
mod tests {
    use super::abi::*;
    use super::endian::*;
    use super::headers::*;

    // --- Packet header layout tests ---

    #[test]
    fn eth_hdr_layout() {
        assert_eq!(core::mem::size_of::<EthHdr>(), 14);
        assert_eq!(core::mem::align_of::<EthHdr>(), 1);
    }

    #[test]
    fn ipv4_hdr_layout() {
        assert_eq!(core::mem::size_of::<Ipv4Hdr>(), 20);
        assert_eq!(core::mem::align_of::<Ipv4Hdr>(), 1);
    }

    #[test]
    fn tcp_hdr_layout() {
        assert_eq!(core::mem::size_of::<TcpHdr>(), 20);
        assert_eq!(core::mem::align_of::<TcpHdr>(), 1);
    }

    #[test]
    fn udp_hdr_layout() {
        assert_eq!(core::mem::size_of::<UdpHdr>(), 8);
        assert_eq!(core::mem::align_of::<UdpHdr>(), 1);
    }

    // --- ABI struct layout tests ---

    #[test]
    fn packet_meta_layout() {
        assert_eq!(core::mem::size_of::<PacketMeta>(), 16);
        assert_eq!(core::mem::align_of::<PacketMeta>(), 1);
    }

    #[test]
    fn frame_index_layout() {
        assert_eq!(core::mem::size_of::<FrameIndex>(), 4);
        assert_eq!(core::mem::align_of::<FrameIndex>(), 1);
    }

    #[test]
    fn umem_addr_layout() {
        assert_eq!(core::mem::size_of::<UmemAddr>(), 8);
        assert_eq!(core::mem::align_of::<UmemAddr>(), 1);
    }

    #[test]
    fn queue_id_layout() {
        assert_eq!(core::mem::size_of::<QueueId>(), 4);
        assert_eq!(core::mem::align_of::<QueueId>(), 1);
    }

    #[test]
    fn policy_action_layout() {
        assert_eq!(core::mem::size_of::<PolicyAction>(), 1);
        assert_eq!(core::mem::align_of::<PolicyAction>(), 1);
    }

    // --- Endian wrapper layout tests ---

    #[test]
    fn be_u16_layout() {
        assert_eq!(core::mem::size_of::<BeU16>(), 2);
        assert_eq!(core::mem::align_of::<BeU16>(), 1);
    }

    #[test]
    fn be_u32_layout() {
        assert_eq!(core::mem::size_of::<BeU32>(), 4);
        assert_eq!(core::mem::align_of::<BeU32>(), 1);
    }

    // --- Endian conversion tests ---

    #[test]
    fn be_u16_roundtrip() {
        let val: u16 = 0x1234;
        let be = BeU16::from_native(val);
        assert_eq!(be.to_native(), val);
        assert_eq!(be.to_be_bytes(), [0x12, 0x34]);
    }

    #[test]
    fn be_u32_roundtrip() {
        let val: u32 = 0xDEAD_BEEF;
        let be = BeU32::from_native(val);
        assert_eq!(be.to_native(), val);
        assert_eq!(be.to_be_bytes(), [0xDE, 0xAD, 0xBE, 0xEF]);
    }

    // --- Session type layout tests ---

    #[test]
    fn session_key_layout() {
        assert_eq!(core::mem::size_of::<SessionKey>(), 8);
        assert_eq!(core::mem::align_of::<SessionKey>(), 1);
    }

    #[test]
    fn session_value_layout() {
        assert_eq!(core::mem::size_of::<SessionValue>(), 4);
        assert_eq!(core::mem::align_of::<SessionValue>(), 1);
    }

    #[test]
    fn be_u16_from_be_bytes() {
        let be = BeU16::from_be_bytes([0xAB, 0xCD]);
        assert_eq!(be.to_native(), 0xABCD);
    }

    #[test]
    fn be_u32_from_be_bytes() {
        let be = BeU32::from_be_bytes([0x01, 0x02, 0x03, 0x04]);
        assert_eq!(be.to_native(), 0x01020304);
    }
}
