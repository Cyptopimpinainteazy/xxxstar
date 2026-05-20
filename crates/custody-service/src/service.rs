use crate::audit::AuditLog;
use crate::error::{CustodyError, Result};
use crate::hsm::{HSMBackend, HSMSigner, MockHSM};
/// Main Custody Service implementation
/// Orchestrates vault operations, authorization, policy enforcement, and settlement linkage
use crate::types::*;
use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};

/// Core custody service trait
#[async_trait]
pub trait CustodyService: Send + Sync {
    /// Request authorization for a vault operation
    async fn authorize_operation(
        &self,
        request: AuthorizationRequest,
    ) -> Result<AuthorizationDecision>;

    /// Execute authorized vault operation
    async fn execute_operation(
        &self,
        command: VaultOperationCommand,
    ) -> Result<VaultOperationResponse>;

    /// Get current vault state snapshot
    async fn get_vault_snapshot(&self, vault_id: &str) -> Result<VaultSnapshot>;

    /// Poll for operation result
    async fn poll_operation(&self, operation_id: &str) -> Result<OperationStatus>;

    /// Get audit entries for compliance
    async fn get_audit_log(&self, since_ms: u64) -> Result<Vec<AuditLogEntry>>;
}

/// In-memory custody service implementation
pub struct CustodyServiceImpl {
    /// Vault state by vault_id
    vaults: RwLock<HashMap<String, VaultSnapshot>>,
    /// In-flight operations by operation_id
    operations: RwLock<HashMap<String, VaultOperationResponse>>,
    /// Authorization requests by request_id
    auth_requests: RwLock<HashMap<String, AuthorizationRequest>>,
    /// Authorization decisions by request_id
    auth_decisions: RwLock<HashMap<String, AuthorizationDecision>>,
    /// Audit trail
    audit: AuditLog,
    /// HSM signer for operation proofs
    signer: HSMSigner,
    /// Policy engine (simplified)
    policy: PolicyEngine,
}

impl CustodyServiceImpl {
    pub async fn new() -> Result<Self> {
        let hsm = Box::new(MockHSM::new());
        let _ = hsm.generate_key("vault-key-1", "ECDSA-P256").await?;
        let signer = HSMSigner::new(hsm, "vault-key-1".to_string());

        Ok(Self {
            vaults: RwLock::new(HashMap::new()),
            operations: RwLock::new(HashMap::new()),
            auth_requests: RwLock::new(HashMap::new()),
            auth_decisions: RwLock::new(HashMap::new()),
            audit: AuditLog::new(),
            signer,
            policy: PolicyEngine::new(),
        })
    }

    /// Initialize vault with starting balance
    pub fn init_vault(
        &self,
        vault_id: String,
        asset: String,
        initial_balance: u128,
        status: VaultStatus,
    ) -> Result<()> {
        let now = Utc::now().timestamp_millis() as u64;
        let snapshot = VaultSnapshot {
            vault_id: vault_id.clone(),
            asset,
            available_balance: initial_balance,
            reserved_balance: 0,
            pending_out_balance: 0,
            pending_in_balance: 0,
            status,
            snapshot_at_ms: now,
            last_operation_id: None,
            merkle_root: String::new(),
        };

        self.vaults.write().insert(vault_id, snapshot);
        Ok(())
    }

    fn check_vault_frozen(&self, vault_id: &str) -> Result<()> {
        let vaults = self.vaults.read();
        if let Some(vault) = vaults.get(vault_id) {
            if vault.status == VaultStatus::Frozen {
                return Err(CustodyError::VaultFrozen(vault_id.to_string()));
            }
        }
        Ok(())
    }

    fn check_policy(&self, cmd: &VaultOperationCommand) -> Result<()> {
        self.policy.validate(cmd)
    }
}

