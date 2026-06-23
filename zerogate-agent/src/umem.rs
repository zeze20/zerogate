//! UMEM allocation and frame offset validation boundary.
//!
//! This module provides a safe abstraction over the contiguous memory region
//! used by AF_XDP for zero-copy packet buffers. The UMEM is divided into
//! fixed-size frames identified by `FrameIndex` and addressed by `UmemAddr`.
//!
//! **MR8 scope:** allocation, validation, and index/offset conversion only.
//! Kernel UMEM registration, XSK socket binding, ring integration, and
//! frame ownership tracking are deferred to future MRs.
//!
//! **Safety invariants:**
//! - No raw pointers in public API.
//! - All frame indices validated against `frame_count`.
//! - All offsets validated against `total_size` and frame alignment.
//! - All arithmetic is checked (no silent overflow).

use zerogate_common::abi::{FrameIndex, UmemAddr};

use crate::error::ZeroGateError;

// ---------------------------------------------------------------------------
// UmemConfig
// ---------------------------------------------------------------------------

/// Configuration for a UMEM region.
///
/// `frame_count` and `frame_size` must be powers of two.
/// `frame_size` must be 2048 or 4096 (standard AF_XDP frame sizes).
/// `headroom` must be strictly less than `frame_size`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UmemConfig {
    pub frame_count: u32,
    pub frame_size: u32,
    pub headroom: u32,
}

#[allow(dead_code)]
impl UmemConfig {
    /// Validate all configuration invariants.
    pub fn validate(&self) -> Result<(), ZeroGateError> {
        if self.frame_count == 0 {
            return Err(ZeroGateError::InvalidUmemConfig(
                "frame_count must be greater than 0".to_string(),
            ));
        }
        if !self.frame_count.is_power_of_two() {
            return Err(ZeroGateError::InvalidUmemConfig(
                "frame_count must be a power of two".to_string(),
            ));
        }
        if self.frame_size == 0 {
            return Err(ZeroGateError::InvalidUmemConfig(
                "frame_size must be greater than 0".to_string(),
            ));
        }
        if !self.frame_size.is_power_of_two() {
            return Err(ZeroGateError::InvalidUmemConfig(
                "frame_size must be a power of two".to_string(),
            ));
        }
        if self.frame_size != 2048 && self.frame_size != 4096 {
            return Err(ZeroGateError::InvalidUmemConfig(
                "frame_size must be 2048 or 4096".to_string(),
            ));
        }
        if self.headroom >= self.frame_size {
            return Err(ZeroGateError::InvalidUmemConfig(
                "headroom must be less than frame_size".to_string(),
            ));
        }
        // Verify total_size does not overflow.
        self.total_size()?;
        Ok(())
    }

    /// Compute the total UMEM size in bytes using checked arithmetic.
    pub fn total_size(&self) -> Result<usize, ZeroGateError> {
        let count = self.frame_count as usize;
        let size = self.frame_size as usize;
        count
            .checked_mul(size)
            .ok_or(ZeroGateError::UmemSizeOverflow)
    }

    /// Return `frame_count` as `usize`.
    pub fn frame_count_usize(&self) -> usize {
        self.frame_count as usize
    }

    /// Return `frame_size` as `usize`.
    pub fn frame_size_usize(&self) -> usize {
        self.frame_size as usize
    }
}

// ---------------------------------------------------------------------------
// UmemRegion
// ---------------------------------------------------------------------------

/// A contiguous memory region divided into fixed-size frames.
///
/// Internally backed by a `Vec<u8>`. No raw pointers are exposed publicly.
/// Kernel AF_XDP UMEM registration is deferred to a future MR.
///
/// Frame identity is expressed through `FrameIndex` (ordinal) and `UmemAddr`
/// (byte offset). The relationship is:
///
///     offset = frame_index * frame_size
///
/// All public methods validate their inputs before use.
#[allow(dead_code)]
pub struct UmemRegion {
    config: UmemConfig,
    buf: Vec<u8>,
}

