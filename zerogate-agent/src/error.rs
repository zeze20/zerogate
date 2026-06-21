// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Error types for the ZeroGate agent.

use std::fmt;

/// Top-level error type for the agent.
#[derive(Debug)]
pub enum ZeroGateError {
    Config(String),
    Ebpf(String),
    Xsk(String),
    Umem(String),
    Ring(String),
    FramePool(FramePoolError),
    Io(std::io::Error),
    System(String),
}

impl fmt::Display for ZeroGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "config error: {msg}"),
            Self::Ebpf(msg) => write!(f, "eBPF error: {msg}"),
            Self::Xsk(msg) => write!(f, "XSK error: {msg}"),
            Self::Umem(msg) => write!(f, "UMEM error: {msg}"),
            Self::Ring(msg) => write!(f, "ring error: {msg}"),
            Self::FramePool(e) => write!(f, "frame pool error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::System(msg) => write!(f, "system error: {msg}"),
        }
    }
}

impl std::error::Error for ZeroGateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::FramePool(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ZeroGateError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<FramePoolError> for ZeroGateError {
    fn from(e: FramePoolError) -> Self {
        Self::FramePool(e)
    }
}

/// Errors from the frame ownership state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramePoolError {
    /// Attempted an illegal state transition.
    IllegalTransition {
        frame_index: u32,
        from: &'static str,
        to: &'static str,
    },
    /// Frame pool exhausted — no free frames available.
    PoolExhausted,
    /// Frame index out of bounds.
    OutOfBounds { frame_index: u32, max: u32 },
    /// Duplicate frame detected in a ring.
    DuplicateFrame { frame_index: u32 },
}

impl fmt::Display for FramePoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalTransition {
                frame_index,
                from,
                to,
            } => write!(
                f,
                "illegal frame transition: frame {frame_index} from {from} to {to}"
            ),
            Self::PoolExhausted => write!(f, "frame pool exhausted"),
            Self::OutOfBounds { frame_index, max } => {
                write!(f, "frame index {frame_index} out of bounds (max {max})")
            }
            Self::DuplicateFrame { frame_index } => {
                write!(f, "duplicate frame {frame_index} in ring")
            }
        }
    }
}

impl std::error::Error for FramePoolError {}
