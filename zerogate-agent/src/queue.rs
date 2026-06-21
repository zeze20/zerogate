// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Per-queue RX processing loop.
//!
//! No `unsafe` in this file. Each queue owns its frame pool, rings, and
//! stats. The RX loop follows the mandated order:
//!
//! 1. Drain COMPLETION ring.
//! 2. Transition completed TX frames: Tx -> Completion -> Free.
//! 3. Refill FILL ring from Free frames: Free -> InFill -> Kernel.
//! 4. Poll RX ring.
//! 5. For each RX descriptor: Kernel -> Rx -> User.
//! 6. Process packet.
//! 7. If forwarding: User -> Tx.
//! 8. If recycling: User -> InFill -> Kernel.
//! 9. Kick TX if needed.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{debug, info, warn};

use crate::cpu;
use crate::frame_pool::FramePool;
use crate::policy::{self, PacketAction};
use crate::rings::{CompletionRingTracker, FillRingTracker};
use crate::stats::QueueStats;
use crate::umem::UmemRegion;

/// Per-queue context that owns all resources for a single NIC queue.
pub struct QueueContext {
    pub queue_id: u32,
    pub cpu_id: usize,
    pub frame_pool: FramePool,
    pub umem: UmemRegion,
    pub stats: QueueStats,
    pub fill_tracker: FillRingTracker,
    pub completion_tracker: CompletionRingTracker,
}

impl QueueContext {
    /// Creates a new queue context.
    pub fn new(queue_id: u32, cpu_id: usize, frame_pool: FramePool, umem: UmemRegion) -> Self {
        Self {
            queue_id,
            cpu_id,
            frame_pool,
            umem,
            stats: QueueStats::new(),
            fill_tracker: FillRingTracker::new(),
            completion_tracker: CompletionRingTracker::new(),
        }
    }

    /// Seeds the fill ring with all available free frames.
    ///
    /// Transitions: Free -> InFill for each frame.
    pub fn seed_fill_ring(&mut self) -> Result<u32, crate::error::ZeroGateError> {
        let mut count = 0u32;
        while self.frame_pool.free_count() > 0 {
            let (idx, _offset) = self.frame_pool.allocate_for_fill()?;
            // In a real implementation, we would submit `offset` to the
            // fill ring descriptor here.
            // Transition InFill -> Kernel happens when the kernel consumes it.
            // For the initial seed, we mark it kernel-owned immediately
            // since we submit them all in a batch.
            self.frame_pool.mark_kernel_owned(idx)?;
            count += 1;
        }
        self.fill_tracker.record_submit(count);
        self.stats.fill_refills += count as u64;
        info!(
            "queue {}: seeded fill ring with {count} frames",
            self.queue_id
        );
        Ok(count)
    }

    /// Processes a batch of received packets.
    ///
    /// For each frame: Kernel -> Rx -> User -> evaluate -> recycle.
    pub fn process_rx_batch(
        &mut self,
        rx_descriptors: &[(u64, u32)], // (umem_offset, length)
    ) -> Result<(), crate::error::ZeroGateError> {
        for &(offset, len) in rx_descriptors {
            let frame_index = self.frame_pool.offset_to_index(offset);

            // Kernel -> Rx
            self.frame_pool.mark_rx(frame_index)?;

            // Rx -> User
            self.frame_pool.acquire_user(frame_index)?;

            // Read packet data from UMEM.
            let pkt = match self.umem.frame_slice(offset, len as usize) {
                Some(data) => data,
                None => {
                    warn!(
                        "queue {}: frame {} offset {} len {} out of UMEM bounds",
                        self.queue_id, frame_index, offset, len
                    );
                    self.stats.rx_drops += 1;
                    // Recycle: User -> InFill
                    let _ = self.frame_pool.recycle_to_fill(frame_index)?;
                    self.frame_pool.mark_kernel_owned(frame_index)?;
                    continue;
                }
            };

            self.stats.rx_packets += 1;
            self.stats.rx_bytes += len as u64;

            // Evaluate packet against policy (pure function).
            match policy::evaluate_packet(pkt) {
                PacketAction::Accept {
                    session_id,
                    sequence_num,
                    payload_len,
                } => {
                    println!(
                        "[queue {}] session_id={:#018x} seq={} payload_len={}",
                        self.queue_id, session_id, sequence_num, payload_len
                    );
                }
                PacketAction::Drop => {
                    debug!(
                        "queue {}: dropped packet from frame {}",
                        self.queue_id, frame_index
                    );
                    self.stats.invalid_packets += 1;
                }
                PacketAction::PassThrough => {
                    debug!(
                        "queue {}: pass-through packet from frame {}",
                        self.queue_id, frame_index
                    );
                }
            }

            // Recycle frame: User -> InFill -> Kernel
            let _recycle_offset = self.frame_pool.recycle_to_fill(frame_index)?;
            self.frame_pool.mark_kernel_owned(frame_index)?;
            self.stats.fill_refills += 1;
        }
        Ok(())
    }

