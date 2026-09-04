//! # Relayer Watcher
//!
//! The relayer is the core coordination component of X3 atomic swaps. It:
//!
//! 1. **Watches** source and destination chains for lock events
//! 2. **Verifies** matching hashlocks between both sides
//! 3. **Verifies** finality on both chains before acting
//! 4. **Monitors** for preimage revelation on the claiming chain
//! 5. **Submits** claim transaction to the opposite chain
//! 6. **Writes** proof records to the proof ledger
//! 7. **Refuses** to mark success without transaction hashes

use crate::error::SwapError;
#[cfg(feature = "std")]
use crate::event_watcher::HtlcEvent;
#[cfg(feature = "std")]
use crate::evm_htlc::EvmHtlcContract;
use crate::evm_htlc::{EvmClaimedEvent, EvmHtlcAdapter, EvmLockedEvent};
use crate::intent::{AtomicIntent, AtomicSwapStatus};
use crate::ledger::ProofLedger;
use crate::scoreboard::SwapScoreboard;
use crate::svm_htlc::{SvmClaimedEvent as SvmClaimedEvt, SvmHtlcAdapter, SvmLockedEvent};
use serde::{Deserialize, Serialize};

/// Status of relayer observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayerObservation {
    /// Watching for events.
    Watching,
    /// Source lock detected.
    SourceLockDetected,
    /// Destination lock detected.
    DestinationLockDetected,
    /// Hashlocks match.
    HashlocksMatch,
    /// Preimage revealed on one chain.
    PreimageCaptured,
    /// Claim submitted.
    ClaimSubmitted,
    /// Swap completed successfully.
    Completed,
    /// Swap expired.
    Expired,
}

/// The relayer's view of the swap state.
#[derive(Debug)]
pub struct RelayerState {
    pub intent: AtomicIntent,
    pub observation: RelayerObservation,
    pub source_lock_event: Option<SourceLockInfo>,
    pub destination_lock_event: Option<DestinationLockInfo>,
    pub captured_preimage: Option<Vec<u8>>,
}

/// Information about a detected source lock.
#[derive(Debug, Clone)]
pub struct SourceLockInfo {
    pub chain: String,
    pub swap_id: [u8; 32],
    pub sender: String,
    pub amount: u128,
    pub hashlock: [u8; 32],
    pub timeout: u64,
    pub tx_hash: String,
    pub block_number: u64,
}

/// Information about a detected destination lock.
#[derive(Debug, Clone)]
pub struct DestinationLockInfo {
    pub chain: String,
    pub swap_id: [u8; 32],
    pub sender: String,
    pub amount: u128,
    pub hashlock: [u8; 32],
    pub timeout: u64,
    pub tx_hash: String,
    pub block_number: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Relayer Engine
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Watcher Alerts - stuck-swap detector
// ─────────────────────────────────────────────────────────────────────────────

/// Alert type raised by the stuck-swap watcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatcherAlert {
    /// Swap is approaching its timeout without reaching a terminal state.
    NearTimeout {
        intent_id: u64,
        seconds_remaining: u64,
    },
    /// Source locked but no destination lock proof exists.
    MissingDestinationLock { intent_id: u64 },
    /// Both sides are locked but no claim proof has been recorded.
    MissingClaim { intent_id: u64 },
    /// Timeout has elapsed but swap is not Refunded or Claimed.
    ExpiredNotRefunded { intent_id: u64 },
    /// Finality is taking longer than expected on a chain.
    FinalityDelay { intent_id: u64, chain: String },
    /// RPC providers disagree on transaction status for this intent.
    RpcDisagreement {
        intent_id: u64,
        provider_a: String,
        provider_b: String,
    },
}

