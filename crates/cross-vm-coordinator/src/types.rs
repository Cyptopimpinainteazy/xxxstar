//! Core types for the Cross-VM coordinator.

use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

// ─── VM & Chain Identifiers ───────────────────────────────────────────────────

/// Target virtual machine type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VmTarget {
    /// Ethereum Virtual Machine (chain_id identifies the network).
    Evm { chain_id: u64 },
    /// Solana Virtual Machine.
    Svm,
    /// X3 Chain native VM (WASM-based).
    X3Vm,
}

impl fmt::Display for VmTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmTarget::Evm { chain_id } => write!(f, "EVM({chain_id})"),
            VmTarget::Svm => write!(f, "SVM"),
            VmTarget::X3Vm => write!(f, "X3VM"),
        }
    }
}

// ─── HTLC Types ───────────────────────────────────────────────────────────────

/// HTLC secret (32-byte preimage).
#[derive(Clone)]
pub struct HtlcSecret(pub [u8; 32]);

impl HtlcSecret {
    /// Generate a cryptographically secure random secret using the OS CSPRNG.
    ///
    /// # Security
    /// Uses `rand::rngs::OsRng` which reads from the OS entropy source
    /// (getrandom syscall / /dev/urandom on Linux, CryptGenRandom on Windows).
    /// **DO NOT** use time- or PID-based entropy for HTLC secrets in production —
    /// such seeds are predictable and allow attackers to brute-force HTLC locks.
    pub fn generate() -> Self {
        let mut rng = rand::rngs::OsRng;
        let mut rng_bytes = [0u8; 32];
        rng.fill_bytes(&mut rng_bytes);
        Self(rng_bytes)
    }

    /// Compute SHA-256 hash of the secret.
    pub fn hash(&self) -> HtlcHash {
        let mut hasher = Sha256::new();
        hasher.update(self.0);
        let hash = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&hash);
        HtlcHash(out)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for HtlcSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HtlcSecret(***)")
    }
}

/// SHA-256 hash of an HTLC secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HtlcHash(pub [u8; 32]);

impl HtlcHash {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for HtlcHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(&self.0[..8]))
    }
}

/// Unique HTLC identifier on a specific chain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HtlcId(pub Vec<u8>);

impl HtlcId {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

/// HTLC status (mirrors on-chain state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HtlcStatus {
    /// Created but not yet fully confirmed.
    Pending,
    /// Funds locked, awaiting claim or refund.
    Funded,
    /// Claimed with valid secret.
    Claimed,
    /// Refunded after timelock expiry.
    Refunded,
    /// Expired (timelock passed, refund available).
    Expired,
}

/// Parameters for creating an HTLC on a specific chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtlcCreateParams {
    /// Target VM.
    pub vm: VmTarget,
    /// Recipient address (chain-specific encoding).
    pub recipient: Vec<u8>,
    /// SHA-256 hash lock.
    pub hash_lock: HtlcHash,
    /// Unix timestamp for timelock.
    pub timelock: u64,
    /// Asset identifier (contract address on EVM, mint on SVM, AssetId on X3).
    pub asset: Vec<u8>,
    /// Amount in smallest denomination.
    pub amount: u128,
}

/// Record of a created HTLC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtlcRecord {
    /// On-chain HTLC identifier.
    pub id: HtlcId,
    /// Creation parameters.
    pub params: HtlcCreateParams,
    /// Current status.
    pub status: HtlcStatus,
    /// Block number when created.
    pub created_at_block: u64,
    /// Required confirmations before proceeding.
    pub confirmations_required: u32,
    /// Current confirmations.
    pub confirmations: u32,
    /// Blake3 hash of the creation parameters for integrity verification.
    pub params_hash: [u8; 32],
}

// ─── Flashloan Types ──────────────────────────────────────────────────────────

/// Flashloan provider on a specific chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlashloanProvider {
    /// Aave V3 (EVM) — 0.05% fee.
    AaveV3,
    /// Balancer V2 Vault (EVM) — 0% fee.
    BalancerV2,
    /// Uniswap V3 flash swap (EVM) — pool fee (0.05–1%).
    UniswapV3 { fee_tier: u32 },
    /// Solend (SVM) — 0.3% fee.
    Solend,
    /// MarginFi (SVM) — 0% fee.
    MarginFi,
    /// Kamino (SVM) — low/0% fee.
    Kamino,
    /// Euler (EVM) — 0% fee.
    Euler,
    /// X3 native flashloan pool.
    X3Native,
}

impl FlashloanProvider {
    /// Fee in basis points.
    pub fn fee_bps(&self) -> u32 {
        match self {
            Self::AaveV3 => 5,     // 0.05%
            Self::BalancerV2 => 0, // 0%
            Self::UniswapV3 { fee_tier } => *fee_tier / 100,
            Self::Solend => 30,  // 0.3%
            Self::MarginFi => 0, // 0%
            Self::Kamino => 0,   // 0%
            Self::Euler => 0,    // 0%
            Self::X3Native => 9, // 0.09% (X3 default)
        }
    }

    /// Whether this provider is available on the given VM.
    pub fn supports_vm(&self, vm: &VmTarget) -> bool {
        match (self, vm) {
            (Self::AaveV3, VmTarget::Evm { .. }) => true,
            (Self::BalancerV2, VmTarget::Evm { .. }) => true,
            (Self::UniswapV3 { .. }, VmTarget::Evm { .. }) => true,
            (Self::Euler, VmTarget::Evm { .. }) => true,
            (Self::Solend, VmTarget::Svm) => true,
            (Self::MarginFi, VmTarget::Svm) => true,
            (Self::Kamino, VmTarget::Svm) => true,
            (Self::X3Native, VmTarget::X3Vm) => true,
            _ => false,
        }
    }
}