#[async_trait]
impl CustodyService for CustodyServiceImpl {
    async fn authorize_operation(
        &self,
        request: AuthorizationRequest,
    ) -> Result<AuthorizationDecision> {
        let request_id = request.request_id.clone();

        // Check if authorization request already exists
        if self.auth_requests.read().contains_key(&request_id) {
            return Err(CustodyError::OperationExists(request_id));
        }

        // Check expiry
        let now = Utc::now().timestamp_millis() as u64;
        if now > request.expires_at_ms {
            return Err(CustodyError::OperationExpired);
        }

        // Store the request
        self.auth_requests
            .write()
            .insert(request_id.clone(), request.clone());

        // In production, this would wait for human decision via operator portal
        // For now, we auto-approve Operational tier and reject higher tiers
        let approved = request.required_tier == AuthorizationTier::Operational;
        let reason = if approved {
            "Auto-approved operational tier".to_string()
        } else {
            "Requires manual approval (Strategic/Emergency/Policy)".to_string()
        };

        let decision = AuthorizationDecision {
            request_id: request_id.clone(),
            approved,
            approver: "system".to_string(),
            reason,
            timestamp_ms: now,
            signer_proof: None,
        };

        self.auth_decisions
            .write()
            .insert(request_id.clone(), decision.clone());

        Ok(decision)
    }

    async fn execute_operation(
        &self,
        command: VaultOperationCommand,
    ) -> Result<VaultOperationResponse> {
        let operation_id = command.operation_id.clone();
        let now = Utc::now().timestamp_millis() as u64;

        // Check if operation already exists (idempotency)
        if self.operations.read().contains_key(&operation_id) {
            return Err(CustodyError::OperationExists(operation_id));
        }

        // Check vault exists and not frozen
        self.check_vault_frozen(&command.source_vault_id)?;

        // Validate policy
        self.check_policy(&command)?;

        // Check if authorized
        let is_authorized = self
            .auth_decisions
            .read()
            .values()
            .any(|d| d.approved && d.request_id == operation_id);

        if !is_authorized && command.required_tier != AuthorizationTier::Operational {
            return Err(CustodyError::AuthorizationFailed(format!(
                "Operation {} requires authorization",
                operation_id
            )));
        }

        // Block 1: Read and clone source vault, check balance
        let (source_vault, insufficient_balance) = {
            let vaults = self.vaults.read();
            let source_vault = vaults
                .get(&command.source_vault_id)
                .ok_or_else(|| CustodyError::VaultNotFound(command.source_vault_id.clone()))?
                .clone();

            let has_insufficient = source_vault.available_balance < command.amount;
            (source_vault, has_insufficient)
        }; // Lock released here

        // Handle insufficient balance error
        if insufficient_balance {
            let response = VaultOperationResponse {
                operation_id: operation_id.clone(),
                status: OperationStatus::Failed,
                failure_reason: Some(format!(
                    "Insufficient balance: need {}, have {}",
                    command.amount, source_vault.available_balance
                )),
                proof: None,
                vault_balance_after: Some(source_vault.available_balance),
                receipt: None,
                responded_at_ms: now,
            };

            // Log failure
            let _ = self.audit.record_operation(
                operation_id.clone(),
                "system".to_string(),
                command.required_tier,
                command.operation_type,
                command.source_vault_id.clone(),
                command.destination.clone(),
                command.asset.clone(),
                command.amount,
                command.policy_rule_ids.clone(),
                false,
                OperationStatus::Failed,
                response.failure_reason.clone(),
            );

            let mut ops = self.operations.write();
            ops.insert(operation_id.clone(), response.clone());
            return Ok(response);
        }

        // Block 2: Update vault state (NO ASYNC CALLS WITHIN THIS BLOCK)
        let updated_vault = {
            let mut vaults = self.vaults.write();
            let mut updated = source_vault.clone();

            match command.operation_type {
                VaultOperationType::Transfer => {
                    updated.available_balance -= command.amount;
                    updated.pending_out_balance += command.amount;
                }
                VaultOperationType::Reserve => {
                    updated.available_balance -= command.amount;
                    updated.reserved_balance += command.amount;
                }
                VaultOperationType::Release => {
                    updated.reserved_balance =
                        updated.reserved_balance.saturating_sub(command.amount);
                    updated.available_balance += command.amount;
                }
                VaultOperationType::Sweep => {
                    updated.available_balance -= command.amount;
                    updated.pending_out_balance += command.amount;
                }
                _ => {
                    return Err(CustodyError::Internal(
                        "unsupported operation type".to_string(),
                    ))
                }
            }

            updated.last_operation_id = Some(operation_id.clone());
            updated.snapshot_at_ms = now;
            updated.merkle_root = crate::hsm::HSMSigner::compute_state_merkle_root(
                &updated.vault_id,
                updated.available_balance,
                updated.reserved_balance,
                updated.pending_out_balance,
                updated.pending_in_balance,
            );

            vaults.insert(command.source_vault_id.clone(), updated.clone());
            updated
        }; // Lock released here, NOW we can do async

        // Block 3: Async operation (no locks held)
        let proof_data = self.signer.sign_operation(&command).await?;
        let proof = OperationProof {
            tx_hash: format!("0x{}", hex::encode(&proof_data[0..32])),
            block_number: 0,
            settled_at_ms: now,
            proof_data,
            proof_type: "hsm_signature".to_string(),
        };

        // Block 4: Create response and log (outside any lock)
        let response = VaultOperationResponse {
            operation_id: operation_id.clone(),
            status: OperationStatus::Succeeded,
            failure_reason: None,
            proof: Some(proof),
            vault_balance_after: Some(updated_vault.available_balance),
            receipt: Some(format!("receipt-{}", operation_id)),
            responded_at_ms: now,
        };

        // Log success
        let _ = self.audit.record_operation(
            operation_id.clone(),
            "system".to_string(),
            command.required_tier,
            command.operation_type,
            command.source_vault_id.clone(),
            command.destination.clone(),
            command.asset.clone(),
            command.amount,
            command.policy_rule_ids.clone(),
            true,
            OperationStatus::Succeeded,
            None,
        );

        // Block 5: Store operation result
        {
            let mut ops = self.operations.write();
            ops.insert(operation_id.clone(), response.clone());
        } // Lock released

        Ok(response)
    }

