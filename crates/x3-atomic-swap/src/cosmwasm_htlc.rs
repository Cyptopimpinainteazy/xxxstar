//! # CosmWasm HTLC Adapter
//!
//! Adapter for CosmWasm chains (Cosmos Hub, Osmosis, Juno, Neutron).
//! Implements [`X3VmAdapter`] with mock/placeholder proof structures.
//!
//! In production, [`lock`] would execute a CosmWasm contract's Lock message,
//! [`claim`] would submit a Claim message with preimage, and [`refund`] would
//! trigger the refund path after timeout. Finality uses Tendermint BFT model
//! (1 block finality with 2/3 validator precommits).

use crate::adapter::{
    AdapterReadinessScore, AssetId, ChainHealth, ChainId, ClaimProof, FeeEstimate, FinalityProof,
    LockProof, RefundProof, TxId, VmType, X3VmAdapter,
};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, IntentId};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

// ─────────────────────────────────────────────────────────────────────────────
// CosmWasm Contract Types
// ─────────────────────────────────────────────────────────────────────────────

/// Execute messages for a CosmWasm HTLC contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExecuteMsg {
    /// Lock funds in the HTLC contract.
    Lock {
        sender: Vec<u8>,
        receiver: Vec<u8>,
        refund_addr: Vec<u8>,
        asset: AssetId,
        amount: u128,
        hashlock: [u8; 32],
        timeout: u64,
    },
    /// Claim locked funds by revealing preimage.
    Claim { preimage: [u8; 32] },
    /// Refund locked funds after timeout.
    Refund {},
}

/// Query messages for a CosmWasm HTLC contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum QueryMsg {
    /// Query the lock status for a given lock ID.
    LockStatus { lock_id: u64 },
}

/// Lock status response from the contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockStatusResponse {
    pub lock_id: u64,
    pub sender: Vec<u8>,
    pub receiver: Vec<u8>,
    pub refund_address: Vec<u8>,
    pub asset: AssetId,
    pub amount: u128,
    pub hashlock: [u8; 32],
    pub timeout: u64,
    pub claimed: bool,
    pub refunded: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// IBC Route Types
// ─────────────────────────────────────────────────────────────────────────────

/// Metadata for an IBC route between two CosmWasm chains.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IbcRouteMetadata {
    pub source_channel: String,
    pub destination_channel: String,
    pub source_port: String,
    pub destination_port: String,
    pub timeout_seconds: u64,
    pub routed_denom: String,
    pub intermediate_denom: String,
}

/// A fully-qualified HTLC route across an IBC channel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IbcHtlcRoute {
    pub metadata: IbcRouteMetadata,
    pub lock_address: String,
    pub claim_address: String,
    pub channel_open: bool,
    pub ibc_denom_trace: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// CosmWasmAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for CosmWasm chains (Cosmos Hub, Osmosis, Juno, Neutron).
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// Cosmos SDK node via RPC and interact with CosmWasm HTLC contracts.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulCosmWasmAdapter`].
#[derive(Debug, Clone)]
pub struct CosmWasmAdapter {
    /// Chain identifier (e.g. "cosmwasm-mainnet", "cosmos-hub").
    pub chain_id: ChainId,
    /// Optional HTTP RPC URL for Cosmos SDK node.
    pub rpc_url: Option<String>,
    /// Current finalized block (Tendermint finality).
    pub last_finalized_block: u64,
    /// Tracked claimed intent IDs.
    pub claimed_intents: Vec<u64>,
    /// Tracked refunded intent IDs.
    pub refunded_intents: Vec<u64>,
    /// Known IBC routes from this chain to others.
    pub ibc_routes: Vec<IbcRouteMetadata>,
    /// Whether IBC routing is enabled and active.
    pub ibc_supported: bool,
}

/// Internal lock state tracked by the stateful adapter.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct InternalLock {
    intent_id: IntentId,
    hashlock: [u8; 32],
    receiver: Vec<u8>,
    refund_address: Vec<u8>,
    timeout: u64,
    tx_id: TxId,
    block_number: u64,
    claimed: bool,
    refunded: bool,
}

