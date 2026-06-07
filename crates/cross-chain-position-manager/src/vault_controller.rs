use chrono::Utc;
use custody_service::client::CustodyServiceClient;
/// Vault Controller: Custody-aware execution boundary for position manager
/// Ticket 11 implementation — integrates custody service with signer policy
///
/// This module provides the interface between the position manager's inventory
/// and the dedicated custody service. It handles vault operations, settlement
/// linkage, and signer policy enforcement.
use custody_service::types::*;
use std::collections::BTreeMap;

/// Result type for vault operations
pub type Result<T> = std::result::Result<T, VaultControllerError>;

/// Vault controller errors
#[derive(Debug)]
pub enum VaultControllerError {
    CustodyError(String),
    InvalidVaultType(String),
    NoAvailableLiquidity { required: u128, available: u128 },
    OperationFailed(String),
    SettlementLinkageFailed(String),
}

impl std::fmt::Display for VaultControllerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultControllerError::CustodyError(e) => write!(f, "Custody error: {}", e),
            VaultControllerError::InvalidVaultType(t) => write!(f, "Invalid vault type: {}", t),
            VaultControllerError::NoAvailableLiquidity {
                required,
                available,
            } => write!(
                f,
                "Insufficient liquidity: need {}, have {}",
                required, available
            ),
            VaultControllerError::OperationFailed(e) => write!(f, "Operation failed: {}", e),
            VaultControllerError::SettlementLinkageFailed(e) => {
                write!(f, "Settlement linkage failed: {}", e)
            }
        }
    }
}

impl std::error::Error for VaultControllerError {}

/// Vault type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultTypeClassification {
    SettlementFloat,
    GasReserve,
    InsuranceReserve,
    TreasuryReserve,
}

impl VaultTypeClassification {
    pub fn as_str(&self) -> &'static str {
        match self {
            VaultTypeClassification::SettlementFloat => "settlement-float",
            VaultTypeClassification::GasReserve => "gas-reserve",
            VaultTypeClassification::InsuranceReserve => "insurance-reserve",
            VaultTypeClassification::TreasuryReserve => "treasury-reserve",
        }
    }

    pub fn vault_id(&self, chain_id: u32, asset: &str) -> String {
        format!("{}-{}-{}", self.as_str(), chain_id, asset)
    }
}

/// Signer policy enforcement for custody operations
#[derive(Debug, Clone)]
pub struct SignerPolicy {
    /// Authorized signer identity
    pub signer_id: String,
    /// Maximum single operation amount (per signer policy)
    pub max_single_operation: u128,
    /// Maximum daily aggregate (per signer)
    pub max_daily_aggregate: u128,
    /// Tiers this signer can approve
    pub allowed_tiers: Vec<AuthorizationTier>,
    /// Policy expires at (Unix ms)
    pub expires_at_ms: u64,
}

/// Vault Controller — orchestrates custody operations with signer enforcement
pub struct VaultController {
    /// Custody service client
    custody_client: Box<dyn CustodyServiceClient>,
    /// Signer policies indexed by signer_id
    signer_policies: parking_lot::RwLock<BTreeMap<String, SignerPolicy>>,
    /// Settlement linkages by route_id
    settlement_linkages: parking_lot::RwLock<BTreeMap<String, SettlementLinkage>>,
}

impl VaultController {
    pub fn new(custody_client: Box<dyn CustodyServiceClient>) -> Self {
        Self {
            custody_client,
            signer_policies: parking_lot::RwLock::new(BTreeMap::new()),
            settlement_linkages: parking_lot::RwLock::new(BTreeMap::new()),
        }
    }

    /// Register a signer's policy for custody operations
    pub fn register_signer_policy(&self, policy: SignerPolicy) -> Result<()> {
        self.signer_policies
            .write()
            .insert(policy.signer_id.clone(), policy);
        Ok(())
    }

    /// Link a settlement obligation to a route for closure tracking
    pub fn link_settlement(
        &self,
        route_id: String,
        lane_id: String,
        source_chain: u32,
        dest_chain: u32,
        settlement_amount: u128,
        timeout_ms: u64,
    ) -> Result<()> {
        let linkage = SettlementLinkage {
            route_id: route_id.clone(),
            lane_id,
            source_chain,
            dest_chain,
            settlement_amount,
            proof_requirement: "settlement-proof".to_string(),
            timeout_ms,
        };
        self.settlement_linkages.write().insert(route_id, linkage);
        Ok(())
    }

