//! Mobile transaction signing
//! 
//! Handles transaction signing on-device without exposing private keys.
//! Supports ED25519 and ECDSA signatures.

use crate::SdkError;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Supported signature algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    ED25519,
    ECDSA, // secp256k1
}

/// Signing request from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningRequest {
    pub request_id: String,
    pub payload: Vec<u8>,
    pub algorithm: SignatureAlgorithm,
    pub account_address: String,
    pub request_time: i64,
    pub expires_at: i64,
}

impl SigningRequest {
    /// Check if request has expired
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now > self.expires_at
    }

    /// Get remaining time before expiration (in seconds)
    pub fn remaining_seconds(&self) -> i64 {
        let now = chrono::Utc::now().timestamp();
        (self.expires_at - now).max(0)
    }
}

/// Signed transaction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub request_id: String,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
    pub algorithm: SignatureAlgorithm,
    pub signed_at: i64,
}

/// Signing queue for pending requests
#[derive(Debug, Clone)]
struct SigningQueueEntry {
    request: SigningRequest,
    priority: SigningPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SigningPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Mobile transaction signer
pub struct MobileTransactionSigner {
    // Private keys stored securely (in production: iOS Secure Enclave / Android KeyStore)
    private_keys: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    
    // Pending signing requests queue
    signing_queue: tokio::sync::Mutex<Vec<SigningQueueEntry>>,
    
    // Signing timeout (120 seconds = 2 minutes)
    signing_timeout: i64,
}

impl MobileTransactionSigner {
    /// Create new signer
    pub fn new(signing_timeout_seconds: i64) -> Self {
        Self {
            private_keys: std::sync::Mutex::new(std::collections::HashMap::new()),
            signing_queue: tokio::sync::Mutex::new(Vec::new()),
            signing_timeout: signing_timeout_seconds,
        }
    }

    /// Store private key for an account (in production: encrypted in Keystore)
    pub async fn add_account(
        &self,
        address: &str,
        private_key: &[u8],
        algorithm: SignatureAlgorithm,
    ) -> Result<(), SdkError> {
        if private_key.is_empty() || private_key.len() > 256 {
            return Err(SdkError::SigningError("Invalid key size".to_string()));
        }

        let mut keys = self.private_keys.lock().expect("private_keys mutex poisoned");
        
        // Store with algorithm prefix
        let key_id = format!("{}:{:?}", address, algorithm);
        keys.insert(key_id, private_key.to_vec());

        tracing::info!("Added signing account: {}", address);
        Ok(())
    }

    /// Remove account (secure deletion)
    pub async fn remove_account(&self, address: &str) -> Result<(), SdkError> {
        let mut keys = self.private_keys.lock().expect("private_keys mutex poisoned");
        
        // Remove all algorithm variants
        keys.retain(|k, v| {
            if k.starts_with(address) {
                v.zeroize(); // Secure erase
                false
            } else {
                true
            }
        });

        tracing::info!("Removed signing account: {}", address);
        Ok(())
    }

    /// Create signing request
    pub fn create_signing_request(
        &self,
        payload: Vec<u8>,
        account: String,
        algorithm: SignatureAlgorithm,
    ) -> SigningRequest {
        let now = chrono::Utc::now().timestamp();
        
        SigningRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload,
            algorithm,
            account_address: account,
            request_time: now,
            expires_at: now + self.signing_timeout,
        }
    }

    /// Queue a signing request (user reviews and approves)
    pub async fn queue_signing_request(&self, request: SigningRequest) -> Result<String, SdkError> {
        if request.is_expired() {
            return Err(SdkError::SigningError("Request expired".to_string()));
        }

        let request_id = request.request_id.clone();
        
        let entry = SigningQueueEntry {
            request,
            priority: SigningPriority::Normal,
        };

        self.signing_queue.lock().await.push(entry);

        tracing::info!("Queued signing request: {}", request_id);
        Ok(request_id)
    }

    /// Get pending signing requests
    pub async fn get_pending_requests(&self) -> Result<Vec<SigningRequest>, SdkError> {
        let mut queue = self.signing_queue.lock().await;
        
        // Remove expired requests
        queue.retain(|entry| !entry.request.is_expired());
        
        // Sort by priority
        queue.sort_by_key(|entry| std::cmp::Reverse(entry.priority));

        Ok(queue.iter().map(|entry| entry.request.clone()).collect())
    }

