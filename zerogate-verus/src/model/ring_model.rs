// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Formal model of ring buffer capacity and ownership invariants.
//!
//! Pure model — no I/O, no unsafe, no syscalls.
//!
//! ## Invariants to prove
//!
//! 1. Ring occupancy cannot exceed capacity.
//! 2. A frame index appears in at most one ring at a time.
//! 3. No duplicate frame index within a single ring.

use std::collections::HashSet;

/// Model of a bounded ring buffer.
pub struct RingModel {
    capacity: usize,
    entries: Vec<u32>, // frame indices
    seen: HashSet<u32>,
}

impl RingModel {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Vec::with_capacity(capacity),
            seen: HashSet::new(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Enqueues a frame index. Returns Err if ring is full or duplicate.
    pub fn enqueue(&mut self, frame_index: u32) -> Result<(), RingError> {
        if self.is_full() {
            return Err(RingError::Full);
        }
        if self.seen.contains(&frame_index) {
            return Err(RingError::Duplicate(frame_index));
        }
        self.seen.insert(frame_index);
        self.entries.push(frame_index);
        Ok(())
    }

    /// Dequeues a frame index from the front. Returns Err if empty.
    pub fn dequeue(&mut self) -> Result<u32, RingError> {
        if self.is_empty() {
            return Err(RingError::Empty);
        }
        let idx = self.entries.remove(0);
        self.seen.remove(&idx);
        Ok(idx)
    }

    /// Checks that no frame appears twice in this ring.
    pub fn no_duplicates(&self) -> bool {
        self.seen.len() == self.entries.len()
    }

    /// Checks that occupancy does not exceed capacity.
    pub fn within_capacity(&self) -> bool {
        self.entries.len() <= self.capacity
    }

    /// Returns true if the given frame index is in this ring.
    pub fn contains(&self, frame_index: u32) -> bool {
        self.seen.contains(&frame_index)
    }
}

/// Errors from ring operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RingError {
    Full,
    Empty,
    Duplicate(u32),
}

/// Checks that a frame index appears in at most one ring.
pub fn frame_in_at_most_one_ring(frame_index: u32, rings: &[&RingModel]) -> bool {
    let count = rings.iter().filter(|r| r.contains(frame_index)).count();
    count <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_capacity_enforced() {
        let mut ring = RingModel::new(2);
        assert!(ring.enqueue(0).is_ok());
        assert!(ring.enqueue(1).is_ok());
        assert_eq!(ring.enqueue(2), Err(RingError::Full));
        assert!(ring.within_capacity());
    }

    #[test]
    fn ring_no_duplicates() {
        let mut ring = RingModel::new(4);
        ring.enqueue(5).unwrap();
        assert_eq!(ring.enqueue(5), Err(RingError::Duplicate(5)));
        assert!(ring.no_duplicates());
    }

    #[test]
    fn ring_dequeue_order() {
        let mut ring = RingModel::new(4);
        ring.enqueue(10).unwrap();
        ring.enqueue(20).unwrap();
        ring.enqueue(30).unwrap();
        assert_eq!(ring.dequeue().unwrap(), 10);
        assert_eq!(ring.dequeue().unwrap(), 20);
        assert_eq!(ring.dequeue().unwrap(), 30);
        assert_eq!(ring.dequeue(), Err(RingError::Empty));
    }

    #[test]
    fn frame_in_at_most_one_ring_check() {
        let mut fill = RingModel::new(4);
        let mut rx = RingModel::new(4);

        fill.enqueue(0).unwrap();
        rx.enqueue(1).unwrap();

        assert!(frame_in_at_most_one_ring(0, &[&fill, &rx]));
        assert!(frame_in_at_most_one_ring(1, &[&fill, &rx]));
        assert!(frame_in_at_most_one_ring(99, &[&fill, &rx])); // not in any

        // If same frame were in both (can't happen via API), check detects it.
    }

    #[test]
    fn fill_rx_tx_completion_lifecycle() {
        let mut fill = RingModel::new(4);
        let mut rx = RingModel::new(4);
        let mut tx = RingModel::new(4);
        let mut comp = RingModel::new(4);

        // Frame 0: enqueue in FILL
        fill.enqueue(0).unwrap();
        assert!(frame_in_at_most_one_ring(0, &[&fill, &rx, &tx, &comp]));

        // Kernel consumes from FILL -> frame now in RX
        let idx = fill.dequeue().unwrap();
        rx.enqueue(idx).unwrap();
        assert!(frame_in_at_most_one_ring(idx, &[&fill, &rx, &tx, &comp]));

        // Userspace consumes from RX -> processes -> submits to TX
        let idx = rx.dequeue().unwrap();
        tx.enqueue(idx).unwrap();
        assert!(frame_in_at_most_one_ring(idx, &[&fill, &rx, &tx, &comp]));

        // Kernel completes TX -> frame in COMPLETION
        let idx = tx.dequeue().unwrap();
        comp.enqueue(idx).unwrap();
        assert!(frame_in_at_most_one_ring(idx, &[&fill, &rx, &tx, &comp]));

        // Userspace drains COMPLETION -> frame is free
        let idx = comp.dequeue().unwrap();
        assert!(!fill.contains(idx));
        assert!(!rx.contains(idx));
        assert!(!tx.contains(idx));
        assert!(!comp.contains(idx));
    }
}