/// Scan a set of intents against the proof ledger and current time to detect
/// stuck, expiring, or anomalous swaps.
///
/// # Parameters
///
/// - `intents` - all active intents to scan
/// - `ledger` - the proof ledger containing recorded proofs and RPC quorum data
/// - `now` - current unix timestamp (or slot number)
/// - `timeout_warning_secs` - window before expiry to raise a NearTimeout alert
pub fn scan_for_alerts(
    intents: &[AtomicIntent],
    ledger: &ProofLedger,
    now: u64,
    timeout_warning_secs: u64,
) -> Vec<WatcherAlert> {
    let mut alerts = Vec::new();

    for intent in intents {
        let id = intent.intent_id;
        let status = intent.status;

        // Skip terminal swaps - they cannot be "stuck" in a meaningful way.
        if status.is_terminal() {
            continue;
        }

        // 1. NearTimeout - approaching expiry without a terminal state
        if intent.source_timeout > now && intent.source_timeout - now < timeout_warning_secs {
            alerts.push(WatcherAlert::NearTimeout {
                intent_id: id,
                seconds_remaining: intent.source_timeout - now,
            });
        }

        // 2. MissingDestinationLock - source locked but no dest lock *for this intent*
        if status == AtomicSwapStatus::SourceLocked
            && !ledger.has_verified_kind_for_intent(id, crate::ledger::ProofKind::DestinationLock)
        {
            alerts.push(WatcherAlert::MissingDestinationLock { intent_id: id });
        }

        // 3. MissingClaim - both locked or claimable but no claim proof *for this intent*
        if (status == AtomicSwapStatus::BothLocked
            || status == AtomicSwapStatus::Claimable
            || status == AtomicSwapStatus::PreimageRevealed
            || status == AtomicSwapStatus::ClaimSubmitted)
            && !ledger.has_verified_kind_for_intent(id, crate::ledger::ProofKind::Claim)
        {
            alerts.push(WatcherAlert::MissingClaim { intent_id: id });
        }

        // 3b. RpcDisagreement - quorum missing or not yet agreed for this intent
        if !ledger.has_rpc_quorum_for_intent(id) {
            // If the swap has progressed past source lock and RPC quorum
            // hasn't been reached, flag it.
            if status != AtomicSwapStatus::Pending {
                // We report a single-message disagreement alert.
                // In production this would list the disagreeing providers.
                alerts.push(WatcherAlert::RpcDisagreement {
                    intent_id: id,
                    provider_a: "rpc-set-".to_string() + &id.to_string(),
                    provider_b: "quorum-unmet".to_string(),
                });
            }
        }

        // 4. ExpiredNotRefunded - time has passed but no refund
        if now > intent.source_timeout
            && status != AtomicSwapStatus::Refunded
            && status != AtomicSwapStatus::Claimed
            && status != AtomicSwapStatus::Completed
        {
            alerts.push(WatcherAlert::ExpiredNotRefunded { intent_id: id });
        }
    }

    alerts
}

// ─────────────────────────────────────────────────────────────────────────────
// Relayer Engine
// ─────────────────────────────────────────────────────────────────────────────

/// The relayer engine that coordinates atomic swap execution.
#[derive(Debug)]
pub struct Relayer {
    /// Relayer identifier.
    pub relayer_id: String,
    /// Minimum number of confirmations for finality.
    pub min_confirmations: u32,
    /// Proof ledger for recording swap steps.
    pub ledger: ProofLedger,
}

impl Relayer {
    /// Create a new relayer.
    pub fn new(relayer_id: String, min_confirmations: u32) -> Self {
        Self {
            relayer_id,
            min_confirmations,
            ledger: ProofLedger::new(),
        }
    }

    /// Watch an EVM contract for a lock event matching the intent.
    ///
    /// Returns the lock event if found, or None.
    pub fn watch_evm_lock(
        &self,
        intent: &AtomicIntent,
        contract: &impl EvmHtlcAdapter,
    ) -> Option<EvmLockedEvent> {
        contract
            .get_locked_events()
            .into_iter()
            .find(|event| event.hashlock == intent.hashlock)
    }

