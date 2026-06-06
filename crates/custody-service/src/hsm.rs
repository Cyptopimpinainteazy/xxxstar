/// HSM (Hardware Security Module) integration layer
/// Handles key management, signing, and cryptographic operations
use crate::error::{CustodyError, Result};
use crate::types::{HSMKeyReference, VaultOperationCommand};
use chrono::Utc;
use sha2::{Digest, Sha256};

/// HSM backend abstraction
#[async_trait::async_trait]
pub trait HSMBackend: Send + Sync {
    /// Sign data with HSM key
    async fn sign(&self, key_id: &str, data: &[u8]) -> Result<Vec<u8>>;

    /// Verify signature with HSM key
    async fn verify(&self, key_id: &str, data: &[u8], signature: &[u8]) -> Result<bool>;

    /// Generate new key pair in HSM
    async fn generate_key(&self, key_id: &str, algorithm: &str) -> Result<HSMKeyReference>;

    /// List all available keys
    async fn list_keys(&self) -> Result<Vec<HSMKeyReference>>;

    /// Rotate key
    async fn rotate_key(&self, key_id: &str) -> Result<HSMKeyReference>;
}

/// Mock HSM backend for testing (never use in production)
pub struct MockHSM {
    keys: parking_lot::RwLock<std::collections::HashMap<String, HSMKeyReference>>,
}

impl Default for MockHSM {
    fn default() -> Self {
        Self::new()
    }
}

impl MockHSM {
    pub fn new() -> Self {
        Self {
            keys: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl HSMBackend for MockHSM {
    async fn sign(&self, key_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        let keys = self.keys.read();
        if !keys.contains_key(key_id) {
            return Err(CustodyError::KeyNotFound(key_id.to_string()));
        }
        // In mock, just return HMAC-like signature
        let mut hasher = Sha256::new();
        hasher.update(key_id.as_bytes());
        hasher.update(data);
        Ok(hasher.finalize().to_vec())
    }

    async fn verify(&self, key_id: &str, data: &[u8], signature: &[u8]) -> Result<bool> {
        let expected = self.sign(key_id, data).await?;
        Ok(expected == signature)
    }

    async fn generate_key(&self, key_id: &str, algorithm: &str) -> Result<HSMKeyReference> {
        let now = Utc::now().timestamp_millis() as u64;
        let key_ref = HSMKeyReference {
            key_id: key_id.to_string(),
            algorithm: algorithm.to_string(),
            created_at_ms: now,
            last_rotated_at_ms: now,
            is_vault_key: true,
        };
        self.keys
            .write()
            .insert(key_id.to_string(), key_ref.clone());
        Ok(key_ref)
    }

    async fn list_keys(&self) -> Result<Vec<HSMKeyReference>> {
        Ok(self.keys.read().values().cloned().collect())
    }

    async fn rotate_key(&self, key_id: &str) -> Result<HSMKeyReference> {
        let mut keys = self.keys.write();
        let mut key = keys.get(key_id).ok_err_mapped()?.clone();
        key.last_rotated_at_ms = Utc::now().timestamp_millis() as u64;
        keys.insert(key_id.to_string(), key.clone());
        Ok(key)
    }
}

trait ErrMapped<T> {
    fn ok_err_mapped(self) -> Result<T>;
}

impl<T> ErrMapped<T> for Option<T> {
    fn ok_err_mapped(self) -> Result<T> {
        self.ok_or_else(|| CustodyError::Internal("option unwrap failed".to_string()))
    }
}

/// HSM-backed operation signer
pub struct HSMSigner {
    backend: Box<dyn HSMBackend>,
    vault_key_id: String,
}

impl HSMSigner {
    pub fn new(backend: Box<dyn HSMBackend>, vault_key_id: String) -> Self {
        Self {
            backend,
            vault_key_id,
        }
    }

    /// Create operation proof by signing the command
    pub async fn sign_operation(&self, cmd: &VaultOperationCommand) -> Result<Vec<u8>> {
        let payload = serde_json::to_vec(cmd)
            .map_err(|e| CustodyError::Internal(format!("serialization failed: {}", e)))?;
        self.backend.sign(&self.vault_key_id, &payload).await
    }

    /// Verify operation proof
    pub async fn verify_operation_proof(
        &self,
        cmd: &VaultOperationCommand,
        proof: &[u8],
    ) -> Result<bool> {
        let payload = serde_json::to_vec(cmd)
            .map_err(|e| CustodyError::Internal(format!("serialization failed: {}", e)))?;
        self.backend
            .verify(&self.vault_key_id, &payload, proof)
            .await
    }

    /// Compute merkle root of vault state for settlement proof
    pub fn compute_state_merkle_root(
        vault_id: &str,
        available: u128,
        reserved: u128,
        pending_out: u128,
        pending_in: u128,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(vault_id.as_bytes());
        hasher.update(available.to_le_bytes());
        hasher.update(reserved.to_le_bytes());
        hasher.update(pending_out.to_le_bytes());
        hasher.update(pending_in.to_le_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_hsm_key_generation() {
        let hsm = MockHSM::new();
        let key = hsm.generate_key("test-key-1", "ECDSA-P256").await.unwrap();
        assert_eq!(key.key_id, "test-key-1");
        assert_eq!(key.algorithm, "ECDSA-P256");
        assert!(key.is_vault_key);
    }

    #[tokio::test]
    async fn test_hsm_sign_and_verify() {
        let hsm = MockHSM::new();
        let _ = hsm.generate_key("test-key-2", "ECDSA-P256").await.unwrap();

        let data = b"test data";
        let signature = hsm.sign("test-key-2", data).await.unwrap();
        let verified = hsm.verify("test-key-2", data, &signature).await.unwrap();
        assert!(verified);
    }

    #[tokio::test]
    async fn test_hsm_key_rotation() {
        let hsm = MockHSM::new();
        let key1 = hsm.generate_key("test-key-3", "ECDSA-P256").await.unwrap();
        let key2 = hsm.rotate_key("test-key-3").await.unwrap();
        assert_eq!(key1.key_id, key2.key_id);
        assert!(key2.last_rotated_at_ms >= key1.last_rotated_at_ms);
    }

    #[test]
    fn test_merkle_root_computation() {
        let root1 = HSMSigner::compute_state_merkle_root("vault-1", 100, 50, 10, 5);
        let root2 = HSMSigner::compute_state_merkle_root("vault-1", 100, 50, 10, 5);
        assert_eq!(root1, root2); // Deterministic

        let root3 = HSMSigner::compute_state_merkle_root("vault-1", 99, 50, 10, 5);
        assert_ne!(root1, root3); // Changes with state
    }
}