#[allow(dead_code)]
impl UmemRegion {
    /// Allocate a UMEM region according to `config`.
    ///
    /// Validates the config and allocates a zeroed buffer of `total_size` bytes.
    /// Does NOT register the region with the kernel — that is future work.
    pub fn allocate(config: UmemConfig) -> Result<Self, ZeroGateError> {
        config.validate()?;
        let total = config.total_size()?;
        let buf = vec![0u8; total];
        Ok(Self { config, buf })
    }

    /// Return a reference to the UMEM configuration.
    pub fn config(&self) -> &UmemConfig {
        &self.config
    }

    /// Total size of the UMEM region in bytes.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Returns `true` if the UMEM region has zero length.
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Convert a frame index to its UMEM byte offset.
    ///
    /// Returns `InvalidFrameIndex` if `frame_index >= frame_count`.
    pub fn frame_offset(&self, frame_index: FrameIndex) -> Result<UmemAddr, ZeroGateError> {
        self.validate_frame_index(frame_index)?;
        let idx = frame_index.index as u64;
        let size = self.config.frame_size as u64;
        // Checked multiply: cannot overflow because frame_index < frame_count
        // and frame_count * frame_size fits in usize (validated at allocation).
        let offset = idx
            .checked_mul(size)
            .ok_or(ZeroGateError::UmemSizeOverflow)?;
        Ok(UmemAddr { addr: offset })
    }

    /// Convert a UMEM byte offset to its frame index.
    ///
    /// The offset must be frame-aligned and within bounds.
    pub fn frame_index_from_offset(&self, addr: UmemAddr) -> Result<FrameIndex, ZeroGateError> {
        self.validate_offset(addr)?;
        let frame_size = self.config.frame_size as u64;
        let index = addr.addr / frame_size;
        // The division result fits in u32 because addr < total_size and
        // total_size = frame_count * frame_size where frame_count is u32.
        Ok(FrameIndex {
            index: index as u32,
        })
    }

    /// Validate that a frame index is within bounds.
    pub fn validate_frame_index(&self, frame_index: FrameIndex) -> Result<(), ZeroGateError> {
        if frame_index.index >= self.config.frame_count {
            return Err(ZeroGateError::InvalidFrameIndex {
                index: frame_index.index,
                frame_count: self.config.frame_count,
            });
        }
        Ok(())
    }

    /// Validate that a UMEM offset is within bounds and frame-aligned.
    pub fn validate_offset(&self, addr: UmemAddr) -> Result<(), ZeroGateError> {
        let total = self.buf.len() as u64;
        if addr.addr >= total {
            return Err(ZeroGateError::InvalidUmemOffset {
                offset: addr.addr,
                total_size: self.buf.len(),
            });
        }
        if !self.is_frame_aligned(addr) {
            return Err(ZeroGateError::UnalignedUmemOffset {
                offset: addr.addr,
                frame_size: self.config.frame_size,
            });
        }
        Ok(())
    }

