use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::proof::ProofResult;

/// Receipt for a proof execution with cryptographic binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Git commit hash at time of proof
    pub repo_commit_hash: String,

    /// Command that generated this receipt
    pub command_run: String,

    /// SHA256 hash of relevant artifacts
    pub artifact_hash: String,

    /// SHA256 hash of policy files
    pub policy_hash: String,

    /// Files that were inspected/tested
    pub relevant_files: Vec<PathBuf>,

    /// Timestamp of proof execution (ISO 8601)
    pub timestamp: DateTime<Utc>,

    /// Proof result
    pub result: ProofResult,

    /// Known limitations of this proof
    pub limitations: Vec<String>,

    /// Cryptographic binding hash (SHA256 of all fields)
    pub binding_hash: String,
}

impl Receipt {
    /// Create a new receipt from a proof result
    pub fn new(
        command: String,
        result: ProofResult,
        relevant_files: Vec<PathBuf>,
        limitations: Vec<String>,
    ) -> Result<Self> {
        let repo_commit_hash = get_git_commit_hash()?;
        let artifact_hash = compute_artifact_hash(&relevant_files)?;
        let policy_hash = compute_policy_hash()?;
        let timestamp = Utc::now();

        let mut receipt = Receipt {
            repo_commit_hash,
            command_run: command,
            artifact_hash,
            policy_hash,
            relevant_files,
            timestamp,
            result,
            limitations,
            binding_hash: String::new(), // Will be computed next
        };

        // Compute binding hash over all fields
        receipt.binding_hash = receipt.compute_binding_hash()?;

        Ok(receipt)
    }

    /// Compute cryptographic binding hash
    fn compute_binding_hash(&self) -> Result<String> {
        let mut hasher = Sha256::new();

        // Hash all fields in deterministic order
        hasher.update(self.repo_commit_hash.as_bytes());
        hasher.update(self.command_run.as_bytes());
        hasher.update(self.artifact_hash.as_bytes());
        hasher.update(self.policy_hash.as_bytes());

        // Hash relevant files list
        for file in &self.relevant_files {
            hasher.update(file.to_string_lossy().as_bytes());
        }

        // Hash timestamp
        hasher.update(self.timestamp.to_rfc3339().as_bytes());

        // Hash result (serialize to JSON for deterministic representation)
        let result_value = serde_json::to_value(&self.result)?;
        let canonical_result = canonicalize_json_value(result_value);
        let result_json = serde_json::to_string(&canonical_result)?;
        hasher.update(result_json.as_bytes());

        // Hash limitations
        for limitation in &self.limitations {
            hasher.update(limitation.as_bytes());
        }

        let hash = hasher.finalize();
        Ok(hex::encode(hash))
    }

    /// Verify receipt integrity by recomputing binding hash
    pub fn verify_integrity(&self) -> Result<bool> {
        let mut temp_receipt = self.clone();
        temp_receipt.binding_hash = String::new();
        let recomputed_hash = temp_receipt.compute_binding_hash()?;
        Ok(recomputed_hash == self.binding_hash)
    }

