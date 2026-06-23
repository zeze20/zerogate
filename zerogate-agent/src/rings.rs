//! AF_XDP ring descriptor validation, ring traits, and fake test implementations.
//!
//! This module defines:
//! - `RingDesc` — a UMEM-offset-based descriptor for AF_XDP ring operations.
//! - `RingConfig` — validation for ring sizes (FILL, RX, TX, COMPLETION).
//! - Safe ring traits (`FillRing`, `RxRing`, `TxRing`, `CompletionRing`).
//! - Fake/test ring implementations for deterministic unit tests.
//!
//! **MR9 scope:** scaffold only. No real kernel ring mmap, no XSK_MAP,
//! no queue loop. Fake rings are test-only and do not claim kernel success.
//!
//! **Safety invariants:**
//! - Descriptors are UMEM offsets, not raw pointers.
//! - All descriptors are validated against UMEM before use.
//! - Ring capacity is enforced.
//! - No unsafe code.

use std::collections::VecDeque;

use zerogate_common::abi::{FrameIndex, UmemAddr};

use crate::error::ZeroGateError;
use crate::umem::UmemRegion;

// ---------------------------------------------------------------------------
// RingDesc
// ---------------------------------------------------------------------------

/// A ring descriptor representing a frame in UMEM.
///
/// `addr` is a UMEM byte offset (not a raw pointer).
/// `len` is the packet/data length.
/// `options` is reserved for future use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub struct RingDesc {
    pub addr: UmemAddr,
    pub len: u32,
    pub options: u32,
}

#[allow(dead_code)]
impl RingDesc {
    /// Create a new ring descriptor.
    pub fn new(addr: UmemAddr, len: u32, options: u32) -> Self {
        Self { addr, len, options }
    }

    /// Validate this descriptor against a UMEM region.
    ///
    /// Checks:
    /// - `addr` is within UMEM bounds.
    /// - `addr` is frame-aligned.
    /// - `len > 0` (strict descriptor policy).
    /// - `len <= frame_size`.
    pub fn validate_for_umem(&self, umem: &UmemRegion) -> Result<(), ZeroGateError> {
        // Validate addr within UMEM (bounds + alignment).
        umem.validate_offset(self.addr)?;

        // len must be greater than 0.
        if self.len == 0 {
            return Err(ZeroGateError::DescriptorZeroLength);
        }

        // len must not exceed frame_size.
        let frame_size = umem.config().frame_size;
        if self.len > frame_size {
            return Err(ZeroGateError::DescriptorTooLarge {
                len: self.len,
                frame_size,
            });
        }

        Ok(())
    }

    /// Convert descriptor addr to a FrameIndex using UMEM validation.
    pub fn frame_index(&self, umem: &UmemRegion) -> Result<FrameIndex, ZeroGateError> {
        umem.frame_index_from_offset(self.addr)
    }
}

// ---------------------------------------------------------------------------
// RingConfig
// ---------------------------------------------------------------------------

/// Configuration for AF_XDP ring sizes.
///
/// All sizes must be greater than zero and powers of two.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct RingConfig {
    pub fill_size: u32,
    pub rx_size: u32,
    pub tx_size: u32,
    pub completion_size: u32,
}

#[allow(dead_code)]
impl RingConfig {
    /// Validate all ring size invariants.
    pub fn validate(&self) -> Result<(), ZeroGateError> {
        Self::validate_size(self.fill_size, "fill_size")?;
        Self::validate_size(self.rx_size, "rx_size")?;
        Self::validate_size(self.tx_size, "tx_size")?;
        Self::validate_size(self.completion_size, "completion_size")?;
        Ok(())
    }

    fn validate_size(size: u32, name: &str) -> Result<(), ZeroGateError> {
        if size == 0 {
            return Err(ZeroGateError::InvalidRingConfig(format!(
                "{name} must be greater than 0"
            )));
        }
        if !size.is_power_of_two() {
            return Err(ZeroGateError::InvalidRingConfig(format!(
                "{name} must be a power of two"
            )));
        }
        Ok(())
    }
}

impl Default for RingConfig {
    fn default() -> Self {
        Self {
            fill_size: 2048,
            rx_size: 2048,
            tx_size: 2048,
            completion_size: 2048,
        }
    }
}

// ---------------------------------------------------------------------------
// Ring Traits
// ---------------------------------------------------------------------------

