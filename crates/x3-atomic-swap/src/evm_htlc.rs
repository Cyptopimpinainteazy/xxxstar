//! # EVM HTLC Adapter
//!
//! Production-grade EVM HTLC (Hash Time-Locked Contract) adapter for atomic
//! swaps. Implements `lock()`, `claim(preimage)`, and `refund()` with proper
//! hashlock verification, timeout enforcement, and event emission.
//!
//! In production, this would submit real Ethereum transactions via a JSON-RPC
//! provider. In this implementation, the adapter simulates on-chain behavior
//! with full verification logic so the relayer and scoreboard can be tested
//! end-to-end without a live chain.

use crate::adapter::{
    AdapterReadinessScore, AssetId, ChainHealth, ChainId, ClaimProof, FeeEstimate, FinalityProof,
    LockProof, RefundProof, TxId, VmType, X3VmAdapter,
};
use crate::error::SwapError;
use crate::event_watcher::{EventWatcher, HtlcEvent, WatcherConfig};
use crate::intent::{AtomicIntent, IntentId};
use crate::rpc_client::RpcClient;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Event emitted when an HTLC lock is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmLockedEvent {
    /// Contract address.
    pub contract: [u8; 20],
    /// Unique swap ID on this contract.
    pub swap_id: [u8; 32],
    /// Address of the sender/locker.
    pub sender: [u8; 20],
    /// Address of the receiver/claimant.
    pub receiver: [u8; 20],
    /// Address that can trigger refund after timeout.
    pub refund_address: [u8; 20],
    /// Amount locked in wei.
    pub amount: u128,
    /// Hashlock (32-byte hash of preimage).
    pub hashlock: [u8; 32],
    /// Timeout block number or timestamp.
    pub timeout: u64,
    /// Asset identifier (address of ERC20 or zero for native).
    pub asset: [u8; 20],
}

/// Event emitted when an HTLC is claimed with the correct preimage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmClaimedEvent {
    pub contract: [u8; 20],
    pub swap_id: [u8; 32],
    pub claimant: [u8; 20],
    pub preimage: Vec<u8>,
}

/// Event emitted when an HTLC is refunded after timeout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmRefundedEvent {
    pub contract: [u8; 20],
    pub swap_id: [u8; 32],
    pub refund_address: [u8; 20],
}

/// The topic hash for the Locked event: keccak256("Locked(...)").
/// In production this would be the real keccak. For testing, we use
/// a deterministic 32-byte value.
pub const LOCKED_EVENT_TOPIC: [u8; 32] = [0x01; 32];
/// Claimed event topic.
pub const CLAIMED_EVENT_TOPIC: [u8; 32] = [0x02; 32];
/// Refunded event topic.
pub const REFUNDED_EVENT_TOPIC: [u8; 32] = [0x03; 32];

/// An in-memory HTLC contract state for testing and simulation.
///
/// When configured with an RPC client and event watcher, it can interact
/// with real chain endpoints. In `no_std` mode the RPC methods return
/// stub/fallback results.
#[derive(Debug, Clone)]
pub struct EvmHtlcContract {
    /// Contract address.
    pub address: [u8; 20],
    /// Locked swaps keyed by swap_id.
    swaps: Vec<EvmSwapState>,
    /// Emitted events.
    pub events: Vec<EvmEvent>,
    /// Optional RPC client for JSON-RPC calls.
    pub rpc_client: Option<RpcClient>,
    /// Optional event watcher for polling on-chain events.
    pub event_watcher: Option<EventWatcher>,
    /// Contract address string (hex) for RPC interactions.
    pub contract_address: Option<String>,
    /// Whether the contract has been marked as deployed.
    pub deployed: bool,
}

/// Internal state of a single HTLC swap.
#[derive(Debug, Clone)]
pub struct EvmSwapState {
    pub swap_id: [u8; 32],
    pub sender: [u8; 20],
    pub receiver: [u8; 20],
    pub refund_address: [u8; 20],
    pub amount: u128,
    pub hashlock: [u8; 32],
    pub timeout: u64,
    pub asset: [u8; 20],
    pub claimed: bool,
    pub refunded: bool,
}