    /// Reserve funds for a route execution with custody boundary enforcement
    pub async fn reserve_for_route(
        &self,
        route_id: &str,
        vault_type: VaultTypeClassification,
        chain_id: u32,
        asset: &str,
        amount: u128,
        signer_id: &str,
    ) -> Result<VaultOperationResponse> {
        // Check signer policy
        let policy = {
            let policies = self.signer_policies.read();
            policies.get(signer_id).cloned().ok_or_else(|| {
                VaultControllerError::OperationFailed(format!(
                    "Signer {} policy not registered",
                    signer_id
                ))
            })?
        };

        // Check signer policy limits
        if amount > policy.max_single_operation {
            return Err(VaultControllerError::OperationFailed(format!(
                "Amount {} exceeds signer max {}",
                amount, policy.max_single_operation
            )));
        }

        // Check policy expiry
        let now = Utc::now().timestamp_millis() as u64;
        if now > policy.expires_at_ms {
            return Err(VaultControllerError::OperationFailed(
                "Signer policy expired".to_string(),
            ));
        }

        // Create custody command
        let vault_id = vault_type.vault_id(chain_id, asset);
        let operation_id = format!("reserve-{}-{}", route_id, Utc::now().timestamp_millis());

        let cmd = VaultOperationCommand {
            operation_id: operation_id.clone(),
            operation_type: VaultOperationType::Reserve,
            source_vault_id: vault_id.clone(),
            destination: route_id.to_string(),
            asset: asset.to_string(),
            amount,
            chain_id,
            required_tier: AuthorizationTier::Operational,
            policy_rule_ids: vec!["max-transfer-10k".to_string()],
            route_id: Some(route_id.to_string()),
            metadata: BTreeMap::new(),
            initiated_at_ms: now,
        };

        // Request authorization
        let auth_req = AuthorizationRequest {
            request_id: format!("auth-{}", operation_id),
            command: cmd.clone(),
            required_tier: AuthorizationTier::Operational,
            requestor: signer_id.to_string(),
            reason: format!("Reserve for route {}", route_id),
            created_at_ms: now,
            expires_at_ms: now + 300_000, // 5 minute window
        };

        let decision = self
            .custody_client
            .authorize(auth_req)
            .await
            .map_err(|e| VaultControllerError::CustodyError(e.to_string()))?;

        if !decision.approved {
            return Err(VaultControllerError::OperationFailed(
                "Authorization rejected".to_string(),
            ));
        }

        // Execute operation
        let response = self
            .custody_client
            .execute(cmd)
            .await
            .map_err(|e| VaultControllerError::CustodyError(e.to_string()))?;

        Ok(response)
    }

    /// Release reserved funds after settlement
    pub async fn release_reservation(
        &self,
        route_id: &str,
        vault_type: VaultTypeClassification,
        chain_id: u32,
        asset: &str,
        amount: u128,
    ) -> Result<VaultOperationResponse> {
        let vault_id = vault_type.vault_id(chain_id, asset);
        let operation_id = format!("release-{}-{}", route_id, Utc::now().timestamp_millis());
        let now = Utc::now().timestamp_millis() as u64;

        let cmd = VaultOperationCommand {
            operation_id,
            operation_type: VaultOperationType::Release,
            source_vault_id: vault_id,
            destination: route_id.to_string(),
            asset: asset.to_string(),
            amount,
            chain_id,
            required_tier: AuthorizationTier::Operational,
            policy_rule_ids: vec![],
            route_id: Some(route_id.to_string()),
            metadata: BTreeMap::new(),
            initiated_at_ms: now,
        };

        self.custody_client
            .execute(cmd)
            .await
            .map_err(|e| VaultControllerError::CustodyError(e.to_string()))
    }