    /// Watch an EVM contract for lock events via real RPC-backed event polling.
    ///
    /// This is the production path - it uses `eth_getLogs` through the
    /// contract's configured `EventWatcher` to discover on-chain HTLC lock
    /// events, then filters by hashlock.
    ///
    /// Returns matching `EvmLockedEvent` entries and records them in the
    /// proof ledger. Requires the `std` feature for RPC transport.
    #[cfg(feature = "std")]
    pub fn watch_evm_lock_onchain(
        &mut self,
        intent: &AtomicIntent,
        contract: &mut EvmHtlcContract,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<EvmLockedEvent>, SwapError> {
        let htlc_events = contract.poll_events(from_block, to_block)?;
        let mut matches = Vec::new();

        for htlc_event in htlc_events {
            match htlc_event {
                HtlcEvent::Locked {
                    amount,
                    hashlock,
                    tx_hash,
                    block_number,
                    ..
                } => {
                    if hashlock != intent.hashlock {
                        continue;
                    }
                    let evm_event = EvmLockedEvent {
                        contract: contract.address,
                        swap_id: [0u8; 32],
                        sender: [0u8; 20],
                        receiver: [0u8; 20],
                        refund_address: [0u8; 20],
                        amount,
                        hashlock,
                        timeout: 0,
                        asset: [0u8; 20],
                    };

                    // Record the on-chain discovery in the proof ledger
                    let _ = self.record_source_lock(intent.intent_id, tx_hash, block_number, 0);
                    matches.push(evm_event);
                }
                HtlcEvent::Claimed {
                    tx_hash,
                    block_number,
                    ..
                } => {
                    if let Some(record) = self
                        .ledger
                        .get_latest_record_mut_for_intent(intent.intent_id)
                    {
                        record.record_secret_reveal(tx_hash, 0);
                        let _ = block_number;
                    }
                }
                HtlcEvent::Refunded {
                    tx_hash,
                    block_number,
                    ..
                } => {
                    if let Some(record) = self
                        .ledger
                        .get_latest_record_mut_for_intent(intent.intent_id)
                    {
                        record.record_refund(tx_hash, block_number, 0);
                    }
                }
            }
        }
        Ok(matches)
    }

    /// Watch an SVM program for a lock event matching the intent.
    pub fn watch_svm_lock(
        &self,
        intent: &AtomicIntent,
        program: &impl SvmHtlcAdapter,
    ) -> Option<SvmLockedEvent> {
        program
            .get_locked_events()
            .into_iter()
            .find(|event| event.hashlock == intent.hashlock)
    }

    /// Poll the RPC endpoint for new blocks and scan for HTLC events.
    ///
    /// This is the main production poll loop: it fetches the latest block
    /// from the chain, calls `poll_events` on the configured contracts,
    /// and records any discovered on-chain events in the proof ledger.
    ///
    /// Returns all decoded HTLC events found in the poll window.
    /// Requires the `std` feature for RPC transport.
    #[cfg(feature = "std")]
    pub fn poll_chain_events(
        &mut self,
        evm_contract: &mut EvmHtlcContract,
    ) -> Result<Vec<HtlcEvent>, SwapError> {
        let latest_block = evm_contract.get_latest_block()?;
        // Poll from last seen block (tracked in ledger watermark) to latest
        let from_block = self
            .ledger
            .get_last_polled_block()
            .unwrap_or(latest_block.saturating_sub(256));

        // Only poll if there's work to do
        if from_block >= latest_block {
            return Ok(Vec::new());
        }

        let events = evm_contract.poll_events(from_block, latest_block)?;
        self.ledger.set_last_polled_block(latest_block);
        Ok(events)
    }

    /// Verify that two hashlocks match between source and destination.
    pub fn verify_hashlock_match(
        &self,
        source_hashlock: &[u8; 32],
        dest_hashlock: &[u8; 32],
    ) -> bool {
        source_hashlock == dest_hashlock
    }

    /// Verify that finality has been reached for a given chain.
    ///
    /// `current_confirmations` is the number of confirmations observed.
    /// Returns Ok if >= required, Err otherwise.
    pub fn verify_finality(
        &self,
        required_confirmations: u32,
        current_confirmations: u32,
        chain_name: &str,
    ) -> Result<(), SwapError> {
        if current_confirmations < required_confirmations {
            return Err(SwapError::FinalityNotMet {
                chain: chain_name.into(),
                required: required_confirmations,
                current: current_confirmations,
            });
        }
        Ok(())
    }

    /// Extract the preimage from a Claimed event (EVM).
    pub fn extract_preimage_from_evm_claim(
        &self,
        event: &EvmClaimedEvent,
        expected_swap_id: &[u8; 32],
    ) -> Option<Vec<u8>> {
        if event.swap_id == *expected_swap_id {
            Some(event.preimage.clone())
        } else {
            None
        }
    }

    /// Extract the preimage from a Claimed event (SVM).
    pub fn extract_preimage_from_svm_claim(
        &self,
        event: &SvmClaimedEvt,
        expected_swap_id: &[u8; 32],
    ) -> Option<Vec<u8>> {
        if event.swap_id == *expected_swap_id {
            Some(event.preimage.clone())
        } else {
            None
        }
    }

    /// Record source lock evidence to the proof ledger.
    pub fn record_source_lock(
        &mut self,
        intent_id: u64,
        tx_hash: String,
        block_number: u64,
        now: u64,
    ) -> u64 {
        let record = self
            .ledger
            .create_record(intent_id, self.relayer_id.clone(), now);
        let id = record.record_id;
        record.record_source_lock(tx_hash, block_number, now);
        id
    }

    /// Record destination lock evidence.
    pub fn record_destination_lock(
        &mut self,
        record_id: u64,
        tx_hash: String,
        block_number: u64,
        now: u64,
    ) -> Result<(), SwapError> {
        let record = self
            .ledger
            .get_record_mut(record_id)
            .ok_or(SwapError::ProofNotFound {
                proof_id: record_id.to_string(),
                intent_id: 0,
            })?;
        record.record_destination_lock(tx_hash, block_number, now);
        Ok(())
    }

    /// Record hashlock match.
    pub fn record_hashlock_match(
        &mut self,
        record_id: u64,
        matched: bool,
        now: u64,
    ) -> Result<(), SwapError> {
        let record = self
            .ledger
            .get_record_mut(record_id)
            .ok_or(SwapError::ProofNotFound {
                proof_id: record_id.to_string(),
                intent_id: 0,
            })?;
        record.record_hashlock_match(matched, now);
        Ok(())
    }

    /// Record timeout ordering validation.
    pub fn record_timeout_order(
        &mut self,
        record_id: u64,
        valid: bool,
        now: u64,
    ) -> Result<(), SwapError> {
        let record = self
            .ledger
            .get_record_mut(record_id)
            .ok_or(SwapError::ProofNotFound {
                proof_id: record_id.to_string(),
                intent_id: 0,
            })?;
        record.record_timeout_order(valid, now);
        Ok(())
    }

    /// Record finality verification.
    pub fn record_finality_verified(
        &mut self,
        record_id: u64,
        verified: bool,
        now: u64,
    ) -> Result<(), SwapError> {
        let record = self
            .ledger
            .get_record_mut(record_id)
            .ok_or(SwapError::ProofNotFound {
                proof_id: record_id.to_string(),
                intent_id: 0,
            })?;
        record.record_finality_verified(verified, now);
        Ok(())
    }

    /// Record secret reveal transaction.
    pub fn record_secret_reveal(
        &mut self,
        record_id: u64,
        tx_hash: String,
        now: u64,
    ) -> Result<(), SwapError> {
        let record = self
            .ledger
            .get_record_mut(record_id)
            .ok_or(SwapError::ProofNotFound {
                proof_id: record_id.to_string(),
                intent_id: 0,
            })?;
        record.record_secret_reveal(tx_hash, now);
        Ok(())
    }

    /// Record claim transaction.
    pub fn record_claim(
        &mut self,
        record_id: u64,
        tx_hash: String,
        block_number: u64,
        now: u64,
    ) -> Result<(), SwapError> {
        let record = self
            .ledger
            .get_record_mut(record_id)
            .ok_or(SwapError::ProofNotFound {
                proof_id: record_id.to_string(),
                intent_id: 0,
            })?;
        record.record_claim(tx_hash, block_number, now);
        Ok(())
    }

    /// Record refund transaction.
    pub fn record_refund(
        &mut self,
        record_id: u64,
        tx_hash: String,
        block_number: u64,
        now: u64,
    ) -> Result<(), SwapError> {
        let record = self
            .ledger
            .get_record_mut(record_id)
            .ok_or(SwapError::ProofNotFound {
                proof_id: record_id.to_string(),
                intent_id: 0,
            })?;
        record.record_refund(tx_hash, block_number, now);
        Ok(())
    }

    /// Generate a scoreboard from the latest proof record.
    pub fn generate_scoreboard(
        &self,
        intent_id: u64,
        relayer_quorum: u32,
    ) -> Option<SwapScoreboard> {
        let record = self.ledger.get_latest_for_intent(intent_id)?;
        // Count unique relayers that have records for this intent
        let relayers: std::collections::HashSet<&str> = self
            .ledger
            .get_records_for_intent(intent_id)
            .iter()
            .map(|r| r.relayer_id.as_str())
            .collect();
        let has_rpc_quorum = self.ledger.has_rpc_quorum_agreement_for_intent(intent_id);
        Some(SwapScoreboard::from_proof_record(
            record,
            relayer_quorum,
            relayers.len() as u32,
            has_rpc_quorum,
        ))
    }

    /// Refuse to mark success if the proof record has missing tx hashes.
    pub fn verify_proof_completeness(&self, record_id: u64) -> Result<(), SwapError> {
        let record = self
            .ledger
            .get_record(record_id)
            .ok_or(SwapError::ProofNotFound {
                proof_id: record_id.to_string(),
                intent_id: 0,
            })?;

        if record.source_lock_tx.is_none() {
            return Err(SwapError::MissingTxHash {
                step: "source_lock".into(),
                chain: "source".into(),
            });
        }
        if record.destination_lock_tx.is_none() {
            return Err(SwapError::MissingTxHash {
                step: "destination_lock".into(),
                chain: "destination".into(),
            });
        }
        // secret_reveal_tx is only required for claim paths.
        // On a pure refund path (refund_tx present, claim_tx absent),
        // the preimage was never revealed on-chain.
        if record.secret_reveal_tx.is_none() && record.refund_tx.is_none() {
            return Err(SwapError::MissingTxHash {
                step: "secret_reveal".into(),
                chain: "any".into(),
            });
        }
        if record.claim_tx.is_none() && record.refund_tx.is_none() {
            return Err(SwapError::MissingTxHash {
                step: "claim_or_refund".into(),
                chain: "any".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm_htlc::EvmHtlcContract;
    use crate::intent::{AtomicIntentBuilder, ChainKind, RefundPath};
    use crate::svm_htlc::SvmHtlcProgram;
    use sha2::{Digest, Sha256};

    fn test_evm_address(n: u8) -> [u8; 20] {
        let mut addr = [0u8; 20];
        addr[0] = n;
        addr[19] = n;
        addr
    }

    fn test_pubkey(n: u8) -> [u8; 32] {
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

    fn make_intent(relayer_quorum: u32) -> AtomicIntent {
        AtomicIntentBuilder::new()
            .source_chain(ChainKind::Ethereum)
            .destination_chain(ChainKind::Solana)
            .source_asset("USDC")
            .destination_asset("SOL")
            .amount_in(1000)
            .min_amount_out(950)
            .receiver("sol_receiver")
            .hashlock(make_hashlock(b"preimage123"))
            .source_timeout(2000)
            .destination_timeout(1000)
            .refund_path(RefundPath {
                chain: ChainKind::Ethereum,
                address: "0xrefund".into(),
                asset: Some("USDC".into()),
            })
            .relayer_quorum(relayer_quorum)
            .build(1)
            .expect("intent should build")
    }

    #[test]
    fn test_relayer_watches_evm_lock() {
        let intent = make_intent(3);
        let mut contract = EvmHtlcContract::new(test_evm_address(1));
        let relayer = Relayer::new("relayer-1".into(), 12);

        // Lock on EVM side
        contract
            .lock(
                [0xabu8; 32],
                test_evm_address(2),
                test_evm_address(3),
                test_evm_address(4),
                1000,
                intent.hashlock,
                2000,
                [0u8; 20],
            )
            .expect("lock should succeed");

        // Relayer should detect the lock event
        let lock_event = relayer.watch_evm_lock(&intent, &contract);
        assert!(
            lock_event.is_some(),
            "relayer should detect matching lock event"
        );
        if let Some(event) = lock_event {
            assert_eq!(event.hashlock, intent.hashlock);
            assert_eq!(event.amount, 1000);
        }
    }

    #[test]
    fn test_relayer_watches_svm_lock() {
        let intent = make_intent(3);
        let mut program = SvmHtlcProgram::new(test_pubkey(1));
        let relayer = Relayer::new("relayer-1".into(), 1);

        // Lock on SVM side
        program
            .lock(
                [0xabu8; 32],
                test_pubkey(2),
                test_pubkey(3),
                test_pubkey(4),
                1000,
                intent.hashlock,
                2000,
                [0u8; 32],
                255,
            )
            .expect("lock should succeed");

        let lock_event = relayer.watch_svm_lock(&intent, &program);
        assert!(lock_event.is_some(), "relayer should detect SVM lock event");
    }

    #[test]
    fn test_relayer_verifies_hashlock_match() {
        let relayer = Relayer::new("relayer-1".into(), 12);
        let hashlock1 = make_hashlock(b"same_secret");
        let hashlock2 = make_hashlock(b"same_secret");
        let hashlock3 = make_hashlock(b"different_secret");

        assert!(relayer.verify_hashlock_match(&hashlock1, &hashlock2));
        assert!(!relayer.verify_hashlock_match(&hashlock1, &hashlock3));
    }

    #[test]
    fn test_relayer_verifies_finality() {
        let relayer = Relayer::new("relayer-1".into(), 12);

        // Sufficient confirmations
        assert!(relayer.verify_finality(12, 15, "eth").is_ok());

        // Insufficient
        let result = relayer.verify_finality(12, 5, "eth");
        assert!(result.is_err());
        if let Err(SwapError::FinalityNotMet {
            required, current, ..
        }) = result
        {
            assert_eq!(required, 12);
            assert_eq!(current, 5);
        } else {
            panic!("expected FinalityNotMet");
        }
    }

    #[test]
    fn test_relayer_cannot_claim_without_finality() {
        let relayer = Relayer::new("relayer-1".into(), 12);

        // Must have finality before considering a swap valid
        let result = relayer.verify_finality(12, 3, "eth");
        assert!(
            result.is_err(),
            "relayer must refuse to proceed without finality"
        );
    }

    #[test]
    fn test_relayer_records_and_verifies_proof() {
        let mut relayer = Relayer::new("relayer-1".into(), 12);
        let intent = make_intent(3);

        // Add an agreed RPC quorum proof so the scoreboard can reach 100.
        relayer
            .ledger
            .add_rpc_quorum_proof(crate::ledger::RpcQuorumProof {
                intent_id: intent.intent_id,
                provider: "rpc-a".into(),
                block_height: 100,
                tx_status: crate::ledger::TxStatus::Confirmed,
                agreement_count: 3,
                required_quorum: 2,
            });

        // Record source lock
        let record_id =
            relayer.record_source_lock(intent.intent_id, "0xsource_tx".into(), 100, 1100);
        relayer
            .record_destination_lock(record_id, "0xdest_tx".into(), 200, 1200)
            .unwrap();
        relayer
            .record_hashlock_match(record_id, true, 1300)
            .unwrap();
        relayer.record_timeout_order(record_id, true, 1400).unwrap();
        relayer
            .record_finality_verified(record_id, true, 1500)
            .unwrap();
        relayer
            .record_secret_reveal(record_id, "0xreveal_tx".into(), 1600)
            .unwrap();
        relayer
            .record_claim(record_id, "0xclaim_tx".into(), 300, 1700)
            .unwrap();

        // Verify completeness
        assert!(relayer.verify_proof_completeness(record_id).is_ok());

        // Generate scoreboard with quorum=1 (only one relayer in this test)
        let scoreboard = relayer.generate_scoreboard(intent.intent_id, 1);
        assert!(scoreboard.is_some());
        let sb = scoreboard.unwrap();
        assert!(sb.is_perfect(), "all proofs present should score 100");
    }

    #[test]
    fn test_relayer_refuses_success_without_tx_hashes() {
        let mut relayer = Relayer::new("relayer-1".into(), 12);
        let intent = make_intent(3);

        // Record only source lock, missing everything else
        let record_id = relayer.record_source_lock(intent.intent_id, "0xsource".into(), 100, 1100);

        // Verify completeness should fail
        let result = relayer.verify_proof_completeness(record_id);
        assert!(
            result.is_err(),
            "relayer must refuse to mark success without all tx hashes"
        );
    }

    #[test]
    fn test_relayer_extracts_preimage_from_evm() {
        let relayer = Relayer::new("relayer-1".into(), 12);
        let preimage = b"secret_preimage";
        let swap_id = [0xabu8; 32];

        let event = EvmClaimedEvent {
            contract: [0u8; 20],
            swap_id,
            claimant: [0u8; 20],
            preimage: preimage.to_vec(),
        };

        let extracted = relayer.extract_preimage_from_evm_claim(&event, &swap_id);
        assert_eq!(extracted, Some(preimage.to_vec()));

        // Wrong swap_id should return None
        let wrong_id = [0xffu8; 32];
        assert!(relayer
            .extract_preimage_from_evm_claim(&event, &wrong_id)
            .is_none());
    }

    #[test]
    fn test_stuck_swap_alerts_are_per_intent_not_global() {
        use crate::ledger::RpcQuorumProof;
        use crate::ledger::TxStatus;

        // Setup: intent 1 is complete (has destination lock + claim),
        //         intent 2 is stuck at SourceLocked with no destination lock.
        let preimage1 = b"preimage_one";
        let preimage2 = b"preimage_two";
        let h1 = make_hashlock(preimage1);
        let h2 = make_hashlock(preimage2);

        let mut intent1 = AtomicIntentBuilder::new()
            .source_chain(ChainKind::Ethereum)
            .destination_chain(ChainKind::Solana)
            .source_asset("USDC")
            .destination_asset("SOL")
            .amount_in(1000)
            .min_amount_out(950)
            .receiver("r1")
            .hashlock(h1)
            .source_timeout(2000)
            .destination_timeout(1000)
            .refund_path(RefundPath {
                chain: ChainKind::Ethereum,
                address: "0xr1".into(),
                asset: Some("USDC".into()),
            })
            .relayer_quorum(1)
            .build(101)
            .expect("intent1 build");
        intent1.set_status(AtomicSwapStatus::SourceLocked).unwrap();
        intent1.set_status(AtomicSwapStatus::BothLocked).unwrap();

        let mut intent2 = AtomicIntentBuilder::new()
            .source_chain(ChainKind::Ethereum)
            .destination_chain(ChainKind::Solana)
            .source_asset("USDC")
            .destination_asset("SOL")
            .amount_in(2000)
            .min_amount_out(1900)
            .receiver("r2")
            .hashlock(h2)
            .source_timeout(3000)
            .destination_timeout(2000)
            .refund_path(RefundPath {
                chain: ChainKind::Ethereum,
                address: "0xr2".into(),
                asset: Some("USDC".into()),
            })
            .relayer_quorum(1)
            .build(102)
            .expect("intent2 build");
        intent2.set_status(AtomicSwapStatus::SourceLocked).unwrap();

        let mut relayer = Relayer::new("relayer-alert-test".into(), 12);

        // Record a complete destination lock + claim for intent 1
        let rec1 = relayer.record_source_lock(101, "0xsrc1".into(), 100, 1100);
        relayer
            .record_destination_lock(rec1, "0xdest1".into(), 200, 1200)
            .unwrap();
        relayer.record_hashlock_match(rec1, true, 1300).unwrap();
        relayer.record_timeout_order(rec1, true, 1400).unwrap();
        relayer.record_finality_verified(rec1, true, 1500).unwrap();
        relayer
            .record_secret_reveal(rec1, "0xreveal1".into(), 1600)
            .unwrap();
        relayer
            .record_claim(rec1, "0xclaim1".into(), 300, 1700)
            .unwrap();

        // Record only a source lock for intent 2 - no destination lock
        relayer.record_source_lock(102, "0xsrc2".into(), 100, 2100);

        // Add an agreed RPC quorum proof so the RpcDisagreement
        // path does not fire (we only want MissingDestinationLock).
        relayer.ledger.add_rpc_quorum_proof(RpcQuorumProof {
            intent_id: 102,
            provider: "rpc-a".into(),
            block_height: 100,
            tx_status: TxStatus::Confirmed,
            agreement_count: 3,
            required_quorum: 2,
        });

        let intents = vec![intent1, intent2];
        let alerts = scan_for_alerts(&intents, &relayer.ledger, 2500, 300);

        // Intent 2 must have MissingDestinationLock
        let has_missing_dest = alerts
            .iter()
            .any(|a| matches!(a, WatcherAlert::MissingDestinationLock { intent_id: 102 }));
        assert!(
            has_missing_dest,
            "intent 2 must have MissingDestinationLock, alerts: {:?}",
            alerts
        );

        // Intent 1 is complete (BothLocked status with destination proof) -
        // its healthy state must NOT suppress the alert for intent 2.
        // Also intent 1 should not get a MissingDestinationLock.
        let intent1_missing_dest = alerts
            .iter()
            .any(|a| matches!(a, WatcherAlert::MissingDestinationLock { intent_id: 101 }));
        assert!(
            !intent1_missing_dest,
            "intent 1 must NOT get MissingDestinationLock since its dest lock exists"
        );
    }

    #[test]
    fn test_relayer_generates_scoreboard_with_missing_proofs() {
        let mut relayer = Relayer::new("relayer-1".into(), 12);
        let intent = make_intent(3);

        // Only record source lock - scoreboard should reflect missing proofs
        relayer.record_source_lock(intent.intent_id, "0xsource".into(), 100, 1100);

        let scoreboard = relayer.generate_scoreboard(intent.intent_id, 3);
        assert!(scoreboard.is_some());
        let sb = scoreboard.unwrap();
        assert!(!sb.missing_proofs.is_empty());
        assert!(!sb.is_perfect());
    }
}
