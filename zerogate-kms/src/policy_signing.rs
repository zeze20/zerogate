// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! Policy signing abstraction.
//!
//! The KMS signs compact policy entries that the agent loads into
//! the eBPF POLICY map. The data plane only sees pre-validated,
//! compact map values — no cryptographic operations on the hot path.

/// A signed policy entry ready for insertion into the BPF POLICY map.
#[derive(Debug, Clone)]
pub struct SignedPolicy {
    /// The compact policy payload (serialized PolicyAction).
    pub payload: Vec<u8>,
    /// Signature over the payload.
    pub signature: Vec<u8>,
}

/// Policy signer trait.
///
/// Implementations provide the actual signing logic (e.g., Ed25519, HMAC).
pub trait PolicySigner {
    /// Signs a policy payload.
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, PolicySignError>;

    /// Verifies a signature over a policy payload.
    fn verify(&self, payload: &[u8], signature: &[u8]) -> Result<bool, PolicySignError>;
}

/// Errors from policy signing operations.
#[derive(Debug)]
pub enum PolicySignError {
    /// Key not loaded.
    KeyNotLoaded,
    /// Invalid signature.
    InvalidSignature,
    /// Internal error.
    Internal(String),
}

impl std::fmt::Display for PolicySignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyNotLoaded => write!(f, "signing key not loaded"),
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for PolicySignError {}

/// Placeholder signer that produces dummy signatures.
///
/// TODO: Replace with a real cryptographic signer.
pub struct PlaceholderSigner;

impl PolicySigner for PlaceholderSigner {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, PolicySignError> {
        // TODO: Real signing.
        let mut sig = vec![0xAA; 32];
        // Simple XOR "signature" for placeholder purposes only.
        for (i, &b) in payload.iter().enumerate() {
            sig[i % 32] ^= b;
        }
        Ok(sig)
    }

    fn verify(&self, payload: &[u8], signature: &[u8]) -> Result<bool, PolicySignError> {
        let expected = self.sign(payload)?;
        Ok(expected == signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_sign_verify() {
        let signer = PlaceholderSigner;
        let payload = b"test policy data";
        let sig = signer.sign(payload).unwrap();
        assert!(signer.verify(payload, &sig).unwrap());
    }

    #[test]
    fn placeholder_verify_rejects_tampered() {
        let signer = PlaceholderSigner;
        let payload = b"test policy data";
        let sig = signer.sign(payload).unwrap();
        let tampered = b"tampered policy data";
        assert!(!signer.verify(tampered, &sig).unwrap());
    }
}
