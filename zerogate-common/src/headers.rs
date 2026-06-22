//! Network packet header structures for eBPF/XDP parsing.
//!
//! All headers use `#[repr(C, packed)]` to match on-wire layout exactly.
//! Fields are fixed-width integers stored in network byte order where applicable.
//! No implicit padding. No references to packed fields.

/// Ethernet frame header (14 bytes on wire).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EthHdr {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ether_type: u16,
}

/// IPv4 header (minimum 20 bytes, no options).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv4Hdr {
    pub version_ihl: u8,
    pub dscp_ecn: u8,
    pub total_length: u16,
    pub identification: u16,
    pub flags_fragment: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src_addr: u32,
    pub dst_addr: u32,
}

/// TCP header (minimum 20 bytes, no options).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TcpHdr {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset_flags: u16,
    pub window: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
}

/// UDP header (8 bytes).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UdpHdr {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}
