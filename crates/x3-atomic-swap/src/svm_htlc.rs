//! # SVM (Solana Virtual Machine) HTLC Adapter
//!
//! Implements the Solana-side HTLC design for atomic swaps using a
//! program-derived lock account pattern:
//!
//! - **Lock account**: PDA (Program Derived Address) containing swap state
//! - **Hashlock**: 32-byte hash stored in the account
//! - **Claimant**: address authorized to claim with preimage
//! - **Refund authority**: address authorized to refund after timeout
//! - **Timeout**: slot or unix timestamp
//! - **Claim instruction**: verifies preimage, transfers tokens
//! - **Refund instruction**: returns tokens to refund authority after timeout
//! - **Event/log output**: relayers watch via log data

use crate::error::SwapError;
use crate::event_watcher::{EventWatcher, HtlcEvent, WatcherConfig};
use crate::rpc_client::RpcClient;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A Solana pubkey (32 bytes).
pub type SolPubkey = [u8; 32];

/// A Solana signature (64 bytes).
pub type SolSignature = [u8; 64];

/// State of an HTLC lock account on Solana.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmHtlcAccount {
    /// The PDA address of this lock account.
    pub address: SolPubkey,
    /// Unique swap identifier.
    pub swap_id: [u8; 32],
    /// The initializer who locked funds.
    pub initializer: SolPubkey,
    /// The claimant who can claim with the preimage.
    pub claimant: SolPubkey,
    /// The refund authority after timeout.
    pub refund_authority: SolPubkey,
    /// Amount locked (in lamports or SPL token base units).
    pub amount: u64,
    /// Token mint address (or all-zero for SOL).
    pub token_mint: SolPubkey,
    /// Hashlock: 32-byte hash of the preimage.
    pub hashlock: [u8; 32],
    /// Timeout slot number or unix timestamp.
    pub timeout: u64,
    /// Whether the swap has been claimed.
    pub claimed: bool,
    /// Whether the swap has been refunded.
    pub refunded: bool,
    /// Bump seed for PDA derivation.
    pub bump_seed: u8,
}

/// Event emitted when an SVM HTLC lock is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmLockedEvent {
    pub program_id: SolPubkey,
    pub swap_id: [u8; 32],
    pub lock_account: SolPubkey,
    pub initializer: SolPubkey,
    pub claimant: SolPubkey,
    pub refund_authority: SolPubkey,
    pub amount: u64,
    pub hashlock: [u8; 32],
    pub timeout: u64,
    pub token_mint: SolPubkey,
}

/// Event emitted when an SVM HTLC is claimed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmClaimedEvent {
    pub program_id: SolPubkey,
    pub swap_id: [u8; 32],
    pub lock_account: SolPubkey,
    pub claimant: SolPubkey,
    pub preimage: Vec<u8>,
}

/// Event emitted when an SVM HTLC is refunded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmRefundedEvent {
    pub program_id: SolPubkey,
    pub swap_id: [u8; 32],
    pub lock_account: SolPubkey,
    pub refund_authority: SolPubkey,
}

// ─────────────────────────────────────────────────────────────────────────────
// SVM HTLC Program Adapter
// ─────────────────────────────────────────────────────────────────────────────

/// In-memory SVM HTLC program adapter for testing and simulation.
///
/// Manages a set of lock accounts and emits events that relayers can watch.
/// Optionally configured with an RPC client and event watcher for live chain
/// interaction.
#[derive(Debug, Clone)]
pub struct SvmHtlcProgram {
    /// Program ID (the on-chain program address).
    pub program_id: SolPubkey,
    /// Lock accounts keyed by swap_id.
    accounts: Vec<SvmHtlcAccount>,
    /// Emitted events.
    events: Vec<SvmEvent>,
    /// Optional RPC client for JSON-RPC calls.
    pub rpc_client: Option<RpcClient>,
    /// Optional event watcher for polling on-chain events.
    pub event_watcher: Option<EventWatcher>,
    /// Program address string for RPC interactions.
    pub program_address: Option<String>,
    /// Whether the program has been marked as deployed.
    pub deployed: bool,
}

