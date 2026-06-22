//! Pure policy/session data types and snapshot application.
//!
//! This module is intentionally free of syscalls, unsafe, global state,
//! and kernel map handles. It converts compact policy representations
//! into map writes via `BpfMapManager`.

use zerogate_common::abi::{PacketMeta, PolicyAction, SessionKey, SessionValue};

use crate::error::ZeroGateError;
use crate::maps::{BpfMapManager, PolicyMapWriter, SessionMapWriter};

/// A point-in-time snapshot of policies and sessions to be applied
/// to BPF maps.
#[allow(dead_code)]
pub struct PolicySnapshot {
    pub policies: Vec<(PacketMeta, PolicyAction)>,
    pub sessions: Vec<(SessionKey, SessionValue)>,
}

#[allow(dead_code)]
impl PolicySnapshot {
    pub fn empty() -> Self {
        Self {
            policies: Vec::new(),
            sessions: Vec::new(),
        }
    }

    pub fn with_policy(mut self, meta: PacketMeta, action: PolicyAction) -> Self {
        self.policies.push((meta, action));
        self
    }

    pub fn with_session(mut self, key: SessionKey, value: SessionValue) -> Self {
        self.sessions.push((key, value));
        self
    }
}

/// Apply all entries from a `PolicySnapshot` into the given `BpfMapManager`.
#[allow(dead_code)]
pub fn apply_policy_snapshot<B>(
    manager: &mut BpfMapManager<B>,
    snapshot: &PolicySnapshot,
) -> Result<(), ZeroGateError>
where
    B: PolicyMapWriter + SessionMapWriter,
{
    for (meta, action) in &snapshot.policies {
        manager.set_policy(*meta, *action)?;
    }
    for (key, value) in &snapshot.sessions {
        manager.backend_mut().upsert_session(*key, *value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::InMemoryMapBackend;

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

    #[test]
    fn empty_snapshot_has_no_entries() {
        let snap = PolicySnapshot::empty();
        assert!(snap.policies.is_empty());
        assert!(snap.sessions.is_empty());
    }

    #[test]
    fn snapshot_builder_adds_policy() {
        let snap = PolicySnapshot::empty().with_policy(
            sample_meta(),
            PolicyAction {
                action: PolicyAction::DROP,
            },
        );
        assert_eq!(snap.policies.len(), 1);
    }

    #[test]
    fn snapshot_builder_adds_session() {
        let snap = PolicySnapshot::empty()
            .with_session(SessionKey { session_id: 1 }, SessionValue { xsk_index: 0 });
        assert_eq!(snap.sessions.len(), 1);
    }

    #[test]
    fn apply_snapshot_writes_entries() {
        let snap = PolicySnapshot::empty()
            .with_policy(
                sample_meta(),
                PolicyAction {
                    action: PolicyAction::DROP,
                },
            )
            .with_session(SessionKey { session_id: 42 }, SessionValue { xsk_index: 1 });

        let mut mgr = BpfMapManager::new(InMemoryMapBackend::new());
        apply_policy_snapshot(&mut mgr, &snap).unwrap();

        assert_eq!(mgr.backend().policy_count(), 1);
        assert_eq!(mgr.backend().session_count(), 1);
        assert_eq!(
            mgr.backend().get_policy(&sample_meta()),
            Some(PolicyAction {
                action: PolicyAction::DROP
            })
        );
        assert_eq!(
            mgr.backend().get_session(&SessionKey { session_id: 42 }),
            Some(SessionValue { xsk_index: 1 })
        );
    }

    #[test]
    fn apply_empty_snapshot_is_noop() {
        let snap = PolicySnapshot::empty();
        let mut mgr = BpfMapManager::new(InMemoryMapBackend::new());
        apply_policy_snapshot(&mut mgr, &snap).unwrap();
        assert_eq!(mgr.backend().policy_count(), 0);
        assert_eq!(mgr.backend().session_count(), 0);
    }
}
