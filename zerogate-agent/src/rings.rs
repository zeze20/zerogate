// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Ring buffer abstractions for AF_XDP FILL, COMPLETION, RX, and TX rings.
//!
//! No `unsafe` in this file. These are high-level wrappers that track
//! ring state; actual kernel ring operations are behind the `xsk` module
//! or the `xdp` crate.
//!
//! No direct raw descriptor mutation outside this module.

/// Descriptor passed through the RX/TX rings.
#[derive(Debug, Clone, Copy)]
pub struct RingDesc {
    /// UMEM byte offset of the frame.
    pub addr: u64,
    /// Length of the packet data.
    pub len: u32,
    /// Options/flags.
    pub options: u32,
}

/// Configuration for ring sizes.
#[derive(Debug, Clone, Copy)]
pub struct RingConfig {
    pub fill_size: u32,
    pub completion_size: u32,
    pub rx_size: u32,
    pub tx_size: u32,
}

impl Default for RingConfig {
    fn default() -> Self {
        Self {
            fill_size: 4096,
            completion_size: 2048,
            rx_size: 2048,
            tx_size: 0,
        }
    }
}

/// Tracks the logical state of the fill ring from userspace perspective.
pub struct FillRingTracker {
    /// Number of descriptors submitted to the fill ring.
    pub submitted: u64,
    /// Number of descriptors consumed by the kernel.
    pub consumed: u64,
}

impl FillRingTracker {
    pub fn new() -> Self {
        Self {
            submitted: 0,
            consumed: 0,
        }
    }

    /// Records that `n` descriptors were submitted to the fill ring.
    pub fn record_submit(&mut self, n: u32) {
        self.submitted += n as u64;
    }
}

/// Tracks the logical state of the completion ring from userspace perspective.
pub struct CompletionRingTracker {
    /// Number of descriptors drained from the completion ring.
    pub drained: u64,
}

impl CompletionRingTracker {
    pub fn new() -> Self {
        Self { drained: 0 }
    }

    /// Records that `n` descriptors were drained from the completion ring.
    pub fn record_drain(&mut self, n: u32) {
        self.drained += n as u64;
    }
}
