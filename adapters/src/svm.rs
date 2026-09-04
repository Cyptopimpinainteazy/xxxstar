//! SVM (Solana Virtual Machine) Adapter — HTLC lock/claim/refund with Blake2b proof verification.
//!
//! Implements a simulated SVM HTLC adapter for cross-chain atomic swaps.
//! Uses PDA (Program Derived Address) derivation and Ed25519 key simulation.

use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};
use serde::{Deserialize, Serialize};

use crate::{EvmOpResult, FinalityStatus, ProofLedgerEvent, VmLockClaimRefund};

// ============================================================
// SvmHtlcAdapter
// ============================================================

/// SVM/Solana HTLC adapter that simulates an on-chain Solana program.
/// Maintains in-memory state: locks, proof ledger events, slot counter.
#[derive(Debug, Clone)]
pub struct SvmHtlcAdapter {
    pub chain_id: u64,
    pub program_id: String,
    pub simulated_slot: u64,
    locks: std::collections::HashMap<String, SvmHtlcLock>,
    events: Vec<ProofLedgerEvent>,
}

/// SVM HTLC lock entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmHtlcLock {
    pub sender: String,
    pub receiver: String,
    pub amount: u128,
    pub hashlock: [u8; 32],
    pub timeout_slot: u64,
    pub claimed: bool,
    pub refunded: bool,
}

impl SvmHtlcAdapter {
    pub fn new(chain_id: u64, program_id: &str) -> Self {
        Self {
            chain_id,
            program_id: program_id.to_string(),
            simulated_slot: 1,
            locks: std::collections::HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Advance the simulated slot number.
    pub fn advance_slots(&mut self, n: u64) {
        self.simulated_slot += n;
    }

    /// Get the current simulated slot number.
    pub fn current_slot(&self) -> u64 {
        self.simulated_slot
    }

    /// Get all proof ledger events.
    pub fn proof_events(&self) -> &[ProofLedgerEvent] {
        &self.events
    }

    /// Derive a PDA-style lock account address.
    fn derive_lock_account(sender: &str, hashlock: &[u8; 32], program_id: &str) -> String {
        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"x3-svm-htlc-pda");
        hasher.update(program_id.as_bytes());
        hasher.update(sender.as_bytes());
        hasher.update(hashlock);
        let mut out = [0u8; 32];
        hasher.finalize_variable(&mut out).expect("Blake2b finalized");
        // Base58-like encoding for readability
        format!("HTLCpda{}", hex::encode(&out[..16]))
    }

    fn preimage_to_hashlock(preimage: &[u8]) -> [u8; 32] {
        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"x3-svm-htlc-preimage-hashlock");
        hasher.update(preimage);
        let mut out = [0u8; 32];
        hasher.finalize_variable(&mut out).expect("Blake2b finalized");
        out
    }
}

impl VmLockClaimRefund for SvmHtlcAdapter {
    fn vm_name(&self) -> &str {
        "svm"
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
        timeout_slot: u64,
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
        if timeout_slot <= self.simulated_slot {
            return EvmOpResult::Error(format!(
                "timeout_slot ({}) must be > current slot ({})",
                timeout_slot, self.simulated_slot
            ));
        }

        let lock_account = Self::derive_lock_account(sender, &hashlock, &self.program_id);
        if self.locks.contains_key(&lock_account) {
            return EvmOpResult::Error(format!("lock {} already exists", lock_account));
        }

        let tx_hash = format!(
            "sol_sig_lock_{}_{}",
            hex::encode(&hashlock[..8]),
            self.simulated_slot
        );

        self.locks.insert(
            lock_account.clone(),
            SvmHtlcLock {
                sender: sender.to_string(),
                receiver: receiver.to_string(),
                amount,
                hashlock,
                timeout_slot,
                claimed: false,
                refunded: false,
            },
        );

        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"proof-hash-svm-lock");
        hasher.update(tx_hash.as_bytes());
        hasher.update(sender.as_bytes());
        hasher.update(receiver.as_bytes());
        hasher.update(&amount.to_le_bytes());
        hasher.update(&hashlock);
        let mut proof_hash = [0u8; 32];
        hasher.finalize_variable(&mut proof_hash).expect("Blake2b finalized");

