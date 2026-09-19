//! GPU Signature Verifier Module
//!
//! Implements high-performance signature verification using GPU acceleration
//! for parallel processing of cryptographic signatures.
//!
//! **Status: no verification exists yet.** The previous implementation answered
//! `Ok` whenever the signature string was longer than 64 characters and the
//! payload was non-empty, so any 65-character string passed — and
//! `import-queue-wrapper` used that answer to decide whether a transaction was
//! `Verified`. Every verification now **fails closed** until a real verifier
//! (GPU or otherwise) is wired in. Do not treat this crate as a security
//! boundary.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Signature verification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierConfig {
    pub batch_size: usize,
    pub timeout_seconds: u64,
    pub max_retries: u8,
    pub gpu_device_id: u32,
    pub enable_profiling: bool,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            batch_size: 256,
            timeout_seconds: 30,
            max_retries: 3,
            gpu_device_id: 0,
            enable_profiling: false,
        }
    }
}

/// Signature verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub signature_id: String,
    pub verified: bool,
    pub verification_time_ms: f64,
    pub error_message: Option<String>,
    pub batch_id: u32,
}

/// Batch verification statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStats {
    pub batch_id: u32,
    pub total_signatures: usize,
    pub successful_verifications: usize,
    pub failed_verifications: usize,
    pub average_time_ms: f64,
    pub throughput_sps: f64,
}

/// GPU signature verifier core
pub struct GPUSignatureVerifier {
    config: VerifierConfig,
    gpu_context: Arc<Mutex<GPUContext>>,
    stats: Arc<Mutex<VerificationStats>>,
}

impl GPUSignatureVerifier {
    /// Create a new GPU signature verifier
    pub fn new(config: VerifierConfig) -> Self {
        // `config` is moved into the struct, so read the device id first.
        let device_id = config.gpu_device_id;
        Self {
            config,
            gpu_context: Arc::new(Mutex::new(GPUContext::new(device_id))),
            stats: Arc::new(Mutex::new(VerificationStats::new())),
        }
    }

    /// Verify a single signature
    pub async fn verify_signature(
        &self,
        signature: &str,
        data: &[u8],
    ) -> Result<VerificationResult> {
        let request = VerificationRequest {
            id: generate_signature_id(),
            signature: signature.to_string(),
            data: data.to_vec(),
        };

        self.process_request(request).await
    }

    /// Verify multiple signatures in parallel
    pub async fn verify_signatures(
        &self,
        signatures: Vec<(&str, &[u8])>,
    ) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::with_capacity(signatures.len());
        let mut requests = Vec::with_capacity(signatures.len());

        for (signature, data) in signatures {
            let request = VerificationRequest {
                id: generate_signature_id(),
                signature: signature.to_string(),
                data: data.to_vec(),
            };
            requests.push(request);
        }

        // Process in batches
        for batch in requests.chunks(self.config.batch_size) {
            let batch_results = self.process_batch(batch).await?;
            results.extend(batch_results);
        }

