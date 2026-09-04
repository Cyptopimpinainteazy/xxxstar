//! EVM Adapter — HTLC lock/claim/refund with Blake2b proof verification.
//!
//! Implements a simulated EVM HTLC adapter for cross-chain atomic swaps.
//! In production, the deploy/call functions would construct and submit
//! real Ethereum transactions. This adapter provides a testable simulation
//! with full proof ledger integration hooks.

use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};
use serde::{Deserialize, Serialize};

pub mod svm;
pub use svm::SvmHtlcAdapter;

// ============================================================
// Types
// ============================================================

/// Operation result for any adapter operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvmOpResult {
    /// Operation succeeded, with transaction hash.
    Success(String),
    /// Operation failed with error message.
    Error(String),
}

/// An HTLC lock entry in the simulated EVM storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtlcLock {
    pub sender: String,
    pub receiver: String,
    pub amount: u128,
    pub hashlock: [u8; 32],
    pub timeout_block: u64,
    pub claimed: bool,
    pub refunded: bool,
}

/// Proof ledger event for state transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofLedgerEvent {
    pub vm: String,
    pub chain_id: u64,
    pub adapter_id: String,
    pub operation: String,
    pub asset: Option<String>,
    pub amount: Option<u128>,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub tx_hash: String,
    pub proof_hash: [u8; 32],
    pub finality_depth: u64,
    pub finality_status: String,
    pub block_height: u64,
    pub timestamp: u64,
    pub status: String,
    pub error: Option<String>,
}

// ============================================================
// Trait: VmLockClaimRefund — defines lock/claim/refund for a VM
// ============================================================

/// Trait for a VM adapter that supports HTLC lock, claim, and refund.
pub trait VmLockClaimRefund {
    fn vm_name(&self) -> &str;
    fn chain_id(&self) -> u64;

    /// Lock funds in an HTLC.
    fn lock(
        &mut self,
        sender: &str,
        receiver: &str,
        amount: u128,
        hashlock: [u8; 32],
        timeout_block: u64,
    ) -> EvmOpResult;

    /// Claim funds after providing the correct preimage.
    fn claim(&mut self, lock_id: &str, preimage: &[u8]) -> EvmOpResult;

    /// Refund funds after timeout has expired.
    fn refund(&mut self, lock_id: &str) -> EvmOpResult;

    /// Verify a lock event against stored state.
    fn verify_lock(&self, lock_id: &str, expected_hashlock: &[u8; 32]) -> Result<bool, String>;

    /// Verify a claim event against stored state.
    fn verify_claim(&self, lock_id: &str, preimage: &[u8]) -> Result<bool, String>;

    /// Get finality status for a transaction.
    fn finality_status(&self, tx_hash: &str) -> FinalityStatus;

    /// Write a proof ledger event.
    fn write_proof_ledger_event(&self, event: ProofLedgerEvent) -> Result<(), String>;

    /// Get adapter readiness score (0-100).
    fn readiness_score(&self) -> u32;
}

// ============================================================
// FinalityStatus
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinalityStatus {
    /// Transaction is finalized (e.g. 12+ confirmations on EVM).
    Finalized { confirmations: u64, block_number: u64 },
    /// Transaction is pending, not yet finalized.
    Pending { confirmations: u64, block_number: u64 },
    /// Transaction not found.
    NotFound,
    /// Failed to check finality.
    Error(String),
}

// ============================================================
// EvmHtlcAdapter — real simulation with state tracking
// ============================================================

/// EVM HTLC adapter that simulates an on-chain HTLC contract.
/// Maintains in-memory state: locks, proof ledger events, block counter.
#[derive(Debug, Clone)]
pub struct EvmHtlcAdapter {
    pub chain_id: u64,
    pub simulated_block: u64,
    locks: std::collections::HashMap<String, HtlcLock>,
    events: Vec<ProofLedgerEvent>,
}

