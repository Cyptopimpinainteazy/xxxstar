/// Core types for custody service operations
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Vault operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum VaultOperationType {
    /// Transfer funds from vault to external address
    Transfer,
    /// Sweep funds to secondary vault for rebalancing
    Sweep,
    /// Reserve funds for route execution
    Reserve,
    /// Release previously reserved funds
    Release,
    /// Deposit funds into vault from external source
    Deposit,
    /// Emergency vault freeze (requires authorization)
    Freeze,
    /// Approve vault unfreezing (after investigation)
    Unfreeze,
}

/// Authorization policy tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationTier {
    /// Operational (routine transfers, reserves, releases)
    Operational,
    /// Strategic (sweeps, large rebalances, policy changes)
    Strategic,
    /// Emergency (freeze, unfreeze, loss-reserve draws)
    Emergency,
    /// Policy (vault creation, class changes, threshold updates)
    Policy,
}

/// Vault operation command — sent to service for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultOperationCommand {
    /// Unique operation ID for idempotency and audit trail
    pub operation_id: String,
    /// Type of operation
    pub operation_type: VaultOperationType,
    /// Source vault ID
    pub source_vault_id: String,
    /// Destination vault ID or external address
    pub destination: String,
    /// Asset being moved (e.g., "USDC", "ETH")
    pub asset: String,
    /// Amount in smallest unit (e.g., wei for ETH)
    pub amount: u128,
    /// Chain ID for cross-chain operations
    pub chain_id: u32,
    /// Required authorization tier
    pub required_tier: AuthorizationTier,
    /// Policy rule IDs that must be satisfied
    pub policy_rule_ids: Vec<String>,
    /// Optional route ID for settlement linkage
    pub route_id: Option<String>,
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
    /// Timestamp when operation was initiated (Unix ms)
    pub initiated_at_ms: u64,
}

/// Operation result status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Pending authorization
    PendingAuthorization,
    /// Authorized, awaiting execution
    Authorized,
    /// Currently being executed on-chain
    Executing,
    /// Successfully completed
    Succeeded,
    /// Failed (see result for reason)
    Failed,
    /// Cancelled before execution
    Cancelled,
}

/// Cryptographic proof of vault operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationProof {
    /// Transaction hash or operation identifier on-chain
    pub tx_hash: String,
    /// Block number where operation settled
    pub block_number: u64,
    /// Timestamp of settlement (Unix ms)
    pub settled_at_ms: u64,
    /// Merkle proof or signature proof from HSM
    pub proof_data: Vec<u8>,
    /// Proof type (e.g., "tx_receipt", "hsm_signature", "merkle_proof")
    pub proof_type: String,
}

/// Vault operation response — service result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultOperationResponse {
    /// Echo of the request operation ID
    pub operation_id: String,
    /// Current status
    pub status: OperationStatus,
    /// Reason if operation failed
    pub failure_reason: Option<String>,
    /// Cryptographic proof if settled
    pub proof: Option<OperationProof>,
    /// Vault balance after operation (if successful)
    pub vault_balance_after: Option<u128>,
    /// Optional execution receipt
    pub receipt: Option<String>,
    /// Timestamp of response (Unix ms)
    pub responded_at_ms: u64,
}

/// Vault state snapshot for inventory integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSnapshot {
    /// Vault ID
    pub vault_id: String,
    /// Asset type
    pub asset: String,
    /// Available balance (not reserved/pending)
    pub available_balance: u128,
    /// Reserved for routes
    pub reserved_balance: u128,
    /// Pending outbound (waiting for proof)
    pub pending_out_balance: u128,
    /// Pending inbound (waiting for settlement)
    pub pending_in_balance: u128,
    /// Vault status (active, frozen, etc.)
    pub status: VaultStatus,
    /// Timestamp of snapshot (Unix ms)
    pub snapshot_at_ms: u64,
    /// Last operation applied
    pub last_operation_id: Option<String>,
    /// Merkle root of vault state for proof
    pub merkle_root: String,
}

/// Vault lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultStatus {
    /// Normal operations allowed
    Active,
    /// No new operations, pending releases only
    Degraded,
    /// Emergency hold, only authorized ops
    Frozen,
    /// Vault being decommissioned
    Sunset,
}

/// Audit log entry for every vault operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique log entry ID
    pub entry_id: String,
    /// Operation ID being logged
    pub operation_id: String,
    /// User/signer who authorized
    pub authorized_by: String,
    /// Authorization tier used
    pub tier_used: AuthorizationTier,
    /// Operation type
    pub operation_type: VaultOperationType,
    /// Source vault
    pub source_vault_id: String,
    /// Destination
    pub destination: String,
    /// Asset and amount
    pub asset: String,
    pub amount: u128,
    /// Policy rules checked
    pub policy_rules_checked: Vec<String>,
    /// All rules passed?
    pub all_rules_passed: bool,
    /// Final status after execution
    pub final_status: OperationStatus,
    /// Failure reason if any
    pub failure_reason: Option<String>,
    /// Timestamp (Unix ms)
    pub timestamp_ms: u64,
    /// Immutable hash of this entry for blockchain anchoring
    pub entry_hash: String,
}

/// Authorization request — for policy approval workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    /// Request ID
    pub request_id: String,
    /// Operation command to authorize
    pub command: VaultOperationCommand,
    /// Required authorization tier
    pub required_tier: AuthorizationTier,
    /// Requestor identity
    pub requestor: String,
    /// Reason for operation
    pub reason: String,
    /// Created at (Unix ms)
    pub created_at_ms: u64,
    /// Expires at (Unix ms) — operations must execute before this
    pub expires_at_ms: u64,
}

/// Authorization decision — approval or rejection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationDecision {
    /// Request ID being decided
    pub request_id: String,
    /// Approved or rejected
    pub approved: bool,
    /// Approver identity
    pub approver: String,
    /// Reason for decision
    pub reason: String,
    /// Timestamp (Unix ms)
    pub timestamp_ms: u64,
    /// Signer path used for cryptographic proof
    pub signer_proof: Option<String>,
}

/// HSM key reference for signing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HSMKeyReference {
    /// Key identifier in HSM
    pub key_id: String,
    /// Key algorithm (RSA-2048, ECDSA-P256, etc.)
    pub algorithm: String,
    /// Key creation timestamp
    pub created_at_ms: u64,
    /// Last rotation timestamp
    pub last_rotated_at_ms: u64,
    /// Can this key be used for vault operations?
    pub is_vault_key: bool,
}

/// Settlement obligation linked to vault operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementLinkage {
    /// Route ID that triggered this vault operation
    pub route_id: String,
    /// Lane ID
    pub lane_id: String,
    /// Source chain
    pub source_chain: u32,
    /// Destination chain
    pub dest_chain: u32,
    /// Expected settlement amount
    pub settlement_amount: u128,
    /// Proof requirement
    pub proof_requirement: String,
    /// Settlement timeout (Unix ms)
    pub timeout_ms: u64,
}
