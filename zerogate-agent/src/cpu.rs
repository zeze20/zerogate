// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! CPU pinning utilities for cache-local RX processing.
//!
//! No `unsafe` in this file.

use log::{info, warn};

/// Pins the calling thread to the specified CPU core.
///
/// Returns `true` if pinning succeeded, `false` otherwise.
pub fn pin_to_cpu(cpu: usize) -> bool {
    let core_ids = core_affinity::get_core_ids().unwrap_or_default();
    if let Some(core_id) = core_ids.get(cpu) {
        if core_affinity::set_for_current(*core_id) {
            info!("thread pinned to CPU {cpu}");
            return true;
        }
        warn!("failed to pin thread to CPU {cpu}");
    } else {
        warn!(
            "CPU {cpu} not available (system has {} cores)",
            core_ids.len()
        );
    }
    false
}