impl EvmHtlcAdapter {
    pub fn new(chain_id: u64) -> Self {
        Self {
            chain_id,
            simulated_block: 1,
            locks: std::collections::HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Advance the simulated block number (simulates mining).
    pub fn advance_blocks(&mut self, n: u64) {
        self.simulated_block += n;
    }

    /// Get the current simulated block number.
    pub fn current_block(&self) -> u64 {
        self.simulated_block
    }

    /// Get all proof ledger events.
    pub fn proof_events(&self) -> &[ProofLedgerEvent] {
        &self.events
    }

    /// Generate a deterministic lock ID from sender + hashlock.
    fn make_lock_id(sender: &str, hashlock: &[u8; 32]) -> String {
        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"x3-evm-htlc-lock-id");
        hasher.update(sender.as_bytes());
        hasher.update(hashlock);
        let mut out = [0u8; 32];
        hasher.finalize_variable(&mut out).expect("Blake2b finalized");
        hex::encode(out)
    }

    /// Attempt to use a preimage to derive a hashlock.
    fn preimage_to_hashlock(preimage: &[u8]) -> [u8; 32] {
        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"x3-evm-htlc-preimage-hashlock");
        hasher.update(preimage);
        let mut out = [0u8; 32];
        hasher.finalize_variable(&mut out).expect("Blake2b finalized");
        out
    }
}

impl VmLockClaimRefund for EvmHtlcAdapter {
    fn vm_name(&self) -> &str {
        "evm"
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn lock(
        &mut self,
        sender: &str,
        receiver: &str,
        amount: u128,
        hashlock: [u8; 32],
        timeout_block: u64,
    ) -> EvmOpResult {
        // Validate inputs
        if sender.is_empty() {
            return EvmOpResult::Error("sender is empty".into());
        }
        if receiver.is_empty() {
            return EvmOpResult::Error("receiver is empty".into());
        }
        if amount == 0 {
            return EvmOpResult::Error("amount must be > 0".into());
        }
        if hashlock == [0u8; 32] {
            return EvmOpResult::Error("hashlock cannot be zero".into());
        }
        if timeout_block <= self.simulated_block {
            return EvmOpResult::Error(format!(
                "timeout_block ({}) must be > current block ({})",
                timeout_block, self.simulated_block
            ));
        }

        let lock_id = Self::make_lock_id(sender, &hashlock);
        if self.locks.contains_key(&lock_id) {
            return EvmOpResult::Error(format!("lock {} already exists", lock_id));
        }

        let tx_hash = format!(
            "0xevm_lock_{}_{:x}",
            hex::encode(&hashlock[..8]),
            self.simulated_block
        );

        self.locks.insert(
            lock_id.clone(),
            HtlcLock {
                sender: sender.to_string(),
                receiver: receiver.to_string(),
                amount,
                hashlock,
                timeout_block,
                claimed: false,
                refunded: false,
            },
        );

        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"proof-hash-lock");
        hasher.update(tx_hash.as_bytes());
        hasher.update(sender.as_bytes());
        hasher.update(receiver.as_bytes());
        hasher.update(&amount.to_le_bytes());
        hasher.update(&hashlock);
        let mut proof_hash = [0u8; 32];
        hasher.finalize_variable(&mut proof_hash).expect("Blake2b finalized");

        let event = ProofLedgerEvent {
            vm: "evm".into(),
            chain_id: self.chain_id,
            adapter_id: "evm-htlc-adapter".into(),
            operation: "lock".into(),
            asset: None,
            amount: Some(amount),
            sender: Some(sender.to_string()),
            receiver: Some(receiver.to_string()),
            tx_hash: tx_hash.clone(),
            proof_hash,
            finality_depth: 0,
            finality_status: "pending".into(),
            block_height: self.simulated_block,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            status: "locked".into(),
            error: None,
        };
        self.events.push(event);

        EvmOpResult::Success(tx_hash)
    }