        Ok(results)
    }

    /// Process verification request with retry logic
    async fn process_request(&self, request: VerificationRequest) -> Result<VerificationResult> {
        let mut attempts = 0;
        let mut result = None;
        // Cumulative across every attempt, not just the last one: a request
        // that exhausted `max_retries` retries did all of that work, and a
        // caller reading `verification_time_ms`/the stats average should see
        // the full cost, not just the final attempt's slice of it.
        let request_start = Instant::now();

        while attempts <= self.config.max_retries {
            let verification = self
                .gpu_context
                .lock()
                .map_err(|e| anyhow!("gpu context mutex poisoned: {e}"))?
                .verify(&request);

            let elapsed_time = request_start.elapsed().as_millis() as f64;

            if verification.is_ok() {
                // Count each *request* once, not each retry attempt: the
                // counters used to be updated inside the retry loop, so a
                // request that failed `max_retries + 1` times was counted that
                // many times.
                self.update_stats(true, elapsed_time);
                result = Some(VerificationResult {
                    signature_id: request.id.clone(),
                    verified: true,
                    verification_time_ms: elapsed_time,
                    error_message: None,
                    batch_id: 0,
                });
                break;
            } else {
                attempts += 1;
                if attempts > self.config.max_retries {
                    self.update_stats(false, elapsed_time);
                    result = Some(VerificationResult {
                        signature_id: request.id.clone(),
                        verified: false,
                        verification_time_ms: elapsed_time,
                        error_message: Some(
                            verification
                                .err()
                                .map(|e| e.to_string())
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        ),
                        batch_id: 0,
                    });
                }
            }
        }

        Ok(result.unwrap())
    }

    /// Process batch of verification requests
    async fn process_batch(
        &self,
        requests: &[VerificationRequest],
    ) -> Result<Vec<VerificationResult>> {
        let batch_id = self
            .stats
            .lock()
            .map_err(|e| anyhow!("stats mutex poisoned: {e}"))?
            .next_batch_id;
        let start_time = Instant::now();

        let mut results = Vec::with_capacity(requests.len());
        let mut successful = 0;
        let mut failed = 0;

        for request in requests {
            let verification = self
                .gpu_context
                .lock()
                .map_err(|e| anyhow!("gpu context mutex poisoned: {e}"))?
                .verify(request);
            let elapsed_time = start_time.elapsed().as_millis() as f64;

            if verification.is_ok() {
                successful += 1;
                results.push(VerificationResult {
                    signature_id: request.id.clone(),
                    verified: true,
                    verification_time_ms: elapsed_time,
                    error_message: None,
                    batch_id,
                });
            } else {
                failed += 1;
                results.push(VerificationResult {
                    signature_id: request.id.clone(),
                    verified: false,
                    verification_time_ms: elapsed_time,
                    error_message: Some(
                        verification
                            .err()
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    ),
                    batch_id,
                });
            }
        }

        // Calculate batch statistics
        let total_time = start_time.elapsed().as_millis() as f64;
        let throughput = (requests.len() as f64 / (total_time / 1000.0)).max(1.0);

        // Record per-request outcomes in the same global counters
        // `process_request` (the single-signature path) updates — otherwise
        // `get_stats()` stays at zero for every batch verification, while
        // `get_batch_stats()` alone shows real numbers.
        for result in &results {
            self.update_stats(result.verified, result.verification_time_ms);
        }

        self.update_batch_stats(
            batch_id,
            requests.len(),
            successful,
            failed,
            total_time / requests.len() as f64,
            throughput,
        );

        Ok(results)
    }

    /// Update verification statistics
    fn update_stats(&self, verified: bool, time_ms: f64) {
        let mut stats = match self.stats.lock() {
            Ok(stats) => stats,
            // A poisoned stats mutex must not stop verification from failing
            // closed; the counters are diagnostics, not the verdict.
            Err(poisoned) => poisoned.into_inner(),
        };
        stats.total_verifications += 1;
        stats.total_time_ms += time_ms;

        if verified {
            stats.successful_verifications += 1;
        } else {
            stats.failed_verifications += 1;
        }

        stats.average_time_ms = stats.total_time_ms / stats.total_verifications as f64;
    }

    /// Update batch statistics
    fn update_batch_stats(
        &self,
        batch_id: u32,
        total: usize,
        successful: usize,
        failed: usize,
        avg_time: f64,
        throughput: f64,
    ) {
        let mut stats = match self.stats.lock() {
            Ok(stats) => stats,
            Err(poisoned) => poisoned.into_inner(),
        };
        stats.batch_stats.push(BatchStats {
            batch_id,
            total_signatures: total,
            successful_verifications: successful,
            failed_verifications: failed,
            average_time_ms: avg_time,
            throughput_sps: throughput,
        });
        stats.next_batch_id += 1;
    }

    /// Get verification statistics
    pub async fn get_stats(&self) -> VerificationStats {
        match self.stats.lock() {
            Ok(stats) => stats.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Get batch statistics
    pub async fn get_batch_stats(&self) -> Vec<BatchStats> {
        match self.stats.lock() {
            Ok(stats) => stats.batch_stats.clone(),
            Err(poisoned) => poisoned.into_inner().batch_stats.clone(),
        }
    }

    /// Clear statistics
    pub async fn clear_stats(&self) {
        match self.stats.lock() {
            Ok(mut stats) => stats.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }
}

/// GPU context for signature verification
struct GPUContext {
    device_id: u32,
    // GPU resources would be managed here
}

impl GPUContext {
    fn new(device_id: u32) -> Self {
        Self { device_id }
    }

    /// Refuse every request.
    ///
    /// There is no GPU (or CPU) verifier behind this type yet. The previous
    /// body returned `Ok(())` for `signature.len() > 64 && !data.is_empty()`,
    /// which accepts `signature = "x".repeat(65)` as valid for any payload;
    /// `import-queue-wrapper` then marked the transaction `Verified` and moved
    /// it to `ReadyForInclusion`. "Not verified" is the only honest verdict
    /// until a real implementation exists.
    fn verify(&self, _request: &VerificationRequest) -> Result<()> {
        Err(anyhow!(
            "GPU signature verification is not implemented (requested device {}); refusing to verify {} ({} signature byte(s) over {} payload byte(s))",
            self.device_id,
            _request.id,
            _request.signature.len(),
            _request.data.len()
        ))
    }
}

/// Verification request structure
#[derive(Debug, Clone)]
struct VerificationRequest {
    id: String,
    signature: String,
    data: Vec<u8>,
}

/// Global verification statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStats {
    pub total_verifications: usize,
    pub successful_verifications: usize,
    pub failed_verifications: usize,
    pub total_time_ms: f64,
    pub average_time_ms: f64,
    pub next_batch_id: u32,
    pub batch_stats: Vec<BatchStats>,
}

impl VerificationStats {
    fn new() -> Self {
        Self {
            total_verifications: 0,
            successful_verifications: 0,
            failed_verifications: 0,
            total_time_ms: 0.0,
            average_time_ms: 0.0,
            next_batch_id: 1,
            batch_stats: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.total_verifications = 0;
        self.successful_verifications = 0;
        self.failed_verifications = 0;
        self.total_time_ms = 0.0;
        self.average_time_ms = 0.0;
        self.next_batch_id = 1;
        self.batch_stats.clear();
    }
}

/// Generate unique signature ID
fn generate_signature_id() -> String {
    format!("{}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn verifier_fails_closed_for_every_signature() {
        let config = VerifierConfig::default();
        let verifier = GPUSignatureVerifier::new(config);

        // The exploit this crate used to accept: any string longer than 64
        // characters was reported as a verified signature for any payload.
        let forged = "x".repeat(65);
        let result = verifier
            .verify_signature(&forged, b"test_data")
            .await
            .unwrap();
        assert!(
            !result.verified,
            "a 65-character string must not verify as a signature"
        );
        assert!(
            result
                .error_message
                .as_deref()
                .is_some_and(|m| m.contains("not implemented")),
            "the failure must say why: {:?}",
            result.error_message
        );

        let result = verifier.verify_signature("", b"").await.unwrap();
        assert!(!result.verified);
    }

    #[tokio::test]
    async fn batch_verification_reports_no_successes() {
        let verifier = GPUSignatureVerifier::new(VerifierConfig::default());
        let long_a = "a".repeat(128);
        let long_b = "b".repeat(128);
        let payload: &[u8] = b"payload";
        let signatures: Vec<(&str, &[u8])> = vec![
            (long_a.as_str(), payload),
            (long_b.as_str(), payload),
            ("", payload),
        ];
        let results = verifier.verify_signatures(signatures).await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(
            results.iter().filter(|r| r.verified).count(),
            0,
            "nothing may be reported verified while no verifier exists"
        );
    }

    #[tokio::test]
    async fn stats_count_failures_not_successes() {
        let config = VerifierConfig::default();
        let verifier = GPUSignatureVerifier::new(config);

        verifier.verify_signature("sig1", b"data1").await.unwrap();
        verifier.verify_signature("sig2", b"data2").await.unwrap();
        verifier.verify_signature("invalid", b"").await.unwrap();

        let stats = verifier.get_stats().await;
        assert_eq!(stats.total_verifications, 3);
        assert_eq!(stats.successful_verifications, 0);
        assert_eq!(stats.failed_verifications, 3);
    }

    #[tokio::test]
    async fn batch_verification_updates_global_stats_too() {
        let verifier = GPUSignatureVerifier::new(VerifierConfig::default());
        let payload: &[u8] = b"payload";
        let signatures: Vec<(&str, &[u8])> =
            vec![("sig1", payload), ("sig2", payload), ("sig3", payload)];

        verifier.verify_signatures(signatures).await.unwrap();

        // Before this fix, process_batch only recorded batch_stats and left
        // the global counters get_stats() reports at zero.
        let stats = verifier.get_stats().await;
        assert_eq!(stats.total_verifications, 3);
        assert_eq!(stats.failed_verifications, 3);

        let batch_stats = verifier.get_batch_stats().await;
        assert_eq!(batch_stats.len(), 1);
        assert_eq!(batch_stats[0].total_signatures, 3);
    }
}