    /// Get vault snapshot for inventory synchronization
    pub async fn get_vault_snapshot(
        &self,
        vault_type: VaultTypeClassification,
        chain_id: u32,
        asset: &str,
    ) -> Result<VaultSnapshot> {
        let vault_id = vault_type.vault_id(chain_id, asset);
        self.custody_client
            .get_vault(&vault_id)
            .await
            .map_err(|e| VaultControllerError::CustodyError(e.to_string()))
    }

    /// Transfer funds between vaults (rebalancing)
    pub async fn transfer_between_vaults(
        &self,
        source_vault_type: VaultTypeClassification,
        dest_vault_type: VaultTypeClassification,
        chain_id: u32,
        asset: &str,
        amount: u128,
        reason: &str,
    ) -> Result<VaultOperationResponse> {
        let source_vault_id = source_vault_type.vault_id(chain_id, asset);
        let dest_vault_id = dest_vault_type.vault_id(chain_id, asset);
        let operation_id = format!(
            "sweep-{}-{}",
            source_vault_id,
            Utc::now().timestamp_millis()
        );
        let now = Utc::now().timestamp_millis() as u64;

        let cmd = VaultOperationCommand {
            operation_id,
            operation_type: VaultOperationType::Sweep,
            source_vault_id,
            destination: dest_vault_id,
            asset: asset.to_string(),
            amount,
            chain_id,
            required_tier: AuthorizationTier::Strategic,
            policy_rule_ids: vec![],
            route_id: None,
            metadata: {
                let mut m = BTreeMap::new();
                m.insert("reason".to_string(), reason.to_string());
                m
            },
            initiated_at_ms: now,
        };

        self.custody_client
            .execute(cmd)
            .await
            .map_err(|e| VaultControllerError::CustodyError(e.to_string()))
    }

    /// Emergency freeze vault (requires Strategic tier authorization)
    pub async fn freeze_vault(
        &self,
        vault_type: VaultTypeClassification,
        chain_id: u32,
        asset: &str,
        reason: &str,
    ) -> Result<()> {
        // In production, would coordinate with custody service via gRPC
        // This is a placeholder for the boundary definition
        tracing::warn!(
            "Vault {}-{}-{} freeze requested: {}",
            vault_type.as_str(),
            chain_id,
            asset,
            reason
        );
        Ok(())
    }

    /// Export audit trail for compliance
    pub async fn export_audit_log(&self, since_ms: u64) -> Result<Vec<AuditLogEntry>> {
        self.custody_client
            .export_audit(since_ms)
            .await
            .map_err(|e| VaultControllerError::CustodyError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custody_service::client::MockCustodyClient;

    #[tokio::test]
    async fn test_vault_controller_creation() {
        let client = Box::new(MockCustodyClient::new());
        let controller = VaultController::new(client);

        let policy = SignerPolicy {
            signer_id: "signer-1".to_string(),
            max_single_operation: 10_000,
            max_daily_aggregate: 100_000,
            allowed_tiers: vec![AuthorizationTier::Operational],
            expires_at_ms: Utc::now().timestamp_millis() as u64 + 86_400_000,
        };

        controller.register_signer_policy(policy).unwrap();
    }

    #[tokio::test]
    async fn test_reserve_for_route() {
        let client = Box::new(MockCustodyClient::new());
        client
            .init_vault(
                "settlement-float-1-USDC".to_string(),
                "USDC".to_string(),
                50_000,
            )
            .unwrap();

        let controller = VaultController::new(client);

        let policy = SignerPolicy {
            signer_id: "signer-1".to_string(),
            max_single_operation: 10_000,
            max_daily_aggregate: 100_000,
            allowed_tiers: vec![AuthorizationTier::Operational],
            expires_at_ms: Utc::now().timestamp_millis() as u64 + 86_400_000,
        };
        controller.register_signer_policy(policy).unwrap();

        let response = controller
            .reserve_for_route(
                "route-1",
                VaultTypeClassification::SettlementFloat,
                1,
                "USDC",
                5_000,
                "signer-1",
            )
            .await
            .unwrap();

        assert_eq!(response.status, OperationStatus::Succeeded);
    }

    #[test]
    fn test_vault_id_generation() {
        let vault_id = VaultTypeClassification::SettlementFloat.vault_id(1, "USDC");
        assert_eq!(vault_id, "settlement-float-1-USDC");
    }
}