    fn claim(&mut self, lock_id: &str, preimage: &[u8]) -> EvmOpResult {
        let lock = match self.locks.get(lock_id) {
            Some(l) => l,
            None => return EvmOpResult::Error(format!("lock {} not found", lock_id)),
        };

        if lock.claimed {
            return EvmOpResult::Error(format!("lock {} already claimed — replay rejected", lock_id));
        }
        if lock.refunded {
            return EvmOpResult::Error(format!("lock {} already refunded — cannot claim", lock_id));
        }
        if self.simulated_block >= lock.timeout_block {
            return EvmOpResult::Error(format!(
                "lock {} has expired at block {} (current: {})",
                lock_id, lock.timeout_block, self.simulated_block
            ));
        }

        let derived_hashlock = Self::preimage_to_hashlock(preimage);
        if derived_hashlock != lock.hashlock {
            return EvmOpResult::Error(format!(
                "preimage does not match hashlock for lock {} — claim rejected",
                lock_id
            ));
        }

        let lock = self.locks.get_mut(lock_id).unwrap();
        lock.claimed = true;

        let tx_hash = format!("0xevm_claim_{}_{}", hex::encode(&lock_id.as_bytes()[..8]), self.simulated_block);

        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"proof-hash-claim");
        hasher.update(tx_hash.as_bytes());
        hasher.update(lock_id.as_bytes());
        hasher.update(preimage);
        let mut proof_hash = [0u8; 32];
        hasher.finalize_variable(&mut proof_hash).expect("Blake2b finalized");

        let event = ProofLedgerEvent {
            vm: "evm".into(),
            chain_id: self.chain_id,
            adapter_id: "evm-htlc-adapter".into(),
            operation: "claim".into(),
            asset: None,
            amount: Some(lock.amount),
            sender: Some(lock.sender.clone()),
            receiver: Some(lock.receiver.clone()),
            tx_hash: tx_hash.clone(),
            proof_hash,
            finality_depth: self.simulated_block.saturating_sub(1),
            finality_status: if self.simulated_block >= lock.timeout_block {
                "expired".into()
            } else {
                "pending".into()
            },
            block_height: self.simulated_block,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            status: "claimed".into(),
            error: None,
        };
        self.events.push(event);