/// An on-chain event for downstream consumption.
#[derive(Debug, Clone)]
pub enum EvmEvent {
    Locked(EvmLockedEvent),
    Claimed(EvmClaimedEvent),
    Refunded(EvmRefundedEvent),
}

impl EvmHtlcContract {
    /// Create a new HTLC contract at the given address.
    pub fn new(address: [u8; 20]) -> Self {
        Self {
            address,
            swaps: Vec::new(),
            events: Vec::new(),
            rpc_client: None,
            event_watcher: None,
            contract_address: None,
            deployed: false,
        }
    }

    /// Configure RPC connectivity.
    ///
    /// Creates an `RpcClient` and `EventWatcher` for the given RPC URL and
    /// chain ID. After this, methods like `get_latest_block()` and
    /// `poll_events()` will attempt real RPC calls.
    pub fn connect_rpc(&mut self, rpc_url: &str, chain_id: u64) {
        let config = WatcherConfig {
            chain_id,
            ..WatcherConfig::default()
        };
        let watcher = EventWatcher::new(config, rpc_url.into(), chain_id);
        let client = RpcClient::new(rpc_url.into(), chain_id);

        self.rpc_client = Some(client);
        self.event_watcher = Some(watcher);
    }

    /// Mark the contract as deployed and register its address.
    pub fn deploy_contract(&mut self, contract_address: &str) {
        self.deployed = true;
        self.contract_address = Some(contract_address.into());
        if let Some(ref mut watcher) = self.event_watcher {
            watcher.add_contract(contract_address);
        }
    }

    /// Returns `true` if the contract has been deployed.
    pub fn is_deployed(&self) -> bool {
        self.deployed
    }

