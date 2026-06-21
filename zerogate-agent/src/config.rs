// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Configuration parser for the ZeroGate agent.

use zerogate_common::constants;

/// Agent configuration.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Path to the compiled eBPF object file.
    pub ebpf_obj: String,
    /// Network interface name.
    pub iface: String,
    /// NIC queue IDs to bind AF_XDP sockets to.
    pub queue_ids: Vec<u32>,
    /// CPU core pinning map: queue_ids[i] -> cpu_ids[i].
    pub cpu_ids: Vec<usize>,
    /// UMEM frame count per queue.
    pub frame_count: u32,
    /// UMEM frame size in bytes.
    pub frame_size: u32,
    /// Force copy mode (instead of zero-copy).
    pub force_copy: bool,
    /// XDP attach mode.
    pub xdp_mode: XdpMode,
    /// Session IDs to pre-admit.
    pub sessions: Vec<u64>,
}

/// XDP attach mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdpMode {
    Skb,
    Driver,
    Hardware,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            ebpf_obj: String::new(),
            iface: String::from("eth0"),
            queue_ids: vec![0],
            cpu_ids: vec![0],
            frame_count: constants::DEFAULT_UMEM_FRAME_COUNT,
            frame_size: constants::DEFAULT_UMEM_FRAME_SIZE,
            force_copy: false,
            xdp_mode: XdpMode::Skb,
            sessions: Vec::new(),
        }
    }
}

impl AgentConfig {
    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), crate::error::ZeroGateError> {
        if self.ebpf_obj.is_empty() {
            return Err(crate::error::ZeroGateError::Config(
                "ebpf_obj path is required".into(),
            ));
        }
        if self.iface.is_empty() {
            return Err(crate::error::ZeroGateError::Config(
                "interface name is required".into(),
            ));
        }
        if self.queue_ids.is_empty() {
            return Err(crate::error::ZeroGateError::Config(
                "at least one queue ID is required".into(),
            ));
        }
        if self.queue_ids.len() != self.cpu_ids.len() {
            return Err(crate::error::ZeroGateError::Config(
                "queue_ids and cpu_ids must have the same length".into(),
            ));
        }
        if self.frame_count == 0 || !self.frame_count.is_power_of_two() {
            return Err(crate::error::ZeroGateError::Config(
                "frame_count must be a non-zero power of two".into(),
            ));
        }
        if self.frame_size != 2048 && self.frame_size != 4096 {
            return Err(crate::error::ZeroGateError::Config(
                "frame_size must be 2048 or 4096".into(),
            ));
        }
        for &qid in &self.queue_ids {
            if qid >= constants::MAX_QUEUES {
                return Err(crate::error::ZeroGateError::Config(format!(
                    "queue ID {qid} exceeds MAX_QUEUES ({})",
                    constants::MAX_QUEUES
                )));
            }
        }
        Ok(())
    }
}