/// Trait for a FILL ring — userspace provides empty frame addresses to kernel.
#[allow(dead_code)]
pub trait FillRing {
    fn capacity(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn reserve(&mut self, count: u32) -> Result<(), ZeroGateError>;
    fn submit(&mut self, desc: RingDesc) -> Result<(), ZeroGateError>;
}

/// Trait for an RX ring — kernel returns received packet descriptors.
#[allow(dead_code)]
pub trait RxRing {
    fn capacity(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn peek(&mut self) -> Result<Option<RingDesc>, ZeroGateError>;
    fn release(&mut self, count: u32) -> Result<(), ZeroGateError>;
}

/// Trait for a TX ring — userspace submits frames for transmission.
#[allow(dead_code)]
pub trait TxRing {
    fn capacity(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn reserve(&mut self, count: u32) -> Result<(), ZeroGateError>;
    fn submit(&mut self, desc: RingDesc) -> Result<(), ZeroGateError>;
}

/// Trait for a COMPLETION ring — kernel returns transmitted frames.
#[allow(dead_code)]
pub trait CompletionRing {
    fn capacity(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn peek(&mut self) -> Result<Option<RingDesc>, ZeroGateError>;
    fn release(&mut self, count: u32) -> Result<(), ZeroGateError>;
}

// ---------------------------------------------------------------------------
// Fake Test Rings
// ---------------------------------------------------------------------------

/// Fake FILL ring for unit tests. Test-only — does not model kernel ring memory.
#[allow(dead_code)]
pub struct FakeFillRing {
    cap: usize,
    entries: VecDeque<RingDesc>,
}

#[allow(dead_code)]
impl FakeFillRing {
    pub fn new(capacity: u32) -> Self {
        Self {
            cap: capacity as usize,
            entries: VecDeque::new(),
        }
    }
}

impl FillRing for FakeFillRing {
    fn capacity(&self) -> usize {
        self.cap
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn reserve(&mut self, count: u32) -> Result<(), ZeroGateError> {
        let available = self.cap.saturating_sub(self.entries.len());
        if (count as usize) > available {
            return Err(ZeroGateError::RingFull(format!(
                "FILL ring: requested {count}, available {available}"
            )));
        }
        Ok(())
    }

    fn submit(&mut self, desc: RingDesc) -> Result<(), ZeroGateError> {
        if self.entries.len() >= self.cap {
            return Err(ZeroGateError::RingFull(
                "FILL ring is at capacity".to_string(),
            ));
        }
        self.entries.push_back(desc);
        Ok(())
    }
}

/// Fake RX ring for unit tests. Test-only.
#[allow(dead_code)]
pub struct FakeRxRing {
    cap: usize,
    entries: VecDeque<RingDesc>,
}

#[allow(dead_code)]
impl FakeRxRing {
    pub fn new(capacity: u32) -> Self {
        Self {
            cap: capacity as usize,
            entries: VecDeque::new(),
        }
    }

    /// Simulate kernel producing a received descriptor.
    pub fn produce(&mut self, desc: RingDesc) -> Result<(), ZeroGateError> {
        if self.entries.len() >= self.cap {
            return Err(ZeroGateError::RingFull(
                "RX ring is at capacity".to_string(),
            ));
        }
        self.entries.push_back(desc);
        Ok(())
    }
}

impl RxRing for FakeRxRing {
    fn capacity(&self) -> usize {
        self.cap
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn peek(&mut self) -> Result<Option<RingDesc>, ZeroGateError> {
        Ok(self.entries.front().copied())
    }

    fn release(&mut self, count: u32) -> Result<(), ZeroGateError> {
        let count = count as usize;
        if count > self.entries.len() {
            return Err(ZeroGateError::RingReleaseFailed(format!(
                "RX ring: requested release {count}, available {}",
                self.entries.len()
            )));
        }
        for _ in 0..count {
            self.entries.pop_front();
        }
        Ok(())
    }
}

/// Fake TX ring for unit tests. Test-only.
#[allow(dead_code)]
pub struct FakeTxRing {
    cap: usize,
    entries: VecDeque<RingDesc>,
}

#[allow(dead_code)]
impl FakeTxRing {
    pub fn new(capacity: u32) -> Self {
        Self {
            cap: capacity as usize,
            entries: VecDeque::new(),
        }
    }
}

impl TxRing for FakeTxRing {
    fn capacity(&self) -> usize {
        self.cap
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn reserve(&mut self, count: u32) -> Result<(), ZeroGateError> {
        let available = self.cap.saturating_sub(self.entries.len());
        if (count as usize) > available {
            return Err(ZeroGateError::RingFull(format!(
                "TX ring: requested {count}, available {available}"
            )));
        }
        Ok(())
    }

    fn submit(&mut self, desc: RingDesc) -> Result<(), ZeroGateError> {
        if self.entries.len() >= self.cap {
            return Err(ZeroGateError::RingFull(
                "TX ring is at capacity".to_string(),
            ));
        }
        self.entries.push_back(desc);
        Ok(())
    }
}

/// Fake COMPLETION ring for unit tests. Test-only.
#[allow(dead_code)]
pub struct FakeCompletionRing {
    cap: usize,
    entries: VecDeque<RingDesc>,
}

#[allow(dead_code)]
impl FakeCompletionRing {
    pub fn new(capacity: u32) -> Self {
        Self {
            cap: capacity as usize,
            entries: VecDeque::new(),
        }
    }