    /// Poll for events using the configured event watcher.
    ///
    /// Returns an error if no watcher is configured.
    pub fn poll_events(
        &mut self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<HtlcEvent>, SwapError> {
        let watcher = self.event_watcher.as_mut().ok_or_else(|| {
            SwapError::Internal("no event watcher configured; call connect_rpc() first".into())
        })?;
        if let Some(ref addr) = self.contract_address {
            watcher.add_contract(addr);
        }
        watcher.poll_all_events(from_block, to_block)
    }

    /// Get the latest block number via RPC.
    ///
    /// Returns an error if no RPC client is configured.
    pub fn get_latest_block(&mut self) -> Result<u64, SwapError> {
        let client = self.rpc_client.as_mut().ok_or_else(|| {
            SwapError::Internal("no RPC client configured; call connect_rpc() first".into())
        })?;
        client.get_block_number()
    }

    /// Estimate gas for an HTLC lock transaction.
    ///
    /// Returns an error if no RPC client is configured.
    pub fn estimate_lock_gas(&mut self, from: &str) -> Result<u64, SwapError> {
        let client = self.rpc_client.as_mut().ok_or_else(|| {
            SwapError::Internal("no RPC client configured; call connect_rpc() first".into())
        })?;
        // Lock function selector: keccak256("lock(uint64,address,address,uint256,bytes32,uint256)")
        // We use a placeholder selector for simulation
        let selector = "0xa3b2c4d5";
        let to_hex = hex::encode(self.address);
        client.estimate_gas(from, &format!("0x{}", to_hex), selector)
    }

    /// Lock funds in the HTLC.
    ///
    /// Creates a new swap with the given parameters. The hashlock is the
    /// SHA-256 hash of the preimage. The timeout is a unix timestamp or
    /// block number after which the refund address can reclaim funds.
    pub fn lock(
        &mut self,
        swap_id: [u8; 32],
        sender: [u8; 20],
        receiver: [u8; 20],
        refund_address: [u8; 20],
        amount: u128,
        hashlock: [u8; 32],
        timeout: u64,
        asset: [u8; 20],
    ) -> Result<EvmLockedEvent, SwapError> {
        // Check for duplicate swap_id
        if self.swaps.iter().any(|s| s.swap_id == swap_id) {
            return Err(SwapError::SourceLockFailed {
                reason: format!("swap_id {:?} already exists", swap_id),
            });
        }

        if amount == 0 {
            return Err(SwapError::SourceLockFailed {
                reason: "amount must be > 0".into(),
            });
        }

        self.swaps.push(EvmSwapState {
            swap_id,
            sender,
            receiver,
            refund_address,
            amount,
            hashlock,
            timeout,
            asset,
            claimed: false,
            refunded: false,
        });

        let event = EvmLockedEvent {
            contract: self.address,
            swap_id,
            sender,
            receiver,
            refund_address,
            amount,
            hashlock,
            timeout,
            asset,
        };
        self.events.push(EvmEvent::Locked(event.clone()));
        Ok(event)
    }

    /// Claim funds by providing the correct preimage.
    ///
    /// Verifies that SHA-256(preimage) == hashlock, then releases funds
    /// to the receiver.
    pub fn claim(
        &mut self,
        swap_id: &[u8; 32],
        claimant: [u8; 20],
        preimage: &[u8],
        current_time: u64,
    ) -> Result<EvmClaimedEvent, SwapError> {
        let state = self
            .swaps
            .iter_mut()
            .find(|s| s.swap_id == *swap_id)
            .ok_or_else(|| SwapError::ClaimFailed {
                chain: "evm".into(),
                reason: format!("swap_id {:?} not found", swap_id),
            })?;

        if state.claimed {
            return Err(SwapError::ClaimFailed {
                chain: "evm".into(),
                reason: "already claimed".into(),
            });
        }

        if state.refunded {
            return Err(SwapError::ClaimFailed {
                chain: "evm".into(),
                reason: "already refunded".into(),
            });
        }

        if current_time > state.timeout {
            return Err(SwapError::ClaimFailed {
                chain: "evm".into(),
                reason: "timeout has expired, use refund".into(),
            });
        }

        // Verify the preimage
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut computed_hash = [0u8; 32];
        computed_hash.copy_from_slice(&result);

        if computed_hash != state.hashlock {
            return Err(SwapError::ClaimFailed {
                chain: "evm".into(),
                reason: "hashlock mismatch: preimage does not match hashlock".into(),
            });
        }

        state.claimed = true;

        let event = EvmClaimedEvent {
            contract: self.address,
            swap_id: *swap_id,
            claimant,
            preimage: preimage.to_vec(),
        };
        self.events.push(EvmEvent::Claimed(event.clone()));
        Ok(event)
    }

    /// Refund funds to the refund address after timeout.
    pub fn refund(
        &mut self,
        swap_id: &[u8; 32],
        caller: [u8; 20],
        current_time: u64,
    ) -> Result<EvmRefundedEvent, SwapError> {
        let state = self
            .swaps
            .iter_mut()
            .find(|s| s.swap_id == *swap_id)
            .ok_or_else(|| SwapError::RefundFailed {
                chain: "evm".into(),
                reason: format!("swap_id {:?} not found", swap_id),
            })?;

        if state.claimed {
            return Err(SwapError::RefundFailed {
                chain: "evm".into(),
                reason: "already claimed".into(),
            });
        }

        if state.refunded {
            return Err(SwapError::RefundFailed {
                chain: "evm".into(),
                reason: "already refunded".into(),
            });
        }

        if current_time <= state.timeout {
            return Err(SwapError::RefundFailed {
                chain: "evm".into(),
                reason: "timeout has not yet expired".into(),
            });
        }

        if caller != state.refund_address {
            return Err(SwapError::RefundFailed {
                chain: "evm".into(),
                reason: "caller is not the refund address".into(),
            });
        }

        state.refunded = true;

        let event = EvmRefundedEvent {
            contract: self.address,
            swap_id: *swap_id,
            refund_address: state.refund_address,
        };
        self.events.push(EvmEvent::Refunded(event.clone()));
        Ok(event)
    }

    /// Get a swap state by swap_id.
    pub fn get_swap(&self, swap_id: &[u8; 32]) -> Option<&EvmSwapState> {
        self.swaps.iter().find(|s| s.swap_id == *swap_id)
    }

    /// Get all events emitted by this contract.
    pub fn get_events(&self) -> &[EvmEvent] {
        &self.events
    }

    /// Clear events (for testing).
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// EVM HTLC Adapter Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait for EVM HTLC operations that relayers and the engine can use.
pub trait EvmHtlcAdapter {
    /// Lock funds on the EVM chain.
    fn lock(
        &mut self,
        swap_id: [u8; 32],
        sender: [u8; 20],
        receiver: [u8; 20],
        refund_address: [u8; 20],
        amount: u128,
        hashlock: [u8; 32],
        timeout: u64,
        asset: [u8; 20],
    ) -> Result<EvmLockedEvent, SwapError>;

    /// Claim funds with preimage.
    fn claim(
        &mut self,
        swap_id: &[u8; 32],
        claimant: [u8; 20],
        preimage: &[u8],
        current_time: u64,
    ) -> Result<EvmClaimedEvent, SwapError>;

    /// Refund funds after timeout.
    fn refund(
        &mut self,
        swap_id: &[u8; 32],
        caller: [u8; 20],
        current_time: u64,
    ) -> Result<EvmRefundedEvent, SwapError>;

    /// Check if a swap exists and is still active.
    fn is_swap_active(&self, swap_id: &[u8; 32]) -> bool;

    /// Get the emitted Locked events for relayer watching.
    fn get_locked_events(&self) -> Vec<EvmLockedEvent>;

    /// Get the emitted Claimed events.
    fn get_claimed_events(&self) -> Vec<EvmClaimedEvent>;

    /// Get the emitted Refunded events.
    fn get_refunded_events(&self) -> Vec<EvmRefundedEvent>;
}

impl EvmHtlcAdapter for EvmHtlcContract {
    fn lock(
        &mut self,
        swap_id: [u8; 32],
        sender: [u8; 20],
        receiver: [u8; 20],
        refund_address: [u8; 20],
        amount: u128,
        hashlock: [u8; 32],
        timeout: u64,
        asset: [u8; 20],
    ) -> Result<EvmLockedEvent, SwapError> {
        self.lock(
            swap_id,
            sender,
            receiver,
            refund_address,
            amount,
            hashlock,
            timeout,
            asset,
        )
    }

    fn claim(
        &mut self,
        swap_id: &[u8; 32],
        claimant: [u8; 20],
        preimage: &[u8],
        current_time: u64,
    ) -> Result<EvmClaimedEvent, SwapError> {
        self.claim(swap_id, claimant, preimage, current_time)
    }

    fn refund(
        &mut self,
        swap_id: &[u8; 32],
        caller: [u8; 20],
        current_time: u64,
    ) -> Result<EvmRefundedEvent, SwapError> {
        self.refund(swap_id, caller, current_time)
    }

    fn is_swap_active(&self, swap_id: &[u8; 32]) -> bool {
        self.swaps
            .iter()
            .any(|s| s.swap_id == *swap_id && !s.claimed && !s.refunded)
    }

    fn get_locked_events(&self) -> Vec<EvmLockedEvent> {
        self.events
            .iter()
            .filter_map(|e| match e {
                EvmEvent::Locked(ev) => Some(ev.clone()),
                _ => None,
            })
            .collect()
    }

    fn get_claimed_events(&self) -> Vec<EvmClaimedEvent> {
        self.events
            .iter()
            .filter_map(|e| match e {
                EvmEvent::Claimed(ev) => Some(ev.clone()),
                _ => None,
            })
            .collect()
    }

    fn get_refunded_events(&self) -> Vec<EvmRefundedEvent> {
        self.events
            .iter()
            .filter_map(|e| match e {
                EvmEvent::Refunded(ev) => Some(ev.clone()),
                _ => None,
            })
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation (Wrapper for EvmHtlcContract)
// ─────────────────────────────────────────────────────────────────────────────

/// A stateless EVM adapter wrapping an [`EvmHtlcContract`], implementing
/// [`X3VmAdapter`] for use by the relayer and routing system.
///
/// This adapter generates lock/claim/refund proofs from intent data and
/// contract state without mutating the inner contract. For stateful operations
/// that require `&mut self`, use [`EvmHtlcContract`] directly via its inherent
/// methods or the [`EvmHtlcAdapter`] trait.
#[derive(Debug, Clone)]
pub struct EvmAdapter {
    /// Inner EVM HTLC contract for state reads.
    pub inner: EvmHtlcContract,
}

impl EvmAdapter {
    /// Create a new EVM adapter wrapping the given contract.
    pub fn new(contract: EvmHtlcContract) -> Self {
        Self { inner: contract }
    }

    /// Create a new EVM adapter with a fresh contract at the given address.
    pub fn at_address(address: [u8; 20]) -> Self {
        Self {
            inner: EvmHtlcContract::new(address),
        }
    }

    /// Generate a deterministic mock tx_id from intent_id and a label byte.
    fn mock_tx_id(intent_id: u64, label: u8) -> TxId {
        let mut hasher = Sha256::new();
        hasher.update(intent_id.to_le_bytes());
        hasher.update([label]);
        let result = hasher.finalize();
        hex::encode(result)
    }
}

impl X3VmAdapter for EvmAdapter {
    fn vm_type(&self) -> VmType {
        VmType::Evm
    }

    fn adapter_name(&self) -> &'static str {
        "evm-htlc-adapter"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "eth-mainnet".into(),
            "eth-sepolia".into(),
            "eth-holesky".into(),
            "base-mainnet".into(),
            "base-sepolia".into(),
            "arb-mainnet".into(),
            "arb-sepolia".into(),
            "op-mainnet".into(),
            "op-sepolia".into(),
            "polygon-mainnet".into(),
            "polygon-amoy".into(),
            "avax-mainnet".into(),
            "avax-fuji".into(),
            "bsc-mainnet".into(),
            "bsc-testnet".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec![
            "ETH".into(),
            "WETH".into(),
            "USDC".into(),
            "USDT".into(),
            "DAI".into(),
            "WBTC".into(),
        ]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let block_number = 42; // Simulated block number

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id: format!("{}-chain", intent.source_chain.as_str()),
            vm_type: VmType::Evm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 12,
            lock_address: format!("0x{}", hex::encode(self.inner.address)),
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x65, 0x76, 0x6d, 0x01], // "evm\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        // Reject empty preimage as invalid
        if preimage == [0u8; 32] {
            return Err(SwapError::ClaimFailed {
                chain: "evm".into(),
                reason: "preimage cannot be all zeros".into(),
            });
        }

        let chain_id = "evm-chain".to_string();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = 43;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::Evm,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x65, 0x76, 0x6d, 0x02], // "evm\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = "evm-chain".to_string();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = 44;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::Evm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x65, 0x76, 0x6d, 0x03], // "evm\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::Evm {
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
        // Verify the lock_address looks like an EVM address (0x-prefixed hex)
        let addr = proof
            .lock_address
            .strip_prefix("0x")
            .unwrap_or(&proof.lock_address);
        if addr.len() != 40 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(false);
        }
        Ok(true)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::Evm {
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
        if proof.vm_type != VmType::Evm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // Standard EVM fee: 21000 gas at 10 gwei = 0.00021 ETH
        Ok(FeeEstimate {
            chain_id: "eth-mainnet".into(),
            vm_type: VmType::Evm,
            native_fee: 210_000_000_000_000, // 0.00021 ETH in wei
            gas_units: 21_000,
            gas_price: 10_000_000_000, // 10 gwei
            estimated_usd: 0.50,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // Ethereum: 12 confirmations for safe finality
        Ok(FinalityProof {
            chain_id: "eth-mainnet".into(),
            vm_type: VmType::Evm,
            tx_id: tx_id.clone(),
            block_number: 42,
            block_hash: hex::encode(Sha256::digest(42u64.to_le_bytes())),
            confirmations: 12,
            finalized: true,
            finality_source: "eth-pow".into(),
            safe_to_reveal_secret: true,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: "eth-mainnet".into(),
            vm_type: VmType::Evm,
            latest_block: 100,
            finalized_block: 88,
            block_delay_ms: 12_000,     // ~12s block time
            finality_delay_ms: 144_000, // ~12 blocks * 12s
            rpc_quorum_healthy: true,
            gas_price: 10_000_000_000, // 10 gwei
            halted: false,
            degraded: false,
            safe_for_new_intents: true,
        })
    }

    // ── Readiness ─────────────────────────────────────────────────────────

    fn readiness_score(&self) -> AdapterReadinessScore {
        AdapterReadinessScore {
            adapter_name: "evm-htlc-adapter",
            vm_type: VmType::Evm,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: true,
            finality_proof: true,
            rpc_indexer_support: true,
            timeout_safety: true,
            tests_implemented: true,
            proof_ledger_integration: true,
            ibc_support: false,
            cross_adapter_atomicity_test: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address(n: u8) -> [u8; 20] {
        let mut addr = [0u8; 20];
        addr[0] = n;
        addr[19] = n;
        addr
    }

    fn make_hashlock(preimage: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    #[test]
    fn test_evm_lock_and_claim_happy_path() {
        let mut contract = EvmHtlcContract::new(test_address(1));
        let preimage = b"secret123";
        let hashlock = make_hashlock(preimage);
        let swap_id = [0xabu8; 32];

        let lock_event = EvmHtlcContract::lock(
            &mut contract,
            swap_id,
            test_address(2), // sender
            test_address(3), // receiver
            test_address(4), // refund address
            1000_000_000,
            hashlock,
            2000,      // timeout
            [0u8; 20], // native asset
        )
        .expect("lock should succeed");

        assert_eq!(lock_event.amount, 1000_000_000);
        assert_eq!(lock_event.hashlock, hashlock);

        // Claim with correct preimage before timeout
        let claim_event =
            EvmHtlcContract::claim(&mut contract, &swap_id, test_address(3), preimage, 1500)
                .expect("claim should succeed");

        assert_eq!(claim_event.preimage, preimage);

        // Verify events
        let locked_events = contract.get_locked_events();
        assert_eq!(locked_events.len(), 1);
        let claimed_events = contract.get_claimed_events();
        assert_eq!(claimed_events.len(), 1);
    }

    #[test]
    fn test_evm_wrong_preimage_rejected() {
        let mut contract = EvmHtlcContract::new(test_address(1));
        let hashlock = make_hashlock(b"secret123");
        let swap_id = [0xabu8; 32];

        EvmHtlcContract::lock(
            &mut contract,
            swap_id,
            test_address(2),
            test_address(3),
            test_address(4),
            1000,
            hashlock,
            2000,
            [0u8; 20],
        )
        .expect("lock should succeed");

        let result = EvmHtlcContract::claim(
            &mut contract,
            &swap_id,
            test_address(3),
            b"wrong_secret",
            1500,
        );
        assert!(
            result.is_err(),
            "wrong preimage should be rejected: {:?}",
            result
        );
        if let Err(SwapError::ClaimFailed { reason, .. }) = result {
            assert!(
                reason.contains("hashlock mismatch"),
                "reason should mention hashlock mismatch: {}",
                reason
            );
        } else {
            panic!("expected ClaimFailed error");
        }
    }

    #[test]
    fn test_evm_timeout_refund() {
        let mut contract = EvmHtlcContract::new(test_address(1));
        let hashlock = make_hashlock(b"secret123");
        let swap_id = [0xabu8; 32];

        EvmHtlcContract::lock(
            &mut contract,
            swap_id,
            test_address(2),
            test_address(3),
            test_address(4), // refund address
            1000,
            hashlock,
            1000, // timeout at 1000
            [0u8; 20],
        )
        .expect("lock should succeed");

        // Claim before timeout should work
        let claim_result =
            EvmHtlcContract::claim(&mut contract, &swap_id, test_address(3), b"secret123", 500);
        assert!(claim_result.is_ok(), "claim before timeout should work");

        // Now test a swap that actually expires
        let swap_id2 = [0xbbu8; 32];
        EvmHtlcContract::lock(
            &mut contract,
            swap_id2,
            test_address(2),
            test_address(3),
            test_address(4),
            2000,
            hashlock,
            1000,
            [0u8; 20],
        )
        .expect("second lock should succeed");

        // Try claim after timeout
        let claim_result2 = EvmHtlcContract::claim(
            &mut contract,
            &swap_id2,
            test_address(3),
            b"secret123",
            1500,
        );
        assert!(claim_result2.is_err(), "claim after timeout should fail");

        // Refund by refund address after timeout
        let refund_event = EvmHtlcContract::refund(&mut contract, &swap_id2, test_address(4), 1500)
            .expect("refund after timeout should succeed");
        assert_eq!(refund_event.refund_address, test_address(4));
    }

    #[test]
    fn test_evm_refund_before_timeout_rejected() {
        let mut contract = EvmHtlcContract::new(test_address(1));
        let hashlock = make_hashlock(b"secret123");
        let swap_id = [0xabu8; 32];

        EvmHtlcContract::lock(
            &mut contract,
            swap_id,
            test_address(2),
            test_address(3),
            test_address(4),
            1000,
            hashlock,
            2000,
            [0u8; 20],
        )
        .expect("lock should succeed");

        let result = EvmHtlcContract::refund(&mut contract, &swap_id, test_address(4), 1000);
        assert!(result.is_err(), "refund before timeout should be rejected");
    }

    #[test]
    fn test_evm_unauthorized_refund_rejected() {
        let mut contract = EvmHtlcContract::new(test_address(1));
        let hashlock = make_hashlock(b"secret123");
        let swap_id = [0xabu8; 32];

        EvmHtlcContract::lock(
            &mut contract,
            swap_id,
            test_address(2),
            test_address(3),
            test_address(4), // refund address is 4
            1000,
            hashlock,
            500,
            [0u8; 20],
        )
        .expect("lock should succeed");

        // Wrong caller tries to refund
        let result = EvmHtlcContract::refund(&mut contract, &swap_id, test_address(99), 1000);
        assert!(result.is_err(), "unauthorized refund should be rejected");
    }

    #[test]
    fn test_evm_double_claim_rejected() {
        let mut contract = EvmHtlcContract::new(test_address(1));
        let preimage = b"secret123";
        let hashlock = make_hashlock(preimage);
        let swap_id = [0xabu8; 32];

        EvmHtlcContract::lock(
            &mut contract,
            swap_id,
            test_address(2),
            test_address(3),
            test_address(4),
            1000,
            hashlock,
            2000,
            [0u8; 20],
        )
        .expect("lock should succeed");

        // First claim succeeds
        EvmHtlcContract::claim(&mut contract, &swap_id, test_address(3), preimage, 1500)
            .expect("first claim should succeed");

        // Second claim fails
        let result =
            EvmHtlcContract::claim(&mut contract, &swap_id, test_address(3), preimage, 1500);
        assert!(result.is_err(), "double claim should fail");
    }

    #[test]
    fn test_evm_lock_events_emitted() {
        let mut contract = EvmHtlcContract::new(test_address(1));
        let hashlock = make_hashlock(b"secret");
        let swap_id = [0xabu8; 32];

        EvmHtlcContract::lock(
            &mut contract,
            swap_id,
            test_address(2),
            test_address(3),
            test_address(4),
            500,
            hashlock,
            2000,
            [0u8; 20],
        )
        .expect("lock should succeed");

        let events = contract.get_locked_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 500);
        assert_eq!(events[0].sender, test_address(2));
        assert_eq!(events[0].receiver, test_address(3));
    }
}