    /// Drains completed TX frames.
    ///
    /// Transitions: Tx -> Completion -> Free for each completed frame.
    pub fn drain_completion(
        &mut self,
        completed_offsets: &[u64],
    ) -> Result<(), crate::error::ZeroGateError> {
        for &offset in completed_offsets {
            let frame_index = self.frame_pool.offset_to_index(offset);
            self.frame_pool.complete_tx(frame_index)?;
            self.frame_pool.free_completed(frame_index)?;
            self.stats.completion_drains += 1;
        }
        self.completion_tracker
            .record_drain(completed_offsets.len() as u32);
        Ok(())
    }
}

/// Runs the per-queue RX loop.
///
/// This function is intended to be spawned as a thread per queue.
/// It pins to the configured CPU core and processes packets until
/// the shutdown signal is received.
///
/// TODO: The actual AF_XDP ring polling is hardware-specific and
/// requires the `xdp` crate on Linux. This implementation provides
/// the control flow and state machine logic.
pub fn run_queue_loop(mut ctx: QueueContext, shutdown: Arc<AtomicBool>) -> QueueContext {
    cpu::pin_to_cpu(ctx.cpu_id);

    info!(
        "queue {} RX loop started on CPU {}",
        ctx.queue_id, ctx.cpu_id
    );

    // Seed the fill ring.
    if let Err(e) = ctx.seed_fill_ring() {
        warn!("queue {}: failed to seed fill ring: {e}", ctx.queue_id);
    }

    // RX loop (placeholder — real implementation polls AF_XDP rings).
    while !shutdown.load(Ordering::Relaxed) {
        // TODO: Hardware-specific AF_XDP polling:
        //
        // 1. Drain COMPLETION ring -> ctx.drain_completion(...)
        // 2. Refill FILL ring from free frames
        // 3. Poll RX ring with timeout
        // 4. For each RX descriptor:
        //    ctx.process_rx_batch(&descriptors)
        // 5. Kick TX if needed

        // Sleep briefly to avoid busy-spinning in placeholder mode.
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    info!(
        "queue {} RX loop stopped. stats: {}",
        ctx.queue_id, ctx.stats
    );

    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_pool::FramePool;
    use crate::umem::{UmemConfig, UmemRegion};

    fn make_test_context(frame_count: u32) -> QueueContext {
        let frame_size = 4096;
        let pool = FramePool::new(frame_count, frame_size);
        let umem = UmemRegion::allocate(UmemConfig {
            frame_count,
            frame_size,
        })
        .unwrap();
        QueueContext::new(0, 0, pool, umem)
    }

    #[test]
    fn seed_fill_ring() {
        let mut ctx = make_test_context(8);
        let count = ctx.seed_fill_ring().unwrap();
        assert_eq!(count, 8);
        assert_eq!(ctx.frame_pool.free_count(), 0);
    }

    #[test]
    fn fill_rx_lifecycle_simulation() {
        let frame_count = 4u32;
        let frame_size = 4096u32;
        let mut pool = FramePool::new(frame_count, frame_size);

        let mut offsets = Vec::new();
        // Free -> InFill -> Kernel (seeding)
        for _ in 0..frame_count {
            let (idx, offset) = pool.allocate_for_fill().unwrap();
            pool.mark_kernel_owned(idx).unwrap();
            offsets.push(offset);
        }

        // Simulate RX: Kernel -> Rx -> User -> InFill -> Kernel
        for &offset in &offsets {
            let idx = pool.offset_to_index(offset);
            pool.mark_rx(idx).unwrap();
            pool.acquire_user(idx).unwrap();
            let _recycled = pool.recycle_to_fill(idx).unwrap();
            pool.mark_kernel_owned(idx).unwrap();
        }

        // All frames should be in Kernel state.
        for &offset in &offsets {
            let idx = pool.offset_to_index(offset);
            assert_eq!(
                pool.state(idx).unwrap(),
                crate::frame_pool::FrameState::Kernel
            );
        }
    }

    #[test]
    fn tx_completion_lifecycle() {
        let mut pool = FramePool::new(4, 4096);

        // Simulate: Free -> InFill -> Kernel -> Rx -> User -> Tx -> Completion -> Free
        let (idx, _) = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(idx).unwrap();
        pool.mark_rx(idx).unwrap();
        pool.acquire_user(idx).unwrap();
        pool.submit_tx(idx).unwrap();
        pool.complete_tx(idx).unwrap();
        pool.free_completed(idx).unwrap();

        assert_eq!(
            pool.state(idx).unwrap(),
            crate::frame_pool::FrameState::Free
        );
    }
}