    /// Check whether a UMEM offset is aligned to the frame size.
    pub fn is_frame_aligned(&self, addr: UmemAddr) -> bool {
        let frame_size = self.config.frame_size as u64;
        addr.addr.is_multiple_of(frame_size)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> UmemConfig {
        UmemConfig {
            frame_count: 256,
            frame_size: 4096,
            headroom: 0,
        }
    }

    fn small_config() -> UmemConfig {
        UmemConfig {
            frame_count: 4,
            frame_size: 2048,
            headroom: 0,
        }
    }

    // -- UmemConfig validation --

    #[test]
    fn valid_config_passes() {
        assert!(default_config().validate().is_ok());
    }

    #[test]
    fn valid_small_config_passes() {
        assert!(small_config().validate().is_ok());
    }

    #[test]
    fn valid_config_with_headroom_passes() {
        let cfg = UmemConfig {
            frame_count: 64,
            frame_size: 4096,
            headroom: 256,
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn zero_frame_count_rejected() {
        let cfg = UmemConfig {
            frame_count: 0,
            frame_size: 4096,
            headroom: 0,
        };
        match cfg.validate() {
            Err(ZeroGateError::InvalidUmemConfig(msg)) => {
                assert!(msg.contains("frame_count"));
            }
            other => panic!("expected InvalidUmemConfig, got: {other:?}"),
        }
    }

    #[test]
    fn non_power_of_two_frame_count_rejected() {
        let cfg = UmemConfig {
            frame_count: 3,
            frame_size: 4096,
            headroom: 0,
        };
        match cfg.validate() {
            Err(ZeroGateError::InvalidUmemConfig(msg)) => {
                assert!(msg.contains("power of two"));
            }
            other => panic!("expected InvalidUmemConfig, got: {other:?}"),
        }
    }

    #[test]
    fn zero_frame_size_rejected() {
        let cfg = UmemConfig {
            frame_count: 256,
            frame_size: 0,
            headroom: 0,
        };
        match cfg.validate() {
            Err(ZeroGateError::InvalidUmemConfig(msg)) => {
                assert!(msg.contains("frame_size"));
            }
            other => panic!("expected InvalidUmemConfig, got: {other:?}"),
        }
    }

    #[test]
    fn non_power_of_two_frame_size_rejected() {
        let cfg = UmemConfig {
            frame_count: 256,
            frame_size: 3000,
            headroom: 0,
        };
        match cfg.validate() {
            Err(ZeroGateError::InvalidUmemConfig(msg)) => {
                assert!(msg.contains("power of two"));
            }
            other => panic!("expected InvalidUmemConfig, got: {other:?}"),
        }
    }

    #[test]
    fn unsupported_frame_size_rejected() {
        let cfg = UmemConfig {
            frame_count: 256,
            frame_size: 1024,
            headroom: 0,
        };
        match cfg.validate() {
            Err(ZeroGateError::InvalidUmemConfig(msg)) => {
                assert!(msg.contains("2048 or 4096"));
            }
            other => panic!("expected InvalidUmemConfig, got: {other:?}"),
        }
    }

    #[test]
    fn headroom_equal_frame_size_rejected() {
        let cfg = UmemConfig {
            frame_count: 256,
            frame_size: 4096,
            headroom: 4096,
        };
        match cfg.validate() {
            Err(ZeroGateError::InvalidUmemConfig(msg)) => {
                assert!(msg.contains("headroom"));
            }
            other => panic!("expected InvalidUmemConfig, got: {other:?}"),
        }
    }

    #[test]
    fn headroom_exceeds_frame_size_rejected() {
        let cfg = UmemConfig {
            frame_count: 256,
            frame_size: 2048,
            headroom: 4096,
        };
        match cfg.validate() {
            Err(ZeroGateError::InvalidUmemConfig(msg)) => {
                assert!(msg.contains("headroom"));
            }
            other => panic!("expected InvalidUmemConfig, got: {other:?}"),
        }
    }

    #[test]
    fn total_size_computed_correctly() {
        let cfg = default_config();
        assert_eq!(cfg.total_size().unwrap(), 256 * 4096);
    }

    #[test]
    fn frame_count_usize_works() {
        let cfg = default_config();
        assert_eq!(cfg.frame_count_usize(), 256);
    }

    #[test]
    fn frame_size_usize_works() {
        let cfg = default_config();
        assert_eq!(cfg.frame_size_usize(), 4096);
    }

    // -- UmemRegion allocation --

    #[test]
    fn allocate_valid_umem() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert_eq!(region.len(), 4 * 2048);
        assert!(!region.is_empty());
    }

    #[test]
    fn allocate_invalid_config_fails() {
        let cfg = UmemConfig {
            frame_count: 0,
            frame_size: 4096,
            headroom: 0,
        };
        assert!(UmemRegion::allocate(cfg).is_err());
    }

    #[test]
    fn len_equals_total_size() {
        let cfg = default_config();
        let expected = cfg.total_size().unwrap();
        let region = UmemRegion::allocate(cfg).unwrap();
        assert_eq!(region.len(), expected);
    }

    #[test]
    fn config_accessor_returns_config() {
        let cfg = small_config();
        let region = UmemRegion::allocate(cfg.clone()).unwrap();
        assert_eq!(region.config(), &cfg);
    }

    // -- frame_offset --

    #[test]
    fn frame_offset_zero_returns_zero() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let addr = region.frame_offset(FrameIndex { index: 0 }).unwrap();
        assert_eq!({ addr.addr }, 0);
    }

    #[test]
    fn frame_offset_one_returns_frame_size() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let addr = region.frame_offset(FrameIndex { index: 1 }).unwrap();
        assert_eq!({ addr.addr }, 2048);
    }