/// A flashloan leg — borrow + swap + repay on a single chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLeg {
    /// Target VM for this leg.
    pub vm: VmTarget,
    /// Flashloan provider to use.
    pub provider: FlashloanProvider,
    /// Asset to borrow.
    pub borrow_asset: Vec<u8>,
    /// Amount to borrow.
    pub borrow_amount: u128,
    /// DEX/swap target contract/program.
    pub swap_target: Vec<u8>,
    /// Swap calldata/instruction data.
    pub swap_data: Vec<u8>,
    /// Expected minimum output (slippage protection).
    pub min_output: u128,
    /// Gas/compute limit for this leg.
    pub gas_limit: u64,
}

/// Outcome of executing a flashloan leg.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlashLegOutcome {
    /// Leg succeeded: borrowed, swapped, repaid within one tx.
    Success {
        tx_hash: Vec<u8>,
        gas_used: u64,
        output_amount: u128,
        premium_paid: u128,
    },
    /// Leg reverted: entire atomic tx rolled back.
    Reverted { reason: String },
}

// ─── Swap Session Types ───────────────────────────────────────────────────────

/// The overall atomic swap session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Hash lock shared by all HTLCs.
    pub hash_lock: HtlcHash,
    /// Source/fast chain HTLC.
    pub htlc_fast: Option<HtlcRecord>,
    /// Destination/slow chain HTLC.
    pub htlc_slow: Option<HtlcRecord>,
    /// Flashloan legs.
    pub flash_legs: Vec<FlashLeg>,
    /// Leg outcomes.
    pub leg_outcomes: Vec<FlashLegOutcome>,
    /// Current phase.
    pub phase: SwapPhase,
    /// Timelock for fast chain.
    pub timelock_fast: u64,
    /// Timelock for slow chain.
    pub timelock_slow: u64,
    /// Creation timestamp (unix seconds).
    pub created_at: u64,
    /// Last update timestamp.
    pub updated_at: u64,
    /// Whether this session requires Merkle proof verification during settlement.
    ///
    /// Set `true` for cross-chain swaps that involve chains requiring
    /// transaction inclusion proofs (e.g. Ethereum, Solana finalized commitment).
    /// When `true`, `MerkleSettlementCoordinator::session_requires_merkle` returns
    /// `true` and the settlement path verifies the Merkle proof before releasing funds.
    pub requires_merkle_verification: bool,
}

/// Phase of the atomic swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapPhase {
    /// Initial setup: generating secret, computing hash.
    Setup,
    /// Locking funds in HTLCs on both chains.
    LockingHtlcs,
    /// HTLCs locked, waiting for confirmations.
    HtlcsLocked,
    /// Executing flashloan legs on each chain.
    ExecutingFlashLegs,
    /// All legs succeeded, ready to reveal secret.
    LegsComplete,
    /// Revealing secret (claiming) on fast chain.
    ClaimingFast,
    /// Claiming on slow chain after secret revealed.
    ClaimingSlow,
    /// Both sides claimed, swap complete.
    Complete,
    /// Swap aborted — refunding via timelock.
    Aborting,
    /// Refunds complete.
    Refunded,
    /// Fatal error — manual intervention required.
    Failed,
}

impl fmt::Display for SwapPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Setup => "SETUP",
            Self::LockingHtlcs => "LOCKING_HTLCS",
            Self::HtlcsLocked => "HTLCS_LOCKED",
            Self::ExecutingFlashLegs => "EXECUTING_FLASH_LEGS",
            Self::LegsComplete => "LEGS_COMPLETE",
            Self::ClaimingFast => "CLAIMING_FAST",
            Self::ClaimingSlow => "CLAIMING_SLOW",
            Self::Complete => "COMPLETE",
            Self::Aborting => "ABORTING",
            Self::Refunded => "REFUNDED",
            Self::Failed => "FAILED",
        };
        write!(f, "{s}")
    }
}

/// Errors from the coordinator.
#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    #[error("Invalid phase transition: {from} → {to}")]
    InvalidPhaseTransition { from: String, to: String },

    #[error("HTLC creation failed on {vm}: {reason}")]
    HtlcCreationFailed { vm: String, reason: String },

    #[error("HTLC confirmation timeout on {vm}")]
    HtlcConfirmationTimeout { vm: String },

    #[error("Flashloan leg reverted on {vm}: {reason}")]
    FlashLegReverted { vm: String, reason: String },

    #[error("Secret reveal failed on {vm}: {reason}")]
    SecretRevealFailed { vm: String, reason: String },

    #[error("Timelock expired: {htlc_id}")]
    TimelockExpired { htlc_id: String },

    #[error("Insufficient confirmations: {have}/{need}")]
    InsufficientConfirmations { have: u32, need: u32 },

    #[error("Flashloan provider unavailable: {provider:?} on {vm}")]
    ProviderUnavailable {
        provider: FlashloanProvider,
        vm: String,
    },

    #[error("Inventory insufficient: need {need}, have {have}")]
    InsufficientInventory { need: u128, have: u128 },

    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Internal error: {0}")]
    Internal(String),
}