    /// Approve and sign a request
    pub async fn approve_and_sign(
        &self,
        request_id: &str,
    ) -> Result<SignedTransaction, SdkError> {
        let mut queue = self.signing_queue.lock().await;

        if let Some(pos) = queue.iter().position(|entry| entry.request.request_id == request_id) {
            let entry = queue.remove(pos);
            let request = entry.request;

            // Find private key
            let keys = self.private_keys.lock().expect("private_keys mutex poisoned");
            let key_id = format!("{}:{:?}", request.account_address, request.algorithm);
            
            let private_key = keys
                .get(&key_id)
                .ok_or_else(|| SdkError::SigningError("Account not found".to_string()))?;

            // Sign payload
            let signature = match request.algorithm {
                SignatureAlgorithm::ED25519 => {
                    sign_ed25519(private_key, &request.payload)?
                }
                SignatureAlgorithm::ECDSA => {
                    sign_ecdsa(private_key, &request.payload)?
                }
            };

            // Derive public key from private key
            let public_key = derive_public_key(&request.algorithm, private_key)?;

            let signed_tx = SignedTransaction {
                request_id: request_id.to_string(),
                signature,
                public_key,
                algorithm: request.algorithm,
                signed_at: chrono::Utc::now().timestamp(),
            };

            tracing::info!("Signed transaction: {}", request_id);
            Ok(signed_tx)
        } else {
            Err(SdkError::SigningError(
                "Signing request not found".to_string(),
            ))
        }
    }

    /// Reject a signing request
    pub async fn reject_signing_request(&self, request_id: &str) -> Result<(), SdkError> {
        let mut queue = self.signing_queue.lock().await;

        if let Some(pos) = queue.iter().position(|entry| entry.request.request_id == request_id) {
            let _ = queue.remove(pos);
            tracing::info!("Rejected signing request: {}", request_id);
            Ok(())
        } else {
            Err(SdkError::SigningError(
                "Signing request not found".to_string(),
            ))
        }
    }

    /// Bulk sign requests (batch signing)
    pub async fn batch_sign(&self, request_ids: Vec<String>) -> Result<Vec<SignedTransaction>, SdkError> {
        let mut results = Vec::new();

        for request_id in request_ids {
            match self.approve_and_sign(&request_id).await {
                Ok(signed_tx) => results.push(signed_tx),
                Err(e) => {
                    tracing::warn!("Failed to sign {}: {}", request_id, e);
                    // Continue with other requests
                }
            }
        }

        Ok(results)
    }

    /// Verify a signature
    pub async fn verify_signature(
        &self,
        payload: &[u8],
        signature: &[u8],
        public_key: &[u8],
        algorithm: SignatureAlgorithm,
    ) -> Result<bool, SdkError> {
        match algorithm {
            SignatureAlgorithm::ED25519 => {
                verify_ed25519_sig(payload, signature, public_key)
            }
            SignatureAlgorithm::ECDSA => {
                verify_ecdsa_sig(payload, signature, public_key)
            }
        }
    }

    /// Clear all pending requests
    pub async fn clear_queue(&self) -> Result<(), SdkError> {
        self.signing_queue.lock().await.clear();
        tracing::info!("Cleared signing queue");
        Ok(())
    }

    /// Get queue size
    pub async fn queue_size(&self) -> Result<usize, SdkError> {
        Ok(self.signing_queue.lock().await.len())
    }
}

// ============================================================================
// Cryptographic signing functions (placeholder implementations)
// In production: use proper cryptographic libraries
// ============================================================================

fn sign_ed25519(private_key: &[u8], payload: &[u8]) -> Result<Vec<u8>, SdkError> {
    // In production: use ed25519-zebra or similar
    // For now: hash and return as placeholder
    let mut hasher = sha2::Sha512::new();
    hasher.update(private_key);
    hasher.update(payload);
    Ok(hasher.finalize().to_vec())
}

fn sign_ecdsa(private_key: &[u8], payload: &[u8]) -> Result<Vec<u8>, SdkError> {
    // In production: use k256 or similar
    let mut hasher = sha2::Sha256::new();
    hasher.update(private_key);
    hasher.update(payload);
    Ok(hasher.finalize().to_vec())
}

fn derive_public_key(algorithm: &SignatureAlgorithm, private_key: &[u8]) -> Result<Vec<u8>, SdkError> {
    // In production: proper key derivation
    let mut hasher = sha2::Sha256::new();
    hasher.update(private_key);
    hasher.update(algorithm.to_string().as_bytes());
    Ok(hasher.finalize().to_vec())
}

