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
        }
    }
}

impl std::error::Error for ZeroGateError {}
