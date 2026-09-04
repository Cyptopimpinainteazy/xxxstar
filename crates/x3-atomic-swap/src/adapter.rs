//! # X3 VM Adapter Interface
//!
//! Defines the shared trait interface for all VM-family adapters, plus shared
//! types for proofs, fees, chain health, finality, and readiness scoring.
//!
//! Each supported VM (EVM, SVM, Substrate, Bitcoin Script, X3VM, MoveVM, ...)
//! implements the [`X3VmAdapter`] trait so the relayer and swap pipeline can
//! interact uniformly across execution environments.

use crate::error::SwapError;
use crate::intent::{AtomicIntent, IntentId};

// ─────────────────────────────────────────────────────────────────────────────
// Re-exported type aliases (canonical sources in intent.rs)
// ─────────────────────────────────────────────────────────────────────────────
/// Chain identifier (free-form string, e.g. "ethereum-mainnet").
pub type ChainId = alloc::string::String;
/// Asset identifier (free-form string, e.g. "USDC").
pub type AssetId = alloc::string::String;
/// Transaction ID / hash.
pub type TxId = alloc::string::String;

// ─────────────────────────────────────────────────────────────────────────────
// VmType
// ─────────────────────────────────────────────────────────────────────────────

/// Supported VM execution environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VmType {
    Evm,
    Svm,
    Substrate,
    BitcoinScript,
    X3Vm,
    MoveVm,
    CosmWasm,
    CairoVm,
    PlutusEutxo,
    TonTvm,
    FuelVm,
    NearWasm,
    SorobanWasm,
    PolkadotPvm,
    InkWasm,
    ZkVm,
}

impl VmType {
    pub fn name(&self) -> &'static str {
        match self {
            VmType::Evm => "EVM",
            VmType::Svm => "SVM/Solana",
            VmType::Substrate => "Substrate",
            VmType::BitcoinScript => "Bitcoin Script/Taproot",
            VmType::X3Vm => "X3VM",
            VmType::MoveVm => "MoveVM",
            VmType::CosmWasm => "CosmWasm",
            VmType::CairoVm => "CairoVM",
            VmType::PlutusEutxo => "Plutus/eUTXO",
            VmType::TonTvm => "TON TVM",
            VmType::FuelVm => "FuelVM",
            VmType::NearWasm => "NEAR WASM",
            VmType::SorobanWasm => "Soroban WASM",
            VmType::PolkadotPvm => "Polkadot PVM",
            VmType::InkWasm => "ink! WASM",
            VmType::ZkVm => "zkVM",
        }
    }

    /// Family grouping for dispatch / capability detection.
    pub fn family(&self) -> &'static str {
        match self {
            VmType::Evm => "evm-family",
            VmType::Svm => "svm-family",
            VmType::Substrate => "substrate-family",
            VmType::BitcoinScript => "utxo-family",
            VmType::X3Vm => "x3-native",
            VmType::MoveVm => "move-family",
            VmType::CosmWasm => "cosmwasm-family",
            VmType::CairoVm => "cairo-family",
            VmType::PlutusEutxo => "eutxo-family",
            VmType::TonTvm => "ton-family",
            VmType::FuelVm => "fuel-family",
            VmType::NearWasm => "near-family",
            VmType::SorobanWasm => "soroban-family",
            VmType::PolkadotPvm => "polkadot-family",
            VmType::InkWasm => "ink-family",
            VmType::ZkVm => "zk-family",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Proof Types
// ─────────────────────────────────────────────────────────────────────────────

/// Proof of a lock transaction on a VM chain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockProof {
    pub tx_id: TxId,
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub block_number: u64,
    pub block_hash: alloc::string::String,
    pub confirmations: u64,
    pub lock_address: alloc::string::String,
    pub locked_amount: u128,
    pub hashlock: [u8; 32],
    pub receiver: alloc::vec::Vec<u8>,
    pub refund_address: alloc::vec::Vec<u8>,
    pub timeout: u64,
    pub raw_proof: alloc::vec::Vec<u8>,
}

/// Proof of a claim transaction on a VM chain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClaimProof {
    pub tx_id: TxId,
    pub intent_id: IntentId,
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub preimage: [u8; 32],
    pub block_number: u64,
    pub block_hash: alloc::string::String,
    pub raw_proof: alloc::vec::Vec<u8>,
}

/// Proof of a refund transaction on a VM chain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefundProof {
    pub tx_id: TxId,
    pub intent_id: IntentId,
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub block_number: u64,
    pub block_hash: alloc::string::String,
    pub raw_proof: alloc::vec::Vec<u8>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Fee & Health Types
// ─────────────────────────────────────────────────────────────────────────────

/// Fee estimation result for a given chain / VM.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeeEstimate {
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub native_fee: u128,
    pub gas_units: u64,
    pub gas_price: u128,
    pub estimated_usd: f64,
}