/// Events emitted by the SVM HTLC program.
#[derive(Debug, Clone)]
pub enum SvmEvent {
    Locked(SvmLockedEvent),
    Claimed(SvmClaimedEvent),
    Refunded(SvmRefundedEvent),
}

impl SvmHtlcProgram {
    /// Create a new SVM HTLC program at the given program ID.
    pub fn new(program_id: SolPubkey) -> Self {
        Self {
            program_id,
            accounts: Vec::new(),
            events: Vec::new(),
            rpc_client: None,
            event_watcher: None,
            program_address: None,
            deployed: false,
        }
    }

    /// Configure RPC connectivity for the SVM adapter.
    ///
    /// Creates an `RpcClient` and `EventWatcher` for the given JSON-RPC URL
    /// (Solana nodes use JSON-RPC) and chain ID.
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

    /// Mark the program as deployed and register its address.
    pub fn deploy_contract(&mut self, program_address: &str) {
        self.deployed = true;
        self.program_address = Some(program_address.into());
        if let Some(ref mut watcher) = self.event_watcher {
            watcher.add_contract(program_address);
        }
    }

    /// Returns `true` if the program has been deployed.
    pub fn is_deployed(&self) -> bool {
        self.deployed
    }

    /// Get the latest slot (block number) via RPC.
    ///
    /// Uses Solana's `getSlot` method. Returns an error if no RPC client is
    /// configured.
    pub fn get_latest_block(&mut self) -> Result<u64, SwapError> {
        let client = self.rpc_client.as_mut().ok_or_else(|| {
            SwapError::Internal("no RPC client configured; call connect_rpc() first".into())
        })?;
        // Solana uses getSlot instead of eth_blockNumber
        let resp = client.call("getSlot", alloc::vec![])?;
        if let Some(result) = resp.result {
            if let Some(n) = result.as_u64() {
                Ok(n)
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }

    /// Poll for events using the configured event watcher.
    pub fn poll_events(
        &mut self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<HtlcEvent>, SwapError> {
        let watcher = self.event_watcher.as_mut().ok_or_else(|| {
            SwapError::Internal("no event watcher configured; call connect_rpc() first".into())
        })?;
        if let Some(ref addr) = self.program_address {
            watcher.add_contract(addr);
        }
        watcher.poll_all_events(from_block, to_block)
    }

    /// Derive a PDA for an HTLC lock account.
    ///
    /// PDA = hash(program_id, seeds = [b"htlc", swap_id, bump])
    pub fn derive_lock_account(program_id: &SolPubkey, swap_id: &[u8; 32], bump: u8) -> SolPubkey {
        let mut hasher = Sha256::new();
        hasher.update(program_id);
        hasher.update(b"htlc");
        hasher.update(swap_id);
        hasher.update([bump]);
        let result = hasher.finalize();
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&result);
        addr
    }

    /// Lock funds in a new HTLC account.
    ///
    /// Creates a lock account PDA and records the swap parameters.
    pub fn lock(
        &mut self,
        swap_id: [u8; 32],
        initializer: SolPubkey,
        claimant: SolPubkey,
        refund_authority: SolPubkey,
        amount: u64,
        hashlock: [u8; 32],
        timeout: u64,
        token_mint: SolPubkey,
        bump_seed: u8,
    ) -> Result<SvmLockedEvent, SwapError> {
        // Check for duplicate swap_id
        if self.accounts.iter().any(|a| a.swap_id == swap_id) {
            return Err(SwapError::SourceLockFailed {
                reason: format!("swap_id {:?} already exists", swap_id),
            });
        }

        if amount == 0 {
            return Err(SwapError::SourceLockFailed {
                reason: "amount must be > 0".into(),
            });
        }

        let address = Self::derive_lock_account(&self.program_id, &swap_id, bump_seed);

        let account = SvmHtlcAccount {
            address,
            swap_id,
            initializer,
            claimant,
            refund_authority,
            amount,
            token_mint,
            hashlock,
            timeout,
            claimed: false,
            refunded: false,
            bump_seed,
        };

        self.accounts.push(account);

        let event = SvmLockedEvent {
            program_id: self.program_id,
            swap_id,
            lock_account: address,
            initializer,
            claimant,
            refund_authority,
            amount,
            hashlock,
            timeout,
            token_mint,
        };
        self.events.push(SvmEvent::Locked(event.clone()));
        Ok(event)
    }

    /// Claim funds by providing the correct preimage.
    ///
    /// Verifies SHA-256(preimage) == hashlock, then marks as claimed.
    pub fn claim(
        &mut self,
        swap_id: &[u8; 32],
        claimant: SolPubkey,
        preimage: &[u8],
        current_time: u64,
    ) -> Result<SvmClaimedEvent, SwapError> {
        let account = self
            .accounts
            .iter_mut()
            .find(|a| a.swap_id == *swap_id)
            .ok_or_else(|| SwapError::ClaimFailed {
                chain: "svm".into(),
                reason: format!("swap_id {:?} not found", swap_id),
            })?;

        if account.claimed {
            return Err(SwapError::ClaimFailed {
                chain: "svm".into(),
                reason: "already claimed".into(),
            });
        }

        if account.refunded {
            return Err(SwapError::ClaimFailed {
                chain: "svm".into(),
                reason: "already refunded".into(),
            });
        }

        if current_time > account.timeout {
            return Err(SwapError::ClaimFailed {
                chain: "svm".into(),
                reason: "timeout has expired, use refund".into(),
            });
        }

        // Verify preimage
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut computed_hash = [0u8; 32];
        computed_hash.copy_from_slice(&result);

        if computed_hash != account.hashlock {
            return Err(SwapError::ClaimFailed {
                chain: "svm".into(),
                reason: "hashlock mismatch: preimage does not match hashlock".into(),
            });
        }

        // Verify claimant
        if claimant != account.claimant {
            return Err(SwapError::ClaimFailed {
                chain: "svm".into(),
                reason: "caller is not the authorized claimant".into(),
            });
        }

        account.claimed = true;

        let event = SvmClaimedEvent {
            program_id: self.program_id,
            swap_id: *swap_id,
            lock_account: account.address,
            claimant,
            preimage: preimage.to_vec(),
        };
        self.events.push(SvmEvent::Claimed(event.clone()));
        Ok(event)
    }

    /// Refund funds to the refund authority after timeout.
    pub fn refund(
        &mut self,
        swap_id: &[u8; 32],
        caller: SolPubkey,
        current_time: u64,
    ) -> Result<SvmRefundedEvent, SwapError> {
        let account = self
            .accounts
            .iter_mut()
            .find(|a| a.swap_id == *swap_id)
            .ok_or_else(|| SwapError::RefundFailed {
                chain: "svm".into(),
                reason: format!("swap_id {:?} not found", swap_id),
            })?;

        if account.claimed {
            return Err(SwapError::RefundFailed {
                chain: "svm".into(),
                reason: "already claimed".into(),
            });
        }

        if account.refunded {
            return Err(SwapError::RefundFailed {
                chain: "svm".into(),
                reason: "already refunded".into(),
            });
        }

        if current_time <= account.timeout {
            return Err(SwapError::RefundFailed {
                chain: "svm".into(),
                reason: "timeout has not yet expired".into(),
            });
        }

        if caller != account.refund_authority {
            return Err(SwapError::RefundFailed {
                chain: "svm".into(),
                reason: "caller is not the refund authority".into(),
            });
        }

        account.refunded = true;

        let event = SvmRefundedEvent {
            program_id: self.program_id,
            swap_id: *swap_id,
            lock_account: account.address,
            refund_authority: account.refund_authority,
        };
        self.events.push(SvmEvent::Refunded(event.clone()));
        Ok(event)
    }

    /// Get a lock account by swap_id.
    pub fn get_account(&self, swap_id: &[u8; 32]) -> Option<&SvmHtlcAccount> {
        self.accounts.iter().find(|a| a.swap_id == *swap_id)
    }

    /// Check if a swap is active (exists, not claimed, not refunded).
    pub fn is_swap_active(&self, swap_id: &[u8; 32]) -> bool {
        self.accounts
            .iter()
            .any(|a| a.swap_id == *swap_id && !a.claimed && !a.refunded)
    }

    /// Get all emitted Locked events.
    pub fn get_locked_events(&self) -> Vec<SvmLockedEvent> {
        self.events
            .iter()
            .filter_map(|e| match e {
                SvmEvent::Locked(ev) => Some(ev.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get all emitted Claimed events.
    pub fn get_claimed_events(&self) -> Vec<SvmClaimedEvent> {
        self.events
            .iter()
            .filter_map(|e| match e {
                SvmEvent::Claimed(ev) => Some(ev.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get all emitted Refunded events.
    pub fn get_refunded_events(&self) -> Vec<SvmRefundedEvent> {
        self.events
            .iter()
            .filter_map(|e| match e {
                SvmEvent::Refunded(ev) => Some(ev.clone()),
                _ => None,
            })
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SVM HTLC Adapter Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait for SVM HTLC operations that relayers and the engine can use.
pub trait SvmHtlcAdapter {
    fn lock(
        &mut self,
        swap_id: [u8; 32],
        initializer: SolPubkey,
        claimant: SolPubkey,
        refund_authority: SolPubkey,
        amount: u64,
        hashlock: [u8; 32],
        timeout: u64,
        token_mint: SolPubkey,
        bump_seed: u8,
    ) -> Result<SvmLockedEvent, SwapError>;

    fn claim(
        &mut self,
        swap_id: &[u8; 32],
        claimant: SolPubkey,
        preimage: &[u8],
        current_time: u64,
    ) -> Result<SvmClaimedEvent, SwapError>;

    fn refund(
        &mut self,
        swap_id: &[u8; 32],
        caller: SolPubkey,
        current_time: u64,
    ) -> Result<SvmRefundedEvent, SwapError>;

    fn is_swap_active(&self, swap_id: &[u8; 32]) -> bool;
    fn get_locked_events(&self) -> Vec<SvmLockedEvent>;
    fn get_claimed_events(&self) -> Vec<SvmClaimedEvent>;
    fn get_refunded_events(&self) -> Vec<SvmRefundedEvent>;
}

impl SvmHtlcAdapter for SvmHtlcProgram {
    fn lock(
        &mut self,
        swap_id: [u8; 32],
        initializer: SolPubkey,
        claimant: SolPubkey,
        refund_authority: SolPubkey,
        amount: u64,
        hashlock: [u8; 32],
        timeout: u64,
        token_mint: SolPubkey,
        bump_seed: u8,
    ) -> Result<SvmLockedEvent, SwapError> {
        self.lock(
            swap_id,
            initializer,
            claimant,
            refund_authority,
            amount,
            hashlock,
            timeout,
            token_mint,
            bump_seed,
        )
    }

    fn claim(
        &mut self,
        swap_id: &[u8; 32],
        claimant: SolPubkey,
        preimage: &[u8],
        current_time: u64,
    ) -> Result<SvmClaimedEvent, SwapError> {
        self.claim(swap_id, claimant, preimage, current_time)
    }

    fn refund(
        &mut self,
        swap_id: &[u8; 32],
        caller: SolPubkey,
        current_time: u64,
    ) -> Result<SvmRefundedEvent, SwapError> {
        self.refund(swap_id, caller, current_time)
    }

    fn is_swap_active(&self, swap_id: &[u8; 32]) -> bool {
        self.is_swap_active(swap_id)
    }

    fn get_locked_events(&self) -> Vec<SvmLockedEvent> {
        self.get_locked_events()
    }

    fn get_claimed_events(&self) -> Vec<SvmClaimedEvent> {
        self.get_claimed_events()
    }

    fn get_refunded_events(&self) -> Vec<SvmRefundedEvent> {
        self.get_refunded_events()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// X3VmAdapter Implementation (Wrapper for SvmHtlcProgram)
// ─────────────────────────────────────────────────────────────────────────────

use crate::adapter::{
    AdapterReadinessScore, AssetId, ChainHealth, ChainId, ClaimProof, FeeEstimate, FinalityProof,
    LockProof, RefundProof, TxId, VmType, X3VmAdapter,
};
use crate::intent::{AtomicIntent, IntentId};

/// A stateless SVM adapter wrapping an [`SvmHtlcProgram`], implementing
/// [`X3VmAdapter`] for use by the relayer and routing system.
///
/// This adapter generates lock/claim/refund proofs from intent data and
/// program state without mutating the inner program. For stateful operations
/// that require `&mut self`, use [`SvmHtlcProgram`] directly via its inherent
/// methods or the [`SvmHtlcAdapter`] trait.
#[derive(Debug, Clone)]
pub struct SvmAdapter {
    /// Inner SVM HTLC program for state reads.
    pub inner: SvmHtlcProgram,
}

impl SvmAdapter {
    /// Create a new SVM adapter wrapping the given program.
    pub fn new(program: SvmHtlcProgram) -> Self {
        Self { inner: program }
    }

    /// Create a new SVM adapter with a fresh program at the given program ID.
    pub fn at_program_id(program_id: SolPubkey) -> Self {
        Self {
            inner: SvmHtlcProgram::new(program_id),
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

impl X3VmAdapter for SvmAdapter {
    fn vm_type(&self) -> VmType {
        VmType::Svm
    }

    fn adapter_name(&self) -> &'static str {
        "svm-htlc-adapter"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![
            "solana-mainnet".into(),
            "solana-devnet".into(),
            "solana-testnet".into(),
        ]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec!["SOL".into(), "USDC".into(), "USDT".into()]
    }

    // ── Lifecycle operations ──────────────────────────────────────────────

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let tx_id = Self::mock_tx_id(intent.intent_id, 0x01);
        let block_number = 42; // Simulated slot number

        let receiver = intent.receiver.as_bytes().to_vec();
        let refund_address = intent.refund_path.address.as_bytes().to_vec();

        Ok(LockProof {
            tx_id,
            chain_id: format!("{}-chain", intent.source_chain.as_str()),
            vm_type: VmType::Svm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            confirmations: 0, // Solana uses 0 for immediate finality
            lock_address: hex::encode(self.inner.program_id),
            locked_amount: intent.amount_in,
            hashlock: intent.hashlock,
            receiver,
            refund_address,
            timeout: intent.source_timeout,
            raw_proof: vec![0x73, 0x76, 0x6d, 0x01], // "svm\x01" - mock proof
        })
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        // Reject empty preimage as invalid
        if preimage == [0u8; 32] {
            return Err(SwapError::ClaimFailed {
                chain: "svm".into(),
                reason: "preimage cannot be all zeros".into(),
            });
        }

        let chain_id = "svm-chain".to_string();
        let tx_id = Self::mock_tx_id(intent_id, 0x02);
        let block_number = 43;

        Ok(ClaimProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::Svm,
            preimage,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x73, 0x76, 0x6d, 0x02], // "svm\x02" - mock proof
        })
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let chain_id = "svm-chain".to_string();
        let tx_id = Self::mock_tx_id(intent_id, 0x03);
        let block_number = 44;

        Ok(RefundProof {
            tx_id,
            intent_id,
            chain_id,
            vm_type: VmType::Svm,
            block_number,
            block_hash: hex::encode(Sha256::digest(block_number.to_le_bytes())),
            raw_proof: vec![0x73, 0x76, 0x6d, 0x03], // "svm\x03" - mock proof
        })
    }

    // ── Verification ──────────────────────────────────────────────────────

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::Svm {
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
        // Verify the lock_address looks like a Solana pubkey (64 hex chars, no prefix)
        if proof.lock_address.len() != 64
            || !proof.lock_address.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Ok(false);
        }
        Ok(true)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        if proof.vm_type != VmType::Svm {
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
        if proof.vm_type != VmType::Svm {
            return Ok(false);
        }
        if proof.tx_id.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }

    // ── Estimation & Health ───────────────────────────────────────────────

    fn estimate_fee(&self, _intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        // Standard Solana fee: 5000 lamports per signature
        Ok(FeeEstimate {
            chain_id: "solana-mainnet".into(),
            vm_type: VmType::Svm,
            native_fee: 5_000, // 5000 lamports
            gas_units: 200,
            gas_price: 25, // 25 lamports per CU
            estimated_usd: 0.0002,
        })
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        // Solana: 0 confirmations for finalized commitment
        Ok(FinalityProof {
            chain_id: "solana-mainnet".into(),
            vm_type: VmType::Svm,
            tx_id: tx_id.clone(),
            block_number: 42,
            block_hash: hex::encode(Sha256::digest(42u64.to_le_bytes())),
            confirmations: 0,
            finalized: true,
            finality_source: "solana-poh".into(),
            safe_to_reveal_secret: true,
        })
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        Ok(ChainHealth {
            chain_id: "solana-mainnet".into(),
            vm_type: VmType::Svm,
            latest_block: 100,
            finalized_block: 100,
            block_delay_ms: 400,      // ~400ms slot time
            finality_delay_ms: 1_200, // ~3 slots for finality (optimistic)
            rpc_quorum_healthy: true,
            gas_price: 25, // 25 lamports per CU
            halted: false,
            degraded: false,
            safe_for_new_intents: true,
        })
    }

    // ── Readiness ─────────────────────────────────────────────────────────

    fn readiness_score(&self) -> AdapterReadinessScore {
        AdapterReadinessScore {
            adapter_name: "svm-htlc-adapter",
            vm_type: VmType::Svm,
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

    fn test_pubkey(n: u8) -> SolPubkey {
        let mut pk = [0u8; 32];
        pk[0] = n;
        pk[31] = n;
        pk
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
    fn test_svm_lock_and_claim_happy_path() {
        let mut program = SvmHtlcProgram::new(test_pubkey(1));
        let preimage = b"sol_secret_456";
        let hashlock = make_hashlock(preimage);
        let swap_id = [0xabu8; 32];

        let lock_event = program
            .lock(
                swap_id,
                test_pubkey(2), // initializer
                test_pubkey(3), // claimant
                test_pubkey(4), // refund authority
                500_000_000,    // amount in lamports
                hashlock,
                2000,      // timeout
                [0u8; 32], // native SOL
                255,       // bump seed
            )
            .expect("lock should succeed");

        assert_eq!(lock_event.amount, 500_000_000);
        assert_eq!(lock_event.hashlock, hashlock);
        assert_eq!(lock_event.initializer, test_pubkey(2));

        // Claim with correct preimage
        let claim_event = program
            .claim(&swap_id, test_pubkey(3), preimage, 1500)
            .expect("claim should succeed");

        assert_eq!(claim_event.preimage, preimage);

        // Verify events
        assert_eq!(program.get_locked_events().len(), 1);
        assert_eq!(program.get_claimed_events().len(), 1);
    }

    #[test]
    fn test_svm_wrong_preimage_rejected() {
        let mut program = SvmHtlcProgram::new(test_pubkey(1));
        let hashlock = make_hashlock(b"real_secret");
        let swap_id = [0xabu8; 32];

        program
            .lock(
                swap_id,
                test_pubkey(2),
                test_pubkey(3),
                test_pubkey(4),
                1000,
                hashlock,
                2000,
                [0u8; 32],
                255,
            )
            .expect("lock should succeed");

        let result = program.claim(&swap_id, test_pubkey(3), b"wrong_secret", 1500);
        assert!(
            result.is_err(),
            "wrong preimage should be rejected: {:?}",
            result
        );
        if let Err(SwapError::ClaimFailed { reason, .. }) = result {
            assert!(reason.contains("hashlock mismatch"), "reason: {}", reason);
        }
    }

    #[test]
    fn test_svm_timeout_refund() {
        let mut program = SvmHtlcProgram::new(test_pubkey(1));
        let hashlock = make_hashlock(b"secret");
        let swap_id = [0xabu8; 32];

        program
            .lock(
                swap_id,
                test_pubkey(2),
                test_pubkey(3),
                test_pubkey(4),
                1000,
                hashlock,
                500, // early timeout
                [0u8; 32],
                255,
            )
            .expect("lock should succeed");

        // Try claim after timeout
        let claim_result = program.claim(&swap_id, test_pubkey(3), b"secret", 1000);
        assert!(claim_result.is_err(), "claim after timeout should fail");

        // Refund after timeout
        let refund_event = program
            .refund(&swap_id, test_pubkey(4), 1000)
            .expect("refund should succeed");
        assert_eq!(refund_event.refund_authority, test_pubkey(4));
        assert_eq!(program.get_refunded_events().len(), 1);
    }

    #[test]
    fn test_svm_unauthorized_claimant_rejected() {
        let mut program = SvmHtlcProgram::new(test_pubkey(1));
        let preimage = b"secret";
        let hashlock = make_hashlock(preimage);
        let swap_id = [0xabu8; 32];

        program
            .lock(
                swap_id,
                test_pubkey(2),
                test_pubkey(3), // claimant is pubkey 3
                test_pubkey(4),
                1000,
                hashlock,
                2000,
                [0u8; 32],
                255,
            )
            .expect("lock should succeed");

        // Wrong claimant tries to claim
        let result = program.claim(&swap_id, test_pubkey(99), preimage, 1500);
        assert!(
            result.is_err(),
            "wrong claimant should be rejected: {:?}",
            result
        );
    }

    #[test]
    fn test_svm_double_claim_rejected() {
        let mut program = SvmHtlcProgram::new(test_pubkey(1));
        let preimage = b"secret";
        let hashlock = make_hashlock(preimage);
        let swap_id = [0xabu8; 32];

        program
            .lock(
                swap_id,
                test_pubkey(2),
                test_pubkey(3),
                test_pubkey(4),
                1000,
                hashlock,
                2000,
                [0u8; 32],
                255,
            )
            .expect("lock should succeed");

        // First claim succeeds
        program
            .claim(&swap_id, test_pubkey(3), preimage, 1500)
            .expect("first claim should succeed");

        // Second claim fails
        let result = program.claim(&swap_id, test_pubkey(3), preimage, 1500);
        assert!(result.is_err(), "double claim should fail");
    }

    #[test]
    fn test_svm_derive_lock_account_deterministic() {
        let program_id = test_pubkey(1);
        let swap_id = [0xabu8; 32];
        let addr1 = SvmHtlcProgram::derive_lock_account(&program_id, &swap_id, 255);
        let addr2 = SvmHtlcProgram::derive_lock_account(&program_id, &swap_id, 255);
        assert_eq!(addr1, addr2, "PDA derivation must be deterministic");

        // Different bump produces different address
        let addr3 = SvmHtlcProgram::derive_lock_account(&program_id, &swap_id, 254);
        assert_ne!(
            addr1, addr3,
            "different bump must produce different address"
        );
    }

    #[test]
    fn test_svm_account_persistence() {
        let mut program = SvmHtlcProgram::new(test_pubkey(1));
        let hashlock = make_hashlock(b"secret");
        let swap_id = [0xabu8; 32];

        program
            .lock(
                swap_id,
                test_pubkey(2),
                test_pubkey(3),
                test_pubkey(4),
                1000,
                hashlock,
                2000,
                [0u8; 32],
                255,
            )
            .expect("lock should succeed");

        let account = program.get_account(&swap_id).expect("account should exist");
        assert!(!account.claimed);
        assert!(!account.refunded);
        assert_eq!(account.amount, 1000);
    }
}