    /// Check if receipt is fresh (within 24 hours)
    pub fn is_fresh(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.timestamp);
        age < Duration::hours(24)
    }

    /// Check if receipt is stale (relevant files have changed since timestamp)
    pub fn is_stale(&self) -> Result<bool> {
        // Check if any relevant files have been modified since receipt timestamp
        for file in &self.relevant_files {
            // Ignore claim-receipt files as freshness inputs to avoid cyclical
            // staleness (one claim refresh updating another claim's evidence).
            if file.to_string_lossy().contains("proof/receipts/claims/")
                && file.extension().map(|ext| ext == "json").unwrap_or(false)
            {
                continue;
            }

            if !file.exists() {
                // File deleted since receipt - definitely stale
                return Ok(true);
            }

            let metadata = fs::metadata(file)
                .context(format!("Failed to get metadata for {}", file.display()))?;

            let modified = metadata.modified().context(format!(
                "Failed to get modified time for {}",
                file.display()
            ))?;

            let modified_dt: DateTime<Utc> = modified.into();

            if modified_dt > self.timestamp {
                // File modified after receipt - stale
                return Ok(true);
            }
        }

        // Check if git has changes in relevant files since commit
        let current_commit = get_git_commit_hash()?;
        if current_commit != self.repo_commit_hash {
            // Commit changed - check if any relevant files changed
            let changed = check_git_files_changed(&self.repo_commit_hash, &self.relevant_files)?;
            if changed {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Save receipt to file
    pub fn save(&self, path: &Path) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;

        Ok(())
    }

    /// Load receipt from file
    pub fn load(path: &Path) -> Result<Self> {
        let json = fs::read_to_string(path)
            .context(format!("Failed to read receipt from {}", path.display()))?;
        let receipt: Receipt =
            serde_json::from_str(&json).context("Failed to parse receipt JSON")?;
        Ok(receipt)
    }
}

fn canonicalize_json_value(value: Value) -> Value {
    match value {
        Value::Object(obj) => {
            let mut keys: Vec<String> = obj.keys().cloned().collect();
            keys.sort();

            let mut canonical = Map::new();
            for key in keys {
                if let Some(v) = obj.get(&key) {
                    canonical.insert(key, canonicalize_json_value(v.clone()));
                }
            }

            Value::Object(canonical)
        }
        Value::Array(arr) => {
            let canonicalized: Vec<Value> = arr.into_iter().map(canonicalize_json_value).collect();
            Value::Array(canonicalized)
        }
        other => other,
    }
}

/// Get current git commit hash
fn get_git_commit_hash() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .context("Failed to run git rev-parse HEAD")?;

    if !output.status.success() {
        anyhow::bail!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let hash = String::from_utf8(output.stdout)?.trim().to_string();

    Ok(hash)
}

/// Compute SHA256 hash of file contents for relevant files
fn compute_artifact_hash(files: &[PathBuf]) -> Result<String> {
    let mut hasher = Sha256::new();

    for file in files {
        if file.exists() && file.is_file() {
            let contents =
                fs::read(file).context(format!("Failed to read file {}", file.display()))?;
            hasher.update(&contents);
        }
    }

    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

/// Compute SHA256 hash of all policy files
fn compute_policy_hash() -> Result<String> {
    let mut hasher = Sha256::new();

    let policy_files = vec![
        "proof/policies/todo_policy.yml",
        "proof/policies/gap_policy.yml",
        "proof/policies/release_gates.yml",
    ];

    for policy_file in policy_files {
        let path = PathBuf::from(policy_file);
        if path.exists() {
            let contents = fs::read(&path)
                .context(format!("Failed to read policy file {}", path.display()))?;
            hasher.update(&contents);
        }
    }

    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

/// Check if any of the given files changed between commit and HEAD
fn check_git_files_changed(old_commit: &str, files: &[PathBuf]) -> Result<bool> {
    for file in files {
        let output = Command::new("git")
            .args([
                "diff",
                "--name-only",
                old_commit,
                "HEAD",
                "--",
                &file.to_string_lossy(),
            ])
            .output()
            .context("Failed to run git diff")?;

        if !output.status.success() {
            // Ignore git errors - assume file may have changed
            return Ok(true);
        }

        if !output.stdout.is_empty() {
            // File changed
            return Ok(true);
        }
    }

    Ok(false)
}

/// Generate receipt for a claim verification
pub fn generate_claim_receipt(
    claim_id: &str,
    result: ProofResult,
    relevant_files: Vec<PathBuf>,
    limitations: Vec<String>,
) -> Result<Receipt> {
    let command = format!("x3-proof verify {}", claim_id);
    let receipt = Receipt::new(command, result, relevant_files, limitations)?;

    // Save to proof/receipts/claims/{claim_id}.receipt.json
    let receipt_path = PathBuf::from(format!("proof/receipts/claims/{}.receipt.json", claim_id));
    receipt.save(&receipt_path)?;

    Ok(receipt)
}

/// Load claim receipt
#[allow(dead_code)]
pub fn load_claim_receipt(claim_id: &str) -> Result<Receipt> {
    let receipt_path = PathBuf::from(format!("proof/receipts/claims/{}.receipt.json", claim_id));
    Receipt::load(&receipt_path)
}

/// Check all claim receipts for freshness and integrity
pub fn check_all_receipts() -> Result<HashMap<String, ReceiptStatus>> {
    let mut statuses = HashMap::new();

    let receipts_dir = PathBuf::from("proof/receipts/claims");
    if !receipts_dir.exists() {
        return Ok(statuses);
    }

    for entry in fs::read_dir(receipts_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            // Extract claim_id (remove .receipt suffix)
            let claim_id = if file_stem.ends_with(".receipt") {
                file_stem.trim_end_matches(".receipt").to_string()
            } else {
                file_stem.to_string()
            };

            match Receipt::load(&path) {
                Ok(receipt) => {
                    let status = if !receipt.verify_integrity()? {
                        ReceiptStatus::IntegrityFailed
                    } else if receipt.is_stale()? {
                        ReceiptStatus::Stale
                    } else if !receipt.is_fresh() {
                        ReceiptStatus::NotFresh
                    } else {
                        ReceiptStatus::Fresh
                    };

                    statuses.insert(claim_id, status);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load receipt for {}: {}", claim_id, e);
                    statuses.insert(claim_id, ReceiptStatus::Invalid);
                }
            }
        }
    }

    Ok(statuses)
}

/// Receipt validation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptStatus {
    Fresh,           // <24h, not stale, integrity ok
    NotFresh,        // >24h but not stale, integrity ok
    Stale,           // Files changed, needs regeneration
    IntegrityFailed, // Binding hash mismatch
    Invalid,         // Failed to load or parse
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::{ProofLevel, ProofStatus};
    use std::collections::HashMap;

    #[test]
    fn test_receipt_integrity() {
        let result = ProofResult {
            claim_id: "test.claim".to_string(),
            claim: "Test claim statement".to_string(),
            status: ProofStatus::Verified,
            proof_level: Some(ProofLevel::P3),
            edge_case_level: None,
            hack_level: None,
            operator_level: None,
            degraded_level: None,
            files_inspected: vec![],
            commands_run: vec![],
            passed_checks: vec![],
            failed_checks: vec![],
            missing_proofs: vec![],
            blockers: vec![],
            score: 0.9,
            evidence: HashMap::new(),
            timestamp: Utc::now(),
            duration_ms: 0,
        };

        let receipt = Receipt::new(
            "x3-proof test".to_string(),
            result,
            vec![PathBuf::from("test.rs")],
            vec!["Test limitation".to_string()],
        )
        .unwrap();

        // Verify integrity
        assert!(receipt.verify_integrity().unwrap());

        // Tamper with receipt
        let mut tampered = receipt.clone();
        tampered.result.score = 0.5;

        // Integrity should fail
        assert!(!tampered.verify_integrity().unwrap());
    }

    #[test]
    fn test_receipt_integrity_stable_with_evidence_map_order() {
        let mut evidence = HashMap::new();
        evidence.insert("zeta".to_string(), "1".to_string());
        evidence.insert("alpha".to_string(), "2".to_string());
        evidence.insert("mid".to_string(), "3".to_string());

        let result = ProofResult {
            claim_id: "test.claim.map".to_string(),
            claim: "Map determinism claim".to_string(),
            status: ProofStatus::Verified,
            proof_level: Some(ProofLevel::P3),
            edge_case_level: None,
            hack_level: None,
            operator_level: None,
            degraded_level: None,
            files_inspected: vec![],
            commands_run: vec![],
            passed_checks: vec![],
            failed_checks: vec![],
            missing_proofs: vec![],
            blockers: vec![],
            score: 1.0,
            evidence,
            timestamp: Utc::now(),
            duration_ms: 1,
        };

        let receipt = Receipt::new(
            "x3-proof verify test.claim.map".to_string(),
            result,
            vec![PathBuf::from("proof/claims/registry.yml")],
            vec![],
        )
        .unwrap();

        // Re-load and verify to ensure map deserialization order does not break binding.
        let temp_path = PathBuf::from("/tmp/proofforge-map-order-receipt.json");
        receipt.save(&temp_path).unwrap();
        let loaded = Receipt::load(&temp_path).unwrap();

        assert!(loaded.verify_integrity().unwrap());

        let _ = std::fs::remove_file(temp_path);
    }
}