    /// Simulate kernel completing a transmitted descriptor.
    pub fn produce(&mut self, desc: RingDesc) -> Result<(), ZeroGateError> {
        if self.entries.len() >= self.cap {
            return Err(ZeroGateError::RingFull(
                "COMPLETION ring is at capacity".to_string(),
            ));
        }
        self.entries.push_back(desc);
        Ok(())
    }
}

impl CompletionRing for FakeCompletionRing {
    fn capacity(&self) -> usize {
        self.cap
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn peek(&mut self) -> Result<Option<RingDesc>, ZeroGateError> {
        Ok(self.entries.front().copied())
    }

    fn release(&mut self, count: u32) -> Result<(), ZeroGateError> {
        let count = count as usize;
        if count > self.entries.len() {
            return Err(ZeroGateError::RingReleaseFailed(format!(
                "COMPLETION ring: requested release {count}, available {}",
                self.entries.len()
            )));
        }
        for _ in 0..count {
            self.entries.pop_front();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::umem::UmemConfig;

    fn default_umem() -> UmemRegion {
        let config = UmemConfig {
            frame_count: 16,
            frame_size: 4096,
            headroom: 0,
        };
        UmemRegion::allocate(config).unwrap()
    }

    fn valid_ring_config() -> RingConfig {
        RingConfig {
            fill_size: 2048,
            rx_size: 2048,
            tx_size: 2048,
            completion_size: 2048,
        }
    }

    // --- RingConfig tests ---

    #[test]
    fn valid_ring_config_passes() {
        assert!(valid_ring_config().validate().is_ok());
    }

    #[test]
    fn default_ring_config_valid() {
        assert!(RingConfig::default().validate().is_ok());
    }

    #[test]
    fn zero_fill_size_rejected() {
        let mut cfg = valid_ring_config();
        cfg.fill_size = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn zero_rx_size_rejected() {
        let mut cfg = valid_ring_config();
        cfg.rx_size = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn zero_tx_size_rejected() {
        let mut cfg = valid_ring_config();
        cfg.tx_size = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn zero_completion_size_rejected() {
        let mut cfg = valid_ring_config();
        cfg.completion_size = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn non_power_of_two_fill_size_rejected() {
        let mut cfg = valid_ring_config();
        cfg.fill_size = 3;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn non_power_of_two_rx_size_rejected() {
        let mut cfg = valid_ring_config();
        cfg.rx_size = 5;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn non_power_of_two_tx_size_rejected() {
        let mut cfg = valid_ring_config();
        cfg.tx_size = 6;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn non_power_of_two_completion_size_rejected() {
        let mut cfg = valid_ring_config();
        cfg.completion_size = 7;
        assert!(cfg.validate().is_err());
    }

    // --- RingDesc tests ---

    #[test]
    fn valid_descriptor_passes() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        assert!(desc.validate_for_umem(&umem).is_ok());
    }

    #[test]
    fn descriptor_at_last_frame_valid() {
        let umem = default_umem();
        // last frame is at offset (16-1)*4096 = 61440
        let desc = RingDesc::new(UmemAddr { addr: 61440 }, 100, 0);
        assert!(desc.validate_for_umem(&umem).is_ok());
    }

    #[test]
    fn descriptor_offset_outside_umem_rejected() {
        let umem = default_umem();
        // total_size = 16*4096 = 65536, use offset == total_size
        let desc = RingDesc::new(UmemAddr { addr: 65536 }, 64, 0);
        assert!(desc.validate_for_umem(&umem).is_err());
    }

    #[test]
    fn descriptor_offset_beyond_umem_rejected() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 100_000 }, 64, 0);
        assert!(desc.validate_for_umem(&umem).is_err());
    }

    #[test]
    fn descriptor_unaligned_offset_rejected() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 1 }, 64, 0);
        assert!(desc.validate_for_umem(&umem).is_err());
    }

    #[test]
    fn descriptor_zero_length_rejected() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 0, 0);
        let result = desc.validate_for_umem(&umem);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("zero length"));
    }

