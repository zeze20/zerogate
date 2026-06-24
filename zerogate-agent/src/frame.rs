//! Frame ownership state machine for UMEM frames.
//!
//! Enforces that every frame is in exactly one ownership state at a time
//! and can only move through legal lifecycle transitions.
//!
//! Target lifecycle:
//!   Free -> InFill -> Kernel -> Rx -> User -> InFill  (RX recycle)
//!   Free -> InFill -> Kernel -> Rx -> User -> Tx -> Completion -> Free  (TX path)
//!
//! **MR10 scope:** ownership tracking and transition validation only.
//! No queue loop, no ring polling, no AF_XDP bind, no UMEM kernel registration.

use std::collections::VecDeque;
use std::fmt;

use zerogate_common::abi::FrameIndex;

use crate::error::ZeroGateError;

// ---------------------------------------------------------------------------
// FrameState
// ---------------------------------------------------------------------------

/// Ownership state of a single UMEM frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameState {
    /// Available for allocation via the fill ring.
    #[default]
    Free,
    /// Queued for submission to the fill ring.
    InFill,
    /// Owned by the kernel (submitted via fill ring, awaiting RX).
    Kernel,
    /// Received by kernel, available for user consumption.
    Rx,
    /// Owned by userspace for processing.
    User,
    /// Submitted for transmission via the TX ring.
    Tx,
    /// Transmission completed by kernel, awaiting release.
    Completion,
}

impl fmt::Display for FrameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameState::Free => write!(f, "Free"),
            FrameState::InFill => write!(f, "InFill"),
            FrameState::Kernel => write!(f, "Kernel"),
            FrameState::Rx => write!(f, "Rx"),
            FrameState::User => write!(f, "User"),
            FrameState::Tx => write!(f, "Tx"),
            FrameState::Completion => write!(f, "Completion"),
        }
    }
}

#[allow(dead_code)]
impl FrameState {
    /// Returns `true` if transitioning from `self` to `next` is legal.
    pub fn can_transition_to(self, next: FrameState) -> bool {
        matches!(
            (self, next),
            (FrameState::Free, FrameState::InFill)
                | (FrameState::InFill, FrameState::Kernel)
                | (FrameState::Kernel, FrameState::Rx)
                | (FrameState::Rx, FrameState::User)
                | (FrameState::User, FrameState::InFill)
                | (FrameState::User, FrameState::Tx)
                | (FrameState::Tx, FrameState::Completion)
                | (FrameState::Completion, FrameState::Free)
        )
    }
}

// ---------------------------------------------------------------------------
// FramePool
// ---------------------------------------------------------------------------

/// Tracks ownership state of all UMEM frames and manages the free list.
///
/// Every frame is in exactly one state. The free list contains indices of
/// frames in `Free` state only. State mutations occur exclusively through
/// controlled transition methods that validate before mutating.
#[derive(Debug)]
#[allow(dead_code)]
pub struct FramePool {
    states: Vec<FrameState>,
    free_list: VecDeque<usize>,
}

#[allow(dead_code)]
impl FramePool {
    /// Create a new pool with `frame_count` frames, all initially `Free`.
    ///
    /// Returns `FramePoolExhausted` if `frame_count` is zero.
    pub fn new(frame_count: usize) -> Result<Self, ZeroGateError> {
        if frame_count == 0 {
            return Err(ZeroGateError::FramePoolExhausted { frame_count: 0 });
        }
        let states = vec![FrameState::Free; frame_count];
        let free_list: VecDeque<usize> = (0..frame_count).collect();
        Ok(Self { states, free_list })
    }

    /// Total number of frames managed by this pool.
    pub fn frame_count(&self) -> usize {
        self.states.len()
    }

    /// Number of frames currently in `Free` state.
    pub fn free_count(&self) -> usize {
        self.free_list.len()
    }

    /// Query the current state of a frame.
    pub fn state(&self, frame: FrameIndex) -> Result<FrameState, ZeroGateError> {
        let idx = self.validated_index(frame)?;
        Ok(self.states[idx])
    }

