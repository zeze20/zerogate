// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Per-queue cache-padded statistics counters.
//!
//! No `unsafe` in this file.
//! Each queue has its own counters to avoid atomic contention.

use std::fmt;

/// Cache line size for padding.
const CACHE_LINE_SIZE: usize = 64;

/// Per-queue statistics. Padded to a cache line to avoid false sharing
/// when multiple queue threads update adjacent counters.
#[repr(C)]
pub struct QueueStats {
    pub rx_packets: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub tx_bytes: u64,
    pub rx_drops: u64,
    pub fill_refills: u64,
    pub completion_drains: u64,
    pub invalid_packets: u64,
    _pad: [u8; CACHE_LINE_SIZE - 64], // 8 fields * 8 bytes = 64 = CACHE_LINE_SIZE
}

const _: () = assert!(
    core::mem::size_of::<QueueStats>() == CACHE_LINE_SIZE,
    "QueueStats must be exactly one cache line"
);

impl QueueStats {
    pub const fn new() -> Self {
        Self {
            rx_packets: 0,
            rx_bytes: 0,
            tx_packets: 0,
            tx_bytes: 0,
            rx_drops: 0,
            fill_refills: 0,
            completion_drains: 0,
            invalid_packets: 0,
            _pad: [0; 0],
        }
    }

    /// Resets all counters to zero.
    pub fn reset(&mut self) {
        self.rx_packets = 0;
        self.rx_bytes = 0;
        self.tx_packets = 0;
        self.tx_bytes = 0;
        self.rx_drops = 0;
        self.fill_refills = 0;
        self.completion_drains = 0;
        self.invalid_packets = 0;
    }
}

impl fmt::Display for QueueStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rx_pkts={} rx_bytes={} tx_pkts={} tx_bytes={} drops={} fills={} completions={} invalid={}",
            self.rx_packets,
            self.rx_bytes,
            self.tx_packets,
            self.tx_bytes,
            self.rx_drops,
            self.fill_refills,
            self.completion_drains,
            self.invalid_packets,
        )
    }
}

/// Aggregated statistics across all queues.
pub struct AggregateStats {
    pub total_rx_packets: u64,
    pub total_rx_bytes: u64,
    pub total_tx_packets: u64,
    pub total_tx_bytes: u64,
    pub total_drops: u64,
}

impl AggregateStats {
    /// Aggregates stats from multiple queue counters.
    pub fn from_queues(queues: &[QueueStats]) -> Self {
        let mut agg = Self {
            total_rx_packets: 0,
            total_rx_bytes: 0,
            total_tx_packets: 0,
            total_tx_bytes: 0,
            total_drops: 0,
        };
        for q in queues {
            agg.total_rx_packets += q.rx_packets;
            agg.total_rx_bytes += q.rx_bytes;
            agg.total_tx_packets += q.tx_packets;
            agg.total_tx_bytes += q.tx_bytes;
            agg.total_drops += q.rx_drops;
        }
        agg
    }
}

impl fmt::Display for AggregateStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "total: rx_pkts={} rx_bytes={} tx_pkts={} tx_bytes={} drops={}",
            self.total_rx_packets,
            self.total_rx_bytes,
            self.total_tx_packets,
            self.total_tx_bytes,
            self.total_drops,
        )
    }
}
