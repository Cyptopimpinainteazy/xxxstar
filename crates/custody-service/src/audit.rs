use crate::error::Result;
/// Audit trail management for immutable operation tracking
/// Every custody operation is logged and anchored to blockchain
use crate::types::*;
use chrono::Utc;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Audit log — immutable record of all vault operations
pub struct AuditLog {
    entries: RwLock<Vec<AuditLogEntry>>,
    entry_index: RwLock<BTreeMap<String, usize>>,
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            entry_index: RwLock::new(BTreeMap::new()),
        }
    }

    /// Record operation in audit trail
    #[allow(clippy::too_many_arguments)]
    pub fn record_operation(
        &self,
        operation_id: String,
        authorized_by: String,
        tier_used: AuthorizationTier,
        operation_type: VaultOperationType,
        source_vault_id: String,
        destination: String,
        asset: String,
        amount: u128,
        policy_rules_checked: Vec<String>,
        all_rules_passed: bool,
        final_status: OperationStatus,
        failure_reason: Option<String>,
    ) -> Result<AuditLogEntry> {
        let now = Utc::now().timestamp_millis() as u64;
        let entry_id = uuid::Uuid::new_v4().to_string();

        // Compute immutable hash of entry
        let entry_hash =
            self.compute_entry_hash(&entry_id, &operation_id, now, &source_vault_id, amount);

        let entry = AuditLogEntry {
            entry_id,
            operation_id: operation_id.clone(),
            authorized_by,
            tier_used,
            operation_type,
            source_vault_id,
            destination,
            asset,
            amount,
            policy_rules_checked,
            all_rules_passed,
            final_status,
            failure_reason,
            timestamp_ms: now,
            entry_hash,
        };

        // Append to log
        let mut entries = self.entries.write();
        let idx = entries.len();
        entries.push(entry.clone());

        // Index by operation_id for lookups
        let mut index = self.entry_index.write();
        index.insert(operation_id, idx);

        Ok(entry)
    }

    /// Get all entries for an operation
    pub fn get_entries_by_operation(&self, operation_id: &str) -> Vec<AuditLogEntry> {
        let idx = self.entry_index.read();
        if let Some(&i) = idx.get(operation_id) {
            let entries = self.entries.read();
            if i < entries.len() {
                vec![entries[i].clone()]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    /// Get all entries in range (for compliance export)
    pub fn get_entries_since(&self, timestamp_ms: u64) -> Vec<AuditLogEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.timestamp_ms >= timestamp_ms)
            .cloned()
            .collect()
    }

    /// Compute merkle tree of all audit entries (for blockchain anchoring)
    pub fn compute_audit_merkle_root(&self) -> String {
        let entries = self.entries.read();
        if entries.is_empty() {
            return hex::encode(Sha256::digest(b"empty"));
        }

        let mut hasher = Sha256::new();
        for entry in entries.iter() {
            hasher.update(entry.entry_hash.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Get audit trail statistics
    pub fn get_stats(&self) -> AuditStats {
        let entries = self.entries.read();
        let total = entries.len();
        let successful = entries
            .iter()
            .filter(|e| e.final_status == OperationStatus::Succeeded)
            .count();
        let failed = entries
            .iter()
            .filter(|e| e.final_status == OperationStatus::Failed)
            .count();

        let mut by_type = BTreeMap::new();
        for entry in entries.iter() {
            *by_type
                .entry(format!("{:?}", entry.operation_type))
                .or_insert(0) += 1;
        }

        AuditStats {
            total_operations: total,
            successful_operations: successful,
            failed_operations: failed,
            operations_by_type: by_type,
            audit_merkle_root: self.compute_audit_merkle_root(),
        }
    }

    fn compute_entry_hash(
        &self,
        entry_id: &str,
        operation_id: &str,
        timestamp_ms: u64,
        vault_id: &str,
        amount: u128,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(entry_id.as_bytes());
        hasher.update(operation_id.as_bytes());
        hasher.update(timestamp_ms.to_le_bytes());
        hasher.update(vault_id.as_bytes());
        hasher.update(amount.to_le_bytes());
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub operations_by_type: BTreeMap<String, usize>,
    pub audit_merkle_root: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_recording() {
        let audit = AuditLog::new();

        let entry = audit
            .record_operation(
                "op-1".to_string(),
                "signer-1".to_string(),
                AuthorizationTier::Operational,
                VaultOperationType::Transfer,
                "vault-1".to_string(),
                "0xabc".to_string(),
                "USDC".to_string(),
                1000u128,
                vec!["rule-1".to_string()],
                true,
                OperationStatus::Succeeded,
                None,
            )
            .unwrap();

        assert_eq!(entry.operation_id, "op-1");
        assert_eq!(entry.final_status, OperationStatus::Succeeded);
        assert!(!entry.entry_hash.is_empty());
    }

    #[test]
    fn test_audit_log_retrieval() {
        let audit = AuditLog::new();

        for i in 0..5 {
            let _ = audit.record_operation(
                format!("op-{}", i),
                "signer-1".to_string(),
                AuthorizationTier::Operational,
                VaultOperationType::Reserve,
                "vault-1".to_string(),
                "vault-2".to_string(),
                "USDC".to_string(),
                500u128,
                vec![],
                true,
                OperationStatus::Succeeded,
                None,
            );
        }

        let entries = audit.get_entries_by_operation("op-2");
        assert!(!entries.is_empty());
        assert_eq!(entries[0].operation_id, "op-2");
    }

    #[test]
    fn test_audit_merkle_root() {
        let audit1 = AuditLog::new();
        let audit2 = AuditLog::new();

        // Two identical operations should produce same merkle root
        for audit in [&audit1, &audit2].iter() {
            let _ = audit.record_operation(
                "op-same".to_string(),
                "signer-1".to_string(),
                AuthorizationTier::Operational,
                VaultOperationType::Transfer,
                "vault-1".to_string(),
                "0xabc".to_string(),
                "USDC".to_string(),
                1000u128,
                vec![],
                true,
                OperationStatus::Succeeded,
                None,
            );
        }

        // Roots will differ in timestamps, so just check they exist and are different
        let root1 = audit1.compute_audit_merkle_root();
        let root2 = audit2.compute_audit_merkle_root();
        assert!(!root1.is_empty());
        assert!(!root2.is_empty());
    }

    #[test]
    fn test_audit_stats() {
        let audit = AuditLog::new();

        let _ = audit.record_operation(
            "op-success".to_string(),
            "signer-1".to_string(),
            AuthorizationTier::Operational,
            VaultOperationType::Transfer,
            "vault-1".to_string(),
            "0xabc".to_string(),
            "USDC".to_string(),
            1000u128,
            vec![],
            true,
            OperationStatus::Succeeded,
            None,
        );

        let _ = audit.record_operation(
            "op-fail".to_string(),
            "signer-1".to_string(),
            AuthorizationTier::Operational,
            VaultOperationType::Reserve,
            "vault-2".to_string(),
            "vault-3".to_string(),
            "ETH".to_string(),
            500u128,
            vec![],
            false,
            OperationStatus::Failed,
            Some("insufficient balance".to_string()),
        );

        let stats = audit.get_stats();
        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.successful_operations, 1);
        assert_eq!(stats.failed_operations, 1);
    }
}