    #[test]
    fn last_frame_offset_valid() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let addr = region.frame_offset(FrameIndex { index: 3 }).unwrap();
        assert_eq!({ addr.addr }, 3 * 2048);
    }

    #[test]
    fn frame_index_out_of_bounds_rejected() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        match region.frame_offset(FrameIndex { index: 4 }) {
            Err(ZeroGateError::InvalidFrameIndex {
                index: 4,
                frame_count: 4,
            }) => {}
            other => panic!("expected InvalidFrameIndex, got: {other:?}"),
        }
    }

    #[test]
    fn frame_index_large_out_of_bounds_rejected() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        match region.frame_offset(FrameIndex { index: u32::MAX }) {
            Err(ZeroGateError::InvalidFrameIndex { .. }) => {}
            other => panic!("expected InvalidFrameIndex, got: {other:?}"),
        }
    }

    // -- validate_offset --

    #[test]
    fn offset_zero_valid() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert!(region.validate_offset(UmemAddr { addr: 0 }).is_ok());
    }

    #[test]
    fn offset_frame_size_valid() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert!(region.validate_offset(UmemAddr { addr: 2048 }).is_ok());
    }

    #[test]
    fn last_frame_offset_validated() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let last = (4 - 1) * 2048;
        assert!(region.validate_offset(UmemAddr { addr: last }).is_ok());
    }

    #[test]
    fn offset_equal_total_size_rejected() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let total = region.len() as u64;
        match region.validate_offset(UmemAddr { addr: total }) {
            Err(ZeroGateError::InvalidUmemOffset { .. }) => {}
            other => panic!("expected InvalidUmemOffset, got: {other:?}"),
        }
    }

    #[test]
    fn offset_exceeds_total_size_rejected() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let total = region.len() as u64;
        match region.validate_offset(UmemAddr { addr: total + 4096 }) {
            Err(ZeroGateError::InvalidUmemOffset { .. }) => {}
            other => panic!("expected InvalidUmemOffset, got: {other:?}"),
        }
    }

    #[test]
    fn offset_not_aligned_rejected() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        match region.validate_offset(UmemAddr { addr: 100 }) {
            Err(ZeroGateError::UnalignedUmemOffset { .. }) => {}
            other => panic!("expected UnalignedUmemOffset, got: {other:?}"),
        }
    }

    #[test]
    fn offset_one_byte_off_alignment_rejected() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        match region.validate_offset(UmemAddr { addr: 2049 }) {
            Err(ZeroGateError::UnalignedUmemOffset { .. }) => {}
            other => panic!("expected UnalignedUmemOffset, got: {other:?}"),
        }
    }

    // -- is_frame_aligned --

    #[test]
    fn aligned_offsets_detected() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert!(region.is_frame_aligned(UmemAddr { addr: 0 }));
        assert!(region.is_frame_aligned(UmemAddr { addr: 2048 }));
        assert!(region.is_frame_aligned(UmemAddr { addr: 4096 }));
    }

    #[test]
    fn unaligned_offsets_detected() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert!(!region.is_frame_aligned(UmemAddr { addr: 1 }));
        assert!(!region.is_frame_aligned(UmemAddr { addr: 1024 }));
        assert!(!region.is_frame_aligned(UmemAddr { addr: 2047 }));
    }

    // -- frame_index_from_offset --

    #[test]
    fn offset_to_index_zero() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let idx = region
            .frame_index_from_offset(UmemAddr { addr: 0 })
            .unwrap();
        assert_eq!({ idx.index }, 0);
    }

    #[test]
    fn offset_to_index_frame_size() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let idx = region
            .frame_index_from_offset(UmemAddr { addr: 2048 })
            .unwrap();
        assert_eq!({ idx.index }, 1);
    }

    #[test]
    fn offset_to_index_last_frame() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let idx = region
            .frame_index_from_offset(UmemAddr { addr: 3 * 2048 })
            .unwrap();
        assert_eq!({ idx.index }, 3);
    }

    #[test]
    fn offset_to_index_invalid_offset_fails() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert!(
            region
                .frame_index_from_offset(UmemAddr { addr: 100 })
                .is_err()
        );
    }

    #[test]
    fn offset_to_index_out_of_bounds_fails() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        let total = region.len() as u64;
        assert!(
            region
                .frame_index_from_offset(UmemAddr { addr: total })
                .is_err()
        );
    }

    // -- roundtrip --

    #[test]
    fn roundtrip_index_to_offset_to_index() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        for i in 0..4 {
            let fi = FrameIndex { index: i };
            let offset = region.frame_offset(fi).unwrap();
            let back = region.frame_index_from_offset(offset).unwrap();
            assert_eq!(back, fi);
        }
    }

    #[test]
    fn roundtrip_all_frames_default_config() {
        let cfg = UmemConfig {
            frame_count: 16,
            frame_size: 4096,
            headroom: 0,
        };
        let region = UmemRegion::allocate(cfg).unwrap();
        for i in 0..16 {
            let fi = FrameIndex { index: i };
            let offset = region.frame_offset(fi).unwrap();
            assert_eq!({ offset.addr }, i as u64 * 4096);
            let back = region.frame_index_from_offset(offset).unwrap();
            assert_eq!(back, fi);
        }
    }

    // -- error Display --

    #[test]
    fn umem_error_display_strings_non_empty() {
        let errors: Vec<ZeroGateError> = vec![
            ZeroGateError::InvalidUmemConfig("test".to_string()),
            ZeroGateError::InvalidFrameIndex {
                index: 5,
                frame_count: 4,
            },
            ZeroGateError::InvalidUmemOffset {
                offset: 9999,
                total_size: 8192,
            },
            ZeroGateError::UnalignedUmemOffset {
                offset: 100,
                frame_size: 4096,
            },
            ZeroGateError::UmemAllocationFailed("oom".to_string()),
            ZeroGateError::UmemSizeOverflow,
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty(), "Display for {err:?} should be non-empty");
        }
    }

    // -- validate_frame_index --

    #[test]
    fn validate_frame_index_zero_ok() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert!(region.validate_frame_index(FrameIndex { index: 0 }).is_ok());
    }

    #[test]
    fn validate_frame_index_last_ok() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert!(region.validate_frame_index(FrameIndex { index: 3 }).is_ok());
    }

    #[test]
    fn validate_frame_index_at_count_fails() {
        let region = UmemRegion::allocate(small_config()).unwrap();
        assert!(
            region
                .validate_frame_index(FrameIndex { index: 4 })
                .is_err()
        );
    }

    // -- 2048 frame size variant --

    #[test]
    fn frame_size_2048_works() {
        let cfg = UmemConfig {
            frame_count: 8,
            frame_size: 2048,
            headroom: 0,
        };
        let region = UmemRegion::allocate(cfg).unwrap();
        assert_eq!(region.len(), 8 * 2048);
        let addr = region.frame_offset(FrameIndex { index: 7 }).unwrap();
        assert_eq!({ addr.addr }, 7 * 2048);
    }

    // -- headroom does not affect frame layout --

    #[test]
    fn headroom_does_not_change_total_size() {
        let cfg = UmemConfig {
            frame_count: 4,
            frame_size: 4096,
            headroom: 128,
        };
        let region = UmemRegion::allocate(cfg).unwrap();
        assert_eq!(region.len(), 4 * 4096);
    }
}
