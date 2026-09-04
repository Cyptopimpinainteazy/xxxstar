//! # Bitcoin Script/Taproot HTLC Adapter
//!
//! Adapter for Bitcoin chains (mainnet, testnet, regtest) using P2SH-style
//! HTLC scripts. Implements [`X3VmAdapter`] with mock/placeholder proof structures.
//!
//! In production, [`lock`] would create a real P2SH HTLC transaction,
//! [`claim`] would spend the HTLC output with the preimage,
//! and [`refund`] would spend after timeout. Finality uses 6 confirmations.

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
// BitcoinNetwork
// ─────────────────────────────────────────────────────────────────────────────

/// Bitcoin network variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BitcoinNetwork {
    Mainnet,
    Testnet,
    Regtest,
}

impl BitcoinNetwork {
    /// Human-readable network name.
    pub fn name(&self) -> &'static str {
        match self {
            BitcoinNetwork::Mainnet => "bitcoin-mainnet",
            BitcoinNetwork::Testnet => "bitcoin-testnet",
            BitcoinNetwork::Regtest => "bitcoin-regtest",
        }
    }

    /// Default JSON-RPC port for this network.
    pub fn default_port(&self) -> u16 {
        match self {
            BitcoinNetwork::Mainnet => 8332,
            BitcoinNetwork::Testnet => 18332,
            BitcoinNetwork::Regtest => 18443,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BitcoinScript
// ─────────────────────────────────────────────────────────────────────────────

/// A raw Bitcoin script (pushdata + opcodes) as a byte vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitcoinScript(pub Vec<u8>);

impl BitcoinScript {
    /// Generate a P2SH HTLC redeem script.
    ///
    /// The script checks:
    ///   - `OP_SHA256 <hashlock> OP_EQUALVERIFY OP_DUP OP_HASH160 <receiver_pubkey_hash> OP_EQUALVERIFY OP_CHECKSIG` (claim path)
    ///   - `OP_IF` branch for claim, `OP_ELSE` branch for refund after timeout.
    ///
    /// Returns a placeholder script with the hashlock embedded plus metadata.
    pub fn generate_p2sh_htlc(
        hashlock: &[u8; 32],
        receiver_pubkey_hash: &[u8; 20],
        refund_pubkey_hash: &[u8; 20],
        timeout_blocks: u32,
    ) -> Self {
        // Build a P2SH HTLC script structure.
        // Format: [header: 4 bytes][hashlock: 32 bytes][receiver_hash: 20 bytes][refund_hash: 20 bytes][timeout: 4 bytes]
        let mut script = Vec::with_capacity(80);
        // Magic bytes for P2SH HTLC script type
        script.extend_from_slice(b"p2sh");
        // Hashlock
        script.extend_from_slice(hashlock);
        // Receiver pubkey hash
        script.extend_from_slice(receiver_pubkey_hash);
        // Refund pubkey hash
        script.extend_from_slice(refund_pubkey_hash);
        // Timeout blocks as big-endian u32
        script.extend_from_slice(&timeout_blocks.to_be_bytes());
        BitcoinScript(script)
    }

    /// Raw bytes of the script.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Length of the script in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the script is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BtcTransactionBuilder
// ─────────────────────────────────────────────────────────────────────────────

/// A builder for constructing mock Bitcoin transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcTransactionBuilder {
    pub tx_version: u32,
    pub inputs: Vec<BtcTxInput>,
    pub outputs: Vec<BtcTxOutput>,
    pub locktime: u32,
    pub script: Vec<u8>,
}

/// Mock Bitcoin transaction input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcTxInput {
    pub prev_txid: String,
    pub vout: u32,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

/// Mock Bitcoin transaction output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcTxOutput {
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

impl BtcTransactionBuilder {
    /// Create a new transaction builder with version 2 (BIP-68).
    pub fn new() -> Self {
        Self {
            tx_version: 2,
            inputs: Vec::new(),
            outputs: Vec::new(),
            locktime: 0,
            script: Vec::new(),
        }
    }

    /// Add a transaction input.
    pub fn add_input(
        &mut self,
        prev_txid: String,
        vout: u32,
        script_sig: Vec<u8>,
        sequence: u32,
    ) -> &mut Self {
        self.inputs.push(BtcTxInput {
            prev_txid,
            vout,
            script_sig,
            sequence,
        });
        self
    }

    /// Add a transaction output.
    pub fn add_output(&mut self, value: u64, script_pubkey: Vec<u8>) -> &mut Self {
        self.outputs.push(BtcTxOutput {
            value,
            script_pubkey,
        });
        self
    }

    /// Set the locktime.
    pub fn with_locktime(&mut self, locktime: u32) -> &mut Self {
        self.locktime = locktime;
        self
    }

    /// Set the script.
    pub fn with_script(&mut self, script: Vec<u8>) -> &mut Self {
        self.script = script;
        self
    }

    /// Build the transaction and return a mock tx_id (SHA-256 of serialized tx).
    pub fn build(&self) -> (String, Vec<u8>) {
        let mut serialized = Vec::new();
        serialized.extend_from_slice(&self.tx_version.to_le_bytes());
        serialized.push(self.inputs.len() as u8);
        for input in &self.inputs {
            serialized.extend_from_slice(input.prev_txid.as_bytes());
            serialized.extend_from_slice(&input.vout.to_le_bytes());
            serialized.extend_from_slice(&input.script_sig);
            serialized.extend_from_slice(&input.sequence.to_le_bytes());
        }
        serialized.push(self.outputs.len() as u8);
        for output in &self.outputs {
            serialized.extend_from_slice(&output.value.to_le_bytes());
            serialized.extend_from_slice(&output.script_pubkey);
        }
        serialized.extend_from_slice(&self.locktime.to_le_bytes());
        serialized.extend_from_slice(&self.script);

        let tx_id = hex::encode(Sha256::digest(&serialized));
        (tx_id, serialized)
    }
}

impl Default for BtcTransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BtcHtlcAdapter
// ─────────────────────────────────────────────────────────────────────────────

/// Adapter for Bitcoin chains using P2SH HTLC scripts.
///
/// Uses mock/placeholder proof data. In real operation this would connect to a
/// Bitcoin node via RPC and construct/manipulate P2SH transactions.
///
/// For stateful double-claim/double-refund enforcement, use
/// [`StatefulBtcAdapter`].
#[derive(Debug, Clone)]
pub struct BtcHtlcAdapter {
    /// Chain identifier (e.g. "bitcoin-mainnet", "bitcoin-testnet").
    pub chain_id: ChainId,
    /// Bitcoin network variant.
    pub network: BitcoinNetwork,
    /// Optional RPC URL for Bitcoin node.
    pub rpc_url: Option<String>,
    /// Current finalized block number.
    pub finalized_block: u64,
    /// Number of confirmations for the current best block.
    pub confirmed_blocks: u64,
}

/// Internal lock state tracked by the stateful adapter.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct InternalBtcLock {
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

impl BtcHtlcAdapter {
    /// Create a new adapter for the given Bitcoin network.
    pub fn new(network: BitcoinNetwork) -> Self {
        let chain_id = network.name().to_string();
        Self {
            chain_id,
            network,
            rpc_url: None,
            finalized_block: 0,
            confirmed_blocks: 0,
        }
    }

    /// Create a new adapter with a specific chain identifier.
    pub fn with_chain_id(chain_id: ChainId, network: BitcoinNetwork) -> Self {
        Self {
            chain_id,
            network,
            rpc_url: None,
            finalized_block: 0,
            confirmed_blocks: 0,
        }
    }

    /// Set the RPC URL.
    pub fn set_rpc(&mut self, rpc_url: &str) {
        self.rpc_url = Some(rpc_url.to_string());
    }

    /// Derive a mock P2SH address from the hashlock.
    fn derive_script_address(hashlock: &[u8; 32]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"bitcoin-p2sh-htlc:");
        hasher.update(hashlock);
        let result = hasher.finalize();
        // Bitcoin P2SH addresses start with '3' on mainnet.
        // For mock purposes, we produce an address-like string.
        format!("3{}", hex::encode(&result[..20]))
    }

    /// Generate a deterministic mock tx_id from intent_id and a label byte.
    fn mock_tx_id(intent_id: IntentId, label: u8) -> TxId {
        let mut hasher = Sha256::new();
        hasher.update(intent_id.to_le_bytes());
        hasher.update([label]);
        let result = hasher.finalize();
        hex::encode(result)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl X3VmAdapter for BtcHtlcAdapter {
    fn vm_type(&self) -> VmType {
        VmType::BitcoinScript
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-bitcoin"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "bitcoin-mainnet".into(),
            "bitcoin-testnet".into(),
            "bitcoin-regtest".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec!["BTC".into()]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let lock_address = Self::derive_script_address(&intent.hashlock);
        let block_number = self.finalized_block + 1;

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id,
            vm_type: VmType::BitcoinScript,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0,
            lock_address,
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x62, 0x74, 0x63, 0x01], // "btc\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = self.finalized_block + 2;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::BitcoinScript,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x62, 0x74, 0x63, 0x02], // "btc\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = self.chain_id.clone();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = self.finalized_block + 3;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::BitcoinScript,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x62, 0x74, 0x63, 0x03], // "btc\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::BitcoinScript {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        if proof.lock_address.is_empty() {
            return Ok(false);
        }
        // Bitcoin addresses start with 1 (P2PKH), 3 (P2SH), bc1 (bech32),
        // tb1 (testnet bech32), or mock prefix for tests.
        let addr = &proof.lock_address;
        let valid_prefix = addr.starts_with('1')
            || addr.starts_with('3')
            || addr.starts_with("bc1")
            || addr.starts_with("tb1")
            || addr.starts_with("mock");
        if !valid_prefix {
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
        if proof.vm_type != VmType::BitcoinScript {
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
        if proof.vm_type != VmType::BitcoinScript {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // Estimate complexity from intent data (simple mock)
        let is_complex = _intent.receiver.len() > 40;
        let native_fee = if is_complex { 20_000 } else { 10_000 }; // satoshis
        let estimated_usd = if is_complex { 0.0002 } else { 0.0001 };

        Ok(FeeEstimate {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::BitcoinScript,
            native_fee,
            gas_units: 0,
            gas_price: 0,
            estimated_usd,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // Bitcoin standard: 6 confirmations for finality
        let confirmations = self.confirmed_blocks;
        let finalized = confirmations >= 6;
        let safe = finalized;

        Ok(FinalityProof {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::BitcoinScript,
            tx_id: tx_id.clone(),
            block_number: self.finalized_block,
            block_hash: hex::encode(Sha256::digest(self.finalized_block.to_le_bytes())),
            confirmations,
            finalized,
            finality_source: "bitcoin-pow".into(),
            safe_to_reveal_secret: safe,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: self.chain_id.clone(),
            vm_type: VmType::BitcoinScript,
            latest_block: self.finalized_block,
            finalized_block: self.finalized_block.saturating_sub(6),
            block_delay_ms: 600_000,      // ~10 min block time
            finality_delay_ms: 3_600_000, // ~60 min for 6 confirmations
            rpc_quorum_healthy: self.rpc_url.is_some(),
            gas_price: 50, // avg sat/vB
            halted: false,
            degraded: self.rpc_url.is_none(),
            safe_for_new_intents: self.rpc_url.is_some(),
        })
    }

    // ── Readiness ─────────────────────────────────────────────────────────

    fn readiness_score(&self) -> AdapterReadinessScore {
        AdapterReadinessScore {
            adapter_name: "x3-adapter-bitcoin",
            vm_type: VmType::BitcoinScript,
            interface_implemented: true,
            lock_path: true,
            claim_path: true,
            refund_path: true,
            event_proof_extraction: false, // no Bitcoin indexer
            finality_proof: true,
            rpc_indexer_support: false, // needs real RPC/indexer integration
            timeout_safety: true,
            tests_implemented: true,
            proof_ledger_integration: true,
            ibc_support: false,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stateful Wrapper with Double-Claim Protection
// ─────────────────────────────────────────────────────────────────────────────

/// A stateful wrapper around [`BtcHtlcAdapter`] that tracks lock state
/// in order to reject double claims and double refunds.
#[derive(Debug, Clone)]
pub struct StatefulBtcAdapter {
    pub inner: BtcHtlcAdapter,
    locks: Vec<InternalBtcLock>,
}

impl StatefulBtcAdapter {
    pub fn new(network: BitcoinNetwork) -> Self {
        Self {
            inner: BtcHtlcAdapter::new(network),
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

        self.locks.push(InternalBtcLock {
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
            source_chain: ChainKind::Bitcoin,
            destination_chain: ChainKind::X3,
            source_asset: "BTC".into(),
            destination_asset: "X3".into(),
            amount_in: 1_000_000, // 0.01 BTC in satoshis
            min_amount_out: 500_000,
            receiver: "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq".into(),
            hashlock,
            source_timeout: 1_800_000,
            destination_timeout: 1_700_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::Bitcoin,
                level: FinalityLevel::Confirmations(6),
            }],
            refund_path: RefundPath {
                chain: ChainKind::Bitcoin,
                address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".into(),
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

    // ── Adapter Identity Tests ────────────────────────────────────────────

    #[test]
    fn test_adapter_identity() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        assert_eq!(adapter.vm_type(), VmType::BitcoinScript);
        assert_eq!(adapter.adapter_name(), "x3-adapter-bitcoin");

        let chains = adapter.supported_chains();
        assert!(chains.contains(&"bitcoin-mainnet".into()));
        assert!(chains.contains(&"bitcoin-testnet".into()));
        assert!(chains.contains(&"bitcoin-regtest".into()));

        let assets = adapter.supported_assets();
        assert!(assets.contains(&"BTC".into()));
    }

    #[test]
    fn test_adapter_name_const() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Testnet);
        let name: &'static str = adapter.adapter_name();
        assert_eq!(name, "x3-adapter-bitcoin");
    }

    // ── Network Enum Tests ────────────────────────────────────────────────

    #[test]
    fn test_bitcoin_network_names() {
        assert_eq!(BitcoinNetwork::Mainnet.name(), "bitcoin-mainnet");
        assert_eq!(BitcoinNetwork::Testnet.name(), "bitcoin-testnet");
        assert_eq!(BitcoinNetwork::Regtest.name(), "bitcoin-regtest");
    }

    #[test]
    fn test_bitcoin_network_ports() {
        assert_eq!(BitcoinNetwork::Mainnet.default_port(), 8332);
        assert_eq!(BitcoinNetwork::Testnet.default_port(), 18332);
        assert_eq!(BitcoinNetwork::Regtest.default_port(), 18443);
    }

    // ── Script Generation Tests ───────────────────────────────────────────

    #[test]
    fn test_bitcoin_script_helper_generates_correct_length() {
        let hashlock = make_hashlock(b"secret_preimage");
        let receiver_hash = [0xABu8; 20];
        let refund_hash = [0xCDu8; 20];
        let timeout_blocks = 144u32; // ~1 day

        let script = BitcoinScript::generate_p2sh_htlc(
            &hashlock,
            &receiver_hash,
            &refund_hash,
            timeout_blocks,
        );

        // 4 (header) + 32 (hashlock) + 20 (receiver) + 20 (refund) + 4 (timeout) = 80
        assert_eq!(script.len(), 80);
        assert_eq!(&script.as_bytes()[..4], b"p2sh");
        assert_eq!(&script.as_bytes()[4..36], &hashlock);
        assert_eq!(&script.as_bytes()[36..56], &receiver_hash);
        assert_eq!(&script.as_bytes()[56..76], &refund_hash);
        assert_eq!(&script.as_bytes()[76..80], &timeout_blocks.to_be_bytes());
    }

    #[test]
    fn test_bitcoin_script_different_hashlocks() {
        let h1 = make_hashlock(b"preimage1");
        let h2 = make_hashlock(b"preimage2");
        let receiver = [0xAAu8; 20];
        let refund = [0xBBu8; 20];

        let s1 = BitcoinScript::generate_p2sh_htlc(&h1, &receiver, &refund, 144);
        let s2 = BitcoinScript::generate_p2sh_htlc(&h2, &receiver, &refund, 144);

        assert_ne!(s1, s2);
    }

    // ── BtcTransactionBuilder Tests ───────────────────────────────────────

    #[test]
    fn test_btc_tx_builder_creates_valid_mock_tx() {
        let mut builder = BtcTransactionBuilder::new();
        builder.add_input("abc123".into(), 0, vec![0x00; 8], 0xffffffff);
        builder.add_output(100_000_000, vec![0x00; 25]);
        builder.with_locktime(0);

        let (tx_id, serialized) = builder.build();

        assert!(!tx_id.is_empty());
        assert_eq!(tx_id.len(), 64); // hex-encoded SHA-256 is 64 chars
        assert!(!serialized.is_empty());
    }

    #[test]
    fn test_btc_tx_builder_default() {
        let builder = BtcTransactionBuilder::default();
        assert_eq!(builder.tx_version, 2);
        assert!(builder.inputs.is_empty());
        assert!(builder.outputs.is_empty());
        assert_eq!(builder.locktime, 0);
    }

    // ── Lock Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_lock_creates_proof() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);
        let hashlock = make_hashlock(b"test_bitcoin_lock");
        let intent = make_test_intent(42, hashlock);

        let proof = adapter.lock(&intent).expect("lock should succeed");

        assert_eq!(proof.vm_type, VmType::BitcoinScript);
        assert_eq!(proof.hashlock, hashlock);
        assert_eq!(proof.locked_amount, intent.amount_in);
        assert!(!proof.tx_id.is_empty());
        assert!(!proof.lock_address.is_empty());
        // Verify it's a mock P2SH address (starts with '3')
        assert!(proof.lock_address.starts_with('3'));
        assert_eq!(proof.receiver, intent.receiver.as_bytes());
        assert_eq!(proof.refund_address, intent.refund_path.address.as_bytes());
        assert_ne!(proof.block_number, 0);
    }

    #[test]
    fn test_lock_differs_per_intent() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Testnet);
        let h1 = make_hashlock(b"secret1");
        let h2 = make_hashlock(b"secret2");

        let proof1 = adapter.lock(&make_test_intent(1, h1)).expect("lock 1");
        let proof2 = adapter.lock(&make_test_intent(2, h2)).expect("lock 2");

        assert_ne!(proof1.tx_id, proof2.tx_id);
        assert_ne!(proof1.lock_address, proof2.lock_address);
    }

    // ── Claim Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_claim_with_preimage() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Regtest);
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"btc_c");
            p
        };

        let proof = adapter.claim(100, preimage).expect("claim should succeed");

        assert_eq!(proof.intent_id, 100);
        assert_eq!(proof.preimage, preimage);
        assert_eq!(proof.vm_type, VmType::BitcoinScript);
        assert!(!proof.tx_id.is_empty());
    }

    // ── Refund Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_refund_after_timeout() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        let proof = adapter.refund(200).expect("refund should succeed");

        assert_eq!(proof.intent_id, 200);
        assert_eq!(proof.vm_type, VmType::BitcoinScript);
        assert!(!proof.tx_id.is_empty());
        assert_ne!(proof.block_number, 0);
    }

    // ── Verification Tests ────────────────────────────────────────────────

    #[test]
    fn test_verify_valid_lock() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);
        let hashlock = make_hashlock(b"valid_bitcoin_lock");
        let intent = make_test_intent(10, hashlock);

        let proof = adapter.lock(&intent).expect("lock");
        let valid = adapter.verify_lock(&proof).expect("verify");

        assert!(valid, "well-formed lock proof should verify");
    }

    #[test]
    fn test_verify_invalid_lock_wrong_vm() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        let bad_proof = LockProof {
            tx_id: "some_tx".into(),
            chain_id: "bitcoin-mainnet".into(),
            vm_type: VmType::Evm, // wrong!
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "3abc123".into(),
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
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        let bad_proof = LockProof {
            tx_id: String::new(),
            chain_id: "bitcoin-mainnet".into(),
            vm_type: VmType::BitcoinScript,
            block_number: 0,
            block_hash: "".into(),
            confirmations: 0,
            lock_address: "3addr".into(),
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
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "bitcoin-mainnet".into(),
            vm_type: VmType::BitcoinScript,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "3addr".into(),
            locked_amount: 0, // zero amount is invalid
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
    fn test_verify_invalid_lock_bad_address() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        let bad_proof = LockProof {
            tx_id: "tx_123".into(),
            chain_id: "bitcoin-mainnet".into(),
            vm_type: VmType::BitcoinScript,
            block_number: 1,
            block_hash: "hash".into(),
            confirmations: 0,
            lock_address: "xyz_not_a_bitcoin_address".into(), // doesn't start with 1, 3, bc1, tb1
            locked_amount: 100,
            hashlock: [0u8; 32],
            receiver: vec![],
            refund_address: vec![],
            timeout: 1000,
            raw_proof: vec![],
        };

        let valid = adapter.verify_lock(&bad_proof).expect("verify");
        assert!(!valid, "bad address prefix should fail verification");
    }

    #[test]
    fn test_verify_valid_claim() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);
        let preimage: [u8; 32] = {
            let mut p = [0u8; 32];
            p[..5].copy_from_slice(b"claim");
            p
        };

        let proof = adapter.claim(42, preimage).expect("claim");
        let valid = adapter.verify_claim(&proof).expect("verify");
        assert!(valid, "well-formed claim proof should verify");
    }

    #[test]
    fn test_verify_invalid_claim_zero_preimage() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        let bad_proof = ClaimProof {
            tx_id: "tx_claim".into(),
            intent_id: 42,
            chain_id: "bitcoin-mainnet".into(),
            vm_type: VmType::BitcoinScript,
            preimage: [0u8; 32], // zero preimage is invalid
            block_number: 1,
            block_hash: "hash".into(),
            raw_proof: vec![],
        };

        let valid = adapter.verify_claim(&bad_proof).expect("verify");
        assert!(!valid, "zero preimage should fail verification");
    }

    #[test]
    fn test_verify_valid_refund() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);
        let proof = adapter.refund(42).expect("refund");
        let valid = adapter.verify_refund(&proof).expect("verify");
        assert!(valid, "well-formed refund proof should verify");
    }

    #[test]
    fn test_verify_invalid_refund_empty_tx() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        let bad_proof = RefundProof {
            tx_id: String::new(),
            intent_id: 42,
            chain_id: "bitcoin-mainnet".into(),
            vm_type: VmType::BitcoinScript,
            block_number: 1,
            block_hash: "hash".into(),
            raw_proof: vec![],
        };

        let valid = adapter.verify_refund(&bad_proof).expect("verify");
        assert!(!valid, "empty tx_id should fail refund verification");
    }

    // ── Finality Tests ────────────────────────────────────────────────────

    #[test]
    fn test_finality_zero_confirmations_not_finalized() {
        let adapter = BtcHtlcAdapter {
            confirmed_blocks: 0,
            ..BtcHtlcAdapter::new(BitcoinNetwork::Mainnet)
        };

        let proof = adapter
            .finality_status(&"some_tx".into())
            .expect("finality");
        assert!(!proof.finalized);
        assert_eq!(proof.confirmations, 0);
    }

    #[test]
    fn test_finality_six_confirmations_finalized() {
        let adapter = BtcHtlcAdapter {
            confirmed_blocks: 6,
            ..BtcHtlcAdapter::new(BitcoinNetwork::Mainnet)
        };

        let proof = adapter
            .finality_status(&"some_tx".into())
            .expect("finality");
        assert!(proof.finalized);
        assert_eq!(proof.confirmations, 6);
    }

    #[test]
    fn test_finality_six_plus_confirmations_finalized() {
        let adapter = BtcHtlcAdapter {
            confirmed_blocks: 12,
            ..BtcHtlcAdapter::new(BitcoinNetwork::Mainnet)
        };

        let proof = adapter
            .finality_status(&"some_tx".into())
            .expect("finality");
        assert!(proof.finalized);
        assert_eq!(proof.confirmations, 12);
    }

    // ── Chain Health Tests ────────────────────────────────────────────────

    #[test]
    fn test_chain_health_default_healthy() {
        let adapter = BtcHtlcAdapter {
            rpc_url: Some("http://localhost:18443".into()),
            ..BtcHtlcAdapter::new(BitcoinNetwork::Regtest)
        };

        let health = adapter.chain_health().expect("health");
        assert!(health.rpc_quorum_healthy);
        assert!(!health.halted);
        assert!(!health.degraded);
        assert!(health.safe_for_new_intents);
    }

    #[test]
    fn test_chain_health_no_rpc_degraded() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);

        let health = adapter.chain_health().expect("health");
        assert!(!health.rpc_quorum_healthy);
        assert!(health.degraded);
        assert!(!health.safe_for_new_intents);
    }

    // ── Fee Estimation Tests ──────────────────────────────────────────────

    #[test]
    fn test_estimate_fee_simple_tx() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);
        let hashlock = make_hashlock(b"fee_test");
        // Use a short receiver to test the simple (cheaper) fee path
        let mut intent = make_test_intent(1, hashlock);
        intent.receiver = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".into(); // 34 chars

        let fee = adapter.estimate_fee(&intent).expect("fee");
        assert_eq!(fee.vm_type, VmType::BitcoinScript);
        assert_eq!(fee.native_fee, 10_000); // 0.0001 BTC
        assert_eq!(fee.estimated_usd, 0.0001);
    }

    #[test]
    fn test_estimate_fee_complex_tx() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);
        let hashlock = make_hashlock(b"fee_complex");
        // Long receiver address makes it "complex"
        let mut intent = make_test_intent(2, hashlock);
        intent.receiver =
            "bc1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq".into();

        let fee = adapter.estimate_fee(&intent).expect("fee");
        assert_eq!(fee.native_fee, 20_000); // 0.0002 BTC
        assert_eq!(fee.estimated_usd, 0.0002);
    }

    // ── Readiness Score Tests ─────────────────────────────────────────────

    #[test]
    fn test_readiness_score() {
        let adapter = BtcHtlcAdapter::new(BitcoinNetwork::Mainnet);
        let score = adapter.readiness_score();

        assert_eq!(score.adapter_name, "x3-adapter-bitcoin");
        assert_eq!(score.vm_type, VmType::BitcoinScript);
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
    }

    // ── Stateful Adapter Tests ────────────────────────────────────────────

    #[test]
    fn test_stateful_double_claim_rejected() {
        let mut adapter = StatefulBtcAdapter::new(BitcoinNetwork::Regtest);
        let preimage = make_hashlock(b"real_preimage"); // use hash of a preimage as the "preimage" for mock
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(50, hashlock);

        adapter.lock(&intent).expect("lock");

        // First claim should succeed
        adapter.claim(50, preimage).expect("first claim");

        // Second claim should fail
        let err = adapter.claim(50, preimage).unwrap_err();
        match err {
            SwapError::ClaimFailed { reason, .. } => {
                assert_eq!(reason, "already claimed");
            }
            _ => panic!("Expected ClaimFailed error"),
        }
    }

    #[test]
    fn test_stateful_double_refund_rejected() {
        let mut adapter = StatefulBtcAdapter::new(BitcoinNetwork::Regtest);
        let hashlock = make_hashlock(b"refund_test");
        let intent = make_test_intent(60, hashlock);

        adapter.lock(&intent).expect("lock");

        // First refund after timeout should succeed
        let current_time = intent.source_timeout + 100;
        adapter.refund(60, current_time).expect("first refund");

        // Second refund should fail
        let err = adapter.refund(60, current_time).unwrap_err();
        match err {
            SwapError::RefundFailed { reason, .. } => {
                assert_eq!(reason, "already refunded");
            }
            _ => panic!("Expected RefundFailed error"),
        }
    }

    #[test]
    fn test_stateful_claim_then_refund_rejected() {
        let mut adapter = StatefulBtcAdapter::new(BitcoinNetwork::Regtest);
        let preimage = make_hashlock(b"claim_first");
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(70, hashlock);

        adapter.lock(&intent).expect("lock");
        adapter.claim(70, preimage).expect("claim");

        // Refund after claim should fail
        let current_time = intent.source_timeout + 100;
        let err = adapter.refund(70, current_time).unwrap_err();
        match err {
            SwapError::RefundFailed { reason, .. } => {
                assert_eq!(reason, "already claimed");
            }
            _ => panic!("Expected RefundFailed error"),
        }
    }

    #[test]
    fn test_stateful_is_claimed() {
        let mut adapter = StatefulBtcAdapter::new(BitcoinNetwork::Mainnet);
        let preimage = make_hashlock(b"check_claimed");
        let hashlock = make_hashlock(&preimage);
        let intent = make_test_intent(80, hashlock);

        adapter.lock(&intent).expect("lock");
        assert!(!adapter.is_claimed(80));

        adapter.claim(80, preimage).expect("claim");
        assert!(adapter.is_claimed(80));
    }

    #[test]
    fn test_stateful_is_refunded() {
        let mut adapter = StatefulBtcAdapter::new(BitcoinNetwork::Mainnet);
        let hashlock = make_hashlock(b"check_refunded");
        let intent = make_test_intent(90, hashlock);

        adapter.lock(&intent).expect("lock");
        assert!(!adapter.is_refunded(90));

        let current_time = intent.source_timeout + 100;
        adapter.refund(90, current_time).expect("refund");
        assert!(adapter.is_refunded(90));
    }

    #[test]
    fn test_stateful_refund_before_timeout_rejected() {
        let mut adapter = StatefulBtcAdapter::new(BitcoinNetwork::Mainnet);
        let hashlock = make_hashlock(b"early_refund");
        let intent = make_test_intent(100, hashlock);

        adapter.lock(&intent).expect("lock");

        // Refund before timeout should fail
        let current_time = intent.source_timeout - 100;
        let err = adapter.refund(100, current_time).unwrap_err();
        match err {
            SwapError::RefundFailed { reason, .. } => {
                assert!(reason.contains("timeout"), "error should mention timeout");
            }
            _ => panic!("Expected RefundFailed error"),
        }
    }
}
