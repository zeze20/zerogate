//! BPF map control-plane boundary.
//!
//! Defines writer traits for POLICY, SESSIONS, and XSK_MAP,
//! an in-memory backend for host-safe testing, and a generic
//! `BpfMapManager` that orchestrates map updates.
//!
//! The `InMemoryMapBackend` is test/development only — it does NOT
//! write to real kernel BPF maps. A future `AyaMapBackend` will
//! provide the real kernel integration once the loader exposes
//! map handles.

use std::collections::HashMap;

use zerogate_common::abi::{PacketMeta, PolicyAction, SessionKey, SessionValue};

use crate::error::ZeroGateError;

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Write policy entries into the POLICY BPF map.
pub trait PolicyMapWriter {
    fn upsert_policy(&mut self, key: PacketMeta, value: PolicyAction) -> Result<(), ZeroGateError>;

    fn remove_policy(&mut self, key: &PacketMeta) -> Result<(), ZeroGateError>;
}

/// Write session entries into the SESSIONS BPF map.
pub trait SessionMapWriter {
    fn upsert_session(&mut self, key: SessionKey, value: SessionValue)
    -> Result<(), ZeroGateError>;

    fn remove_session(&mut self, key: &SessionKey) -> Result<(), ZeroGateError>;
}

/// Write XSK socket file descriptors into the XSK_MAP.
#[allow(dead_code)]
pub trait XskMapWriter {
    fn upsert_xsk(&mut self, queue_id: u32, fd: i32) -> Result<(), ZeroGateError>;

    fn remove_xsk(&mut self, queue_id: u32) -> Result<(), ZeroGateError>;
}

// ---------------------------------------------------------------------------
// InMemoryMapBackend — test/development only
// ---------------------------------------------------------------------------

/// In-memory map backend for host-safe testing.
///
/// This backend stores map entries in `HashMap`s and does NOT
/// interact with the kernel BPF subsystem. It is intended for
/// unit tests and dry-run demonstrations only.
#[allow(dead_code)]
pub struct InMemoryMapBackend {
    policies: HashMap<PacketMeta, PolicyAction>,
    sessions: HashMap<SessionKey, SessionValue>,
    xsks: HashMap<u32, i32>,
}

#[allow(dead_code)]
impl InMemoryMapBackend {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            sessions: HashMap::new(),
            xsks: HashMap::new(),
        }
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn xsk_count(&self) -> usize {
        self.xsks.len()
    }

    pub fn get_policy(&self, key: &PacketMeta) -> Option<PolicyAction> {
        self.policies.get(key).copied()
    }

    pub fn get_session(&self, key: &SessionKey) -> Option<SessionValue> {
        self.sessions.get(key).copied()
    }

    pub fn get_xsk(&self, queue_id: u32) -> Option<i32> {
        self.xsks.get(&queue_id).copied()
    }
}

impl PolicyMapWriter for InMemoryMapBackend {
    fn upsert_policy(&mut self, key: PacketMeta, value: PolicyAction) -> Result<(), ZeroGateError> {
        self.policies.insert(key, value);
        Ok(())
    }

    fn remove_policy(&mut self, key: &PacketMeta) -> Result<(), ZeroGateError> {
        // Idempotent: removing a missing key is a no-op.
        self.policies.remove(key);
        Ok(())
    }
}

impl SessionMapWriter for InMemoryMapBackend {
    fn upsert_session(
        &mut self,
        key: SessionKey,
        value: SessionValue,
    ) -> Result<(), ZeroGateError> {
        self.sessions.insert(key, value);
        Ok(())
    }

    fn remove_session(&mut self, key: &SessionKey) -> Result<(), ZeroGateError> {
        // Idempotent: removing a missing key is a no-op.
        self.sessions.remove(key);
        Ok(())
    }
}

impl XskMapWriter for InMemoryMapBackend {
    fn upsert_xsk(&mut self, queue_id: u32, fd: i32) -> Result<(), ZeroGateError> {
        self.xsks.insert(queue_id, fd);
        Ok(())
    }

