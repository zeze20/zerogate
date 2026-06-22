use std::fmt;

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
    InvalidFrameIndex { index: u32, frame_count: u32 },
    InvalidUmemOffset { offset: u64, total_size: usize },
    UnalignedUmemOffset { offset: u64, frame_size: u32 },
    UmemAllocationFailed(String),
    UmemSizeOverflow,
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
        }
    }
}

impl std::error::Error for ZeroGateError {}