impl CosmWasmAdapter {
    /// Create a new adapter for the given chain identifier.
    ///
    /// Example chain IDs: `"cosmwasm-mainnet"`, `"cosmos-hub"`, `"osmosis"`, `"juno"`, `"neutron"`.
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            chain_id,
            rpc_url: None,
            last_finalized_block: 0,
            claimed_intents: Vec::new(),
            refunded_intents: Vec::new(),
            ibc_routes: Vec::new(),
            ibc_supported: false,
        }
    }

    /// Set the RPC URL.
    pub fn set_rpc(&mut self, rpc_url: &str) {
        self.rpc_url = Some(rpc_url.to_string());
    }

    /// Generate a deterministic mock tx_id from intent_id and a label byte.
    fn mock_tx_id(intent_id: IntentId, label: u8) -> TxId {
        let mut hasher = Sha256::new();
        hasher.update(intent_id.to_le_bytes());
        hasher.update([label]);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Generate a mock contract address from chain_id.
    fn mock_contract_address(chain_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"x3-cosmwasm-htlc:");
        hasher.update(chain_id.as_bytes());
        let result = hasher.finalize();
        // Cosmos addresses are bech32; use hex representation for mock
        format!("cosmos1{}", hex::encode(&result[..20]))
    }

    // ── IBC Route Management ──────────────────────────────────────────────

    /// Register a new IBC route from this chain to a remote chain.
    pub fn add_ibc_route(&mut self, route: IbcRouteMetadata) {
        self.ibc_routes.push(route);
    }

    /// Remove an IBC route by its source channel identifier.
    pub fn remove_ibc_route(&mut self, source_channel: &str) {
        self.ibc_routes
            .retain(|r| r.source_channel != source_channel);
    }

    /// Look up an IBC route by source channel.
    pub fn get_ibc_route(&self, source_channel: &str) -> Option<&IbcRouteMetadata> {
        self.ibc_routes
            .iter()
            .find(|r| r.source_channel == source_channel)
    }

    /// Check whether any registered IBC route can reach the given target chain
    /// (checks destination_channel against the target chain name as a heuristic).
    pub fn has_ibc_route_to(&self, target_chain: &str) -> bool {
        self.ibc_routes.iter().any(|r| {
            r.destination_channel.contains(target_chain)
                || r.intermediate_denom.contains(target_chain)
        })
    }

    /// Enable IBC routing support.
    pub fn enable_ibc(&mut self) {
        self.ibc_supported = true;
    }

    /// Disable IBC routing support.
    pub fn disable_ibc(&mut self) {
        self.ibc_supported = false;
    }

    /// Return the number of registered IBC routes.
    pub fn ibc_route_count(&self) -> usize {
        self.ibc_routes.len()
    }

    /// Check whether a swap between source_chain and dest_chain for a given
    /// asset can be routed via an IBC channel.
    pub fn can_route_via_ibc(&self, source_chain: &str, dest_chain: &str, asset: &str) -> bool {
        if !self.ibc_supported || self.ibc_routes.is_empty() {
            return false;
        }
        self.ibc_routes.iter().any(|r| {
            r.source_channel.contains(source_chain)
                && (r.destination_channel.contains(dest_chain)
                    || r.intermediate_denom.contains(dest_chain))
                && (r.routed_denom == asset || r.intermediate_denom == asset)
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for CosmWasmAdapter {
    fn vm_type(&self) -> VmType {
        VmType::CosmWasm
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-cosmwasm"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        let mut chains: Vec<ChainId> = vec![
            "cosmwasm-mainnet".into(),
            "cosmos-hub".into(),
            "osmosis".into(),
            "juno".into(),
            "neutron".into(),
        ];
        // When IBC is enabled, add intermediate chains reachable via IBC routes.
        if self.ibc_supported {
            for route in &self.ibc_routes {
                // Derive chain name from intermediate_denom or destination_channel
                let ibc_chain = if !route.intermediate_denom.is_empty()
                    && route.intermediate_denom != route.routed_denom
                {
                    route.intermediate_denom.clone()
                } else {
                    route.destination_channel.clone()
                };
                if !chains.contains(&ibc_chain) {
                    chains.push(ibc_chain);
                }
            }
        }
        chains
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        let mut assets: Vec<AssetId> =
            vec!["ATOM".into(), "OSMO".into(), "JUNO".into(), "USDC".into()];
        // When IBC is enabled, add assets from IBC route metadata.
        if self.ibc_supported {
            for route in &self.ibc_routes {
                if !assets.contains(&route.routed_denom) {
                    assets.push(route.routed_denom.clone());
                }
                if !route.intermediate_denom.is_empty()
                    && !assets.contains(&route.intermediate_denom)
                {
                    assets.push(route.intermediate_denom.clone());
                }
            }
        }
        assets
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = Self::mock_contract_address(&chain_id);
        let block_number = self.last_finalized_block + 1;

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::CosmWasm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x63, 0x77, 0x61, 0x01], // "cwa\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.last_finalized_block + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::CosmWasm,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x63, 0x77, 0x61, 0x02], // "cwa\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.last_finalized_block + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::CosmWasm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x63, 0x77, 0x61, 0x03], // "cwa\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::CosmWasm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        if proof.lock_address.is_empty() {
            return Ok(false);
        }
        if proof.locked_amount == 0 {
            return Ok(false);
        }
        if proof.timeout == 0 {
            return Ok(false);
        }
        Ok(true)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::CosmWasm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        if proof.preimage == [0u8; 32] {
            return Ok(false);
        }
        Ok(true)
    }

    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::CosmWasm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // ~0.0025 ATOM equivalent (250_000 uatom) for a CosmWasm execute
        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::CosmWasm,
            native_fee: 250_000, // 0.0025 ATOM in uatom
            gas_units: 0,
            gas_price: 0,
            estimated_usd: 0.02,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // Tendermint finality: 2/3 validator precommits, 1 block = finalized
        let finalized = self.last_finalized_block >= 1;

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::CosmWasm,
            tx_id: tx_id.clone(),
            block_number: self.last_finalized_block,
            block_hash: hex::encode(Sha256::digest(self.last_finalized_block.to_le_bytes())),
            confirmations: if finalized { 1 } else { 0 },
            finalized,
            finality_source: "tendermint".into(),
            safe_to_reveal_secret: finalized,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::CosmWasm,
            latest_block: self.last_finalized_block,
            finalized_block: self.last_finalized_block,
            block_delay_ms: 6_000,    // ~6s block time (Cosmos SDK)
            finality_delay_ms: 6_000, // Tendermint finality in ~6s
            rpc_quorum_healthy: true,
            gas_price: 0,
            halted: false,
            degraded: false,
            safe_for_new_intents: true,
        })
    }

    // ── Readiness ─────────────────────────────────────────────────────────

    fn readiness_score(&self) -> AdapterReadinessScore {
        AdapterReadinessScore {
            adapter_name: "x3-adapter-cosmwasm",
            vm_type: VmType::CosmWasm,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // needs actual CosmWasm event listening
            finality_proof: true,
            rpc_indexer_support: false, // needs real RPC/indexer integration
            timeout_safety: true,
            tests_implemented: true,
            proof_ledger_integration: true,
            ibc_support: self.ibc_supported,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stateful Wrapper with Double-Claim Protection
// ─────────────────────────────────────────────────────────────────────────────

/// A stateful wrapper around [`CosmWasmAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
#[derive(Debug, Clone)]
pub struct StatefulCosmWasmAdapter {
    pub inner: CosmWasmAdapter,
    locks: Vec<InternalLock>,
}

impl StatefulCosmWasmAdapter {
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            inner: CosmWasmAdapter::new(chain_id),
            locks: Vec::new(),
        }
    }

    pub fn set_rpc(&mut self, rpc_url: &str) {
        self.inner.set_rpc(rpc_url);
    }

    /// Lock funds and record the lock state internally.
    pub fn lock(&mut self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        if self.locks.iter().any(|l| l.intent_id == intent.intent_id) {
            return Err(SwapError::AlreadyLocked {
                chain: intent.source_chain,
            });
        }

        let proof = self.inner.lock(intent)?;

        self.locks.push(InternalLock {
            intent_id: intent.intent_id,
            hashlock: intent.hashlock,
            receiver: intent.receiver.as_bytes().to_vec(),
            refund_address: intent.refund_path.address.as_bytes().to_vec(),
            timeout: intent.source_timeout,
            tx_id: proof.tx_id.clone(),
            block_number: proof.block_number,
            claimed: false,
            refunded: false,
        });

        Ok(proof)
    }

    /// Claim with preimage, enforcing no double-claim.
    pub fn claim(
        &mut self,
        intent_id: IntentId,
        preimage: [u8; 32],
    ) -> Result<ClaimProof, SwapError> {
        let lock = self
            .locks
            .iter_mut()
            .find(|l| l.intent_id == intent_id)
            .ok_or_else(|| SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "no lock record found for this intent".into(),
            })?;

        if lock.claimed {
            return Err(SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already claimed".into(),
            });
        }

        if lock.refunded {
            return Err(SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already refunded".into(),
            });
        }

        // Verify preimage matches hashlock.
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut computed = [0u8; 32];
        computed.copy_from_slice(&result);
        if computed != lock.hashlock {
            return Err(SwapError::ClaimFailed {
                chain: self.inner.chain_id.clone(),
                reason: "hashlock mismatch: preimage does not match hashlock".into(),
            });
        }

        let proof = self.inner.claim(intent_id, preimage)?;
        lock.claimed = true;
        Ok(proof)
    }

    /// Refund after timeout, enforcing no double-refund.
    pub fn refund(
        &mut self,
        intent_id: IntentId,
        current_time: u64,
    ) -> Result<RefundProof, SwapError> {
        let lock = self
            .locks
            .iter_mut()
            .find(|l| l.intent_id == intent_id)
            .ok_or_else(|| SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "no lock record found for this intent".into(),
            })?;

        if lock.claimed {
            return Err(SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already claimed".into(),
            });
        }

        if lock.refunded {
            return Err(SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "already refunded".into(),
            });
        }

        if current_time < lock.timeout {
            return Err(SwapError::RefundFailed {
                chain: self.inner.chain_id.clone(),
                reason: "timeout has not yet elapsed".into(),
            });
        }

        let proof = self.inner.refund(intent_id)?;
        lock.refunded = true;
        Ok(proof)
    }

    /// Check if a given intent has been claimed.
    pub fn is_claimed(&self, intent_id: IntentId) -> bool {
        self.locks
            .iter()
            .find(|l| l.intent_id == intent_id)
            .map(|l| l.claimed)
            .unwrap_or(false)
    }

    /// Check if a given intent has been refunded.
    pub fn is_refunded(&self, intent_id: IntentId) -> bool {
        self.locks
            .iter()
            .find(|l| l.intent_id == intent_id)
            .map(|l| l.refunded)
            .unwrap_or(false)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::X3VmAdapter;
    use crate::intent::{
        AtomicIntent, AtomicSwapStatus, ChainKind, FinalityLevel, FinalityRequirement, RefundPath,
        RouteMode,
    };

    /// Helper: create a simple test intent.
    fn make_test_intent(intent_id: IntentId, hashlock: [u8; 32]) -> AtomicIntent {
        AtomicIntent {
            intent_id,
            source_chain: ChainKind::X3,
            destination_chain: ChainKind::Ethereum,
            source_asset: "ATOM".into(),
            destination_asset: "USDC".into(),
            amount_in: 1_000_000_000_000,
            min_amount_out: 500_000_000,
            receiver: "cosmos1abc123def456".into(),
            hashlock,
            source_timeout: 1_800_000,
            destination_timeout: 1_700_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "cosmos1refundaddr789".into(),
                asset: None,
            },
            route_mode: RouteMode::DirectHtlc,
            max_slippage_bps: 100,
            relayer_quorum_requirement: 3,
            status: AtomicSwapStatus::Pending,
            intent_hash: [0u8; 32],
        }
    }

    /// Helper: compute hashlock from preimage.
    fn make_hashlock(preimage: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    // ── CosmWasm Contract Type Tests ──────────────────────────────────────

    #[test]
    fn test_execute_msg_lock() {
        let msg = ExecuteMsg::Lock {
            sender: vec![0x01],
            receiver: vec![0x02],
            refund_addr: vec![0x03],
            asset: "ATOM".into(),
            amount: 1000,
            hashlock: [0u8; 32],
            timeout: 100,
        };
        match msg {
            ExecuteMsg::Lock { asset, amount, .. } => {
                assert_eq!(asset, "ATOM");
                assert_eq!(amount, 1000);
            }
            _ => panic!("expected Lock variant"),
        }
    }

    #[test]
    fn test_execute_msg_claim() {
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..3].copy_from_slice(b"abc");
            p
        };
        let msg = ExecuteMsg::Claim { preimage };
        match msg {
            ExecuteMsg::Claim { preimage: p } => {
                assert_eq!(p[..3], [0x61, 0x62, 0x63]);
            }
            _ => panic!("expected Claim variant"),
        }
    }

    #[test]
    fn test_execute_msg_refund() {
        let msg = ExecuteMsg::Refund {};
        match msg {
            ExecuteMsg::Refund {} => {}
            _ => panic!("expected Refund variant"),
        }
    }

    #[test]
    fn test_query_msg_lock_status() {
        let msg = QueryMsg::LockStatus { lock_id: 42 };
        match msg {
            QueryMsg::LockStatus { lock_id } => {
                assert_eq!(lock_id, 42);
            }
        }
    }

    #[test]
    fn test_lock_status_response() {
        let resp = LockStatusResponse {
            lock_id: 1,
            sender: vec![0x01],
            receiver: vec![0x02],
            refund_address: vec![0x03],
            asset: "ATOM".into(),
            amount: 1000,
            hashlock: [0u8; 32],
            timeout: 100,
            claimed: false,
            refunded: false,
        };
        assert_eq!(resp.lock_id, 1);
        assert!(!resp.claimed);
    }

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());

        assert_eq!(adapter.vm_type(), VmType::CosmWasm);
        assert_eq!(adapter.adapter_name(), "x3-adapter-cosmwasm");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"cosmwasm-mainnet".into()));
        assert!(chains.contains(&"cosmos-hub".into()));
        assert!(chains.contains(&"osmosis".into()));
        assert!(chains.contains(&"juno".into()));
        assert!(chains.contains(&"neutron".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"ATOM".into()));
        assert!(assets.contains(&"OSMO".into()));
        assert!(assets.contains(&"JUNO".into()));
        assert!(assets.contains(&"USDC".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        let adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-cosmwasm");
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        let hashlock = make_hashlock(b"test_preimage");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::CosmWasm);
        assert_eq!(proof.hashlock, hashlock);
        assert_eq!(proof.locked_amount, intent.amount_in);
        assert!(!proof.tx_id.is_empty());
        assert!(!proof.lock_address.is_empty());
        assert_eq!(proof.receiver, intent.receiver.as_bytes());
        assert_eq!(proof.refund_address, intent.refund_path.address.as_bytes());
        assert_ne!(proof.block_number, 0);
    }

    #[test]
    fn test_lock_differs_per_intent() {
        let adapter = CosmWasmAdapter::new("osmosis".into());
        let h1 = make_hashlock(b"secret1");
        let h2 = make_hashlock(b"secret2");

        let proof1 = adapter.lock(&make_test_intent(1, h1)).expect("lock 1");
        let proof2 = adapter.lock(&make_test_intent(2, h2)).expect("lock 2");

        assert_ne!(proof1.tx_id, proof2.tx_id);
        assert!(proof1.lock_address.contains("cosmos1"));
    }

    // ── Claim Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_claim_with_preimage() {
        let adapter = CosmWasmAdapter::new("juno".into());
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::CosmWasm);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::CosmWasm);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        let hashlock = make_hashlock(b"valid_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock_wrong_vm() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "cosmwasm-mainnet".into(),
            vm_type: VmType::Evm, // wrong!
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "cosmos1addr".into(),
            locked_amount: 100,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 1000,
            raw_proof: vec![],
        };
        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "wrong VM type should fail verification");
    }

    #[test]
    fn test_verify_invalid_lock_empty_tx() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        let bad_proof = LockProof {
            tx_id: String::new(),
            chain_id: "cosmwasm-mainnet".into(),
            vm_type: VmType::CosmWasm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "addr".into(),
            locked_amount: 100,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 1000,
            raw_proof: vec![],
        };
        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "empty tx_id should fail verification");
    }

    #[test]
    fn test_verify_invalid_lock_zero_amount() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "cosmwasm-mainnet".into(),
            vm_type: VmType::CosmWasm,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "addr".into(),
            locked_amount: 0,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 1000,
            raw_proof: vec![],
        };
        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "zero amount should fail verification");
    }

    #[test]
    fn test_verify_valid_claim() {
        let adapter = CosmWasmAdapter::new("neutron".into());
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..4].copy_from_slice(b"test");
            p
        };

        let proof = adapter.claim(10, preimage).expect("claim");
        let valid = adapter.verify_claim(&proof).expect("verify");

        assert!(valid, "well-formed claim proof should verify");
    }

    #[test]
    fn test_verify_invalid_claim() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        let bad_proof = ClaimProof {
            tx_id: "".into(),
            intent_id: 0,
            chain_id: "".into(),
            vm_type: VmType::Evm,
            preimage: [0u8; 32],
            block_number: 0,
            block_hash: "".into(),
            raw_proof: vec![],
        };
        let valid = adapter.verify_claim(&bad_proof).expect("verify");
        assert!(!valid, "malformed claim proof should fail");
    }

    #[test]
    fn test_verify_valid_refund() {
        let adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let proof = adapter.refund(10).expect("refund");
        let valid = adapter.verify_refund(&proof).expect("verify");
        assert!(valid, "well-formed refund proof should verify");
    }

    #[test]
    fn test_verify_invalid_refund() {
        let adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        let bad_proof = RefundProof {
            tx_id: "".into(),
            intent_id: 0,
            chain_id: "".into(),
            vm_type: VmType::Evm,
            block_number: 0,
            block_hash: "".into(),
            raw_proof: vec![],
        };
        let valid = adapter.verify_refund(&bad_proof).expect("verify");
        assert!(!valid, "malformed refund proof should fail");
    }

    // ── Finality & Health Tests ───────────────────────────────────────────

    #[test]
    fn test_finality_status() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        adapter.last_finalized_block = 42;

        let fp = adapter
            .finality_status(&"mock_tx_id".into())
            .expect("finality status");

        assert_eq!(fp.chain_id, "cosmos-hub");
        assert_eq!(fp.vm_type, VmType::CosmWasm);
        assert!(fp.finalized);
        assert!(fp.safe_to_reveal_secret);
        assert_eq!(fp.finality_source, "tendermint");
    }

    #[test]
    fn test_finality_unfinalized() {
        let mut adapter = CosmWasmAdapter::new("cosmwasm-mainnet".into());
        adapter.last_finalized_block = 0;

        let fp = adapter
            .finality_status(&"new_tx".into())
            .expect("finality status");

        assert!(!fp.finalized, "block 0 should be unfinalized");
    }

    #[test]
    fn test_chain_health() {
        let adapter = CosmWasmAdapter::new("osmosis".into());

        let health = adapter.chain_health().expect("chain health");

        assert_eq!(health.chain_id, "osmosis");
        assert_eq!(health.vm_type, VmType::CosmWasm);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert!(health.safe_for_new_intents);
        assert!(health.rpc_quorum_healthy);
        assert_eq!(health.block_delay_ms, 6_000);
    }

    // ── Fee Estimation Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee() {
        let adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let hashlock = make_hashlock(b"fee_test");
        let intent = make_test_intent(99, hashlock);

        let fee = adapter.estimate_fee(&intent).expect("estimate_fee");

        assert_eq!(fee.chain_id, "cosmos-hub");
        assert_eq!(fee.vm_type, VmType::CosmWasm);
        assert!(fee.native_fee > 0);
        assert!(fee.estimated_usd > 0.0);
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = CosmWasmAdapter::new("neutron".into());

        let score = adapter.readiness_score();

        assert!(score.interface_implemented);
        assert!(score.lock_path);
        assert!(score.claim_path);
        assert!(score.refund_path);
        assert!(!score.event_proof_extraction);
        assert!(score.finality_proof);
        assert!(!score.rpc_indexer_support);
        assert!(score.timeout_safety);
        assert!(score.tests_implemented);
        assert!(score.proof_ledger_integration);

        assert_eq!(score.score(), 80);
        assert_eq!(score.adapter_name, "x3-adapter-cosmwasm");
        assert_eq!(score.vm_type, VmType::CosmWasm);

        let missing = score.missing_items();
        assert!(missing.contains(&"event_proof_extraction"));
        assert!(missing.contains(&"rpc_indexer_support"));
        assert!(missing.contains(&"ibc_support"));
        assert_eq!(missing.len(), 3);
    }

    // ── Stateful Adapter Tests ────────────────────────────────────────────

    #[test]
    fn test_stateful_lock() {
        let mut adapter = StatefulCosmWasmAdapter::new("cosmos-hub".into());
        let hashlock = make_hashlock(b"stateful_lock");
        let intent = make_test_intent(500, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");
        assert!(!proof.tx_id.is_empty());
        assert_eq!(adapter.locks.len(), 1);
    }

    #[test]
    fn test_double_lock_rejected() {
        let mut adapter = StatefulCosmWasmAdapter::new("cosmos-hub".into());
        let hashlock = make_hashlock(b"double_lock");
        let intent = make_test_intent(301, hashlock);

        adapter.lock(&intent).expect("first lock");
        let second = adapter.lock(&intent);
        assert!(second.is_err(), "double lock should be rejected");
    }

    #[test]
    fn test_double_claim_rejected() {
        let mut adapter = StatefulCosmWasmAdapter::new("osmosis".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..8].copy_from_slice(b"double_c");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(300, hashlock);

        adapter.lock(&intent).expect("lock should succeed");

        let claim1 = adapter.claim(300, preimage);
        assert!(claim1.is_ok(), "first claim should succeed");

        let claim2 = adapter.claim(300, preimage);
        assert!(claim2.is_err(), "double claim should be rejected");

        match claim2 {
            Err(SwapError::ClaimFailed { reason, .. }) => {
                assert!(reason.contains("already claimed"));
            }
            _ => panic!("expected ClaimFailed error"),
        }
    }

    #[test]
    fn test_double_refund_rejected() {
        let mut adapter = StatefulCosmWasmAdapter::new("juno".into());

        let hashlock = make_hashlock(b"refund_test");
        let intent = make_test_intent(400, hashlock);

        adapter.lock(&intent).expect("lock");

        let after_timeout = intent.source_timeout + 1;
        let refund1 = adapter.refund(400, after_timeout);
        assert!(refund1.is_ok(), "first refund should succeed");

        let refund2 = adapter.refund(400, after_timeout);
        assert!(refund2.is_err(), "double refund should be rejected");

        match refund2 {
            Err(SwapError::RefundFailed { reason, .. }) => {
                assert!(reason.contains("already refunded"));
            }
            _ => panic!("expected RefundFailed error"),
        }
    }

    #[test]
    fn test_claim_before_timeout_succeeds() {
        let mut adapter = StatefulCosmWasmAdapter::new("cosmos-hub".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..4].copy_from_slice(b"befo");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(501, hashlock);

        adapter.lock(&intent).expect("lock");

        let claim = adapter.claim(501, preimage);
        assert!(claim.is_ok(), "claim before timeout should succeed");
    }

    #[test]
    fn test_refund_before_timeout_rejected() {
        let mut adapter = StatefulCosmWasmAdapter::new("cosmos-hub".into());

        let hashlock = make_hashlock(b"early_refund");
        let intent = make_test_intent(600, hashlock);

        adapter.lock(&intent).expect("lock");

        let before_timeout = intent.source_timeout - 1;
        let refund = adapter.refund(600, before_timeout);

        match refund {
            Err(SwapError::RefundFailed { reason, .. }) => {
                assert!(reason.contains("timeout has not yet elapsed"));
            }
            other => panic!("expected timeout error, got: {:?}", other),
        }
    }

    #[test]
    fn test_claim_after_refund_rejected() {
        let mut adapter = StatefulCosmWasmAdapter::new("cosmos-hub".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..7].copy_from_slice(b"after_r");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(700, hashlock);

        adapter.lock(&intent).expect("lock");

        let after_timeout = intent.source_timeout + 1;
        adapter.refund(700, after_timeout).expect("refund");

        let claim = adapter.claim(700, preimage);
        match claim {
            Err(SwapError::ClaimFailed { reason, .. }) => {
                assert!(reason.contains("already refunded"));
            }
            other => panic!(
                "expected ClaimFailed for already refunded, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_is_claimed_and_is_refunded() {
        let mut adapter = StatefulCosmWasmAdapter::new("neutron".into());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..3].copy_from_slice(b"st1");
            p
        };
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(800, hashlock);

        adapter.lock(&intent).expect("lock");
        assert!(!adapter.is_claimed(800));
        assert!(!adapter.is_refunded(800));

        adapter.claim(800, preimage).expect("claim");
        assert!(adapter.is_claimed(800));
        assert!(!adapter.is_refunded(800));
    }

    #[test]
    fn test_is_refunded_state() {
        let mut adapter = StatefulCosmWasmAdapter::new("cosmos-hub".into());

        let hashlock = make_hashlock(b"state_refund");
        let intent = make_test_intent(900, hashlock);

        adapter.lock(&intent).expect("lock");
        let after_timeout = intent.source_timeout + 1;
        adapter.refund(900, after_timeout).expect("refund");

        assert!(!adapter.is_claimed(900));
        assert!(adapter.is_refunded(900));
    }

    #[test]
    fn test_set_rpc() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        assert!(adapter.rpc_url.is_none());

        adapter.set_rpc("https://custom.cosmos.node.example.com");
        assert_eq!(
            adapter.rpc_url.unwrap(),
            "https://custom.cosmos.node.example.com"
        );
    }

    // ── IBC Route Tests ───────────────────────────────────────────────────

    fn make_ibc_route() -> IbcRouteMetadata {
        IbcRouteMetadata {
            source_channel: "channel-0".into(),
            destination_channel: "channel-1".into(),
            source_port: "transfer".into(),
            destination_port: "transfer".into(),
            timeout_seconds: 600,
            routed_denom: "USDC".into(),
            intermediate_denom: "uosmo".into(),
        }
    }

    #[test]
    fn test_ibc_route_add_remove() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        assert_eq!(adapter.ibc_route_count(), 0);

        adapter.add_ibc_route(make_ibc_route());
        assert_eq!(adapter.ibc_route_count(), 1);

        adapter.remove_ibc_route("channel-0");
        assert_eq!(adapter.ibc_route_count(), 0);
    }

    #[test]
    fn test_ibc_route_lookup() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        adapter.add_ibc_route(make_ibc_route());

        let route = adapter.get_ibc_route("channel-0");
        assert!(route.is_some());
        assert_eq!(route.unwrap().destination_channel, "channel-1");
        assert_eq!(route.unwrap().routed_denom, "USDC");
    }

    #[test]
    fn test_ibc_route_lookup_not_found() {
        let adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let route = adapter.get_ibc_route("nonexistent-channel");
        assert!(route.is_none());
    }

    #[test]
    fn test_ibc_routing_enable_disable() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        assert!(!adapter.ibc_supported);

        adapter.enable_ibc();
        assert!(adapter.ibc_supported);

        adapter.disable_ibc();
        assert!(!adapter.ibc_supported);
    }

    #[test]
    fn test_ibc_expands_supported_chains() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        // Without IBC, only base chains are reported.
        let chains_before = adapter.supported_chains();
        assert!(!chains_before.contains(&"uosmo".into()));

        // Add IBC route and enable IBC.
        let mut route = make_ibc_route();
        route.intermediate_denom = "osmosis".into();
        adapter.add_ibc_route(route);
        adapter.enable_ibc();

        let chains_after = adapter.supported_chains();
        assert!(chains_after.contains(&"osmosis".into()));
    }

    #[test]
    fn test_ibc_expands_supported_assets() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let assets_before = adapter.supported_assets();
        // AXL is not in the default asset list.
        assert!(!assets_before.contains(&"AXL".to_string()));

        let mut route = make_ibc_route();
        route.routed_denom = "axlUSDC".into();
        route.intermediate_denom = "ibc/ABC".into();
        adapter.add_ibc_route(route);
        adapter.enable_ibc();

        let assets_after = adapter.supported_assets();
        assert!(assets_after.contains(&"axlUSDC".into()));
        assert!(assets_after.contains(&"ibc/ABC".into()));
    }

    #[test]
    fn test_ibc_route_count() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        assert_eq!(adapter.ibc_route_count(), 0);

        adapter.add_ibc_route(make_ibc_route());
        assert_eq!(adapter.ibc_route_count(), 1);

        let r2 = IbcRouteMetadata {
            source_channel: "channel-2".into(),
            destination_channel: "channel-3".into(),
            source_port: "transfer".into(),
            destination_port: "transfer".into(),
            timeout_seconds: 300,
            routed_denom: "JUNO".into(),
            intermediate_denom: "ujuno".into(),
        };
        adapter.add_ibc_route(r2);
        assert_eq!(adapter.ibc_route_count(), 2);
    }

    #[test]
    fn test_has_ibc_route_to() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let mut route = make_ibc_route();
        route.destination_channel = "osmosis-channel-1".into();
        route.intermediate_denom = "osmosis".into();
        adapter.add_ibc_route(route);

        assert!(adapter.has_ibc_route_to("osmosis"));
        assert!(!adapter.has_ibc_route_to("juno"));
    }

    #[test]
    fn test_can_route_via_ibc() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let mut route = make_ibc_route();
        route.source_channel = "cosmos-hub-channel-0".into();
        route.destination_channel = "osmosis-channel-1".into();
        route.routed_denom = "ATOM".into();
        adapter.add_ibc_route(route);
        adapter.enable_ibc();

        assert!(adapter.can_route_via_ibc("cosmos-hub", "osmosis", "ATOM"));
    }

    #[test]
    fn test_cannot_route_without_ibc() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let route = make_ibc_route();
        adapter.add_ibc_route(route);
        // ibc_supported is still false
        assert!(!adapter.can_route_via_ibc("cosmos-hub", "osmosis", "USDC"));
    }

    #[test]
    fn test_multiple_ibc_routes() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());

        let r1 = IbcRouteMetadata {
            source_channel: "channel-0".into(),
            destination_channel: "channel-1".into(),
            source_port: "transfer".into(),
            destination_port: "transfer".into(),
            timeout_seconds: 600,
            routed_denom: "ATOM".into(),
            intermediate_denom: "uosmo".into(),
        };
        let r2 = IbcRouteMetadata {
            source_channel: "channel-2".into(),
            destination_channel: "channel-3".into(),
            source_port: "wasm".into(),
            destination_port: "wasm".into(),
            timeout_seconds: 1200,
            routed_denom: "JUNO".into(),
            intermediate_denom: "ujuno".into(),
        };

        adapter.add_ibc_route(r1);
        adapter.add_ibc_route(r2);
        assert_eq!(adapter.ibc_route_count(), 2);

        let r0 = adapter.get_ibc_route("channel-0");
        assert!(r0.is_some());
        assert_eq!(r0.unwrap().timeout_seconds, 600);

        let r2 = adapter.get_ibc_route("channel-2");
        assert!(r2.is_some());
        assert_eq!(r2.unwrap().timeout_seconds, 1200);
    }

    #[test]
    fn test_ibc_route_with_identity_fields() {
        let route = IbcRouteMetadata {
            source_channel: "channel-10".into(),
            destination_channel: "channel-20".into(),
            source_port: "transfer".into(),
            destination_port: "ibc-hooks".into(),
            timeout_seconds: 900,
            routed_denom: "ATOM".into(),
            intermediate_denom: "uatom".into(),
        };

        assert_eq!(route.source_channel, "channel-10");
        assert_eq!(route.destination_channel, "channel-20");
        assert_eq!(route.source_port, "transfer");
        assert_eq!(route.destination_port, "ibc-hooks");
        assert_eq!(route.timeout_seconds, 900);
        assert_eq!(route.routed_denom, "ATOM");
        assert_eq!(route.intermediate_denom, "uatom");
    }

    #[test]
    fn test_ibc_route_with_actual_adapter() {
        // IBC-aware adapter still does lock/claim/refund correctly.
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        adapter.add_ibc_route(make_ibc_route());
        adapter.enable_ibc();

        let hashlock = make_hashlock(b"ibc_adapter_test");
        let intent = make_test_intent(999, hashlock);

        let lock = adapter.lock(&intent).expect("lock should work with IBC");
        assert!(!lock.tx_id.is_empty());
        assert_eq!(lock.vm_type, VmType::CosmWasm);

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..4].copy_from_slice(b"ibc_");
            p
        };
        let claim = adapter
            .claim(999, preimage)
            .expect("claim should work with IBC");
        assert_eq!(claim.intent_id, 999);

        let refund = adapter.refund(888).expect("refund should work with IBC");
        assert_eq!(refund.intent_id, 888);
    }

    #[test]
    fn test_ibc_metadata_serialization() {
        let route = IbcRouteMetadata {
            source_channel: "channel-0".into(),
            destination_channel: "channel-1".into(),
            source_port: "transfer".into(),
            destination_port: "transfer".into(),
            timeout_seconds: 600,
            routed_denom: "USDC".into(),
            intermediate_denom: "ibc/ABCDEF".into(),
        };

        let json = serde_json::to_string(&route).expect("serialize");
        let deserialized: IbcRouteMetadata = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.source_channel, "channel-0");
        assert_eq!(deserialized.destination_channel, "channel-1");
        assert_eq!(deserialized.routed_denom, "USDC");
        assert_eq!(deserialized.intermediate_denom, "ibc/ABCDEF");
        assert_eq!(deserialized.timeout_seconds, 600);
    }

    #[test]
    fn test_ibc_does_not_break_existing() {
        // Existing lock/claim/refund/verify/fee/health tests should still pass
        // even when IBC routes are registered.
        let mut adapter = CosmWasmAdapter::new("juno".into());
        adapter.add_ibc_route(make_ibc_route());
        // IBC not enabled - should not affect anything.

        let hashlock = make_hashlock(b"existing_test");
        let intent = make_test_intent(42, hashlock);

        let lock = adapter.lock(&intent).expect("lock");
        assert!(adapter.verify_lock(&lock).unwrap());

        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..4].copy_from_slice(b"test");
            p
        };
        let claim = adapter.claim(42, preimage).expect("claim");
        assert!(adapter.verify_claim(&claim).unwrap());

        let refund = adapter.refund(43).expect("refund");
        assert!(adapter.verify_refund(&refund).unwrap());

        let fee = adapter.estimate_fee(&intent).expect("fee");
        assert!(fee.native_fee > 0);

        let health = adapter.chain_health().expect("health");
        assert!(health.safe_for_new_intents);
    }

    #[test]
    fn test_ibc_readiness_score_with_ibc() {
        let mut adapter = CosmWasmAdapter::new("cosmos-hub".into());
        adapter.add_ibc_route(make_ibc_route());
        adapter.enable_ibc();

        let score = adapter.readiness_score();
        assert!(score.ibc_support);
        assert_eq!(score.score(), 90);
    }

    #[test]
    fn test_ibc_readiness_score_without_ibc() {
        let adapter = CosmWasmAdapter::new("cosmos-hub".into());
        let score = adapter.readiness_score();
        assert!(!score.ibc_support);
        assert_eq!(score.score(), 80);
    }
}