    /// Pop a free frame and transition it to `InFill`. Free -> InFill.
    pub fn allocate_for_fill(&mut self) -> Result<FrameIndex, ZeroGateError> {
        let idx = self
            .free_list
            .pop_front()
            .ok_or(ZeroGateError::FramePoolExhausted {
                frame_count: self.states.len(),
            })?;
        if self.states[idx] != FrameState::Free {
            self.free_list.push_front(idx);
            return Err(ZeroGateError::FrameOwnershipCorrupt(format!(
                "frame {} in free_list has state {}, expected Free",
                idx, self.states[idx]
            )));
        }
        self.states[idx] = FrameState::InFill;
        Ok(FrameIndex { index: idx as u32 })
    }

    /// Mark a frame as kernel-owned. InFill -> Kernel.
    pub fn mark_kernel_owned(&mut self, frame: FrameIndex) -> Result<(), ZeroGateError> {
        self.transition(frame, FrameState::InFill, FrameState::Kernel)
    }

    /// Mark a frame as received. Kernel -> Rx.
    pub fn mark_rx(&mut self, frame: FrameIndex) -> Result<(), ZeroGateError> {
        self.transition(frame, FrameState::Kernel, FrameState::Rx)
    }

    /// Acquire a received frame for user processing. Rx -> User.
    pub fn acquire_user(&mut self, frame: FrameIndex) -> Result<(), ZeroGateError> {
        self.transition(frame, FrameState::Rx, FrameState::User)
    }

    /// Recycle a user frame back to the fill path. User -> InFill.
    ///
    /// Does NOT push to free list. The frame goes directly to InFill
    /// for resubmission to the fill ring.
    pub fn recycle_to_fill(&mut self, frame: FrameIndex) -> Result<(), ZeroGateError> {
        self.transition(frame, FrameState::User, FrameState::InFill)
    }

    /// Submit a user frame for transmission. User -> Tx.
    pub fn submit_tx(&mut self, frame: FrameIndex) -> Result<(), ZeroGateError> {
        self.transition(frame, FrameState::User, FrameState::Tx)
    }

    /// Mark a transmitted frame as completed by kernel. Tx -> Completion.
    pub fn complete_tx(&mut self, frame: FrameIndex) -> Result<(), ZeroGateError> {
        self.transition(frame, FrameState::Tx, FrameState::Completion)
    }

    /// Release a completed frame back to the free pool. Completion -> Free.
    ///
    /// Pushes the frame onto the free list exactly once.
    pub fn release_completion(&mut self, frame: FrameIndex) -> Result<(), ZeroGateError> {
        let idx = self.validated_index(frame)?;
        let current = self.states[idx];
        if current != FrameState::Completion {
            return Err(ZeroGateError::InvalidFrameTransition {
                index: frame.index,
                current,
                attempted: FrameState::Free,
            });
        }
        self.states[idx] = FrameState::Free;
        self.free_list.push_back(idx);
        Ok(())
    }

