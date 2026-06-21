// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Frame ownership state machine for UMEM frames.
//!
//! Implements the mandatory ownership states and legal transitions:
//!
//! ```text
//!   Free -> InFill
//!   InFill -> Kernel
//!   Kernel -> Rx
//!   Rx -> User
//!   User -> InFill
//!   User -> Tx
//!   Tx -> Completion
//!   Completion -> Free
//! ```
//!
//! No `unsafe` in this file. Frame identity is by index, not raw pointer.

use crate::error::FramePoolError;

/// Ownership state of a single UMEM frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameState {
    Free,
    InFill,
    Kernel,
    Rx,
    User,
    Tx,
    Completion,
}

impl FrameState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::InFill => "InFill",
            Self::Kernel => "Kernel",
            Self::Rx => "Rx",
            Self::User => "User",
            Self::Tx => "Tx",
            Self::Completion => "Completion",
        }
    }
}

/// Checks whether a state transition is legal.
pub fn valid_transition(from: FrameState, to: FrameState) -> bool {
    matches!(
        (from, to),
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

/// Owns and tracks the state of every UMEM frame.
///
/// A frame may exist in exactly one state at a time.
/// All transitions are validated; illegal transitions return errors.
pub struct FramePool {
    states: Vec<FrameState>,
    free_list: Vec<u32>,
    frame_size: u32,
}

impl FramePool {
    /// Creates a new frame pool. All frames start in `Free` state.
    pub fn new(frame_count: u32, frame_size: u32) -> Self {
        let states = vec![FrameState::Free; frame_count as usize];
        let free_list: Vec<u32> = (0..frame_count).collect();
        Self {
            states,
            free_list,
            frame_size,
        }
    }

    /// Total number of frames in the pool.
    pub fn capacity(&self) -> u32 {
        self.states.len() as u32
    }

    /// Number of frames currently in `Free` state.
    pub fn free_count(&self) -> usize {
        self.free_list.len()
    }

    /// Returns the frame size.
    pub fn frame_size(&self) -> u32 {
        self.frame_size
    }

    /// Returns the current state of a frame.
    pub fn state(&self, frame_index: u32) -> Result<FrameState, FramePoolError> {
        self.states
            .get(frame_index as usize)
            .copied()
            .ok_or(FramePoolError::OutOfBounds {
                frame_index,
                max: self.capacity(),
            })
    }

    /// Performs a validated state transition.
    fn transition(
        &mut self,
        frame_index: u32,
        expected_from: FrameState,
        to: FrameState,
    ) -> Result<(), FramePoolError> {
        let idx = frame_index as usize;
        let current = *self.states.get(idx).ok_or(FramePoolError::OutOfBounds {
            frame_index,
            max: self.capacity(),
        })?;

        if current != expected_from {
            return Err(FramePoolError::IllegalTransition {
                frame_index,
                from: current.as_str(),
                to: to.as_str(),
            });
        }

        debug_assert!(
            valid_transition(expected_from, to),
            "BUG: invalid transition {expected_from:?} -> {to:?} for frame {frame_index}"
        );

        if !valid_transition(expected_from, to) {
            return Err(FramePoolError::IllegalTransition {
                frame_index,
                from: expected_from.as_str(),
                to: to.as_str(),
            });
        }

        self.states[idx] = to;
        Ok(())
    }

    // ----- Public transition API -----

    /// Allocates a free frame for the fill ring.
    /// Transition: Free -> InFill.
    /// Returns the frame index and its UMEM byte offset.
    pub fn allocate_for_fill(&mut self) -> Result<(u32, u64), FramePoolError> {
        let frame_index = self.free_list.pop().ok_or(FramePoolError::PoolExhausted)?;
        self.transition(frame_index, FrameState::Free, FrameState::InFill)?;
        let offset = frame_index as u64 * self.frame_size as u64;
        Ok((frame_index, offset))
    }

    /// Marks a frame as owned by the kernel (submitted to fill ring).
    /// Transition: InFill -> Kernel.
    pub fn mark_kernel_owned(&mut self, frame_index: u32) -> Result<(), FramePoolError> {
        self.transition(frame_index, FrameState::InFill, FrameState::Kernel)
    }

    /// Marks a frame as received from the kernel (appeared in RX ring).
    /// Transition: Kernel -> Rx.
    pub fn mark_rx(&mut self, frame_index: u32) -> Result<(), FramePoolError> {
        self.transition(frame_index, FrameState::Kernel, FrameState::Rx)
    }

    /// Acquires a frame for user processing.
    /// Transition: Rx -> User.
    pub fn acquire_user(&mut self, frame_index: u32) -> Result<(), FramePoolError> {
        self.transition(frame_index, FrameState::Rx, FrameState::User)
    }

    /// Recycles a user-held frame back to the fill ring.
    /// Transition: User -> InFill.
    pub fn recycle_to_fill(&mut self, frame_index: u32) -> Result<u64, FramePoolError> {
        self.transition(frame_index, FrameState::User, FrameState::InFill)?;
        let offset = frame_index as u64 * self.frame_size as u64;
        Ok(offset)
    }

    /// Submits a user-held frame for transmission.
    /// Transition: User -> Tx.
    pub fn submit_tx(&mut self, frame_index: u32) -> Result<(), FramePoolError> {
        self.transition(frame_index, FrameState::User, FrameState::Tx)
    }

    /// Completes a transmitted frame (appeared in completion ring).
    /// Transition: Tx -> Completion.
    pub fn complete_tx(&mut self, frame_index: u32) -> Result<(), FramePoolError> {
        self.transition(frame_index, FrameState::Tx, FrameState::Completion)
    }

    /// Frees a completed frame back to the free pool.
    /// Transition: Completion -> Free.
    pub fn free_completed(&mut self, frame_index: u32) -> Result<(), FramePoolError> {
        self.transition(frame_index, FrameState::Completion, FrameState::Free)?;
        self.free_list.push(frame_index);
        Ok(())
    }

    /// Converts a UMEM byte offset to a frame index.
    pub fn offset_to_index(&self, offset: u64) -> u32 {
        (offset / self.frame_size as u64) as u32
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_transitions_accepted() {
        assert!(valid_transition(FrameState::Free, FrameState::InFill));
        assert!(valid_transition(FrameState::InFill, FrameState::Kernel));
        assert!(valid_transition(FrameState::Kernel, FrameState::Rx));
        assert!(valid_transition(FrameState::Rx, FrameState::User));
        assert!(valid_transition(FrameState::User, FrameState::InFill));
        assert!(valid_transition(FrameState::User, FrameState::Tx));
        assert!(valid_transition(FrameState::Tx, FrameState::Completion));
        assert!(valid_transition(FrameState::Completion, FrameState::Free));
    }

    #[test]
    fn invalid_transitions_rejected() {
        assert!(!valid_transition(FrameState::Free, FrameState::Kernel));
        assert!(!valid_transition(FrameState::Free, FrameState::Rx));
        assert!(!valid_transition(FrameState::Free, FrameState::User));
        assert!(!valid_transition(FrameState::Free, FrameState::Tx));
        assert!(!valid_transition(FrameState::Free, FrameState::Completion));
        assert!(!valid_transition(FrameState::Free, FrameState::Free));
        assert!(!valid_transition(FrameState::InFill, FrameState::Free));
        assert!(!valid_transition(FrameState::InFill, FrameState::Rx));
        assert!(!valid_transition(FrameState::Kernel, FrameState::Free));
        assert!(!valid_transition(FrameState::Kernel, FrameState::Tx));
        assert!(!valid_transition(FrameState::Rx, FrameState::Free));
        assert!(!valid_transition(FrameState::Rx, FrameState::Tx));
        assert!(!valid_transition(FrameState::Tx, FrameState::Free));
        assert!(!valid_transition(FrameState::Tx, FrameState::User));
        assert!(!valid_transition(FrameState::Completion, FrameState::User));
    }

    #[test]
    fn full_rx_lifecycle() {
        let mut pool = FramePool::new(4, 4096);
        assert_eq!(pool.free_count(), 4);

        // Free -> InFill
        let (idx, offset) = pool.allocate_for_fill().unwrap();
        assert_eq!(offset, idx as u64 * 4096);
        assert_eq!(pool.state(idx).unwrap(), FrameState::InFill);

        // InFill -> Kernel
        pool.mark_kernel_owned(idx).unwrap();
        assert_eq!(pool.state(idx).unwrap(), FrameState::Kernel);

        // Kernel -> Rx
        pool.mark_rx(idx).unwrap();
        assert_eq!(pool.state(idx).unwrap(), FrameState::Rx);

        // Rx -> User
        pool.acquire_user(idx).unwrap();
        assert_eq!(pool.state(idx).unwrap(), FrameState::User);

        // User -> InFill (recycle without TX)
        let _offset = pool.recycle_to_fill(idx).unwrap();
        assert_eq!(pool.state(idx).unwrap(), FrameState::InFill);
    }

    #[test]
    fn full_tx_lifecycle() {
        let mut pool = FramePool::new(4, 4096);

        let (idx, _) = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(idx).unwrap();
        pool.mark_rx(idx).unwrap();
        pool.acquire_user(idx).unwrap();

        // User -> Tx
        pool.submit_tx(idx).unwrap();
        assert_eq!(pool.state(idx).unwrap(), FrameState::Tx);

        // Tx -> Completion
        pool.complete_tx(idx).unwrap();
        assert_eq!(pool.state(idx).unwrap(), FrameState::Completion);

        // Completion -> Free
        pool.free_completed(idx).unwrap();
        assert_eq!(pool.state(idx).unwrap(), FrameState::Free);
    }

    #[test]
    fn illegal_transition_returns_error() {
        let mut pool = FramePool::new(4, 4096);
        let (idx, _) = pool.allocate_for_fill().unwrap();

        // InFill -> User is not legal
        let result = pool.acquire_user(idx);
        assert!(result.is_err());
        match result.unwrap_err() {
            FramePoolError::IllegalTransition { frame_index, .. } => {
                assert_eq!(frame_index, idx);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn pool_exhaustion() {
        let mut pool = FramePool::new(2, 4096);
        pool.allocate_for_fill().unwrap();
        pool.allocate_for_fill().unwrap();
        assert!(matches!(
            pool.allocate_for_fill(),
            Err(FramePoolError::PoolExhausted)
        ));
    }

    #[test]
    fn out_of_bounds_frame() {
        let pool = FramePool::new(4, 4096);
        assert!(matches!(
            pool.state(99),
            Err(FramePoolError::OutOfBounds { .. })
        ));
    }

    #[test]
    fn no_double_ownership() {
        let mut pool = FramePool::new(4, 4096);
        let (idx, _) = pool.allocate_for_fill().unwrap();
        pool.mark_kernel_owned(idx).unwrap();

        // Cannot mark_kernel_owned again (already Kernel, not InFill)
        assert!(pool.mark_kernel_owned(idx).is_err());
    }

    #[test]
    fn offset_to_index_conversion() {
        let pool = FramePool::new(8, 4096);
        assert_eq!(pool.offset_to_index(0), 0);
        assert_eq!(pool.offset_to_index(4096), 1);
        assert_eq!(pool.offset_to_index(8192), 2);
    }

    #[test]
    fn full_cycle_all_frames_return_to_free() {
        let n = 8u32;
        let mut pool = FramePool::new(n, 4096);

        let mut indices = Vec::new();
        for _ in 0..n {
            let (idx, _) = pool.allocate_for_fill().unwrap();
            indices.push(idx);
        }
        assert_eq!(pool.free_count(), 0);

        for &idx in &indices {
            pool.mark_kernel_owned(idx).unwrap();
            pool.mark_rx(idx).unwrap();
            pool.acquire_user(idx).unwrap();
            pool.submit_tx(idx).unwrap();
            pool.complete_tx(idx).unwrap();
            pool.free_completed(idx).unwrap();
        }

        assert_eq!(pool.free_count(), n as usize);
        for &idx in &indices {
            assert_eq!(pool.state(idx).unwrap(), FrameState::Free);
        }
    }
}
