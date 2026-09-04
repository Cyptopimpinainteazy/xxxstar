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
// VmFamily
// ─────────────────────────────────────────────────────────────────────────────

/// Major execution-model families. Organises adapters by how they execute
/// smart contracts, not by chain name. A single family may contain several
/// [`VmType`] variants that share similar accounting and finality models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VmFamily {
    /// Account-based model (EVM, NEAR, Substrate).
    Account,
    /// UTXO-based model (Bitcoin, Cardano, Fuel).
    Utxo,
    /// Object/resource-based model (Move/Sui/Aptos, Solana).
    Object,
    /// WASM-based smart-contract model (CosmWasm, NEAR, Soroban, ICP, MultiversX, Archway).
    Wasm,
    /// Message/actor-based model (TON TVM).
    Message,
    /// ZK/rollup model (CairoVM, RISC Zero, SP1).
    Zk,
    /// Runtime pallet model (Substrate, Polkadot PVM).
    Runtime,
    /// Native X3VM.
    Native,
}

impl VmFamily {
    pub fn name(&self) -> &'static str {
        match self {
            VmFamily::Account => "Account",
            VmFamily::Utxo => "UTXO",
            VmFamily::Object => "Object",
            VmFamily::Wasm => "WASM",
            VmFamily::Message => "Message",
            VmFamily::Zk => "ZK",
            VmFamily::Runtime => "Runtime",
            VmFamily::Native => "Native",
        }
    }
}

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
    /// WASM L1 family (Internet Computer, MultiversX, Archway).
    WasmL1,
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
            VmType::WasmL1 => "WASM L1 (ICP/MultiversX/Archway)",
            VmType::ZkVm => "zkVM",
        }
    }

    /// Family grouping for dispatch / capability detection.
    pub fn family(&self) -> VmFamily {
        match self {
            VmType::Evm => VmFamily::Account,
            VmType::Svm => VmFamily::Object,
            VmType::Substrate => VmFamily::Runtime,
            VmType::BitcoinScript => VmFamily::Utxo,
            VmType::X3Vm => VmFamily::Native,
            VmType::MoveVm => VmFamily::Object,
            VmType::CosmWasm => VmFamily::Wasm,
            VmType::CairoVm => VmFamily::Zk,
            VmType::PlutusEutxo => VmFamily::Utxo,
            VmType::TonTvm => VmFamily::Message,
            VmType::FuelVm => VmFamily::Utxo,
            VmType::NearWasm => VmFamily::Wasm,
            VmType::SorobanWasm => VmFamily::Wasm,
            VmType::PolkadotPvm => VmFamily::Runtime,
            VmType::InkWasm => VmFamily::Wasm,
            VmType::WasmL1 => VmFamily::Wasm,
            VmType::ZkVm => VmFamily::Zk,
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

/// Adapter readiness score (0-120) for a VM-family adapter.
///
/// 12 criteria × 10 points each = max 120. The scoreboard normalises to 100.
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
    /// IBC support (relevant for CosmWasm / Cosmos-family chains).
    pub ibc_support: bool,
    /// Cross-adapter atomicity test exists (even with mock counterpart).
    /// Prevents interface drift between different VM adapters.
    pub cross_adapter_atomicity_test: bool,
}

impl AdapterReadinessScore {
    /// Compute the total readiness score (0-120).
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
        if self.cross_adapter_atomicity_test {
            s += 10;
        }
        s.min(100)
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
        if !self.cross_adapter_atomicity_test {
            missing.push("cross_adapter_atomicity_test");
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

    // ── Production gate ─────────────────────────────────────────────────

    /// Whether this adapter is in simulation mode.
    ///
    /// Simulated adapters fabricate tx hashes from deterministic derivation
    /// (no real chain transaction, no real key material).  They are safe for
    /// testing and cross-adapter integration but **must not** be used in
    /// production paths where real funds are at stake.
    ///
    /// Adapters that return `true` will be rejected by the production relayer
    /// and settlement engine unless explicitly overridden via a compile-time
    /// or governance-controlled feature flag.
    ///
    /// ## Production-readiness bar (all must hold before overriding to `false`)
    ///
    /// 1. **Real chain RPC / indexer data** — lock/claim/refund operations
    ///    submit actual transactions to a real chain endpoint (not a mock RPC
    ///    client) and read on-chain state from a real indexer.
    /// 2. **Verifiable lock/claim/refund proofs** — every proof carries a
    ///    real on-chain transaction ID, block hash, and block number obtained
    ///    from the chain, plus a verifiable cryptographic proof (receipt,
    ///    Merkle path, or light-client inclusion proof).
    /// 3. **Real-environment integration tests** — the adapter passes a test
    ///    suite that runs against a real testnet or local node for that VM
    ///    family (not mocked chain state).
    /// 4. **Key-material separation** — signing keys are provided externally
    ///    (env, KMS, HSM); no private key is hardcoded in the adapter.
    /// 5. **Failure-path coverage** — lock timeout, claim-after-refund,
    ///    refund-before-timeout, double-claim, and double-refund are tested
    ///    with real chain finality semantics.
    ///
    /// Adapters that do not meet all five criteria **must** keep the default
    /// `true` return.  Marking an adapter non-simulated before it satisfies
    /// these criteria is a production-safety violation.
    fn is_simulated(&self) -> bool {
        true // safe default: every adapter is simulated until proven otherwise
    }

    /// Self-reported readiness score for this adapter.
    fn readiness_score(&self) -> AdapterReadinessScore;
}

// ─────────────────────────────────────────────────────────────────────────────
// Cross-Adapter Atomicity Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod cross_adapter_tests {
    use super::*;
    use crate::intent::{AtomicIntentBuilder, ChainKind, RefundPath};

    fn sample_intent(id: u64, _src_chain: &str, _dst_chain: &str) -> AtomicIntent {
        AtomicIntentBuilder::new()
            .source_chain(ChainKind::Ethereum)
            .destination_chain(ChainKind::Solana)
            .source_asset("SRC")
            .destination_asset("DST")
            .amount_in(1000)
            .min_amount_out(990)
            .receiver("receiver_alice")
            .hashlock([1u8; 32])
            .source_timeout(200)
            .destination_timeout(100)
            .refund_path(RefundPath {
                chain: ChainKind::Ethereum,
                address: "refund_bob".into(),
                asset: None,
            })
            .build(id)
            .expect("intent should build")
    }

    /// Helper: lock with source adapter, claim with destination adapter using
    /// the preimage from the lock proof.
    fn lock_and_claim(
        src: &dyn X3VmAdapter,
        dst: &dyn X3VmAdapter,
        intent: &AtomicIntent,
    ) -> Result<(LockProof, ClaimProof), SwapError> {
        let lock = src.lock(intent)?;
        let preimage = lock.hashlock; // hashlock IS the preimage in mock adapters
        let claim = dst.claim(intent.intent_id, preimage)?;
        Ok((lock, claim))
    }

    // ── Account family → Object family ─────────────────────────────────────

    #[test]
    fn test_cross_adapter_evm_to_svm() {
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let svm = crate::svm_htlc::SvmAdapter::at_program_id([0x02u8; 32]);
        let intent = sample_intent(1, "eth", "sol");
        let (lock, claim) = lock_and_claim(&evm, &svm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(claim.preimage.iter().any(|&b| b != 0));
        // Verify families
        assert_eq!(evm.vm_type().family(), VmFamily::Account);
        assert_eq!(svm.vm_type().family(), VmFamily::Object);
        // Verify both verify methods work
        assert!(evm.verify_lock(&lock).unwrap());
        assert!(svm.verify_claim(&claim).unwrap());
    }

    #[test]
    fn test_cross_adapter_evm_to_movevm() {
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let move_vm = crate::move_vm_htlc::MoveVmAdapter::new("sui-mainnet".into());
        let intent = sample_intent(2, "eth", "sui");
        let (lock, claim) = lock_and_claim(&evm, &move_vm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(evm.verify_lock(&lock).unwrap());
        assert!(move_vm.verify_claim(&claim).unwrap());
        assert_eq!(evm.vm_type().family(), VmFamily::Account);
        assert_eq!(move_vm.vm_type().family(), VmFamily::Object);
    }

    // ── UTXO family → Account family ───────────────────────────────────────

    #[test]
    fn test_cross_adapter_bitcoin_to_evm() {
        let btc = crate::bitcoin_htlc::BtcHtlcAdapter::new(crate::bitcoin_htlc::BitcoinNetwork::Mainnet);
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let intent = sample_intent(3, "btc", "eth");
        let (lock, claim) = lock_and_claim(&btc, &evm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(btc.verify_lock(&lock).unwrap());
        assert!(evm.verify_claim(&claim).unwrap());
        assert_eq!(btc.vm_type().family(), VmFamily::Utxo);
        assert_eq!(evm.vm_type().family(), VmFamily::Account);
    }

    #[test]
    fn test_cross_adapter_fuel_to_evm() {
        let fuel = crate::fuel_htlc::FuelHtlcAdapter::new(
            "fuel-mainnet".into(),
            crate::fuel_htlc::FuelNetwork::Mainnet,
        );
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let intent = sample_intent(4, "fuel", "eth");
        let (lock, claim) = lock_and_claim(&fuel, &evm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(fuel.verify_lock(&lock).unwrap());
        assert!(evm.verify_claim(&claim).unwrap());
        assert_eq!(fuel.vm_type().family(), VmFamily::Utxo);
    }

    // ── WASM family → Account family ───────────────────────────────────────

    #[test]
    fn test_cross_adapter_cosmwasm_to_evm() {
        let cw = crate::cosmwasm_htlc::CosmWasmAdapter::new("osmosis-1".into());
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let intent = sample_intent(5, "osmo", "eth");
        let (lock, claim) = lock_and_claim(&cw, &evm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(cw.verify_lock(&lock).unwrap());
        assert!(evm.verify_claim(&claim).unwrap());
        assert_eq!(cw.vm_type().family(), VmFamily::Wasm);
    }

    #[test]
    fn test_cross_adapter_near_to_svm() {
        let near = crate::near_htlc::NearHtlcAdapter::new(
            "near-mainnet".into(),
            crate::near_htlc::NearNetwork::Mainnet,
        );
        let svm = crate::svm_htlc::SvmAdapter::at_program_id([0x02u8; 32]);
        let intent = sample_intent(6, "near", "sol");
        let (lock, claim) = lock_and_claim(&near, &svm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(near.verify_lock(&lock).unwrap());
        assert!(svm.verify_claim(&claim).unwrap());
        assert_eq!(near.vm_type().family(), VmFamily::Wasm);
    }

    #[test]
    fn test_cross_adapter_wasm_l1_to_evm() {
        let icp = crate::wasm_l1_htlc::WasmL1Adapter::new(
            "icp-mainnet".into(),
            crate::wasm_l1_htlc::WasmL1Runtime::InternetComputer,
        );
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let intent = sample_intent(7, "icp", "eth");
        let (lock, claim) = lock_and_claim(&icp, &evm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(icp.verify_lock(&lock).unwrap());
        assert!(evm.verify_claim(&claim).unwrap());
        assert_eq!(icp.vm_type().family(), VmFamily::Wasm);
        assert_eq!(icp.vm_type(), VmType::WasmL1);
    }

    // ── ZK family → Account family ─────────────────────────────────────────

    #[test]
    fn test_cross_adapter_cairo_to_evm() {
        let cairo = crate::cairo_vm_htlc::CairoVmAdapter::new("starknet-mainnet".into());
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let intent = sample_intent(8, "starknet", "eth");
        let (lock, claim) = lock_and_claim(&cairo, &evm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(cairo.verify_lock(&lock).unwrap());
        assert!(evm.verify_claim(&claim).unwrap());
        assert_eq!(cairo.vm_type().family(), VmFamily::Zk);
    }

    // ── Message family → Account family ────────────────────────────────────

    #[test]
    fn test_cross_adapter_ton_to_evm() {
        let ton = crate::ton_htlc::TonHtlcAdapter::new(
            "ton-mainnet".into(),
            crate::ton_htlc::TonNetwork::Mainnet,
        );
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let intent = sample_intent(9, "ton", "eth");
        let (lock, claim) = lock_and_claim(&ton, &evm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(ton.verify_lock(&lock).unwrap());
        assert!(evm.verify_claim(&claim).unwrap());
        assert_eq!(ton.vm_type().family(), VmFamily::Message);
    }

    // ── Runtime family → Account family ────────────────────────────────────

    #[test]
    fn test_cross_adapter_substrate_to_evm() {
        let sub = crate::substrate_htlc::SubstrateHtlcAdapter::new("polkadot-mainnet".into());
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let intent = sample_intent(10, "dot", "eth");
        let (lock, claim) = lock_and_claim(&sub, &evm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(sub.verify_lock(&lock).unwrap());
        assert!(evm.verify_claim(&claim).unwrap());
        assert_eq!(sub.vm_type().family(), VmFamily::Runtime);
    }

    // ── Native → Account family ────────────────────────────────────────────

    #[test]
    fn test_cross_adapter_x3vm_to_evm() {
        let x3 = crate::x3vm_htlc::X3VmAdapterImpl::new("x3-mainnet".into());
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let intent = sample_intent(11, "x3", "eth");
        let (lock, claim) = lock_and_claim(&x3, &evm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(x3.verify_lock(&lock).unwrap());
        assert!(evm.verify_claim(&claim).unwrap());
        assert_eq!(x3.vm_type().family(), VmFamily::Native);
    }

    // ── UTXO → UTXO (Bitcoin → Cardano) ────────────────────────────────────

    #[test]
    fn test_cross_adapter_bitcoin_to_cardano() {
        let btc = crate::bitcoin_htlc::BtcHtlcAdapter::new(crate::bitcoin_htlc::BitcoinNetwork::Mainnet);
        let cardano = crate::plutus_htlc::PlutusHtlcAdapter::new(
            "cardano-mainnet".into(),
            crate::plutus_htlc::PlutusNetwork::Mainnet,
        );
        let intent = sample_intent(12, "btc", "ada");
        let (lock, claim) = lock_and_claim(&btc, &cardano, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(btc.verify_lock(&lock).unwrap());
        assert!(cardano.verify_claim(&claim).unwrap());
        assert_eq!(btc.vm_type().family(), VmFamily::Utxo);
        assert_eq!(cardano.vm_type().family(), VmFamily::Utxo);
    }

    // ── Object → Object (Solana → MoveVM) ──────────────────────────────────

    #[test]
    fn test_cross_adapter_svm_to_movevm() {
        let svm = crate::svm_htlc::SvmAdapter::at_program_id([0x02u8; 32]);
        let move_vm = crate::move_vm_htlc::MoveVmAdapter::new("aptos-mainnet".into());
        let intent = sample_intent(13, "sol", "apt");
        let (lock, claim) = lock_and_claim(&svm, &move_vm, &intent).unwrap();
        assert!(lock.locked_amount > 0);
        assert!(svm.verify_lock(&lock).unwrap());
        assert!(move_vm.verify_claim(&claim).unwrap());
        assert_eq!(svm.vm_type().family(), VmFamily::Object);
        assert_eq!(move_vm.vm_type().family(), VmFamily::Object);
    }

    // ── All families can produce compatible LockProof/ClaimProof types ──────
    // Regression test: ensures no adapter returns a structurally different
    // proof that would break the relayer pipeline.

    #[test]
    fn test_all_adapters_produce_consistent_lock_proof_structure() {
        let adapters: Vec<Box<dyn X3VmAdapter>> = vec![
            Box::new(crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20])),
            Box::new(crate::svm_htlc::SvmAdapter::at_program_id([0x02u8; 32])),
            Box::new(crate::substrate_htlc::SubstrateHtlcAdapter::new("sub".into())),
            Box::new(crate::bitcoin_htlc::BtcHtlcAdapter::new(crate::bitcoin_htlc::BitcoinNetwork::Mainnet)),
            Box::new(crate::x3vm_htlc::X3VmAdapterImpl::new("x3".into())),
            Box::new(crate::move_vm_htlc::MoveVmAdapter::new("sui".into())),
            Box::new(crate::cosmwasm_htlc::CosmWasmAdapter::new("osmo".into())),
            Box::new(crate::cairo_vm_htlc::CairoVmAdapter::new("stark".into())),
            Box::new(crate::plutus_htlc::PlutusHtlcAdapter::new("cardano".into(), crate::plutus_htlc::PlutusNetwork::Mainnet)),
            Box::new(crate::fuel_htlc::FuelHtlcAdapter::new("fuel".into(), crate::fuel_htlc::FuelNetwork::Mainnet)),
            Box::new(crate::ton_htlc::TonHtlcAdapter::new("ton".into(), crate::ton_htlc::TonNetwork::Mainnet)),
            Box::new(crate::near_htlc::NearHtlcAdapter::new("near".into(), crate::near_htlc::NearNetwork::Mainnet)),
            Box::new(crate::soroban_htlc::SorobanHtlcAdapter::new("soroban".into(), crate::soroban_htlc::SorobanNetwork::Mainnet)),
            Box::new(crate::polkadot_ink_htlc::InkHtlcAdapter::new("ink".into(), crate::polkadot_ink_htlc::InkNetwork::PolkadotMainnet)),
            Box::new(crate::wasm_l1_htlc::WasmL1Adapter::new("icp".into(), crate::wasm_l1_htlc::WasmL1Runtime::InternetComputer)),
        ];

        // ZkVmAdapter intentionally returns SourceLockFailed (zkVMs don't support lock),
        // so we exclude it from this structural proof-consistency test.
        let skip_zk = |name: &str| name == "zk";
        let intent = sample_intent(99, "all", "all");
        for adapter in &adapters {
            if skip_zk(adapter.adapter_name()) {
                continue;
            }
            let lock = adapter.lock(&intent).unwrap();
            // Every adapter must produce a lock proof with these fields populated:
            assert!(!lock.tx_id.is_empty(), "{}: tx_id empty", adapter.adapter_name());
            assert!(!lock.chain_id.is_empty(), "{}: chain_id empty", adapter.adapter_name());
            assert_eq!(lock.vm_type, adapter.vm_type(), "{}: vm_type mismatch", adapter.adapter_name());
            assert!(lock.block_number > 0 || adapter.vm_type() == VmType::WasmL1, "{}: block_number zero", adapter.adapter_name());
            assert!(lock.locked_amount > 0, "{}: locked_amount zero", adapter.adapter_name());
            assert!(adapter.verify_lock(&lock).unwrap(), "{}: verify_lock failed", adapter.adapter_name());
        }
    }
}
