// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Key loading and storage interface.
//!
//! The keyring provides a secure interface for loading and retrieving
//! cryptographic keys. No key material is ever passed to the eBPF program.
//! The agent receives already-validated compact policy entries.

/// Identifier for a stored key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyId(pub String);

/// Key material (opaque bytes).
///
/// In a production system, this would be stored in a secure enclave
/// or HSM. For the placeholder, we store it in memory.
pub struct KeyMaterial {
    id: KeyId,
    data: Vec<u8>,
}

impl KeyMaterial {
    pub fn new(id: KeyId, data: Vec<u8>) -> Self {
        Self { id, data }
    }

    pub fn id(&self) -> &KeyId {
        &self.id
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Key loading interface.
pub trait KeyLoader {
    /// Loads a key by its identifier.
    fn load_key(&self, id: &KeyId) -> Result<KeyMaterial, KeyringError>;

    /// Lists available key identifiers.
    fn list_keys(&self) -> Result<Vec<KeyId>, KeyringError>;
}

/// Errors from keyring operations.
#[derive(Debug)]
pub enum KeyringError {
    KeyNotFound(KeyId),
    AccessDenied,
    Internal(String),
}

impl std::fmt::Display for KeyringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyNotFound(id) => write!(f, "key not found: {}", id.0),
            Self::AccessDenied => write!(f, "access denied"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for KeyringError {}

/// In-memory keyring for development/testing.
///
/// TODO: Replace with secure key storage in production.
pub struct InMemoryKeyring {
    keys: std::collections::HashMap<KeyId, Vec<u8>>,
}

impl InMemoryKeyring {
    pub fn new() -> Self {
        Self {
            keys: std::collections::HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: KeyId, data: Vec<u8>) {
        self.keys.insert(id, data);
    }
}

impl KeyLoader for InMemoryKeyring {
    fn load_key(&self, id: &KeyId) -> Result<KeyMaterial, KeyringError> {
        let data = self
            .keys
            .get(id)
            .ok_or_else(|| KeyringError::KeyNotFound(id.clone()))?;
        Ok(KeyMaterial::new(id.clone(), data.clone()))
    }

    fn list_keys(&self) -> Result<Vec<KeyId>, KeyringError> {
        Ok(self.keys.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_keyring_load() {
        let mut kr = InMemoryKeyring::new();
        let id = KeyId("test-key".into());
        kr.insert(id.clone(), vec![1, 2, 3]);

        let key = kr.load_key(&id).unwrap();
        assert_eq!(key.data(), &[1, 2, 3]);
    }

    #[test]
    fn in_memory_keyring_not_found() {
        let kr = InMemoryKeyring::new();
        let id = KeyId("missing".into());
        assert!(kr.load_key(&id).is_err());
    }

    #[test]
    fn in_memory_keyring_list() {
        let mut kr = InMemoryKeyring::new();
        kr.insert(KeyId("a".into()), vec![]);
        kr.insert(KeyId("b".into()), vec![]);
        let keys = kr.list_keys().unwrap();
        assert_eq!(keys.len(), 2);
    }
}