    fn remove_xsk(&mut self, queue_id: u32) -> Result<(), ZeroGateError> {
        // Idempotent: removing a missing key is a no-op.
        self.xsks.remove(&queue_id);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BpfMapManager — higher-level map orchestration
// ---------------------------------------------------------------------------

/// Orchestrates BPF map updates through a pluggable backend.
///
/// Generic over `B` so unit tests use `InMemoryMapBackend` while
/// production will eventually use a kernel/Aya backend.
#[allow(dead_code)]
pub struct BpfMapManager<B> {
    backend: B,
}

#[allow(dead_code)]
impl<B> BpfMapManager<B>
where
    B: PolicyMapWriter + SessionMapWriter,
{
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Admit a session by writing its key/value into the SESSIONS map.
    pub fn admit_session(&mut self, session_id: u64, xsk_index: u32) -> Result<(), ZeroGateError> {
        let key = SessionKey { session_id };
        let value = SessionValue { xsk_index };
        self.backend.upsert_session(key, value)
    }

    /// Revoke a session by removing its key from the SESSIONS map.
    pub fn revoke_session(&mut self, session_id: u64) -> Result<(), ZeroGateError> {
        let key = SessionKey { session_id };
        self.backend.remove_session(&key)
    }

    /// Set a policy entry in the POLICY map.
    pub fn set_policy(
        &mut self,
        meta: PacketMeta,
        action: PolicyAction,
    ) -> Result<(), ZeroGateError> {
        self.backend.upsert_policy(meta, action)
    }

    /// Remove a policy entry from the POLICY map.
    pub fn remove_policy(&mut self, meta: &PacketMeta) -> Result<(), ZeroGateError> {
        self.backend.remove_policy(meta)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_meta() -> PacketMeta {
        PacketMeta {
            src_ip: 0x0A000001,
            dst_ip: 0x0A000002,
            src_port: 12345,
            dst_port: 80,
            protocol: 6,
            _reserved: [0; 3],
        }
    }

    fn sample_meta_alt() -> PacketMeta {
        PacketMeta {
            src_ip: 0x0A000003,
            dst_ip: 0x0A000004,
            src_port: 54321,
            dst_port: 443,
            protocol: 17,
            _reserved: [0; 3],
        }
    }

    // -- InMemoryMapBackend basics --

    #[test]
    fn backend_starts_empty() {
        let b = InMemoryMapBackend::new();
        assert_eq!(b.policy_count(), 0);
        assert_eq!(b.session_count(), 0);
        assert_eq!(b.xsk_count(), 0);
    }

    #[test]
    fn insert_and_get_policy() {
        let mut b = InMemoryMapBackend::new();
        let meta = sample_meta();
        let action = PolicyAction {
            action: PolicyAction::DROP,
        };
        b.upsert_policy(meta, action).unwrap();
        assert_eq!(b.policy_count(), 1);
        assert_eq!(b.get_policy(&meta), Some(action));
    }

    #[test]
    fn remove_policy_existing() {
        let mut b = InMemoryMapBackend::new();
        let meta = sample_meta();
        b.upsert_policy(
            meta,
            PolicyAction {
                action: PolicyAction::PASS,
            },
        )
        .unwrap();
        b.remove_policy(&meta).unwrap();
        assert_eq!(b.policy_count(), 0);
        assert_eq!(b.get_policy(&meta), None);
    }

    #[test]
    fn remove_policy_missing_is_idempotent() {
        let mut b = InMemoryMapBackend::new();
        let meta = sample_meta();
        assert!(b.remove_policy(&meta).is_ok());
    }

    #[test]
    fn upsert_policy_overwrites() {
        let mut b = InMemoryMapBackend::new();
        let meta = sample_meta();
        b.upsert_policy(
            meta,
            PolicyAction {
                action: PolicyAction::PASS,
            },
        )
        .unwrap();
        b.upsert_policy(
            meta,
            PolicyAction {
                action: PolicyAction::DROP,
            },
        )
        .unwrap();
        assert_eq!(b.policy_count(), 1);
        assert_eq!(
            b.get_policy(&meta),
            Some(PolicyAction {
                action: PolicyAction::DROP
            })
        );
    }

    #[test]
    fn insert_and_get_session() {
        let mut b = InMemoryMapBackend::new();
        let key = SessionKey { session_id: 42 };
        let val = SessionValue { xsk_index: 3 };
        b.upsert_session(key, val).unwrap();
        assert_eq!(b.session_count(), 1);
        assert_eq!(b.get_session(&key), Some(val));
    }

    #[test]
    fn remove_session_existing() {
        let mut b = InMemoryMapBackend::new();
        let key = SessionKey { session_id: 42 };
        b.upsert_session(key, SessionValue { xsk_index: 0 })
            .unwrap();
        b.remove_session(&key).unwrap();
        assert_eq!(b.session_count(), 0);
        assert_eq!(b.get_session(&key), None);
    }

    #[test]
    fn remove_session_missing_is_idempotent() {
        let mut b = InMemoryMapBackend::new();
        let key = SessionKey { session_id: 99 };
        assert!(b.remove_session(&key).is_ok());
    }

    #[test]
    fn upsert_session_overwrites() {
        let mut b = InMemoryMapBackend::new();
        let key = SessionKey { session_id: 7 };
        b.upsert_session(key, SessionValue { xsk_index: 1 })
            .unwrap();
        b.upsert_session(key, SessionValue { xsk_index: 5 })
            .unwrap();
        assert_eq!(b.session_count(), 1);
        assert_eq!(b.get_session(&key), Some(SessionValue { xsk_index: 5 }));
    }

    #[test]
    fn xsk_upsert_and_get() {
        let mut b = InMemoryMapBackend::new();
        b.upsert_xsk(0, 42).unwrap();
        assert_eq!(b.xsk_count(), 1);
        assert_eq!(b.get_xsk(0), Some(42));
    }

    #[test]
    fn xsk_remove_existing() {
        let mut b = InMemoryMapBackend::new();
        b.upsert_xsk(0, 42).unwrap();
        b.remove_xsk(0).unwrap();
        assert_eq!(b.xsk_count(), 0);
        assert_eq!(b.get_xsk(0), None);
    }

    #[test]
    fn xsk_remove_missing_is_idempotent() {
        let mut b = InMemoryMapBackend::new();
        assert!(b.remove_xsk(99).is_ok());
    }

    // -- BpfMapManager --

    #[test]
    fn manager_admit_session() {
        let mut mgr = BpfMapManager::new(InMemoryMapBackend::new());
        mgr.admit_session(100, 2).unwrap();
        let key = SessionKey { session_id: 100 };
        assert_eq!(
            mgr.backend().get_session(&key),
            Some(SessionValue { xsk_index: 2 })
        );
    }

    #[test]
    fn manager_revoke_session() {
        let mut mgr = BpfMapManager::new(InMemoryMapBackend::new());
        mgr.admit_session(100, 2).unwrap();
        mgr.revoke_session(100).unwrap();
        let key = SessionKey { session_id: 100 };
        assert_eq!(mgr.backend().get_session(&key), None);
    }

    #[test]
    fn manager_set_policy() {
        let mut mgr = BpfMapManager::new(InMemoryMapBackend::new());
        let meta = sample_meta();
        let action = PolicyAction {
            action: PolicyAction::REDIRECT,
        };
        mgr.set_policy(meta, action).unwrap();
        assert_eq!(mgr.backend().get_policy(&meta), Some(action));
    }

    #[test]
    fn manager_remove_policy() {
        let mut mgr = BpfMapManager::new(InMemoryMapBackend::new());
        let meta = sample_meta();
        mgr.set_policy(
            meta,
            PolicyAction {
                action: PolicyAction::DROP,
            },
        )
        .unwrap();
        mgr.remove_policy(&meta).unwrap();
        assert_eq!(mgr.backend().get_policy(&meta), None);
    }

    #[test]
    fn multiple_policies() {
        let mut mgr = BpfMapManager::new(InMemoryMapBackend::new());
        let m1 = sample_meta();
        let m2 = sample_meta_alt();
        mgr.set_policy(
            m1,
            PolicyAction {
                action: PolicyAction::DROP,
            },
        )
        .unwrap();
        mgr.set_policy(
            m2,
            PolicyAction {
                action: PolicyAction::PASS,
            },
        )
        .unwrap();
        assert_eq!(mgr.backend().policy_count(), 2);
    }

    // -- Error Display --

    #[test]
    fn new_error_display_strings_are_non_empty() {
        let errors = [
            ZeroGateError::MapUpdateFailed("test".to_string()),
            ZeroGateError::MapDeleteFailed("test".to_string()),
            ZeroGateError::InvalidPolicy("test".to_string()),
            ZeroGateError::InvalidSession("test".to_string()),
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty(), "Display for {err:?} should be non-empty");
        }
    }
}