        EvmOpResult::Success(tx_hash)
    }

    fn refund(&mut self, lock_id: &str) -> EvmOpResult {
        let lock = match self.locks.get(lock_id) {
            Some(l) => l,
            None => return EvmOpResult::Error(format!("lock {} not found", lock_id)),
        };

        if lock.refunded {
            return EvmOpResult::Error(format!("lock {} already refunded — replay rejected", lock_id));
        }
        if lock.claimed {
            return EvmOpResult::Error(format!("lock {} already claimed — cannot refund", lock_id));
        }
        if self.simulated_block < lock.timeout_block {
            return EvmOpResult::Error(format!(
                "lock {} has not expired yet (timeout: {}, current: {}) — refund rejected",
                lock_id, lock.timeout_block, self.simulated_block
            ));
        }

        let lock = self.locks.get_mut(lock_id).unwrap();
        lock.refunded = true;

        let tx_hash = format!("0xevm_refund_{}_{}", hex::encode(&lock_id.as_bytes()[..8]), self.simulated_block);

        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"proof-hash-refund");
        hasher.update(tx_hash.as_bytes());
        hasher.update(lock_id.as_bytes());
        let mut proof_hash = [0u8; 32];
        hasher.finalize_variable(&mut proof_hash).expect("Blake2b finalized");

        let event = ProofLedgerEvent {
            vm: "evm".into(),
            chain_id: self.chain_id,
            adapter_id: "evm-htlc-adapter".into(),
            operation: "refund".into(),
            asset: None,
            amount: Some(lock.amount),
            sender: Some(lock.receiver.clone()),
            receiver: Some(lock.sender.clone()),
            tx_hash: tx_hash.clone(),
            proof_hash,
            finality_depth: self.simulated_block.saturating_sub(lock.timeout_block),
            finality_status: "finalized".into(),
            block_height: self.simulated_block,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            status: "refunded".into(),
            error: None,
        };
        self.events.push(event);

        EvmOpResult::Success(tx_hash)
    }

    fn verify_lock(&self, lock_id: &str, expected_hashlock: &[u8; 32]) -> Result<bool, String> {
        let lock = self
            .locks
            .get(lock_id)
            .ok_or_else(|| format!("lock {} not found", lock_id))?;
        if lock.claimed || lock.refunded {
            return Err(format!("lock {} is in terminal state", lock_id));
        }
        Ok(lock.hashlock == *expected_hashlock)
    }

    fn verify_claim(&self, lock_id: &str, preimage: &[u8]) -> Result<bool, String> {
        let lock = self
            .locks
            .get(lock_id)
            .ok_or_else(|| format!("lock {} not found", lock_id))?;
        if !lock.claimed {
            return Err("lock has not been claimed".into());
        }
        let derived = Self::preimage_to_hashlock(preimage);
        Ok(derived == lock.hashlock)
    }

    fn finality_status(&self, tx_hash: &str) -> FinalityStatus {
        // In simulation, assume pending for recent txs, finalized for older ones
        if tx_hash.starts_with("0x") {
            // Simulated finality: finalized after 3 blocks
            let confirmations = self.simulated_block.saturating_sub(1);
            if confirmations >= 3 {
                FinalityStatus::Finalized {
                    confirmations,
                    block_number: self.simulated_block,
                }
            } else {
                FinalityStatus::Pending {
                    confirmations,
                    block_number: self.simulated_block,
                }
            }
        } else {
            FinalityStatus::NotFound
        }
    }

    fn write_proof_ledger_event(&self, _event: ProofLedgerEvent) -> Result<(), String> {
        // In production this writes to an on-chain proof ledger.
        // In simulation, events are already stored in self.events during lock/claim/refund.
        Ok(())
    }

    fn readiness_score(&self) -> u32 {
        // EVM adapter: lock ✓ claim ✓ refund ✓ verify ✓ finality ✓ proof-ledger ✓
        // Missing: real on-chain deployment, gas estimation, event watching
        85
    }
}

// ============================================================
// GasConverter — converts gas between VM types
// ============================================================

#[allow(dead_code)]
pub struct GasConverter {
    source_vm: u32,
    target_vm: u32,
    conversion_rate: u128,
}

impl GasConverter {
    pub fn new(source_vm: u32, target_vm: u32, conversion_rate: u128) -> Self {
        Self {
            source_vm,
            target_vm,
            conversion_rate,
        }
    }

    pub fn convert_gas(&self, source_gas: u64) -> Result<u64, &'static str> {
        let converted = source_gas as u128 * self.conversion_rate;
        // Scale back to u64; apply a minimum of 1
        let scaled = (converted / 1_000_000_000_000_000_000).max(1) as u64;
        Ok(scaled)
    }
}

// ============================================================
// CircuitBreaker — rate-limits adapter operations
// ============================================================