    /// Verify free-list invariants (O(n) debug/assertion check).
    ///
    /// Checks:
    /// - Every free-list entry is in bounds.
    /// - Every free-list entry has state `Free`.
    /// - No duplicate entries in free list.
    /// - `free_count` matches the number of `Free`-state frames.
    pub fn assert_no_duplicate_ownership(&self) -> Result<(), ZeroGateError> {
        let mut seen = vec![false; self.states.len()];

        for &idx in &self.free_list {
            if idx >= self.states.len() {
                return Err(ZeroGateError::FrameOwnershipCorrupt(format!(
                    "free_list contains out-of-bounds index {} (frame_count={})",
                    idx,
                    self.states.len()
                )));
            }
            if self.states[idx] != FrameState::Free {
                return Err(ZeroGateError::FrameOwnershipCorrupt(format!(
                    "free_list contains frame {} with state {}, expected Free",
                    idx, self.states[idx]
                )));
            }
            if seen[idx] {
                return Err(ZeroGateError::FrameOwnershipCorrupt(format!(
                    "free_list contains duplicate index {}",
                    idx
                )));
            }
            seen[idx] = true;
        }

        let free_state_count = self
            .states
            .iter()
            .filter(|s| **s == FrameState::Free)
            .count();
        if free_state_count != self.free_list.len() {
            return Err(ZeroGateError::FrameOwnershipCorrupt(format!(
                "free_count mismatch: {} frames in Free state but free_list has {} entries",
                free_state_count,
                self.free_list.len()
            )));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    fn validated_index(&self, frame: FrameIndex) -> Result<usize, ZeroGateError> {
        let idx = frame.index as usize;
        if idx >= self.states.len() {
            return Err(ZeroGateError::InvalidFrameIndex {
                index: frame.index,
                frame_count: self.states.len() as u32,
            });
        }
        Ok(idx)
    }

    fn transition(
        &mut self,
        frame: FrameIndex,
        expected: FrameState,
        next: FrameState,
    ) -> Result<(), ZeroGateError> {
        let idx = self.validated_index(frame)?;
        let current = self.states[idx];
        if current != expected {
            return Err(ZeroGateError::InvalidFrameTransition {
                index: frame.index,
                current,
                attempted: next,
            });
        }
        self.states[idx] = next;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fi(index: u32) -> FrameIndex {
        FrameIndex { index }
    }

    // =======================================================================
    // 1. FrameState transition table
    // =======================================================================

    #[test]
    fn transition_free_to_infill_valid() {
        assert!(FrameState::Free.can_transition_to(FrameState::InFill));
    }

    #[test]
    fn transition_infill_to_kernel_valid() {
        assert!(FrameState::InFill.can_transition_to(FrameState::Kernel));
    }

    #[test]
    fn transition_kernel_to_rx_valid() {
        assert!(FrameState::Kernel.can_transition_to(FrameState::Rx));
    }

    #[test]
    fn transition_rx_to_user_valid() {
        assert!(FrameState::Rx.can_transition_to(FrameState::User));
    }

    #[test]
    fn transition_user_to_infill_valid() {
        assert!(FrameState::User.can_transition_to(FrameState::InFill));
    }

    #[test]
    fn transition_user_to_tx_valid() {
        assert!(FrameState::User.can_transition_to(FrameState::Tx));
    }

    #[test]
    fn transition_tx_to_completion_valid() {
        assert!(FrameState::Tx.can_transition_to(FrameState::Completion));
    }

    #[test]
    fn transition_completion_to_free_valid() {
        assert!(FrameState::Completion.can_transition_to(FrameState::Free));
    }

    #[test]
    fn transition_free_to_tx_invalid() {
        assert!(!FrameState::Free.can_transition_to(FrameState::Tx));
    }

    #[test]
    fn transition_free_to_kernel_invalid() {
        assert!(!FrameState::Free.can_transition_to(FrameState::Kernel));
    }

    #[test]
    fn transition_infill_to_rx_invalid() {
        assert!(!FrameState::InFill.can_transition_to(FrameState::Rx));
    }

    #[test]
    fn transition_kernel_to_user_invalid() {
        assert!(!FrameState::Kernel.can_transition_to(FrameState::User));
    }

    #[test]
    fn transition_rx_to_tx_invalid() {
        assert!(!FrameState::Rx.can_transition_to(FrameState::Tx));
    }

    #[test]
    fn transition_user_to_free_invalid() {
        assert!(!FrameState::User.can_transition_to(FrameState::Free));
    }

    #[test]
    fn transition_tx_to_free_invalid() {
        assert!(!FrameState::Tx.can_transition_to(FrameState::Free));
    }

    #[test]
    fn transition_completion_to_tx_invalid() {
        assert!(!FrameState::Completion.can_transition_to(FrameState::Tx));
    }

    #[test]
    fn same_state_transitions_invalid() {
        let states = [
            FrameState::Free,
            FrameState::InFill,
            FrameState::Kernel,
            FrameState::Rx,
            FrameState::User,
            FrameState::Tx,
            FrameState::Completion,
        ];
        for s in states {
            assert!(!s.can_transition_to(s), "{s} -> {s} should be invalid");
        }
    }

    #[test]
    fn frame_state_default_is_free() {
        assert_eq!(FrameState::default(), FrameState::Free);
    }

    #[test]
    fn frame_state_display() {
        assert_eq!(format!("{}", FrameState::Free), "Free");
        assert_eq!(format!("{}", FrameState::InFill), "InFill");
        assert_eq!(format!("{}", FrameState::Kernel), "Kernel");
        assert_eq!(format!("{}", FrameState::Rx), "Rx");
        assert_eq!(format!("{}", FrameState::User), "User");
        assert_eq!(format!("{}", FrameState::Tx), "Tx");
        assert_eq!(format!("{}", FrameState::Completion), "Completion");
    }

    // =======================================================================
    // 2. FramePool initialization
    // =======================================================================

    #[test]
    fn new_pool_all_frames_free() {
        let pool = FramePool::new(8).unwrap();
        for i in 0..8 {
            assert_eq!(pool.state(fi(i)).unwrap(), FrameState::Free);
        }
    }

    #[test]
    fn new_pool_free_count_equals_frame_count() {
        let pool = FramePool::new(16).unwrap();
        assert_eq!(pool.free_count(), 16);
        assert_eq!(pool.frame_count(), 16);
    }

    #[test]
    fn new_pool_frame_count_correct() {
        let pool = FramePool::new(32).unwrap();
        assert_eq!(pool.frame_count(), 32);
    }

    #[test]
    fn zero_frame_count_rejected() {
        let result = FramePool::new(0);
        assert!(result.is_err());
        match result.unwrap_err() {
            ZeroGateError::FramePoolExhausted { frame_count: 0 } => {}
            other => panic!("expected FramePoolExhausted(0), got: {other:?}"),
        }
    }

    #[test]
    fn new_pool_invariants_pass() {
        let pool = FramePool::new(64).unwrap();
        pool.assert_no_duplicate_ownership().unwrap();
    }

    // =======================================================================
    // 3. Allocation tests
    // =======================================================================

    #[test]
    fn allocate_returns_valid_frame() {
        let mut pool = FramePool::new(4).unwrap();
        let frame = pool.allocate_for_fill().unwrap();
        assert!(frame.index < 4);
    }

    #[test]
    fn allocated_frame_becomes_infill() {
        let mut pool = FramePool::new(4).unwrap();
        let frame = pool.allocate_for_fill().unwrap();
        assert_eq!(pool.state(frame).unwrap(), FrameState::InFill);
    }

    #[test]
    fn free_count_decreases_on_allocate() {
        let mut pool = FramePool::new(4).unwrap();
        assert_eq!(pool.free_count(), 4);
        pool.allocate_for_fill().unwrap();
        assert_eq!(pool.free_count(), 3);
        pool.allocate_for_fill().unwrap();
        assert_eq!(pool.free_count(), 2);
    }

    #[test]
    fn exhaust_all_frames_returns_exhausted() {
        let mut pool = FramePool::new(2).unwrap();
        pool.allocate_for_fill().unwrap();
        pool.allocate_for_fill().unwrap();
        match pool.allocate_for_fill() {
            Err(ZeroGateError::FramePoolExhausted { frame_count: 2 }) => {}
            other => panic!("expected FramePoolExhausted, got: {other:?}"),
        }
    }

    #[test]
    fn failed_allocation_does_not_mutate() {
        let mut pool = FramePool::new(1).unwrap();
        pool.allocate_for_fill().unwrap();
        let free_before = pool.free_count();
        let _ = pool.allocate_for_fill();
        assert_eq!(pool.free_count(), free_before);
    }

    // =======================================================================
    // 4. RX recycle lifecycle
    // =======================================================================

    #[test]
    fn rx_recycle_full_path() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.recycle_to_fill(f).unwrap();
        assert_eq!(pool.state(f).unwrap(), FrameState::InFill);
    }

    #[test]
    fn recycled_frame_is_infill_not_free() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.recycle_to_fill(f).unwrap();
        assert_eq!(pool.state(f).unwrap(), FrameState::InFill);
        assert_ne!(pool.state(f).unwrap(), FrameState::Free);
    }

    #[test]
    fn recycled_frame_not_in_free_list() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        let initial_free = pool.free_count();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.recycle_to_fill(f).unwrap();
        // free_count should not increase from recycle
        assert_eq!(pool.free_count(), initial_free);
    }

