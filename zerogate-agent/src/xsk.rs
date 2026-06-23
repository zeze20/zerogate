//! XSK socket configuration and lifecycle scaffold.
//!
//! This module provides safe abstractions for AF_XDP socket (XSK) configuration
//! and lifecycle management. MR9 implements the scaffold only — no real AF_XDP
//! socket creation, bind, or XSK_MAP registration occurs.
//!
//! **Safety invariants:**
//! - No raw socket fd exposed publicly.
//! - XSK configuration is validated before handle creation.
//! - `bind()` does NOT fake real AF_XDP bind success.
//! - State transitions are enforced.

use crate::error::ZeroGateError;
use crate::umem::UmemConfig;

// ---------------------------------------------------------------------------
// XskConfig
// ---------------------------------------------------------------------------

/// Configuration for an AF_XDP socket (XSK).
///
/// `frame_count` and `frame_size` must match UMEM configuration constraints.
/// `interface_name` identifies the NIC to bind.
/// `queue_id` identifies the NIC queue for AF_XDP binding.
/// `force_copy` selects copy mode over zero-copy if the driver doesn't support it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct XskConfig {
    pub interface_name: String,
    pub queue_id: u32,
    pub frame_count: u32,
    pub frame_size: u32,
    pub force_copy: bool,
}

#[allow(dead_code)]
impl XskConfig {
    /// Validate all XSK configuration invariants.
    pub fn validate(&self) -> Result<(), ZeroGateError> {
        if self.interface_name.is_empty() {
            return Err(ZeroGateError::InvalidXskConfig(
                "interface_name must not be empty".to_string(),
            ));
        }
        if self.frame_count == 0 {
            return Err(ZeroGateError::InvalidXskConfig(
                "frame_count must be greater than 0".to_string(),
            ));
        }
        if !self.frame_count.is_power_of_two() {
            return Err(ZeroGateError::InvalidXskConfig(
                "frame_count must be a power of two".to_string(),
            ));
        }
        if self.frame_size == 0 {
            return Err(ZeroGateError::InvalidXskConfig(
                "frame_size must be greater than 0".to_string(),
            ));
        }
        if !self.frame_size.is_power_of_two() {
            return Err(ZeroGateError::InvalidXskConfig(
                "frame_size must be a power of two".to_string(),
            ));
        }
        if self.frame_size != 2048 && self.frame_size != 4096 {
            return Err(ZeroGateError::InvalidXskConfig(
                "frame_size must be 2048 or 4096".to_string(),
            ));
        }
        Ok(())
    }

    /// Create an XskConfig from an existing UmemConfig, reusing its validated parameters.
    pub fn from_umem_config(
        interface_name: String,
        queue_id: u32,
        umem: &UmemConfig,
        force_copy: bool,
    ) -> Result<Self, ZeroGateError> {
        let config = Self {
            interface_name,
            queue_id,
            frame_count: umem.frame_count,
            frame_size: umem.frame_size,
            force_copy,
        };
        config.validate()?;
        Ok(config)
    }
}

// ---------------------------------------------------------------------------
// XskState
// ---------------------------------------------------------------------------

/// Lifecycle state of an XSK handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum XskState {
    /// Handle created, not bound to kernel.
    Created,
    /// Socket bound to interface/queue (future: real AF_XDP bind).
    Bound,
    /// Handle closed.
    Closed,
}

// ---------------------------------------------------------------------------
// XskHandle
// ---------------------------------------------------------------------------

/// Scaffold for an AF_XDP socket handle.
///
/// Tracks lifecycle state. In MR9, no real socket fd exists.
/// `bind()` returns `NotImplemented` because real AF_XDP bind is future work.
#[allow(dead_code)]
pub struct XskHandle {
    queue_id: u32,
    state: XskState,
}

#[allow(dead_code)]
impl XskHandle {
    /// Create an XSK handle from a validated config.
    ///
    /// Returns a handle in `Created` state. No real socket is opened.
    pub fn create(config: &XskConfig) -> Result<Self, ZeroGateError> {
        config.validate()?;
        Ok(Self {
            queue_id: config.queue_id,
            state: XskState::Created,
        })
    }

    /// Return the queue ID associated with this handle.
    pub fn queue_id(&self) -> u32 {
        self.queue_id
    }

    /// Return the current lifecycle state.
    pub fn state(&self) -> XskState {
        self.state
    }

    /// Return `true` if the handle is in `Bound` state.
    pub fn is_bound(&self) -> bool {
        self.state == XskState::Bound
    }