    async fn get_vault_snapshot(&self, vault_id: &str) -> Result<VaultSnapshot> {
        self.vaults
            .read()
            .get(vault_id)
            .cloned()
            .ok_or_else(|| CustodyError::VaultNotFound(vault_id.to_string()))
    }

    async fn poll_operation(&self, operation_id: &str) -> Result<OperationStatus> {
        self.operations
            .read()
            .get(operation_id)
            .map(|r| r.status)
            .ok_or_else(|| CustodyError::OperationNotFound(operation_id.to_string()))
    }

    async fn get_audit_log(&self, since_ms: u64) -> Result<Vec<AuditLogEntry>> {
        Ok(self.audit.get_entries_since(since_ms))
    }
}

/// Simplified policy engine
struct PolicyEngine {
    rules: BTreeMap<String, PolicyRule>,
}

struct PolicyRule {
    #[allow(dead_code)]
    rule_id: String,
    #[allow(dead_code)]
    description: String,
    max_amount: Option<u128>,
    allowed_tiers: Vec<AuthorizationTier>,
}

impl PolicyEngine {
    fn new() -> Self {
        let mut rules = BTreeMap::new();
        rules.insert(
            "max-transfer-10k".to_string(),
            PolicyRule {
                rule_id: "max-transfer-10k".to_string(),
                description: "Max transfer 10k units".to_string(),
                max_amount: Some(10_000),
                allowed_tiers: vec![AuthorizationTier::Operational, AuthorizationTier::Strategic],
            },
        );

        Self { rules }
    }

