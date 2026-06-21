// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Formal model of the UMEM frame ownership state machine.
//!
//! This module is a pure model — no I/O, no unsafe, no syscalls.
//! It mirrors the runtime `frame_pool::FrameState` and `valid_transition`
//! using the same naming and semantics.
//!
//! ## Invariants to prove
//!
//! 1. Only legal transitions are accepted.
//! 2. No frame has two owners (single-state per frame).
//! 3. A frame may appear in exactly one ring at a time.

/// Ownership state of a UMEM frame (model).
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

/// Returns true iff `(from, to)` is a legal frame state transition.
///
/// Legal transitions:
/// ```text
///   Free -> InFill
///   InFill -> Kernel
///   Kernel -> Rx
///   Rx -> User
///   User -> InFill
///   User -> Tx
///   Tx -> Completion
///   Completion -> Free
/// ```
///
/// # Verus spec
/// ```verus
/// ensures |result: bool|
///     result <==> (
///         (from == Free       && to == InFill)     ||
///         (from == InFill     && to == Kernel)     ||
///         (from == Kernel     && to == Rx)          ||
///         (from == Rx         && to == User)        ||
///         (from == User       && to == InFill)     ||
///         (from == User       && to == Tx)          ||
///         (from == Tx         && to == Completion)  ||
///         (from == Completion && to == Free)
///     )
/// ```
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

/// Model of a frame pool with N frames.
///
/// Each frame is in exactly one state at a time.
pub struct FramePoolModel {
    states: Vec<FrameState>,
}

impl FramePoolModel {
    /// Creates a pool where all frames are `Free`.
    pub fn new(n: usize) -> Self {
        Self {
            states: vec![FrameState::Free; n],
        }
    }

    /// Returns the number of frames.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Returns the state of frame `i`.
    pub fn state(&self, i: usize) -> Option<FrameState> {
        self.states.get(i).copied()
    }

    /// Attempts a transition on frame `i`.
    ///
    /// Returns `Ok(())` if `valid_transition(current, to)` holds.
    /// Returns `Err` otherwise (no state change).
    pub fn transition(&mut self, i: usize, to: FrameState) -> Result<(), ()> {
        let current = *self.states.get(i).ok_or(())?;
        if valid_transition(current, to) {
            self.states[i] = to;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Counts frames in a given state.
    pub fn count_in_state(&self, state: FrameState) -> usize {
        self.states.iter().filter(|&&s| s == state).count()
    }

    /// Checks that no frame is in two states simultaneously.
    ///
    /// This is trivially true by construction (each slot holds one value),
    /// but we model it explicitly for the formal proof.
    pub fn no_double_ownership(&self) -> bool {
        // Each slot has exactly one FrameState — no frame appears twice
        // in the states vector. True by construction.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_legal_transitions() {
        let legal = [
            (FrameState::Free, FrameState::InFill),
            (FrameState::InFill, FrameState::Kernel),
            (FrameState::Kernel, FrameState::Rx),
            (FrameState::Rx, FrameState::User),
            (FrameState::User, FrameState::InFill),
            (FrameState::User, FrameState::Tx),
            (FrameState::Tx, FrameState::Completion),
            (FrameState::Completion, FrameState::Free),
        ];
        for (from, to) in legal {
            assert!(
                valid_transition(from, to),
                "{from:?} -> {to:?} should be legal"
            );
        }
    }

    #[test]
    fn all_illegal_transitions() {
        let all_states = [
            FrameState::Free,
            FrameState::InFill,
            FrameState::Kernel,
            FrameState::Rx,
            FrameState::User,
            FrameState::Tx,
            FrameState::Completion,
        ];
        // Count: should be exactly 8 legal transitions.
        let mut legal_count = 0;
        for &from in &all_states {
            for &to in &all_states {
                if valid_transition(from, to) {
                    legal_count += 1;
                }
            }
        }
        assert_eq!(legal_count, 8, "exactly 8 legal transitions");
    }

    #[test]
    fn model_full_rx_cycle() {
        let mut pool = FramePoolModel::new(4);

        // Frame 0: Free -> InFill -> Kernel -> Rx -> User -> InFill -> Kernel
        assert!(pool.transition(0, FrameState::InFill).is_ok());
        assert!(pool.transition(0, FrameState::Kernel).is_ok());
        assert!(pool.transition(0, FrameState::Rx).is_ok());
        assert!(pool.transition(0, FrameState::User).is_ok());
        assert!(pool.transition(0, FrameState::InFill).is_ok());
        assert!(pool.transition(0, FrameState::Kernel).is_ok());
    }

    #[test]
    fn model_full_tx_cycle() {
        let mut pool = FramePoolModel::new(4);

        // Free -> InFill -> Kernel -> Rx -> User -> Tx -> Completion -> Free
        assert!(pool.transition(0, FrameState::InFill).is_ok());
        assert!(pool.transition(0, FrameState::Kernel).is_ok());
        assert!(pool.transition(0, FrameState::Rx).is_ok());
        assert!(pool.transition(0, FrameState::User).is_ok());
        assert!(pool.transition(0, FrameState::Tx).is_ok());
        assert!(pool.transition(0, FrameState::Completion).is_ok());
        assert!(pool.transition(0, FrameState::Free).is_ok());
        assert_eq!(pool.state(0).unwrap(), FrameState::Free);
    }

    #[test]
    fn model_illegal_transition_rejected() {
        let mut pool = FramePoolModel::new(4);
        // Free -> Kernel should fail (must go through InFill first).
        assert!(pool.transition(0, FrameState::Kernel).is_err());
        assert_eq!(pool.state(0).unwrap(), FrameState::Free);
    }

    #[test]
    fn model_no_double_ownership() {
        let pool = FramePoolModel::new(8);
        assert!(pool.no_double_ownership());
    }

    #[test]
    fn model_all_frames_return_to_free() {
        let n = 8;
        let mut pool = FramePoolModel::new(n);

        for i in 0..n {
            pool.transition(i, FrameState::InFill).unwrap();
            pool.transition(i, FrameState::Kernel).unwrap();
            pool.transition(i, FrameState::Rx).unwrap();
            pool.transition(i, FrameState::User).unwrap();
            pool.transition(i, FrameState::Tx).unwrap();
            pool.transition(i, FrameState::Completion).unwrap();
            pool.transition(i, FrameState::Free).unwrap();
        }

        assert_eq!(pool.count_in_state(FrameState::Free), n);
    }
}
