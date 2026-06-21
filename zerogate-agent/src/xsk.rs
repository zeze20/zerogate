// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! AF_XDP socket creation, UMEM registration, and XSK map update.
//!
//! No `unsafe` in this file. Low-level operations delegate to the `xdp`
//! crate and `aya` crate, which handle unsafe internally.
//! Hardware-specific integration points are marked with TODO.

use crate::config::AgentConfig;
use crate::error::ZeroGateError;
use log::info;

/// Represents the XSK socket setup for a single queue.
///
/// This struct bundles the configuration needed to create and bind an
/// AF_XDP socket. Actual socket creation requires Linux and is
/// performed via the `xdp` crate at runtime.
pub struct XskConfig {
    pub interface_index: u32,
    pub queue_id: u32,
    pub frame_count: u32,
    pub frame_size: u32,
    pub force_copy: bool,
}

impl XskConfig {
    /// Creates an XSK configuration from the agent config for a specific queue.
    pub fn from_agent_config(config: &AgentConfig, queue_id: u32, if_index: u32) -> Self {
        Self {
            interface_index: if_index,
            queue_id,
            frame_count: config.frame_count,
            frame_size: config.frame_size,
            force_copy: config.force_copy,
        }
    }
}

/// Placeholder for the bound XSK socket handle.
///
/// On Linux, this would wrap the `xdp::socket::XdpSocket` and ring handles.
/// Marked as TODO for hardware-specific integration.
pub struct XskHandle {
    pub queue_id: u32,
    pub socket_fd: i32,
}

impl XskHandle {
    /// Creates a new XSK handle.
    ///
    /// TODO: On Linux, this would create the AF_XDP socket, set up UMEM,
    /// build rings, and bind to the interface queue.
    pub fn create(_config: &XskConfig) -> Result<Self, ZeroGateError> {
        // TODO: Hardware-specific AF_XDP socket creation.
        // This is a placeholder that returns a dummy handle for
        // compilation and testing on non-Linux platforms.
        info!(
            "XSK handle created for queue {} (placeholder)",
            _config.queue_id
        );
        Ok(Self {
            queue_id: _config.queue_id,
            socket_fd: -1,
        })
    }

    /// Registers this XSK socket's fd in the eBPF XSK_MAP.
    ///
    /// TODO: On Linux, this would call `XskMap::set(queue_id, socket_fd, 0)`.
    pub fn register_in_xsk_map(&self) -> Result<(), ZeroGateError> {
        // TODO: Hardware-specific XSK_MAP registration.
        info!(
            "XSK fd registered in XSK_MAP[{}] (placeholder)",
            self.queue_id
        );
        Ok(())
    }
}