fn verify_ed25519_sig(payload: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, SdkError> {
    // Placeholder: verify hash matches
    let mut hasher = sha2::Sha512::new();
    hasher.update(public_key);
    hasher.update(payload);
    Ok(hasher.finalize().to_vec() == signature)
}

fn verify_ecdsa_sig(payload: &[u8], signature: &[u8], public_key: &[u8]) -> Result<bool, SdkError> {
    // Placeholder: verify hash matches
    let mut hasher = sha2::Sha256::new();
    hasher.update(public_key);
    hasher.update(payload);
    Ok(hasher.finalize().to_vec() == signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signing_request_validity() {
        let request = SigningRequest {
            request_id: "req1".to_string(),
            payload: vec![1, 2, 3],
            algorithm: SignatureAlgorithm::ED25519,
            account_address: "x3:account".to_string(),
            request_time: 0,
            expires_at: 999999999,
        };
        assert!(!request.is_expired());
    }

    #[test]
    fn test_signing_request_expired() {
        let request = SigningRequest {
            request_id: "req1".to_string(),
            payload: vec![1, 2, 3],
            algorithm: SignatureAlgorithm::ED25519,
            account_address: "x3:account".to_string(),
            request_time: 0,
            expires_at: 0,
        };
        assert!(request.is_expired());
    }

    #[tokio::test]
    async fn test_signer_creation() {
        let signer = MobileTransactionSigner::new(120);
        let queue = signer.get_pending_requests().await.unwrap();
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn test_add_account() {
        let signer = MobileTransactionSigner::new(120);
        let private_key = vec![1, 2, 3, 4, 5];

        let result = signer
            .add_account("x3:account", &private_key, SignatureAlgorithm::ED25519)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_signing_request() {
        let signer = MobileTransactionSigner::new(120);
        
        let request = signer.create_signing_request(
            vec![1, 2, 3],
            "x3:account".to_string(),
            SignatureAlgorithm::ED25519,
        );

        assert_eq!(request.account_address, "x3:account");
        assert!(!request.is_expired());
    }

    #[tokio::test]
    async fn test_queue_signing_request() {
        let signer = MobileTransactionSigner::new(120);
        
        let request = signer.create_signing_request(
            vec![1, 2, 3],
            "x3:account".to_string(),
            SignatureAlgorithm::ED25519,
        );

        let request_id = request.request_id.clone();
        let result = signer.queue_signing_request(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), request_id);
    }

    #[tokio::test]
    async fn test_get_pending_requests() {
        let signer = MobileTransactionSigner::new(120);
        
        let request = signer.create_signing_request(
            vec![1, 2, 3],
            "x3:account".to_string(),
            SignatureAlgorithm::ECDSA,
        );

        signer.queue_signing_request(request.clone()).await.unwrap();
        
        let pending = signer.get_pending_requests().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request_id, request.request_id);
    }

    #[tokio::test]
    async fn test_reject_signing_request() {
        let signer = MobileTransactionSigner::new(120);
        
        let request = signer.create_signing_request(
            vec![1, 2, 3],
            "x3:account".to_string(),
            SignatureAlgorithm::ED25519,
        );

        let request_id = request.request_id.clone();
        signer.queue_signing_request(request).await.unwrap();
        
        let result = signer.reject_signing_request(&request_id).await;
        assert!(result.is_ok());

        let pending = signer.get_pending_requests().await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_remove_account() {
        let signer = MobileTransactionSigner::new(120);
        let private_key = vec![1, 2, 3, 4, 5];

        signer
            .add_account("x3:account", &private_key, SignatureAlgorithm::ED25519)
            .await
            .unwrap();

        let result = signer.remove_account("x3:account").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_queue_size() {
        let signer = MobileTransactionSigner::new(120);
        
        let request1 = signer.create_signing_request(
            vec![1],
            "x3:account".to_string(),
            SignatureAlgorithm::ED25519,
        );
        let request2 = signer.create_signing_request(
            vec![2],
            "x3:account".to_string(),
            SignatureAlgorithm::ECDSA,
        );

        signer.queue_signing_request(request1).await.unwrap();
        signer.queue_signing_request(request2).await.unwrap();

        let size = signer.queue_size().await.unwrap();
        assert_eq!(size, 2);
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let signer = MobileTransactionSigner::new(120);
        
        let request = signer.create_signing_request(
            vec![1, 2, 3],
            "x3:account".to_string(),
            SignatureAlgorithm::ED25519,
        );

        signer.queue_signing_request(request).await.unwrap();
        signer.clear_queue().await.unwrap();

        let pending = signer.get_pending_requests().await.unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_signature_algorithm_enum() {
        assert_eq!(SignatureAlgorithm::ED25519, SignatureAlgorithm::ED25519);
        assert_ne!(SignatureAlgorithm::ED25519, SignatureAlgorithm::ECDSA);
    }
}