    #[test]
    fn descriptor_len_exceeds_frame_size_rejected() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 4097, 0);
        let result = desc.validate_for_umem(&umem);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("too large"));
    }

    #[test]
    fn descriptor_len_equals_frame_size_valid() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 4096, 0);
        assert!(desc.validate_for_umem(&umem).is_ok());
    }

    #[test]
    fn descriptor_frame_index_works() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 4096 }, 64, 0);
        let fi = desc.frame_index(&umem).unwrap();
        assert_eq!({ fi.index }, 1);
    }

    #[test]
    fn descriptor_frame_index_zero() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        let fi = desc.frame_index(&umem).unwrap();
        assert_eq!({ fi.index }, 0);
    }

    #[test]
    fn descriptor_frame_index_invalid_offset_fails() {
        let umem = default_umem();
        let desc = RingDesc::new(UmemAddr { addr: 100_000 }, 64, 0);
        assert!(desc.frame_index(&umem).is_err());
    }

    // --- FakeFillRing tests ---

    #[test]
    fn fake_fill_ring_starts_empty() {
        let ring = FakeFillRing::new(4);
        assert!(ring.is_empty());
        assert_eq!(ring.len(), 0);
        assert_eq!(ring.capacity(), 4);
    }

    #[test]
    fn fake_fill_ring_submit_increases_len() {
        let mut ring = FakeFillRing::new(4);
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        ring.submit(desc).unwrap();
        assert_eq!(ring.len(), 1);
        assert!(!ring.is_empty());
    }

    #[test]
    fn fake_fill_ring_capacity_enforced() {
        let mut ring = FakeFillRing::new(2);
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        ring.submit(desc).unwrap();
        ring.submit(desc).unwrap();
        let result = ring.submit(desc);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("full"));
    }

    #[test]
    fn fake_fill_ring_reserve_overflow_rejected() {
        let mut ring = FakeFillRing::new(2);
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        ring.submit(desc).unwrap();
        let result = ring.reserve(2);
        assert!(result.is_err());
    }

    #[test]
    fn fake_fill_ring_reserve_within_capacity_ok() {
        let ring = FakeFillRing::new(4);
        assert!(FakeFillRing::reserve(&mut { ring }, 4).is_ok());
    }

    // --- FakeRxRing tests ---

    #[test]
    fn fake_rx_ring_starts_empty() {
        let ring = FakeRxRing::new(4);
        assert!(ring.is_empty());
        assert_eq!(ring.len(), 0);
    }

    #[test]
    fn fake_rx_ring_peek_returns_descriptor() {
        let mut ring = FakeRxRing::new(4);
        let desc = RingDesc::new(UmemAddr { addr: 4096 }, 128, 0);
        ring.produce(desc).unwrap();
        let peeked = ring.peek().unwrap();
        assert_eq!(peeked, Some(desc));
        // peek does not remove
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn fake_rx_ring_peek_empty_returns_none() {
        let mut ring = FakeRxRing::new(4);
        assert_eq!(ring.peek().unwrap(), None);
    }

    #[test]
    fn fake_rx_ring_release_removes_descriptor() {
        let mut ring = FakeRxRing::new(4);
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        ring.produce(desc).unwrap();
        ring.release(1).unwrap();
        assert!(ring.is_empty());
    }

    #[test]
    fn fake_rx_ring_release_beyond_len_fails() {
        let mut ring = FakeRxRing::new(4);
        let result = ring.release(1);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("release"));
    }

    // --- FakeTxRing tests ---

    #[test]
    fn fake_tx_ring_starts_empty() {
        let ring = FakeTxRing::new(4);
        assert!(ring.is_empty());
        assert_eq!(ring.capacity(), 4);
    }

    #[test]
    fn fake_tx_ring_submit_works() {
        let mut ring = FakeTxRing::new(4);
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        ring.submit(desc).unwrap();
        assert_eq!(ring.len(), 1);
    }

    #[test]
    fn fake_tx_ring_capacity_enforced() {
        let mut ring = FakeTxRing::new(1);
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        ring.submit(desc).unwrap();
        let result = ring.submit(desc);
        assert!(result.is_err());
    }

    #[test]
    fn fake_tx_ring_reserve_overflow_rejected() {
        let mut ring = FakeTxRing::new(2);
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        ring.submit(desc).unwrap();
        ring.submit(desc).unwrap();
        let result = ring.reserve(1);
        assert!(result.is_err());
    }

    // --- FakeCompletionRing tests ---

    #[test]
    fn fake_completion_ring_starts_empty() {
        let ring = FakeCompletionRing::new(4);
        assert!(ring.is_empty());
        assert_eq!(ring.capacity(), 4);
    }

    #[test]
    fn fake_completion_ring_peek_release_works() {
        let mut ring = FakeCompletionRing::new(4);
        let desc = RingDesc::new(UmemAddr { addr: 8192 }, 256, 0);
        ring.produce(desc).unwrap();
        let peeked = ring.peek().unwrap();
        assert_eq!(peeked, Some(desc));
        ring.release(1).unwrap();
        assert!(ring.is_empty());
    }

    #[test]
    fn fake_completion_ring_release_beyond_len_fails() {
        let mut ring = FakeCompletionRing::new(4);
        let result = ring.release(1);
        assert!(result.is_err());
    }

    #[test]
    fn fake_completion_ring_capacity_enforced() {
        let mut ring = FakeCompletionRing::new(1);
        let desc = RingDesc::new(UmemAddr { addr: 0 }, 64, 0);
        ring.produce(desc).unwrap();
        let result = ring.produce(desc);
        assert!(result.is_err());
    }

    // --- Integration-style tests ---

    #[test]
    fn integration_umem_to_fill_ring() {
        let umem = default_umem();
        let mut fill = FakeFillRing::new(16);

        // Get frame offset from UMEM, create descriptor, submit to FILL ring
        let addr = umem.frame_offset(FrameIndex { index: 0 }).unwrap();
        let desc = RingDesc::new(addr, 64, 0);
        desc.validate_for_umem(&umem).unwrap();
        fill.submit(desc).unwrap();
        assert_eq!(fill.len(), 1);
    }

    #[test]
    fn integration_tx_submit_completion_drain() {
        let umem = default_umem();
        let mut tx = FakeTxRing::new(16);
        let mut comp = FakeCompletionRing::new(16);

        // Submit a descriptor to TX
        let addr = umem.frame_offset(FrameIndex { index: 3 }).unwrap();
        let desc = RingDesc::new(addr, 1500, 0);
        desc.validate_for_umem(&umem).unwrap();
        tx.submit(desc).unwrap();

        // Simulate kernel completion
        comp.produce(desc).unwrap();
        let completed = comp.peek().unwrap().unwrap();
        assert_eq!({ completed.addr.addr }, { desc.addr.addr });
        comp.release(1).unwrap();
        assert!(comp.is_empty());
    }

    #[test]
    fn integration_rx_receive_and_release() {
        let umem = default_umem();
        let mut rx = FakeRxRing::new(16);

        // Simulate kernel producing an RX descriptor
        let addr = umem.frame_offset(FrameIndex { index: 5 }).unwrap();
        let desc = RingDesc::new(addr, 512, 0);
        desc.validate_for_umem(&umem).unwrap();
        rx.produce(desc).unwrap();

        // Peek and release
        let received = rx.peek().unwrap().unwrap();
        let fi = received.frame_index(&umem).unwrap();
        assert_eq!({ fi.index }, 5);
        rx.release(1).unwrap();
        assert!(rx.is_empty());
    }

    // --- Error display tests ---

    #[test]
    fn ring_error_display_non_empty() {
        let errors: Vec<ZeroGateError> = vec![
            ZeroGateError::InvalidRingConfig("test".to_string()),
            ZeroGateError::RingFull("test".to_string()),
            ZeroGateError::RingEmpty("test".to_string()),
            ZeroGateError::RingReleaseFailed("test".to_string()),
            ZeroGateError::InvalidDescriptor("test".to_string()),
            ZeroGateError::DescriptorOutOfBounds("test".to_string()),
            ZeroGateError::DescriptorTooLarge {
                len: 5000,
                frame_size: 4096,
            },
            ZeroGateError::DescriptorZeroLength,
            ZeroGateError::DuplicateDescriptor("test".to_string()),
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty(), "error Display must be non-empty");
        }
    }
}
