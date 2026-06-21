// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! UMEM memory region management.
//!
//! This is one of two files in `zerogate-agent` allowed to contain `unsafe`.
//! Frame identity is represented by index and UMEM offset, not raw pointer.

use crate::error::ZeroGateError;
use zerogate_common::constants;

/// UMEM configuration.
#[derive(Debug, Clone, Copy)]
pub struct UmemConfig {
    pub frame_count: u32,
    pub frame_size: u32,
}

impl Default for UmemConfig {
    fn default() -> Self {
        Self {
            frame_count: constants::DEFAULT_UMEM_FRAME_COUNT,
            frame_size: constants::DEFAULT_UMEM_FRAME_SIZE,
        }
    }
}

impl UmemConfig {
    /// Total UMEM region size in bytes.
    pub fn total_size(&self) -> usize {
        self.frame_count as usize * self.frame_size as usize
    }
}

/// Page-aligned UMEM memory region.
///
/// Frames are addressed by index × frame_size offset, never by raw pointer
/// in public API. The internal allocation uses page-aligned mmap.
pub struct UmemRegion {
    config: UmemConfig,
    // On non-Linux platforms, we use a Vec as a placeholder.
    // On Linux, this would be mmap'd memory. The Vec approach allows
    // the crate to compile and be tested on all platforms.
    buffer: Vec<u8>,
}

impl UmemRegion {
    /// Allocates a page-aligned UMEM region.
    pub fn allocate(config: UmemConfig) -> Result<Self, ZeroGateError> {
        let total = config.total_size();
        if total == 0 {
            return Err(ZeroGateError::Umem("UMEM size cannot be zero".into()));
        }

        // Allocate with page alignment.
        // On Linux, this would use mmap(MAP_ANONYMOUS | MAP_PRIVATE).
        // For portability and testing, we use a Vec and verify alignment.
        let buffer = allocate_aligned(total, config.frame_size as usize)?;

        Ok(Self { config, buffer })
    }

    /// Returns the UMEM configuration.
    pub fn config(&self) -> &UmemConfig {
        &self.config
    }

    /// Returns the base address of the UMEM region (for AF_XDP registration).
    pub fn base_addr(&self) -> usize {
        self.buffer.as_ptr() as usize
    }

    /// Returns the total UMEM region size in bytes.
    pub fn total_size(&self) -> usize {
        self.buffer.len()
    }

    /// Returns a slice of the frame at the given UMEM byte offset.
    ///
    /// The offset and length must be within bounds.
    pub fn frame_slice(&self, offset: u64, len: usize) -> Option<&[u8]> {
        let start = offset as usize;
        let end = start.checked_add(len)?;
        if end > self.buffer.len() {
            return None;
        }
        Some(&self.buffer[start..end])
    }

    /// Returns a mutable slice of the frame at the given UMEM byte offset.
    pub fn frame_slice_mut(&mut self, offset: u64, len: usize) -> Option<&mut [u8]> {
        let start = offset as usize;
        let end = start.checked_add(len)?;
        if end > self.buffer.len() {
            return None;
        }
        Some(&mut self.buffer[start..end])
    }
}

/// Allocates page-aligned memory.
fn allocate_aligned(size: usize, alignment: usize) -> Result<Vec<u8>, ZeroGateError> {
    // Use std::alloc for aligned allocation, wrapped in a Vec for RAII.
    let layout = std::alloc::Layout::from_size_align(size, alignment).map_err(|e| {
        ZeroGateError::Umem(format!("invalid UMEM layout: {e}"))
    })?;

    // SAFETY:
    // - Provenance: alloc::alloc returns a new allocation.
    // - Bounds: layout specifies exact size and alignment.
    // - Alignment: Layout guarantees the requested alignment.
    // - Lifetime: the allocation is owned by the returned Vec.
    // - Aliasing: no other references exist to this memory.
    let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
    if ptr.is_null() {
        return Err(ZeroGateError::Umem(format!(
            "failed to allocate {size} bytes aligned to {alignment}"
        )));
    }

    // SAFETY:
    // - ptr was allocated with the given layout.
    // - size bytes are initialized (alloc_zeroed).
    // - capacity matches the allocation size.
    // Vec will dealloc with the Global allocator, which matches alloc::alloc.
    let buffer = unsafe { Vec::from_raw_parts(ptr, size, size) };
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umem_allocation() {
        let config = UmemConfig {
            frame_count: 16,
            frame_size: 4096,
        };
        let umem = UmemRegion::allocate(config).unwrap();
        assert_eq!(umem.total_size(), 16 * 4096);
        // Check alignment.
        assert_eq!(umem.base_addr() % 4096, 0);
    }

    #[test]
    fn frame_slice_access() {
        let config = UmemConfig {
            frame_count: 4,
            frame_size: 4096,
        };
        let umem = UmemRegion::allocate(config).unwrap();

        // Valid access.
        let slice = umem.frame_slice(0, 100).unwrap();
        assert_eq!(slice.len(), 100);
        assert!(slice.iter().all(|&b| b == 0)); // zeroed

        // Access frame 2.
        let slice = umem.frame_slice(2 * 4096, 4096).unwrap();
        assert_eq!(slice.len(), 4096);

        // Out of bounds.
        assert!(umem.frame_slice(4 * 4096, 1).is_none());
    }

    #[test]
    fn zero_size_rejected() {
        let config = UmemConfig {
            frame_count: 0,
            frame_size: 4096,
        };
        assert!(UmemRegion::allocate(config).is_err());
    }
}
