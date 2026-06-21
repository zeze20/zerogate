// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Formal model of packet parser offset bounds.
//!
//! Pure model — no I/O, no unsafe, no syscalls.
//!
//! ## Invariants to prove
//!
//! 1. A read at `offset` of `size` bytes is valid only when
//!    `offset + size <= packet_len`.
//! 2. Parser never accesses memory beyond packet bounds.
//! 3. Packet decision is deterministic for same input metadata and policy.

use zerogate_common::constants;

/// Models whether a read of `size` bytes at `offset` is valid
/// for a packet of `packet_len` bytes.
///
/// # Verus spec
/// ```verus
/// ensures |result: bool|
///     result <==> offset + size <= packet_len
/// ```
pub fn read_valid(packet_len: usize, offset: usize, size: usize) -> bool {
    match offset.checked_add(size) {
        Some(end) => end <= packet_len,
        None => false,
    }
}

/// Minimum packet length for ZeroGate protocol:
/// Eth(14) + IPv4(20) + UDP(8) + ChunkHdr(24) = 66.
pub const MIN_ZEROGATE_PACKET: usize = constants::ETH_HDR_LEN
    + constants::IPV4_MIN_HDR_LEN
    + constants::UDP_HDR_LEN
    + zerogate_common::CHUNK_HDR_SIZE;

/// Models the parser offset progression through a packet.
pub struct ParserOffsetModel {
    pub packet_len: usize,
    pub current_offset: usize,
}

impl ParserOffsetModel {
    pub fn new(packet_len: usize) -> Self {
        Self {
            packet_len,
            current_offset: 0,
        }
    }

    /// Attempts to advance the offset by `size` bytes.
    /// Returns true if the read is valid, advancing the offset.
    /// Returns false if the read would exceed packet bounds (no change).
    pub fn try_advance(&mut self, size: usize) -> bool {
        if read_valid(self.packet_len, self.current_offset, size) {
            self.current_offset += size;
            true
        } else {
            false
        }
    }

    /// Returns the current offset.
    pub fn offset(&self) -> usize {
        self.current_offset
    }

    /// Returns the remaining bytes.
    pub fn remaining(&self) -> usize {
        self.packet_len.saturating_sub(self.current_offset)
    }
}

/// Deterministic decision for same input.
///
/// Given the same packet metadata and policy, the decision is always the same.
/// This models the requirement that the packet pipeline is a pure function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelDecision {
    Pass,
    Drop,
    Redirect { queue_id: u32 },
}

/// Pure packet decision function.
///
/// # Verus spec
/// ```verus
/// ensures
///     forall |m1: PacketMetaModel, m2: PacketMetaModel, p1: PolicyModel, p2: PolicyModel|
///         m1 == m2 && p1 == p2 ==> decide(m1, p1) == decide(m2, p2)
/// ```
pub fn decide(is_admitted: bool, has_policy: bool, policy_action: u8) -> ModelDecision {
    if has_policy {
        match policy_action {
            constants::POLICY_DROP_VAL => ModelDecision::Drop,
            constants::POLICY_REDIRECT_VAL => ModelDecision::Redirect { queue_id: 0 },
            _ => ModelDecision::Pass,
        }
    } else if is_admitted {
        ModelDecision::Redirect { queue_id: 0 }
    } else {
        ModelDecision::Drop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_valid_within_bounds() {
        assert!(read_valid(100, 0, 14));
        assert!(read_valid(100, 14, 20));
        assert!(read_valid(100, 34, 8));
        assert!(read_valid(100, 42, 24));
    }

    #[test]
    fn read_valid_at_boundary() {
        assert!(read_valid(66, 42, 24)); // exactly at end
        assert!(!read_valid(65, 42, 24)); // one byte short
    }

    #[test]
    fn read_valid_overflow() {
        assert!(!read_valid(100, usize::MAX, 1));
        assert!(!read_valid(100, 1, usize::MAX));
    }

    #[test]
    fn parser_offset_progression() {
        let mut p = ParserOffsetModel::new(66);
        assert!(p.try_advance(14)); // Eth
        assert_eq!(p.offset(), 14);
        assert!(p.try_advance(20)); // IPv4
        assert_eq!(p.offset(), 34);
        assert!(p.try_advance(8)); // UDP
        assert_eq!(p.offset(), 42);
        assert!(p.try_advance(24)); // ChunkHdr
        assert_eq!(p.offset(), 66);
        assert!(!p.try_advance(1)); // no more room
    }

    #[test]
    fn parser_short_packet_rejected() {
        let mut p = ParserOffsetModel::new(10);
        assert!(!p.try_advance(14)); // can't read Eth
    }

    #[test]
    fn decision_determinism() {
        let d1 = decide(true, false, 0);
        let d2 = decide(true, false, 0);
        assert_eq!(d1, d2);

        let d3 = decide(false, true, constants::POLICY_DROP_VAL);
        let d4 = decide(false, true, constants::POLICY_DROP_VAL);
        assert_eq!(d3, d4);
        assert_eq!(d3, ModelDecision::Drop);
    }

    #[test]
    fn decision_policy_takes_precedence() {
        let d = decide(true, true, constants::POLICY_DROP_VAL);
        assert_eq!(d, ModelDecision::Drop);

        let d = decide(true, true, constants::POLICY_REDIRECT_VAL);
        assert_eq!(d, ModelDecision::Redirect { queue_id: 0 });
    }

    #[test]
    fn decision_no_policy_admitted_redirects() {
        let d = decide(true, false, 0);
        assert_eq!(d, ModelDecision::Redirect { queue_id: 0 });
    }

    #[test]
    fn decision_no_policy_not_admitted_drops() {
        let d = decide(false, false, 0);
        assert_eq!(d, ModelDecision::Drop);
    }
}
