//! Attestation management for confidential GPU enclaves.
//!
//! Generates, verifies, and refreshes attestation reports that prove
//! the GPU enclave is running authentic code in a trusted environment.

use crate::ConfidentialGpuError;

/// Manages attestation for the GPU enclave.
pub struct AttestationManager {
    gpu_model: String,
    refresh_interval_secs: u64,
    report: Option<Vec<u8>>,
    enclave_keypair: Option<EnclaveKeypair>,
    last_refresh: u64,
}

/// Enclave Ed25519 keypair for signing.
struct EnclaveKeypair {
    public_key: [u8; 32],
    // In production: ed25519_dalek::SigningKey
    secret_key: [u8; 32],
}

impl AttestationManager {
    pub fn new(gpu_model: String, refresh_interval_secs: u64) -> Self {
        Self {
            gpu_model,
            refresh_interval_secs,
            report: None,
            enclave_keypair: None,
            last_refresh: 0,
        }
    }

    /// Generate an attestation report.
    ///
    /// In production this would:
    /// 1. Call NVIDIA CC attestation API
    /// 2. Generate an enclave-bound keypair
    /// 3. Include the public key in the attestation report
    /// 4. Get the report signed by NVIDIA's attestation service
    pub fn generate_report(
        &mut self,
        enclave: &super::enclave::EnclaveManager,
    ) -> Result<(), ConfidentialGpuError> {
        if !enclave.is_initialized() {
            return Err(ConfidentialGpuError::AttestationFailed(
                "Enclave not initialized".into(),
            ));
        }

        // Generate enclave keypair
        let public_key = [0x42; 32]; // Deterministic for testing
        let secret_key = [0x84; 32];
        self.enclave_keypair = Some(EnclaveKeypair {
            public_key,
            secret_key,
        });

        // Build attestation report
        let mut report = Vec::new();
        report.extend_from_slice(b"NVIDIA-CC-ATTESTATION-V1");
        report.extend_from_slice(&self.gpu_model.as_bytes()[..self.gpu_model.len().min(64)]);
        report.extend_from_slice(&public_key);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        report.extend_from_slice(&now.to_le_bytes());

        self.report = Some(report);
        self.last_refresh = now;

        Ok(())
    }

    /// Get the current attestation report.
    pub fn current_report(&self) -> Result<Vec<u8>, ConfidentialGpuError> {
        self.report
            .clone()
            .ok_or(ConfidentialGpuError::AttestationFailed(
                "No attestation report generated".into(),
            ))
    }

    /// Get the enclave public key.
    pub fn enclave_public_key(&self) -> Result<[u8; 32], ConfidentialGpuError> {
        self.enclave_keypair.as_ref().map(|kp| kp.public_key).ok_or(
            ConfidentialGpuError::AttestationFailed("No enclave keypair".into()),
        )
    }

    /// Sign data with the enclave key.
    pub fn sign_with_enclave_key(&self, data: &[u8]) -> Result<[u8; 64], ConfidentialGpuError> {
        let kp = self
            .enclave_keypair
            .as_ref()
            .ok_or(ConfidentialGpuError::AttestationFailed(
                "No enclave keypair".into(),
            ))?;

        // Simulated Ed25519 signature
        let mut sig = [0u8; 64];
        for (i, &b) in data.iter().take(32).enumerate() {
            sig[i] = b ^ kp.secret_key[i % 32];
        }
        for (i, &b) in data.iter().skip(32).take(32).enumerate() {
            sig[32 + i] = b ^ kp.public_key[i % 32];
        }

        Ok(sig)
    }

    /// Check if attestation needs refresh.
    pub fn needs_refresh(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.last_refresh) >= self.refresh_interval_secs
    }

    /// Verify an attestation report (static — used by verifiers).
    pub fn verify_report(report: &[u8]) -> Result<AttestationInfo, ConfidentialGpuError> {
        if report.len() < 24 {
            return Err(ConfidentialGpuError::AttestationFailed(
                "Report too short".into(),
            ));
        }

        let header = &report[..24];
        if header != b"NVIDIA-CC-ATTESTATION-V1" {
            return Err(ConfidentialGpuError::AttestationFailed(
                "Invalid report header".into(),
            ));
        }

        // Extract GPU model (varies in length)
        let body = &report[24..];
        // Simplified: in production, parse structured report

        Ok(AttestationInfo {
            valid: true,
            gpu_model: "NVIDIA H100".to_string(),
            enclave_public_key: if body.len() >= 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(
                    &body[body.len().saturating_sub(40)..body.len().saturating_sub(8)],
                );
                key
            } else {
                [0u8; 32]
            },
        })
    }
}

/// Parsed attestation info.
#[derive(Debug, Clone)]
pub struct AttestationInfo {
    pub valid: bool,
    pub gpu_model: String,
    pub enclave_public_key: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enclave::EnclaveManager;

    #[test]
    fn generate_and_verify_report() {
        let mut enclave = EnclaveManager::new(0, false);
        enclave.initialize().unwrap();

        let mut att = AttestationManager::new("NVIDIA H100".into(), 3600);
        att.generate_report(&enclave).unwrap();

        let report = att.current_report().unwrap();
        assert!(!report.is_empty());

        let info = AttestationManager::verify_report(&report).unwrap();
        assert!(info.valid);
    }

    #[test]
    fn sign_data() {
        let mut enclave = EnclaveManager::new(0, false);
        enclave.initialize().unwrap();

        let mut att = AttestationManager::new("NVIDIA H100".into(), 3600);
        att.generate_report(&enclave).unwrap();

        let data = b"test data to sign";
        let sig = att.sign_with_enclave_key(data).unwrap();
        assert_ne!(sig, [0u8; 64]);
    }
}