    fn validate(&self, cmd: &VaultOperationCommand) -> Result<()> {
        for rule_id in &cmd.policy_rule_ids {
            if let Some(rule) = self.rules.get(rule_id) {
                if let Some(max) = rule.max_amount {
                    if cmd.amount > max {
                        return Err(CustodyError::PolicyViolation(format!(
                            "Amount {} exceeds policy max {}",
                            cmd.amount, max
                        )));
                    }
                }

                if !rule.allowed_tiers.contains(&cmd.required_tier) {
                    return Err(CustodyError::PolicyViolation(format!(
                        "Tier {:?} not allowed for rule {}",
                        cmd.required_tier, rule_id
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let service = CustodyServiceImpl::new().await.unwrap();
        service
            .init_vault(
                "vault-1".to_string(),
                "USDC".to_string(),
                10_000,
                VaultStatus::Active,
            )
            .unwrap();

        let snapshot = service.get_vault_snapshot("vault-1").await.unwrap();
        assert_eq!(snapshot.vault_id, "vault-1");
        assert_eq!(snapshot.available_balance, 10_000);
    }

    #[tokio::test]
    async fn test_operation_authorization() {
        let service = CustodyServiceImpl::new().await.unwrap();

        let request = AuthorizationRequest {
            request_id: "auth-1".to_string(),
            command: VaultOperationCommand {
                operation_id: "op-1".to_string(),
                operation_type: VaultOperationType::Transfer,
                source_vault_id: "vault-1".to_string(),
                destination: "0xabc".to_string(),
                asset: "USDC".to_string(),
                amount: 1000,
                chain_id: 1,
                required_tier: AuthorizationTier::Operational,
                policy_rule_ids: vec![],
                route_id: None,
                metadata: BTreeMap::new(),
                initiated_at_ms: Utc::now().timestamp_millis() as u64,
            },
            required_tier: AuthorizationTier::Operational,
            requestor: "user-1".to_string(),
            reason: "test transfer".to_string(),
            created_at_ms: Utc::now().timestamp_millis() as u64,
            expires_at_ms: Utc::now().timestamp_millis() as u64 + 3600_000,
        };

        let decision = service.authorize_operation(request).await.unwrap();
        assert!(decision.approved);
    }

    #[tokio::test]
    async fn test_execute_transfer() {
        let service = CustodyServiceImpl::new().await.unwrap();
        service
            .init_vault(
                "vault-1".to_string(),
                "USDC".to_string(),
                10_000,
                VaultStatus::Active,
            )
            .unwrap();

        let cmd = VaultOperationCommand {
            operation_id: "op-1".to_string(),
            operation_type: VaultOperationType::Transfer,
            source_vault_id: "vault-1".to_string(),
            destination: "0xabc".to_string(),
            asset: "USDC".to_string(),
            amount: 1000,
            chain_id: 1,
            required_tier: AuthorizationTier::Operational,
            policy_rule_ids: vec!["max-transfer-10k".to_string()],
            route_id: None,
            metadata: BTreeMap::new(),
            initiated_at_ms: Utc::now().timestamp_millis() as u64,
        };

        let response = service.execute_operation(cmd).await.unwrap();
        assert_eq!(response.status, OperationStatus::Succeeded);
        assert_eq!(response.vault_balance_after, Some(9000));
    }

    #[tokio::test]
    async fn test_insufficient_balance() {
        let service = CustodyServiceImpl::new().await.unwrap();
        service
            .init_vault(
                "vault-2".to_string(),
                "USDC".to_string(),
                500,
                VaultStatus::Active,
            )
            .unwrap();

        let cmd = VaultOperationCommand {
            operation_id: "op-fail".to_string(),
            operation_type: VaultOperationType::Transfer,
            source_vault_id: "vault-2".to_string(),
            destination: "0xdef".to_string(),
            asset: "USDC".to_string(),
            amount: 1000,
            chain_id: 1,
            required_tier: AuthorizationTier::Operational,
            policy_rule_ids: vec![],
            route_id: None,
            metadata: BTreeMap::new(),
            initiated_at_ms: Utc::now().timestamp_millis() as u64,
        };

        let response = service.execute_operation(cmd).await.unwrap();
        assert_eq!(response.status, OperationStatus::Failed);
    }
}
