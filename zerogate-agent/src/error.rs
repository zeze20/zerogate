use std::fmt;

use crate::frame::FrameState;

#[derive(Debug)]
#[allow(dead_code)]
pub enum ZeroGateError {
    BpfLoadFailed(String),
    XdpAttachFailed(String),
    XdpDetachFailed(String),
    MapOpenFailed(String),
    MapUpdateFailed(String),
    MapDeleteFailed(String),
    InterfaceResolveFailed(String),
    InvalidEbpfState(String),
    UnsupportedPlatform(String),
    NotImplemented(String),
    InvalidConfig(String),
    InvalidPolicy(String),
    InvalidSession(String),
    InvalidUmemConfig(String),
    InvalidFrameIndex {
        index: u32,
        frame_count: u32,
    },
    InvalidUmemOffset {
        offset: u64,
        total_size: usize,
    },
    UnalignedUmemOffset {
        offset: u64,
        frame_size: u32,
    },
    UmemAllocationFailed(String),
    UmemSizeOverflow,
    InvalidXskConfig(String),
    XskCreateFailed(String),
    XskBindFailed(String),
    XskCloseFailed(String),
    InvalidRingConfig(String),
    RingFull(String),
    RingEmpty(String),
    RingReleaseFailed(String),
    InvalidDescriptor(String),
    DescriptorOutOfBounds(String),
    DescriptorTooLarge {
        len: u32,
        frame_size: u32,
    },
    DescriptorZeroLength,
    DuplicateDescriptor(String),
    InvalidFrameTransition {
        index: u32,
        current: FrameState,
        attempted: FrameState,
    },
    FramePoolExhausted {
        frame_count: usize,
    },
    FrameOwnershipCorrupt(String),
}

impl fmt::Display for ZeroGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZeroGateError::BpfLoadFailed(msg) => write!(f, "BPF load failed: {msg}"),
            ZeroGateError::XdpAttachFailed(msg) => write!(f, "XDP attach failed: {msg}"),
            ZeroGateError::XdpDetachFailed(msg) => write!(f, "XDP detach failed: {msg}"),
            ZeroGateError::MapOpenFailed(msg) => write!(f, "BPF map open failed: {msg}"),
            ZeroGateError::MapUpdateFailed(msg) => write!(f, "BPF map update failed: {msg}"),
            ZeroGateError::MapDeleteFailed(msg) => write!(f, "BPF map delete failed: {msg}"),
            ZeroGateError::InterfaceResolveFailed(msg) => {
                write!(f, "interface resolve failed: {msg}")
            }
            ZeroGateError::InvalidEbpfState(msg) => write!(f, "invalid eBPF state: {msg}"),
            ZeroGateError::UnsupportedPlatform(msg) => write!(f, "unsupported platform: {msg}"),
            ZeroGateError::NotImplemented(msg) => write!(f, "not implemented: {msg}"),
            ZeroGateError::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
            ZeroGateError::InvalidPolicy(msg) => write!(f, "invalid policy: {msg}"),
            ZeroGateError::InvalidSession(msg) => write!(f, "invalid session: {msg}"),
            ZeroGateError::InvalidUmemConfig(msg) => write!(f, "invalid UMEM config: {msg}"),
            ZeroGateError::InvalidFrameIndex { index, frame_count } => {
                write!(
                    f,
                    "invalid frame index: {index} (frame_count={frame_count})"
                )
            }
            ZeroGateError::InvalidUmemOffset { offset, total_size } => {
                write!(f, "invalid UMEM offset: {offset} (total_size={total_size})")
            }
            ZeroGateError::UnalignedUmemOffset { offset, frame_size } => {
                write!(
                    f,
                    "unaligned UMEM offset: {offset} (frame_size={frame_size})"
                )
            }
            ZeroGateError::UmemAllocationFailed(msg) => {
                write!(f, "UMEM allocation failed: {msg}")
            }
            ZeroGateError::UmemSizeOverflow => {
                write!(f, "UMEM total size overflows usize")
            }
            ZeroGateError::InvalidXskConfig(msg) => write!(f, "invalid XSK config: {msg}"),
            ZeroGateError::XskCreateFailed(msg) => write!(f, "XSK create failed: {msg}"),
            ZeroGateError::XskBindFailed(msg) => write!(f, "XSK bind failed: {msg}"),
            ZeroGateError::XskCloseFailed(msg) => write!(f, "XSK close failed: {msg}"),
            ZeroGateError::InvalidRingConfig(msg) => {
                write!(f, "invalid ring config: {msg}")
            }
            ZeroGateError::RingFull(msg) => write!(f, "ring full: {msg}"),
            ZeroGateError::RingEmpty(msg) => write!(f, "ring empty: {msg}"),
            ZeroGateError::RingReleaseFailed(msg) => {
                write!(f, "ring release failed: {msg}")
            }
            ZeroGateError::InvalidDescriptor(msg) => {
                write!(f, "invalid descriptor: {msg}")
            }
            ZeroGateError::DescriptorOutOfBounds(msg) => {
                write!(f, "descriptor out of bounds: {msg}")
            }
            ZeroGateError::DescriptorTooLarge { len, frame_size } => {
                write!(
                    f,
                    "descriptor too large: len={len} exceeds frame_size={frame_size}"
                )
            }
            ZeroGateError::DescriptorZeroLength => {
                write!(f, "descriptor has zero length")
            }
            ZeroGateError::DuplicateDescriptor(msg) => {
                write!(f, "duplicate descriptor: {msg}")
            }
            ZeroGateError::InvalidFrameTransition {
                index,
                current,
                attempted,
            } => {
                write!(
                    f,
                    "invalid frame transition: frame {index} in state {current} \
                     cannot transition to {attempted}"
                )
            }
            ZeroGateError::FramePoolExhausted { frame_count } => {
                write!(
                    f,
                    "frame pool exhausted: all {frame_count} frames are in use"
                )
            }
            ZeroGateError::FrameOwnershipCorrupt(msg) => {
                write!(f, "frame ownership corrupt: {msg}")
            }
        }
    }
}

impl std::error::Error for ZeroGateError {}