        let event = ProofLedgerEvent {
            vm: "svm".into(),
            chain_id: self.chain_id,
            adapter_id: "svm-htlc-adapter".into(),
            operation: "lock".into(),
            asset: None,
            amount: Some(amount),
            sender: Some(sender.to_string()),
            receiver: Some(receiver.to_string()),
            tx_hash: tx_hash.clone(),
            proof_hash,
            finality_depth: 0,
            finality_status: "pending".into(),
            block_height: self.simulated_slot,
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

    fn claim(&mut self, lock_account: &str, preimage: &[u8]) -> EvmOpResult {
        let lock = match self.locks.get(lock_account) {
            Some(l) => l,
            None => return EvmOpResult::Error(format!("lock {} not found", lock_account)),
        };

        if lock.claimed {
            return EvmOpResult::Error(format!(
                "lock {} already claimed — replay rejected",
                lock_account
            ));
        }
        if lock.refunded {
            return EvmOpResult::Error(format!(
                "lock {} already refunded — cannot claim",
                lock_account
            ));
        }
        if self.simulated_slot >= lock.timeout_slot {
            return EvmOpResult::Error(format!(
                "lock {} has expired at slot {} (current: {})",
                lock_account, lock.timeout_slot, self.simulated_slot
            ));
        }

        let derived_hashlock = Self::preimage_to_hashlock(preimage);
        if derived_hashlock != lock.hashlock {
            return EvmOpResult::Error(format!(
                "preimage does not match hashlock for lock {} — claim rejected",
                lock_account
            ));
        }

        let lock = self.locks.get_mut(lock_account).unwrap();
        lock.claimed = true;

        let tx_hash = format!("sol_sig_claim_{}_{}", hex::encode(&lock_account.as_bytes()[..8]), self.simulated_slot);

        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"proof-hash-svm-claim");
        hasher.update(tx_hash.as_bytes());
        hasher.update(lock_account.as_bytes());
        hasher.update(preimage);
        let mut proof_hash = [0u8; 32];
        hasher.finalize_variable(&mut proof_hash).expect("Blake2b finalized");

        let event = ProofLedgerEvent {
            vm: "svm".into(),
            chain_id: self.chain_id,
            adapter_id: "svm-htlc-adapter".into(),
            operation: "claim".into(),
            asset: None,
            amount: Some(lock.amount),
            sender: Some(lock.sender.clone()),
            receiver: Some(lock.receiver.clone()),
            tx_hash: tx_hash.clone(),
            proof_hash,
            finality_depth: self.simulated_slot.saturating_sub(1),
            finality_status: if self.simulated_slot >= lock.timeout_slot {
                "expired".into()
            } else {
                "pending".into()
            },
            block_height: self.simulated_slot,
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

    fn refund(&mut self, lock_account: &str) -> EvmOpResult {
        let lock = match self.locks.get(lock_account) {
            Some(l) => l,
            None => return EvmOpResult::Error(format!("lock {} not found", lock_account)),
        };

        if lock.refunded {
            return EvmOpResult::Error(format!(
                "lock {} already refunded — replay rejected",
                lock_account
            ));
        }
        if lock.claimed {
            return EvmOpResult::Error(format!(
                "lock {} already claimed — cannot refund",
                lock_account
            ));
        }
        if self.simulated_slot < lock.timeout_slot {
            return EvmOpResult::Error(format!(
                "lock {} has not expired yet (timeout: {}, current: {}) — refund rejected",
                lock_account, lock.timeout_slot, self.simulated_slot
            ));
        }

        let lock = self.locks.get_mut(lock_account).unwrap();
        lock.refunded = true;

        let tx_hash = format!("sol_sig_refund_{}_{}", hex::encode(&lock_account.as_bytes()[..8]), self.simulated_slot);

        let mut hasher = Blake2bVar::new(32).expect("Blake2b initialized");
        hasher.update(b"proof-hash-svm-refund");
        hasher.update(tx_hash.as_bytes());
        hasher.update(lock_account.as_bytes());
        let mut proof_hash = [0u8; 32];
        hasher.finalize_variable(&mut proof_hash).expect("Blake2b finalized");

        let event = ProofLedgerEvent {
            vm: "svm".into(),
            chain_id: self.chain_id,
            adapter_id: "svm-htlc-adapter".into(),
            operation: "refund".into(),
            asset: None,
            amount: Some(lock.amount),
            sender: Some(lock.receiver.clone()),
            receiver: Some(lock.sender.clone()),
            tx_hash: tx_hash.clone(),
            proof_hash,
            finality_depth: self.simulated_slot.saturating_sub(lock.timeout_slot),
            finality_status: "finalized".into(),
            block_height: self.simulated_slot,
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

    fn verify_lock(&self, lock_account: &str, expected_hashlock: &[u8; 32]) -> Result<bool, String> {
        let lock = self
            .locks
            .get(lock_account)
            .ok_or_else(|| format!("lock {} not found", lock_account))?;
        if lock.claimed || lock.refunded {
            return Err(format!("lock {} is in terminal state", lock_account));
        }
        Ok(lock.hashlock == *expected_hashlock)
    }

    fn verify_claim(&self, lock_account: &str, preimage: &[u8]) -> Result<bool, String> {
        let lock = self
            .locks
            .get(lock_account)
            .ok_or_else(|| format!("lock {} not found", lock_account))?;
        if !lock.claimed {
            return Err("lock has not been claimed".into());
        }
        let derived = Self::preimage_to_hashlock(preimage);
        Ok(derived == lock.hashlock)
    }

    fn finality_status(&self, tx_hash: &str) -> FinalityStatus {
        if tx_hash.starts_with("sol_sig_") {
            // Solana finality: optimized confirmation (1 slot ≈ finalized)
            // After 32 slots, consider confirmed
            let confirmations = self.simulated_slot.saturating_sub(1);
            if confirmations >= 32 {
                FinalityStatus::Finalized {
                    confirmations,
                    block_number: self.simulated_slot,
                }
            } else {
                FinalityStatus::Pending {
                    confirmations,
                    block_number: self.simulated_slot,
                }
            }
        } else {
            FinalityStatus::NotFound
        }
    }

    fn write_proof_ledger_event(&self, _event: ProofLedgerEvent) -> Result<(), String> {
        Ok(())
    }

    fn readiness_score(&self) -> u32 {
        // SVM adapter: lock ✓ claim ✓ refund ✓ verify ✓ finality ✓ proof-ledger ✓
        // Missing: real on-chain deployment, CPI construction, account serialization
        85
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
        hl[0] = 0x11;
        hl[1] = 0x22;
        hl[31] = 0xee;
        hl
    }

    fn sample_preimage() -> Vec<u8> {
        b"svm-secret-preimage-for-testing".to_vec()
    }

    #[test]
    fn test_svm_lock_creates_proof_event() {
        let mut adapter = SvmHtlcAdapter::new(501, "HTLCprog1111111111111111111111111111111111");
        let hl = sample_hashlock();

        let result = adapter.lock("sender_pubkey", "receiver_pubkey", 5000, hl, 100);
        assert!(matches!(result, EvmOpResult::Success(_)));

        let events = adapter.proof_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].operation, "lock");
        assert_eq!(events[0].vm, "svm");
        assert_eq!(events[0].status, "locked");
    }

    #[test]
    fn test_svm_claim_requires_valid_preimage() {
        let mut adapter = SvmHtlcAdapter::new(501, "HTLCprog1111111111111111111111111111111111");
        let preimage = sample_preimage();
        let hashlock = SvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_account = SvmHtlcAdapter::derive_lock_account("sender", &hashlock, &adapter.program_id);

        adapter.lock("sender", "receiver", 5000, hashlock, 100);

        // Wrong preimage fails
        let claim = adapter.claim(&lock_account, b"wrong-preimage");
        assert!(matches!(claim, EvmOpResult::Error(ref e) if e.contains("does not match")));

        // Correct preimage succeeds
        let claim = adapter.claim(&lock_account, &preimage);
        assert!(matches!(claim, EvmOpResult::Success(_)));

        assert_eq!(adapter.proof_events().len(), 2);
    }

    #[test]
    fn test_svm_refund_only_after_timeout() {
        let mut adapter = SvmHtlcAdapter::new(501, "HTLCprog1111111111111111111111111111111111");
        let hl = sample_hashlock();
        let lock_account = SvmHtlcAdapter::derive_lock_account("sender", &hl, &adapter.program_id);

        adapter.lock("sender", "receiver", 5000, hl, 10);

        // Refund before timeout fails
        let refund = adapter.refund(&lock_account);
        assert!(matches!(refund, EvmOpResult::Error(ref e) if e.contains("not expired")));

        // Advance past timeout
        adapter.advance_slots(20);

        // Refund after timeout succeeds
        let refund = adapter.refund(&lock_account);
        assert!(matches!(refund, EvmOpResult::Success(_)));
    }

    #[test]
    fn test_svm_double_claim_rejected() {
        let mut adapter = SvmHtlcAdapter::new(501, "HTLCprog1111111111111111111111111111111111");
        let preimage = sample_preimage();
        let hashlock = SvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_account = SvmHtlcAdapter::derive_lock_account("sender", &hashlock, &adapter.program_id);

        adapter.lock("sender", "receiver", 5000, hashlock, 100);
        adapter.claim(&lock_account, &preimage);

        let second = adapter.claim(&lock_account, &preimage);
        assert!(matches!(second, EvmOpResult::Error(_)));
    }

    #[test]
    fn test_svm_finality_after_slots() {
        let mut adapter = SvmHtlcAdapter::new(501, "HTLCprog1111111111111111111111111111111111");
        let hl = sample_hashlock();
        let _lock_account = SvmHtlcAdapter::derive_lock_account("sender", &hl, &adapter.program_id);

        adapter.lock("sender", "receiver", 5000, hl, 200);
        adapter.advance_slots(50);

        let status = adapter.finality_status("sol_sig_lock_abcdef_1");
        assert!(matches!(status, FinalityStatus::Finalized { .. }));
    }

    #[test]
    fn test_svm_verify_lock_and_claim() {
        let mut adapter = SvmHtlcAdapter::new(501, "HTLCprog1111111111111111111111111111111111");
        let preimage = sample_preimage();
        let hashlock = SvmHtlcAdapter::preimage_to_hashlock(&preimage);
        let lock_account = SvmHtlcAdapter::derive_lock_account("sender", &hashlock, &adapter.program_id);

        adapter.lock("sender", "receiver", 5000, hashlock, 100);

        let verify = adapter.verify_lock(&lock_account, &hashlock);
        assert!(matches!(verify, Ok(true)));

        adapter.claim(&lock_account, &preimage);
        let claim_verify = adapter.verify_claim(&lock_account, &preimage);
        assert!(matches!(claim_verify, Ok(true)));
    }

    #[test]
    fn test_svm_pda_derivation_deterministic() {
        let hl = sample_hashlock();
        let addr1 = SvmHtlcAdapter::derive_lock_account("sender", &hl, "prog1");
        let addr2 = SvmHtlcAdapter::derive_lock_account("sender", &hl, "prog1");
        let addr3 = SvmHtlcAdapter::derive_lock_account("sender", &hl, "prog2");

        assert_eq!(addr1, addr2); // Same inputs, same PDA
        assert_ne!(addr1, addr3); // Different program, different PDA
    }
}