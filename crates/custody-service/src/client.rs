use crate::error::Result;
/// Client library for custody service integration
/// Used by position-manager to request vault operations
use crate::types::*;
use async_trait::async_trait;

/// Custody service client trait for dependency injection
#[async_trait]
pub trait CustodyServiceClient: Send + Sync {
    /// Request vault operation authorization
    async fn authorize(&self, request: AuthorizationRequest) -> Result<AuthorizationDecision>;

    /// Execute authorized vault operation
    async fn execute(&self, command: VaultOperationCommand) -> Result<VaultOperationResponse>;

    /// Get vault state
    async fn get_vault(&self, vault_id: &str) -> Result<VaultSnapshot>;

    /// Check operation status
    async fn poll(&self, operation_id: &str) -> Result<OperationStatus>;

    /// Export audit log for compliance
    async fn export_audit(&self, since_ms: u64) -> Result<Vec<AuditLogEntry>>;
}

/// Mock client for testing position-manager without service
pub struct MockCustodyClient {
    vaults: parking_lot::RwLock<std::collections::HashMap<String, VaultSnapshot>>,
    operations: parking_lot::RwLock<std::collections::HashMap<String, VaultOperationResponse>>,
    audit_entries: parking_lot::RwLock<Vec<AuditLogEntry>>,
}

impl Default for MockCustodyClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCustodyClient {
    pub fn new() -> Self {
        Self {
            vaults: parking_lot::RwLock::new(std::collections::HashMap::new()),
            operations: parking_lot::RwLock::new(std::collections::HashMap::new()),
            audit_entries: parking_lot::RwLock::new(Vec::new()),
        }
    }

    pub fn init_vault(&self, vault_id: String, asset: String, balance: u128) -> Result<()> {
        let snapshot = VaultSnapshot {
            vault_id,
            asset,
            available_balance: balance,
            reserved_balance: 0,
            pending_out_balance: 0,
            pending_in_balance: 0,
            status: VaultStatus::Active,
            snapshot_at_ms: chrono::Utc::now().timestamp_millis() as u64,
            last_operation_id: None,
            merkle_root: String::new(),
        };
        self.vaults
            .write()
            .insert(snapshot.vault_id.clone(), snapshot);
        Ok(())
    }
}

#[async_trait]
impl CustodyServiceClient for MockCustodyClient {
    async fn authorize(&self, request: AuthorizationRequest) -> Result<AuthorizationDecision> {
        Ok(AuthorizationDecision {
            request_id: request.request_id,
            approved: true,
            approver: "mock".to_string(),
            reason: "test approval".to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
            signer_proof: None,
        })
    }

    async fn execute(&self, command: VaultOperationCommand) -> Result<VaultOperationResponse> {
        let operation_id_clone = command.operation_id.clone();
        let mut vaults = self.vaults.write();
        let vault = vaults.get_mut(&command.source_vault_id).ok_or_else(|| {
            crate::error::CustodyError::VaultNotFound(command.source_vault_id.clone())
        })?;

        if vault.available_balance < command.amount {
            return Ok(VaultOperationResponse {
                operation_id: operation_id_clone,
                status: OperationStatus::Failed,
                failure_reason: Some("insufficient balance".to_string()),
                proof: None,
                vault_balance_after: Some(vault.available_balance),
                receipt: None,
                responded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
            });
        }

        match command.operation_type {
            VaultOperationType::Reserve => {
                vault.available_balance -= command.amount;
                vault.reserved_balance += command.amount;
            }
            VaultOperationType::Release => {
                vault.reserved_balance = vault.reserved_balance.saturating_sub(command.amount);
                vault.available_balance += command.amount;
            }
            VaultOperationType::Transfer => {
                vault.available_balance -= command.amount;
            }
            _ => {}
        }

        let response = VaultOperationResponse {
            operation_id: operation_id_clone.clone(),
            status: OperationStatus::Succeeded,
            failure_reason: None,
            proof: Some(OperationProof {
                tx_hash: "0x0".to_string(),
                block_number: 0,
                settled_at_ms: chrono::Utc::now().timestamp_millis() as u64,
                proof_data: vec![],
                proof_type: "mock".to_string(),
            }),
            vault_balance_after: Some(vault.available_balance),
            receipt: None,
            responded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
        };

        drop(vaults); // Release lock
        self.operations
            .write()
            .insert(operation_id_clone, response.clone());

        Ok(response)
    }

    async fn get_vault(&self, vault_id: &str) -> Result<VaultSnapshot> {
        self.vaults
            .read()
            .get(vault_id)
            .cloned()
            .ok_or_else(|| crate::error::CustodyError::VaultNotFound(vault_id.to_string()))
    }

    async fn poll(&self, operation_id: &str) -> Result<OperationStatus> {
        self.operations
            .read()
            .get(operation_id)
            .map(|r| r.status)
            .ok_or_else(|| crate::error::CustodyError::OperationNotFound(operation_id.to_string()))
    }

    async fn export_audit(&self, since_ms: u64) -> Result<Vec<AuditLogEntry>> {
        Ok(self
            .audit_entries
            .read()
            .iter()
            .filter(|e| e.timestamp_ms >= since_ms)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_initialization() {
        let client = MockCustodyClient::new();
        client
            .init_vault("vault-1".to_string(), "USDC".to_string(), 5000)
            .unwrap();

        let snap = client.get_vault("vault-1").await.unwrap();
        assert_eq!(snap.available_balance, 5000);
    }

    #[tokio::test]
    async fn test_mock_client_reserve() {
        let client = MockCustodyClient::new();
        client
            .init_vault("vault-1".to_string(), "USDC".to_string(), 5000)
            .unwrap();

        let cmd = VaultOperationCommand {
            operation_id: "op-1".to_string(),
            operation_type: VaultOperationType::Reserve,
            source_vault_id: "vault-1".to_string(),
            destination: "".to_string(),
            asset: "USDC".to_string(),
            amount: 1000,
            chain_id: 1,
            required_tier: AuthorizationTier::Operational,
            policy_rule_ids: vec![],
            route_id: None,
            metadata: std::collections::BTreeMap::new(),
            initiated_at_ms: 0,
        };

        let resp = client.execute(cmd).await.unwrap();
        assert_eq!(resp.status, OperationStatus::Succeeded);

        let snap = client.get_vault("vault-1").await.unwrap();
        assert_eq!(snap.available_balance, 4000);
        assert_eq!(snap.reserved_balance, 1000);
    }
}