    /// Attempt to bind the XSK socket to the kernel.
    ///
    /// In MR9 this always returns `NotImplemented` because real AF_XDP bind
    /// requires kernel support and is deferred to a future MR.
    pub fn bind(&mut self) -> Result<(), ZeroGateError> {
        match self.state {
            XskState::Created => Err(ZeroGateError::NotImplemented(
                "real AF_XDP socket bind is not yet implemented".to_string(),
            )),
            XskState::Bound => Err(ZeroGateError::XskBindFailed(
                "socket is already bound".to_string(),
            )),
            XskState::Closed => Err(ZeroGateError::XskBindFailed(
                "cannot bind a closed socket".to_string(),
            )),
        }
    }

    /// Close the XSK handle.
    ///
    /// Transitions to `Closed` state. Idempotent — closing an already-closed
    /// handle is a no-op that returns `Ok(())`.
    pub fn close(&mut self) -> Result<(), ZeroGateError> {
        self.state = XskState::Closed;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_xsk_config() -> XskConfig {
        XskConfig {
            interface_name: "eth0".to_string(),
            queue_id: 0,
            frame_count: 4096,
            frame_size: 4096,
            force_copy: false,
        }
    }

    // --- XskConfig validation tests ---

    #[test]
    fn valid_config_passes() {
        assert!(valid_xsk_config().validate().is_ok());
    }

    #[test]
    fn empty_interface_rejected() {
        let mut cfg = valid_xsk_config();
        cfg.interface_name = String::new();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn zero_frame_count_rejected() {
        let mut cfg = valid_xsk_config();
        cfg.frame_count = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn non_power_of_two_frame_count_rejected() {
        let mut cfg = valid_xsk_config();
        cfg.frame_count = 3;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn zero_frame_size_rejected() {
        let mut cfg = valid_xsk_config();
        cfg.frame_size = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn non_power_of_two_frame_size_rejected() {
        let mut cfg = valid_xsk_config();
        cfg.frame_size = 3000;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn unsupported_frame_size_rejected() {
        let mut cfg = valid_xsk_config();
        cfg.frame_size = 1024;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn frame_size_2048_accepted() {
        let mut cfg = valid_xsk_config();
        cfg.frame_size = 2048;
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn from_umem_config_valid() {
        let umem = UmemConfig {
            frame_count: 1024,
            frame_size: 4096,
            headroom: 0,
        };
        let xsk = XskConfig::from_umem_config("eth0".to_string(), 0, &umem, false);
        assert!(xsk.is_ok());
        let xsk = xsk.unwrap();
        assert_eq!(xsk.frame_count, 1024);
        assert_eq!(xsk.frame_size, 4096);
    }

    #[test]
    fn from_umem_config_empty_interface_rejected() {
        let umem = UmemConfig {
            frame_count: 1024,
            frame_size: 4096,
            headroom: 0,
        };
        let xsk = XskConfig::from_umem_config(String::new(), 0, &umem, false);
        assert!(xsk.is_err());
    }

    // --- XskHandle lifecycle tests ---

    #[test]
    fn handle_starts_in_created_state() {
        let cfg = valid_xsk_config();
        let handle = XskHandle::create(&cfg).unwrap();
        assert_eq!(handle.state(), XskState::Created);
        assert!(!handle.is_bound());
    }

    #[test]
    fn handle_queue_id_matches_config() {
        let mut cfg = valid_xsk_config();
        cfg.queue_id = 7;
        let handle = XskHandle::create(&cfg).unwrap();
        assert_eq!(handle.queue_id(), 7);
    }

    #[test]
    fn create_with_invalid_config_fails() {
        let mut cfg = valid_xsk_config();
        cfg.frame_count = 0;
        assert!(XskHandle::create(&cfg).is_err());
    }

    #[test]
    fn bind_returns_not_implemented() {
        let cfg = valid_xsk_config();
        let mut handle = XskHandle::create(&cfg).unwrap();
        let result = handle.bind();
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("not implemented"));
    }

    #[test]
    fn bind_on_closed_handle_fails() {
        let cfg = valid_xsk_config();
        let mut handle = XskHandle::create(&cfg).unwrap();
        handle.close().unwrap();
        let result = handle.bind();
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("closed"));
    }

    #[test]
    fn close_transitions_to_closed() {
        let cfg = valid_xsk_config();
        let mut handle = XskHandle::create(&cfg).unwrap();
        handle.close().unwrap();
        assert_eq!(handle.state(), XskState::Closed);
    }

    #[test]
    fn close_is_idempotent() {
        let cfg = valid_xsk_config();
        let mut handle = XskHandle::create(&cfg).unwrap();
        handle.close().unwrap();
        assert!(handle.close().is_ok());
        assert_eq!(handle.state(), XskState::Closed);
    }

    #[test]
    fn xsk_error_display_non_empty() {
        let errors = [
            ZeroGateError::InvalidXskConfig("test".to_string()),
            ZeroGateError::XskCreateFailed("test".to_string()),
            ZeroGateError::XskBindFailed("test".to_string()),
            ZeroGateError::XskCloseFailed("test".to_string()),
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty(), "error Display must be non-empty");
        }
    }
}