    // =======================================================================
    // 5. TX completion lifecycle
    // =======================================================================

    #[test]
    fn tx_completion_full_path() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.submit_tx(f).unwrap();
        pool.complete_tx(f).unwrap();
        pool.release_completion(f).unwrap();
        assert_eq!(pool.state(f).unwrap(), FrameState::Free);
    }

    #[test]
    fn release_completion_returns_to_free() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.submit_tx(f).unwrap();
        pool.complete_tx(f).unwrap();
        pool.release_completion(f).unwrap();
        assert_eq!(pool.state(f).unwrap(), FrameState::Free);
    }

    #[test]
    fn free_count_increases_on_release() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        let free_after_alloc = pool.free_count();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.submit_tx(f).unwrap();
        pool.complete_tx(f).unwrap();
        pool.release_completion(f).unwrap();
        assert_eq!(pool.free_count(), free_after_alloc + 1);
    }

    #[test]
    fn double_release_completion_fails() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.submit_tx(f).unwrap();
        pool.complete_tx(f).unwrap();
        pool.release_completion(f).unwrap();
        // Second release should fail — frame is now Free, not Completion
        match pool.release_completion(f) {
            Err(ZeroGateError::InvalidFrameTransition {
                current, attempted, ..
            }) => {
                assert_eq!(current, FrameState::Free);
                assert_eq!(attempted, FrameState::Free);
            }
            other => panic!("expected InvalidFrameTransition, got: {other:?}"),
        }
    }

    // =======================================================================
    // 6. Illegal transition tests
    // =======================================================================

    #[test]
    fn illegal_transition_returns_typed_error() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        // Frame is InFill, try Rx (skip Kernel)
        match pool.mark_rx(f) {
            Err(ZeroGateError::InvalidFrameTransition {
                current: FrameState::InFill,
                attempted: FrameState::Rx,
                ..
            }) => {}
            other => panic!("expected InvalidFrameTransition, got: {other:?}"),
        }
    }

    #[test]
    fn illegal_transition_state_unchanged() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        let state_before = pool.state(f).unwrap();
        let _ = pool.mark_rx(f); // illegal
        assert_eq!(pool.state(f).unwrap(), state_before);
    }

    #[test]
    fn illegal_transition_free_count_unchanged() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        let free_before = pool.free_count();
        let _ = pool.mark_rx(f); // illegal
        assert_eq!(pool.free_count(), free_before);
    }

    #[test]
    fn illegal_transition_after_partial_lifecycle() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        // Kernel -> Tx is illegal (must go through Rx -> User first)
        assert!(pool.submit_tx(f).is_err());
        assert_eq!(pool.state(f).unwrap(), FrameState::Kernel);
    }

    #[test]
    fn tx_to_free_rejected() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.submit_tx(f).unwrap();
        // Tx -> Free (via release_completion) should fail because state is Tx not Completion
        match pool.release_completion(f) {
            Err(ZeroGateError::InvalidFrameTransition {
                current: FrameState::Tx,
                ..
            }) => {}
            other => panic!("expected InvalidFrameTransition, got: {other:?}"),
        }
    }

    #[test]
    fn rx_to_tx_rejected() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        // Rx -> Tx is illegal (must go User first)
        assert!(pool.submit_tx(f).is_err());
        assert_eq!(pool.state(f).unwrap(), FrameState::Rx);
    }

    #[test]
    fn free_frame_cannot_mark_kernel() {
        let mut pool = FramePool::new(4).unwrap();
        // Frame 0 is Free initially (not allocated)
        // But we already allocated frame 0 via allocate_for_fill...
        // Let's use a frame that wasn't allocated
        assert!(pool.mark_kernel_owned(fi(3)).is_err());
        assert_eq!(pool.state(fi(3)).unwrap(), FrameState::Free);
    }

    // =======================================================================
    // 7. Bounds tests
    // =======================================================================

    #[test]
    fn state_invalid_frame_returns_error() {
        let pool = FramePool::new(4).unwrap();
        match pool.state(fi(4)) {
            Err(ZeroGateError::InvalidFrameIndex {
                index: 4,
                frame_count: 4,
            }) => {}
            other => panic!("expected InvalidFrameIndex, got: {other:?}"),
        }
    }

    #[test]
    fn transition_invalid_frame_returns_error() {
        let mut pool = FramePool::new(4).unwrap();
        match pool.mark_kernel_owned(fi(10)) {
            Err(ZeroGateError::InvalidFrameIndex { index: 10, .. }) => {}
            other => panic!("expected InvalidFrameIndex, got: {other:?}"),
        }
    }

    #[test]
    fn invalid_index_does_not_panic() {
        let pool = FramePool::new(4).unwrap();
        let _ = pool.state(fi(u32::MAX));
    }

    #[test]
    fn invalid_index_does_not_mutate() {
        let mut pool = FramePool::new(4).unwrap();
        let free_before = pool.free_count();
        let _ = pool.mark_kernel_owned(fi(100));
        assert_eq!(pool.free_count(), free_before);
        assert_eq!(pool.frame_count(), 4);
    }

    // =======================================================================
    // 8. Free-list invariant tests
    // =======================================================================

    #[test]
    fn fresh_pool_passes_invariant_check() {
        let pool = FramePool::new(8).unwrap();
        pool.assert_no_duplicate_ownership().unwrap();
    }

    #[test]
    fn invariant_check_after_valid_lifecycle() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(f).unwrap();
        pool.mark_rx(f).unwrap();
        pool.acquire_user(f).unwrap();
        pool.submit_tx(f).unwrap();
        pool.complete_tx(f).unwrap();
        pool.release_completion(f).unwrap();
        pool.assert_no_duplicate_ownership().unwrap();
    }

    #[test]
    fn free_list_only_contains_free_frames() {
        let mut pool = FramePool::new(4).unwrap();
        // Allocate 2 frames
        let f0 = pool.allocate_for_fill().unwrap();
        let f1 = pool.allocate_for_fill().unwrap();
        // f0, f1 are InFill; frames 2,3 are Free
        pool.assert_no_duplicate_ownership().unwrap();
        assert_eq!(pool.free_count(), 2);
        // Complete f0 through TX path back to Free
        pool.mark_kernel_owned(f0).unwrap();
        pool.mark_rx(f0).unwrap();
        pool.acquire_user(f0).unwrap();
        pool.submit_tx(f0).unwrap();
        pool.complete_tx(f0).unwrap();
        pool.release_completion(f0).unwrap();
        pool.assert_no_duplicate_ownership().unwrap();
        assert_eq!(pool.free_count(), 3);
        // f1 still InFill
        assert_eq!(pool.state(f1).unwrap(), FrameState::InFill);
    }

    #[test]
    fn no_duplicate_free_entries_after_repeated_lifecycle() {
        let mut pool = FramePool::new(4).unwrap();
        for _ in 0..3 {
            let f = pool.allocate_for_fill().unwrap();
            pool.mark_kernel_owned(f).unwrap();
            pool.mark_rx(f).unwrap();
            pool.acquire_user(f).unwrap();
            pool.submit_tx(f).unwrap();
            pool.complete_tx(f).unwrap();
            pool.release_completion(f).unwrap();
            pool.assert_no_duplicate_ownership().unwrap();
        }
    }

    // =======================================================================
    // 9. Simulation tests
    // =======================================================================

    #[test]
    fn simulate_allocate_all_half_rx_half_tx() {
        let n = 8;
        let mut pool = FramePool::new(n).unwrap();
        let mut frames = Vec::new();

        // Allocate all
        for _ in 0..n {
            frames.push(pool.allocate_for_fill().unwrap());
        }
        assert_eq!(pool.free_count(), 0);

        // Move all through to User
        for &f in &frames {
            pool.mark_kernel_owned(f).unwrap();
            pool.mark_rx(f).unwrap();
            pool.acquire_user(f).unwrap();
        }

        // First half: RX recycle path
        for &f in &frames[..n / 2] {
            pool.recycle_to_fill(f).unwrap();
            assert_eq!(pool.state(f).unwrap(), FrameState::InFill);
        }

        // Second half: TX completion path
        for &f in &frames[n / 2..] {
            pool.submit_tx(f).unwrap();
            pool.complete_tx(f).unwrap();
            pool.release_completion(f).unwrap();
            assert_eq!(pool.state(f).unwrap(), FrameState::Free);
        }

        // free_count should be n/2 (only TX path returns to Free)
        assert_eq!(pool.free_count(), n / 2);
        pool.assert_no_duplicate_ownership().unwrap();
    }

    #[test]
    fn simulate_multi_round_lifecycle() {
        let n = 4;
        let mut pool = FramePool::new(n).unwrap();

        for round in 0..5 {
            let available = pool.free_count();
            let mut round_frames = Vec::new();

            // Allocate all available
            for _ in 0..available {
                round_frames.push(pool.allocate_for_fill().unwrap());
            }

            // Full TX lifecycle
            for &f in &round_frames {
                pool.mark_kernel_owned(f).unwrap();
                pool.mark_rx(f).unwrap();
                pool.acquire_user(f).unwrap();
                pool.submit_tx(f).unwrap();
                pool.complete_tx(f).unwrap();
                pool.release_completion(f).unwrap();
            }

            assert_eq!(
                pool.free_count(),
                n,
                "round {round}: all frames should be free"
            );
            pool.assert_no_duplicate_ownership().unwrap();
        }
    }

    #[test]
    fn simulate_mixed_recycle_and_tx_multiple_rounds() {
        let n = 8;
        let mut pool = FramePool::new(n).unwrap();

        for _ in 0..3 {
            let mut frames = Vec::new();
            let avail = pool.free_count();
            for _ in 0..avail {
                frames.push(pool.allocate_for_fill().unwrap());
            }

            // Move to User
            for &f in &frames {
                pool.mark_kernel_owned(f).unwrap();
                pool.mark_rx(f).unwrap();
                pool.acquire_user(f).unwrap();
            }

            // Odd-indexed: recycle to fill then complete through TX
            // Even-indexed: straight TX
            for (i, &f) in frames.iter().enumerate() {
                if i % 2 == 1 {
                    pool.recycle_to_fill(f).unwrap();
                    pool.mark_kernel_owned(f).unwrap();
                    pool.mark_rx(f).unwrap();
                    pool.acquire_user(f).unwrap();
                }
                pool.submit_tx(f).unwrap();
                pool.complete_tx(f).unwrap();
                pool.release_completion(f).unwrap();
            }

            assert_eq!(pool.free_count(), n);
            pool.assert_no_duplicate_ownership().unwrap();
        }
    }

    // =======================================================================
    // 10. Error message quality
    // =======================================================================

    #[test]
    fn error_messages_contain_context() {
        let pool = FramePool::new(4).unwrap();
        let err = pool.state(fi(10)).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("10"), "error should mention the index");

        let mut pool2 = FramePool::new(2).unwrap();
        pool2.allocate_for_fill().unwrap();
        pool2.allocate_for_fill().unwrap();
        let err2 = pool2.allocate_for_fill().unwrap_err();
        let msg2 = format!("{err2}");
        assert!(
            msg2.contains("2"),
            "exhausted error should mention frame_count"
        );
    }

    #[test]
    fn invalid_transition_error_contains_states() {
        let mut pool = FramePool::new(4).unwrap();
        let f = pool.allocate_for_fill().unwrap();
        let err = pool.mark_rx(f).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("InFill"), "should mention current state");
        assert!(msg.contains("Rx"), "should mention attempted state");
    }
}