/// Chain health status snapshot.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainHealth {
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub latest_block: u64,
    pub finalized_block: u64,
    pub block_delay_ms: u64,
    pub finality_delay_ms: u64,
    pub rpc_quorum_healthy: bool,
    pub gas_price: u128,
    pub halted: bool,
    pub degraded: bool,
    pub safe_for_new_intents: bool,
}

/// Finality proof - confirms a transaction is finalised on a particular chain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FinalityProof {
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub tx_id: TxId,
    pub block_number: u64,
    pub block_hash: alloc::string::String,
    pub confirmations: u64,
    pub finalized: bool,
    pub finality_source: alloc::string::String,
    pub safe_to_reveal_secret: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// AdapterReadinessScore
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter readiness score (0-100) for a VM-family adapter.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdapterReadinessScore {
    pub adapter_name: &'static str,
    pub vm_type: VmType,
    pub interface_implemented: bool,
    pub lock_path: bool,
    pub claim_path: bool,
    pub refund_path: bool,
    pub event_proof_extraction: bool,
    pub finality_proof: bool,
    pub rpc_indexer_support: bool,
    pub timeout_safety: bool,
    pub tests_implemented: bool,
    pub proof_ledger_integration: bool,
    pub ibc_support: bool,
}

impl AdapterReadinessScore {
    /// Compute the total readiness score (0-100).
    pub fn score(&self) -> u32 {
        let mut s = 0u32;
        if self.interface_implemented {
            s += 10;
        }
        if self.lock_path {
            s += 10;
        }
        if self.claim_path {
            s += 10;
        }
        if self.refund_path {
            s += 10;
        }
        if self.event_proof_extraction {
            s += 10;
        }
        if self.finality_proof {
            s += 10;
        }
        if self.rpc_indexer_support {
            s += 10;
        }
        if self.timeout_safety {
            s += 10;
        }
        if self.tests_implemented {
            s += 10;
        }
        if self.proof_ledger_integration {
            s += 10;
        }
        if self.ibc_support {
            s += 10;
        }
        s
    }

    /// Return the list of missing capability names.
    pub fn missing_items(&self) -> alloc::vec::Vec<&'static str> {
        let mut missing = alloc::vec::Vec::new();
        if !self.interface_implemented {
            missing.push("interface_implemented");
        }
        if !self.lock_path {
            missing.push("lock_path");
        }
        if !self.claim_path {
            missing.push("claim_path");
        }
        if !self.refund_path {
            missing.push("refund_path");
        }
        if !self.event_proof_extraction {
            missing.push("event_proof_extraction");
        }
        if !self.finality_proof {
            missing.push("finality_proof");
        }
        if !self.rpc_indexer_support {
            missing.push("rpc_indexer_support");
        }
        if !self.timeout_safety {
            missing.push("timeout_safety");
        }
        if !self.tests_implemented {
            missing.push("tests_implemented");
        }
        if !self.proof_ledger_integration {
            missing.push("proof_ledger_integration");
        }
        if !self.ibc_support {
            missing.push("ibc_support");
        }
        missing
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Shared interface that every VM-family adapter must implement.
///
/// The relayer, dashboard, and settlement pipeline interact with all chains
/// through this single trait, keeping the core swap logic VM-agnostic.
pub trait X3VmAdapter: Send + Sync {
    /// The VM type this adapter implements.
    fn vm_type(&self) -> VmType;

    /// Human-readable adapter name (e.g. "evm-htlc-v2").
    fn adapter_name(&self) -> &'static str;

    /// Chain identifiers this adapter can handle.
    fn supported_chains(&self) -> Vec<ChainId>;

    /// Asset identifiers this adapter can handle.
    fn supported_assets(&self) -> Vec<AssetId>;

    // ── Lifecycle operations ──────────────────────────────────────────────

    /// Lock funds on the source chain as part of an HTLC.
    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError>;

    /// Claim locked funds on the destination chain by revealing the preimage.
    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError>;

    /// Refund locked funds after the timeout has elapsed.
    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError>;

    // ── Verification ──────────────────────────────────────────────────────

    /// Verify a lock proof without submitting a transaction.
    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError>;

    /// Verify a claim proof.
    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError>;

    /// Verify a refund proof.
    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError>;

    // ── Estimation & Health ───────────────────────────────────────────────

    /// Estimate the fee for executing an intent on this chain.
    fn estimate_fee(&self, intent: &AtomicIntent) -> Result<FeeEstimate, SwapError>;

    /// Get finality status for a given transaction.
    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError>;

    /// Get the current chain health snapshot.
    fn chain_health(&self) -> Result<ChainHealth, SwapError>;

    // ── Readiness ─────────────────────────────────────────────────────────

    /// Self-reported readiness score for this adapter.
    fn readiness_score(&self) -> AdapterReadinessScore;
}