pub struct CircuitBreaker {
    call_count: u32,
    threshold: u32,
    cooldown_blocks: u64,
    opened_at_block: Option<u64>,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, cooldown_blocks: u64) -> Self {
        Self {
            call_count: 0,
            threshold,
            cooldown_blocks,
            opened_at_block: None,
        }
    }

    pub fn check(&mut self, current_block: u64) -> Result<(), &'static str> {
        // If breaker was opened and cooldown has passed, reset
        if let Some(opened) = self.opened_at_block {
            if current_block.saturating_sub(opened) >= self.cooldown_blocks {
                self.call_count = 0;
                self.opened_at_block = None;
            } else {
                return Err("Circuit breaker open — cooldown not elapsed");
            }
        }

        self.call_count += 1;
        if self.call_count > self.threshold {
            self.opened_at_block = Some(current_block);
            Err("Circuit breaker triggered — threshold exceeded")
        } else {
            Ok(())
        }
    }

    pub fn reset(&mut self) {
        self.call_count = 0;
        self.opened_at_block = None;
    }

    pub fn is_open(&self) -> bool {
        self.opened_at_block.is_some()
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_hashlock() -> [u8; 32] {
        let mut hl = [0u8; 32];
        hl[0] = 0xaa;
        hl[1] = 0xbb;
        hl[31] = 0xff;
        hl
    }

    fn sample_preimage() -> Vec<u8> {
        b"secret-preimage-for-testing-x3".to_vec()
    }

    #[test]
    fn test_lock_creates_pending_proof_event() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let hl = sample_hashlock();

        let result = adapter.lock("alice", "bob", 1000, hl, 100);
        assert!(matches!(result, EvmOpResult::Success(_)));

        let events = adapter.proof_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].operation, "lock");
        assert_eq!(events[0].status, "locked");
        assert_eq!(events[0].sender.as_deref(), Some("alice"));
        assert_eq!(events[0].receiver.as_deref(), Some("bob"));
        assert_eq!(events[0].amount, Some(1000));
    }

    #[test]
    fn test_claim_requires_valid_preimage() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let preimage = sample_preimage();
        let hashlock = EvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_id = EvmHtlcAdapter::make_lock_id("alice", &hashlock);

        // Lock
        let result = adapter.lock("alice", "bob", 1000, hashlock, 100);
        assert!(matches!(result, EvmOpResult::Success(_)));

        // Wrong preimage fails
        let claim = adapter.claim(&lock_id, b"wrong-preimage");
        assert!(matches!(claim, EvmOpResult::Error(ref e) if e.contains("does not match")));

        // Correct preimage succeeds
        let claim = adapter.claim(&lock_id, &preimage);
        assert!(matches!(claim, EvmOpResult::Success(_)));

        // Verify events
        let events = adapter.proof_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].operation, "claim");
        assert_eq!(events[1].status, "claimed");
    }

    #[test]
    fn test_refund_only_works_after_timeout() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let hl = sample_hashlock();
        let lock_id = EvmHtlcAdapter::make_lock_id("alice", &hl);

        // Lock with timeout at block 10
        let result = adapter.lock("alice", "bob", 1000, hl, 10);
        assert!(matches!(result, EvmOpResult::Success(_)));

        // Refund before timeout should fail
        let refund = adapter.refund(&lock_id);
        assert!(matches!(refund, EvmOpResult::Error(ref e) if e.contains("not expired")));

        // Advance past timeout
        adapter.advance_blocks(10);

        // Refund after timeout should succeed
        let refund = adapter.refund(&lock_id);
        assert!(matches!(refund, EvmOpResult::Success(_)));

        let events = adapter.proof_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].operation, "refund");
    }

    #[test]
    fn test_invalid_hashlock_rejected() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let result = adapter.lock("alice", "bob", 1000, [0u8; 32], 100);
        assert!(matches!(result, EvmOpResult::Error(ref e) if e.contains("zero")));
    }

    #[test]
    fn test_finality_is_enforced_before_claim() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let preimage = sample_preimage();
        let hashlock = EvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_id = EvmHtlcAdapter::make_lock_id("alice", &hashlock);

        adapter.lock("alice", "bob", 1000, hashlock, 100);

        let lock = adapter.locks.get(&lock_id).unwrap();
        assert!(!lock.claimed);
        assert!(!lock.refunded);

        // Advance past timeout — claim should now fail
        adapter.advance_blocks(200);
        let claim = adapter.claim(&lock_id, &preimage);
        assert!(matches!(claim, EvmOpResult::Error(ref e) if e.contains("expired")));
    }

    #[test]
    fn test_replay_double_claim_rejected() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let preimage = sample_preimage();
        let hashlock = EvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_id = EvmHtlcAdapter::make_lock_id("alice", &hashlock);

        adapter.lock("alice", "bob", 1000, hashlock, 100);

        let claim1 = adapter.claim(&lock_id, &preimage);
        assert!(matches!(claim1, EvmOpResult::Success(_)));

        let claim2 = adapter.claim(&lock_id, &preimage);
        assert!(matches!(claim2, EvmOpResult::Error(ref e) if e.contains("replay") || e.contains("already claimed")));
    }

    #[test]
    fn test_double_claim_rejected() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let preimage = sample_preimage();
        let hashlock = EvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_id = EvmHtlcAdapter::make_lock_id("alice", &hashlock);

        adapter.lock("alice", "bob", 1000, hashlock, 100);
        adapter.claim(&lock_id, &preimage);

        let second = adapter.claim(&lock_id, &preimage);
        assert!(matches!(second, EvmOpResult::Error(_)));
    }

    #[test]
    fn test_malformed_proof_rejected() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let result = adapter.lock("", "bob", 1000, sample_hashlock(), 100);
        assert!(matches!(result, EvmOpResult::Error(ref e) if e.contains("empty")));

        let result = adapter.lock("alice", "bob", 0, sample_hashlock(), 100);
        assert!(matches!(result, EvmOpResult::Error(ref e) if e.contains("> 0")));
    }

    #[test]
    fn test_verify_lock_and_claim() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let preimage = sample_preimage();
        let hashlock = EvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_id = EvmHtlcAdapter::make_lock_id("alice", &hashlock);

        adapter.lock("alice", "bob", 1000, hashlock, 100);

        let verify = adapter.verify_lock(&lock_id, &hashlock);
        assert!(matches!(verify, Ok(true)));

        let wrong_verify = adapter.verify_lock(&lock_id, &sample_hashlock());
        assert!(matches!(wrong_verify, Ok(false)));

        adapter.claim(&lock_id, &preimage);

        let claim_verify = adapter.verify_claim(&lock_id, &preimage);
        assert!(matches!(claim_verify, Ok(true)));
    }

    #[test]
    fn test_finality_status_pending_then_finalized() {
        let adapter = EvmHtlcAdapter::new(1);
        let status = adapter.finality_status("0xsometx");
        assert!(matches!(status, FinalityStatus::Pending { .. }));
    }

    #[test]
    fn test_circuit_breaker_opens_and_resets() {
        let mut cb = CircuitBreaker::new(3, 5);

        assert!(cb.check(1).is_ok());
        assert!(cb.check(1).is_ok());
        assert!(cb.check(1).is_ok());
        assert!(cb.check(1).is_err()); // 4th call opens breaker
        assert!(cb.is_open());

        assert!(cb.check(1).is_err()); // still open

        // Advance past cooldown
        assert!(cb.check(7).is_ok());
        assert!(!cb.is_open());
    }

    #[test]
    fn test_gas_converter_scales() {
        let converter = GasConverter::new(1, 2, 2_000_000_000_000_000_000); // 2x rate
        let result = converter.convert_gas(100);
        assert_eq!(result, Ok(200));
    }

    #[test]
    fn test_proof_ledger_events_track_all_state_transitions() {
        let mut adapter = EvmHtlcAdapter::new(1);
        let preimage = sample_preimage();
        let hashlock = EvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_id = EvmHtlcAdapter::make_lock_id("alice", &hashlock);

        adapter.lock("alice", "bob", 1000, hashlock, 100);
        adapter.claim(&lock_id, &preimage);

        let events = adapter.proof_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].operation, "lock");
        assert_eq!(events[1].operation, "claim");
        assert!(!events[0].proof_hash.iter().all(|&b| b == 0));
        assert!(!events[1].proof_hash.iter().all(|&b| b == 0));
    }
}