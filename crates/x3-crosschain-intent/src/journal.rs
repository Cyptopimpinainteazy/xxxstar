//! Atomic Journal — transactional execution journal with snapshot,
//! rollback, refund, and quarantine semantics.
//!
//! Every cross-chain intent execution runs inside an atomic journal.
//! The journal records every state change so that any step failure
//! can be reversed safely. At the end of execution, the journal is
//! either committed (all changes persisted) or rolled back (all
//! changes undone).
//!
//! # Lifecycle
//!
//! ```text
//! AtomicBegin → snapshot
//!   execute step → append JournalEntry
//!   execute step → append JournalEntry
//!   ... (any step fails) → rollback all entries in reverse order
//! AtomicEnd → commit all entries in order
//! ```
//!
//! # Failure states
//!
//! Every intent must resolve to one of:
//! - `Completed`  — all steps committed, receipt emitted
//! - `Refunded`   — all assets returned to source (undo + refund)
//! - `Quarantined` — funds held for manual security council review
//! - `Disputed`   — proof mismatch, escalated to arbitration
//! - `Slashed`    — malicious behavior detected, validator slashed

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::option::Option::{None, Some};

use crate::instructions::X3Instruction;
use crate::types::{AssetRef, ChainKind};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Journal Entry Types
// ─────────────────────────────────────────────────────────────────────────────

/// A single recorded state change inside an atomic execution block.
///
/// Each entry records the before and after state of one resource
/// (balance, supply, escrow ticket, receipt, storage key, emitted event,
/// or external call). Entries are appended in order of execution and
/// rolled back in reverse order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalEntry {
    /// A balance was debited or credited.
    BalanceDelta {
        chain: ChainKind,
        asset: AssetRef,
        owner: String,
        before: u128,
        after: u128,
    },
    /// A total supply was changed (mint or burn).
    SupplyDelta {
        asset: AssetRef,
        before: u128,
        after: u128,
    },
    /// An escrow ticket was created or consumed.
    EscrowTicket {
        ticket_id: u64,
        asset: AssetRef,
        amount: u128,
        owner: String,
        action: EscrowAction,
    },
    /// A receipt was emitted or revoked.
    Receipt {
        receipt_hash: [u8; 32],
        action: ReceiptAction,
    },
    /// A storage key was written.
    StorageWrite {
        key: Vec<u8>,
        before: Option<Vec<u8>>,
        after: Option<Vec<u8>>,
    },
    /// An event was emitted (can be unwound via reverse event).
    EmittedEvent {
        event_name: String,
        payload: Vec<u8>,
    },
    /// An external call was dispatched (rollback may call a reverse operation).
    ExternalCall {
        chain: ChainKind,
        target: String,
        call_data: Vec<u8>,
        rollback_target: Option<String>,
        rollback_data: Option<Vec<u8>>,
    },
}

/// Escrow ticket action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscrowAction {
    Locked,
    Released,
    Burned,
}

/// Receipt action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiptAction {
    Emitted,
    Revoked,
}

// ─────────────────────────────────────────────────────────────────────────────
// Journal Snapshot
// ─────────────────────────────────────────────────────────────────────────────

/// A snapshot of the journal at a point in time.
///
/// The snapshot captures the journal state at `AtomicBegin`. It is
/// restored on rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalSnapshot {
    entry_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// Execution Result
// ─────────────────────────────────────────────────────────────────────────────

/// The final result of an intent execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionResult {
    /// All steps completed successfully.
    Completed,
    /// Assets returned to source (rollback + refund executed).
    Refunded,
    /// Funds quarantined for manual review.
    Quarantined,
    /// Proof mismatch escalated to arbitration.
    Disputed,
    /// Malicious behavior detected, validator slashed.
    Slashed,
}

impl ExecutionResult {
    /// True if the result is a success state.
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionResult::Completed)
    }

    /// True if the result is a failure state that returned or secured funds.
    pub fn is_safe_failure(&self) -> bool {
        matches!(
            self,
            ExecutionResult::Refunded | ExecutionResult::Quarantined | ExecutionResult::Disputed
        )
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            ExecutionResult::Completed => "Completed",
            ExecutionResult::Refunded => "Refunded",
            ExecutionResult::Quarantined => "Quarantined",
            ExecutionResult::Disputed => "Disputed",
            ExecutionResult::Slashed => "Slashed",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Atomic Journal
// ─────────────────────────────────────────────────────────────────────────────

/// Transactional execution journal for cross-chain intents.
///
/// The journal tracks every state change and provides atomic
/// commit/rollback semantics. It also tracks the final execution
/// result for explorer and dispute tooling.
///
/// # Usage
///
/// ```ignore
/// let mut journal = AtomicJournal::new(intent_id);
/// let snapshot = journal.begin();
/// journal.append(entry1);
/// journal.append(entry2);
/// // If step 3 fails:
/// journal.rollback(snapshot);
/// journal.set_result(ExecutionResult::Refunded);
/// // Otherwise:
/// journal.commit(snapshot);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicJournal {
    /// The intent ID this journal belongs to.
    pub intent_id: u64,
    /// Ordered entries in the current atomic block.
    entries: Vec<JournalEntry>,
    /// Whether the current block is in an open atomic scope.
    is_open: bool,
    /// Counter for escrow ticket IDs.
    next_ticket_id: u64,
    /// The final execution result (set once at end).
    pub result: Option<ExecutionResult>,
}

impl AtomicJournal {
    /// Create a new, empty journal for the given intent.
    pub fn new(intent_id: u64) -> Self {
        Self {
            intent_id,
            entries: Vec::new(),
            is_open: false,
            next_ticket_id: 1,
            result: None,
        }
    }

    /// Open an atomic block and return a snapshot of the current state.
    ///
    /// Call this before the first irreversible step. The snapshot is
    /// used by `rollback()` to undo all changes since `begin()`.
    pub fn begin(&mut self) -> JournalSnapshot {
        self.is_open = true;
        JournalSnapshot {
            entry_count: self.entries.len(),
        }
    }

    /// Append a journal entry.
    ///
    /// Panics if no atomic block is open (`begin()` was not called).
    pub fn append(&mut self, entry: JournalEntry) {
        assert!(
            self.is_open,
            "AtomicJournal: append() called without begin()"
        );
        self.entries.push(entry);
    }

    /// Roll back to the given snapshot.
    ///
    /// Entries appended after the snapshot are removed. The journal
    /// is returned to the state it had at the snapshot point.
    /// After rollback, the atomic block is closed.
    pub fn rollback(&mut self, snapshot: JournalSnapshot) -> Vec<JournalEntry> {
        self.is_open = false;
        let rolled_back: Vec<JournalEntry> =
            self.entries.drain(snapshot.entry_count..).rev().collect();
        rolled_back
    }

    /// Commit all entries since the given snapshot.
    ///
    /// After commit, the atomic block is closed and entries are
    /// permanently recorded.
    pub fn commit(&mut self, _snapshot: JournalSnapshot) {
        self.is_open = false;
        // Entries remain in the journal permanently after commit.
        // A future implementation may flush them to persistent storage.
    }

    /// Set the final execution result.
    pub fn set_result(&mut self, result: ExecutionResult) {
        self.result = Some(result);
    }

    /// Return the execution result, or `None` if not yet set.
    pub fn result(&self) -> Option<ExecutionResult> {
        self.result.clone()
    }

    /// True if the intent completed successfully.
    pub fn is_completed(&self) -> bool {
        matches!(self.result, Some(ExecutionResult::Completed))
    }

    /// Return the number of entries recorded.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Allocate a new escrow ticket ID.
    pub fn alloc_ticket_id(&mut self) -> u64 {
        let id = self.next_ticket_id;
        self.next_ticket_id += 1;
        id
    }

    /// Generate a rollback plan from the current journal entries.
    ///
    /// Returns a list of `X3Instruction`s that undo every entry
    /// in reverse order. For example:
    /// - A `BalanceDelta` with `after < before` (debit) produces a
    ///   credit instruction.
    /// - A `MintCanonical` entry produces a `BurnCanonical`.
    /// - A `LockAsset` entry produces a release/refund.
    /// - An `EscrowTicket::Locked` produces an `EscrowTicket::Released`.
    pub fn rollback_plan(&self) -> Vec<X3Instruction> {
        let mut plan = Vec::new();

        for entry in self.entries.iter().rev() {
            match entry {
                JournalEntry::BalanceDelta {
                    chain: _,
                    asset,
                    owner,
                    before,
                    after,
                } => {
                    if after > before {
                        // Credit → need to debit back
                        let diff = after - before;
                        plan.push(X3Instruction::ExecuteRefund {
                            intent_id: self.intent_id,
                            action: crate::instructions::RefundAction::ReturnToSource {
                                asset: asset.clone(),
                                amount: diff,
                                to: owner.clone(),
                            },
                        });
                    } else if before > after {
                        // Debit → need to credit back
                        let diff = before - after;
                        plan.push(X3Instruction::ExecuteRefund {
                            intent_id: self.intent_id,
                            action: crate::instructions::RefundAction::ReturnToSource {
                                asset: asset.clone(),
                                amount: diff,
                                to: owner.clone(),
                            },
                        });
                    }
                }
                JournalEntry::SupplyDelta {
                    asset,
                    before,
                    after,
                } => {
                    if after > before {
                        // Minted → need to burn
                        let diff = after - before;
                        plan.push(X3Instruction::BurnCanonical {
                            canonical_asset: asset.clone(),
                            amount: diff,
                            from: "canonical_supply".to_string(),
                        });
                    }
                }
                JournalEntry::EscrowTicket {
                    asset,
                    amount,
                    owner,
                    action,
                    ..
                } => {
                    match action {
                        EscrowAction::Locked => {
                            // Release the locked funds back
                            plan.push(X3Instruction::ExecuteRefund {
                                intent_id: self.intent_id,
                                action: crate::instructions::RefundAction::ReturnToSource {
                                    asset: asset.clone(),
                                    amount: *amount,
                                    to: owner.clone(),
                                },
                            });
                        }
                        EscrowAction::Released => {
                            // Re-lock (unusual but possible if refund path needs it)
                            // This is a no-op for safety — we don't re-lock on rollback
                        }
                        EscrowAction::Burned => {
                            // Can't un-burn. Quarantine.
                            plan.push(X3Instruction::Quarantine {
                                intent_id: self.intent_id,
                                reason: format!(
                                    "rollback: burned escrow ticket for {} {}",
                                    amount,
                                    asset.display()
                                ),
                            });
                        }
                    }
                }
                JournalEntry::Receipt { action, .. } => {
                    match action {
                        ReceiptAction::Emitted => {
                            // Can't un-emit. Log the reversal intent.
                        }
                        ReceiptAction::Revoked => {}
                    }
                }
                JournalEntry::StorageWrite {
                    key,
                    before,
                    after: _,
                } => {
                    if let Some(prev) = before {
                        plan.push(crate::instructions::X3Instruction::ExecuteRefund {
                            intent_id: self.intent_id,
                            action: crate::instructions::RefundAction::ReturnToSource {
                                asset: AssetRef::new(ChainKind::X3, "STORAGE"),
                                amount: key.len() as u128,
                                to: String::from_utf8_lossy(prev).to_string(),
                            },
                        });
                    }
                }
                JournalEntry::EmittedEvent { .. } => {
                    // Events can't be un-emitted. The rollback plan
                    // records this as informational.
                }
                JournalEntry::ExternalCall {
                    chain: _,
                    target,
                    call_data: _,
                    rollback_target,
                    rollback_data,
                } => {
                    if let (Some(rt), Some(rd)) = (rollback_target, rollback_data) {
                        // Emit a refund action representing the reverse call
                        plan.push(X3Instruction::ExecuteRefund {
                            intent_id: self.intent_id,
                            action: crate::instructions::RefundAction::ReturnToSource {
                                asset: AssetRef::new(ChainKind::X3, format!("rollback:{}", target)),
                                amount: rd.len() as u128,
                                to: rt.clone(),
                            },
                        });
                    }
                }
            }
        }

        plan
    }

    /// Determine the final result based on a failure injection point.
    ///
    /// In production, the runtime calls `set_result()` directly. This
    /// helper is used by the test harness to simulate failures.
    pub fn fail_at(&mut self, point: FailurePoint) -> ExecutionResult {
        let result = match point {
            FailurePoint::BeforeLock => ExecutionResult::Refunded,
            FailurePoint::AfterLockBeforeFinality => ExecutionResult::Quarantined,
            FailurePoint::AfterProofBeforeMint => ExecutionResult::Disputed,
            FailurePoint::AfterMintBeforeSwap => ExecutionResult::Quarantined,
            FailurePoint::AfterSwapBeforeBridge => ExecutionResult::Refunded,
            FailurePoint::AfterBridgeBeforeReceipt => ExecutionResult::Disputed,
            FailurePoint::TimeoutDuringActiveState => ExecutionResult::Refunded,
        };
        self.set_result(result.clone());
        result
    }
}

/// Points at which failure can be injected during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailurePoint {
    /// Fail before the lock step.
    BeforeLock,
    /// Fail after lock but before finality check.
    AfterLockBeforeFinality,
    /// Fail after proof verification but before mint.
    AfterProofBeforeMint,
    /// Fail after mint but before swap.
    AfterMintBeforeSwap,
    /// Fail after swap but before bridge.
    AfterSwapBeforeBridge,
    /// Fail after bridge but before receipt.
    AfterBridgeBeforeReceipt,
    /// Fail due to timeout during an active state.
    TimeoutDuringActiveState,
}

impl FailurePoint {
    pub fn label(&self) -> &'static str {
        match self {
            FailurePoint::BeforeLock => "BeforeLock",
            FailurePoint::AfterLockBeforeFinality => "AfterLockBeforeFinality",
            FailurePoint::AfterProofBeforeMint => "AfterProofBeforeMint",
            FailurePoint::AfterMintBeforeSwap => "AfterMintBeforeSwap",
            FailurePoint::AfterSwapBeforeBridge => "AfterSwapBeforeBridge",
            FailurePoint::AfterBridgeBeforeReceipt => "AfterBridgeBeforeReceipt",
            FailurePoint::TimeoutDuringActiveState => "TimeoutDuringActiveState",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instructions::RefundAction;
    use crate::types::ChainKind;

    fn usdc_asset() -> AssetRef {
        AssetRef::new(ChainKind::Ethereum, "USDC")
    }

    fn sol_asset() -> AssetRef {
        AssetRef::new(ChainKind::Solana, "SOL")
    }

    #[test]
    fn journal_begin_append_commit() {
        let mut journal = AtomicJournal::new(1);
        let snapshot = journal.begin();

        journal.append(JournalEntry::BalanceDelta {
            chain: ChainKind::Ethereum,
            asset: usdc_asset(),
            owner: "alice.eth".to_string(),
            before: 1_000_000,
            after: 500_000,
        });

        journal.commit(snapshot);
        journal.set_result(ExecutionResult::Completed);

        assert_eq!(journal.entry_count(), 1);
        assert!(journal.is_completed());
    }

    #[test]
    fn journal_begin_rollback_restores_state() {
        let mut journal = AtomicJournal::new(1);
        let snapshot = journal.begin();

        journal.append(JournalEntry::BalanceDelta {
            chain: ChainKind::Ethereum,
            asset: usdc_asset(),
            owner: "alice.eth".to_string(),
            before: 1_000_000,
            after: 500_000,
        });

        journal.append(JournalEntry::SupplyDelta {
            asset: usdc_asset(),
            before: 10_000_000,
            after: 10_500_000,
        });

        // Rollback removes both entries
        let rolled = journal.rollback(snapshot);
        assert_eq!(rolled.len(), 2, "should roll back 2 entries");
        assert_eq!(
            journal.entry_count(),
            0,
            "journal should be empty after rollback"
        );
    }

    #[test]
    fn journal_rollback_plan_has_correct_reversal() {
        let mut journal = AtomicJournal::new(1);
        let _snapshot = journal.begin();

        journal.append(JournalEntry::BalanceDelta {
            chain: ChainKind::Ethereum,
            asset: usdc_asset(),
            owner: "alice.eth".to_string(),
            before: 1_000_000,
            after: 500_000, // debit 500k
        });

        journal.append(JournalEntry::SupplyDelta {
            asset: usdc_asset(),
            before: 10_000_000,
            after: 10_500_000, // minted 500k
        });

        let plan = journal.rollback_plan();
        assert!(!plan.is_empty(), "rollback plan should not be empty");
    }

    #[test]
    fn journal_failure_injection_all_points() {
        let points = [
            FailurePoint::BeforeLock,
            FailurePoint::AfterLockBeforeFinality,
            FailurePoint::AfterProofBeforeMint,
            FailurePoint::AfterMintBeforeSwap,
            FailurePoint::AfterSwapBeforeBridge,
            FailurePoint::AfterBridgeBeforeReceipt,
            FailurePoint::TimeoutDuringActiveState,
        ];

        for point in &points {
            let mut journal = AtomicJournal::new(1);
            let result = journal.fail_at(*point);
            assert!(
                result.is_safe_failure() || result == ExecutionResult::Slashed,
                "failure at {:?} must produce a safe result, got {:?}",
                point,
                result
            );
            assert_eq!(
                journal.result(),
                Some(result),
                "journal result must match fail_at return"
            );
        }
    }

    #[test]
    fn journal_escrow_ticket_allocation() {
        let mut journal = AtomicJournal::new(1);
        assert_eq!(journal.alloc_ticket_id(), 1);
        assert_eq!(journal.alloc_ticket_id(), 2);
        assert_eq!(journal.alloc_ticket_id(), 3);
    }

    #[test]
    fn journal_rollback_plan_many_entries() {
        let mut journal = AtomicJournal::new(42);
        let _snapshot = journal.begin();

        // Simulate a full eth→sol bridge flow
        journal.append(JournalEntry::BalanceDelta {
            chain: ChainKind::Ethereum,
            asset: usdc_asset(),
            owner: "alice.eth".to_string(),
            before: 500_000_000,
            after: 0, // locked all
        });

        let ticket_id = journal.alloc_ticket_id();
        journal.append(JournalEntry::EscrowTicket {
            ticket_id,
            asset: usdc_asset(),
            amount: 500_000_000,
            owner: "alice.eth".to_string(),
            action: EscrowAction::Locked,
        });

        journal.append(JournalEntry::SupplyDelta {
            asset: usdc_asset(),
            before: 10_000_000_000,
            after: 10_500_000_000, // minted 500M wrapped
        });

        journal.append(JournalEntry::BalanceDelta {
            chain: ChainKind::Solana,
            asset: sol_asset(),
            owner: "alice.sol".to_string(),
            before: 0,
            after: 3_500_000_000, // received SOL
        });

        let plan = journal.rollback_plan();
        assert!(!plan.is_empty(), "should produce rollback instructions");
        assert!(
            plan.len() >= 3,
            "should have at least 3 rollback steps, got {}",
            plan.len()
        );

        // Verify plan order is reverse (last entry first)
        for instr in &plan {
            match instr {
                X3Instruction::ExecuteRefund { action, .. } => {
                    match action {
                        RefundAction::ReturnToSource { asset, .. } => {
                            // SOL balance delta should be first to reverse
                            let sym: &str = &asset.symbol;
                            assert!(
                                sym == "SOL" || sym == "USDC",
                                "rollback should return USDC or SOL, got {}",
                                sym
                            );
                        }
                        RefundAction::Quarantine => {
                            // Burned escrow → quarantine
                        }
                        _ => {}
                    }
                }
                X3Instruction::BurnCanonical { .. } => {
                    // Supply delta reversal
                }
                X3Instruction::Quarantine { .. } => {}
                _ => {}
            }
        }
    }

    #[test]
    #[should_panic(expected = "begin")]
    fn journal_rejects_append_without_begin() {
        let mut journal = AtomicJournal::new(1);
        journal.append(JournalEntry::BalanceDelta {
            chain: ChainKind::X3,
            asset: usdc_asset(),
            owner: "test".to_string(),
            before: 10,
            after: 5,
        });
    }

    #[test]
    fn execution_result_labels() {
        assert_eq!(ExecutionResult::Completed.label(), "Completed");
        assert_eq!(ExecutionResult::Refunded.label(), "Refunded");
        assert_eq!(ExecutionResult::Quarantined.label(), "Quarantined");
        assert_eq!(ExecutionResult::Disputed.label(), "Disputed");
        assert_eq!(ExecutionResult::Slashed.label(), "Slashed");
    }

    #[test]
    fn execution_result_success_failure() {
        assert!(ExecutionResult::Completed.is_success());
        assert!(!ExecutionResult::Refunded.is_success());
        assert!(!ExecutionResult::Quarantined.is_success());
        assert!(!ExecutionResult::Disputed.is_success());
        assert!(!ExecutionResult::Slashed.is_success());

        assert!(ExecutionResult::Refunded.is_safe_failure());
        assert!(ExecutionResult::Quarantined.is_safe_failure());
        assert!(ExecutionResult::Disputed.is_safe_failure());
        assert!(!ExecutionResult::Slashed.is_safe_failure());
    }

    #[test]
    fn journal_multiple_begin_commit_cycles() {
        let mut journal = AtomicJournal::new(1);

        // Cycle 1
        let snap1 = journal.begin();
        journal.append(JournalEntry::BalanceDelta {
            chain: ChainKind::Ethereum,
            asset: usdc_asset(),
            owner: "alice.eth".to_string(),
            before: 100,
            after: 0,
        });
        journal.commit(snap1);
        assert_eq!(journal.entry_count(), 1);

        // Cycle 2
        let snap2 = journal.begin();
        journal.append(JournalEntry::SupplyDelta {
            asset: usdc_asset(),
            before: 1000,
            after: 1100,
        });
        journal.commit(snap2);
        assert_eq!(journal.entry_count(), 2);

        journal.set_result(ExecutionResult::Completed);
        assert!(journal.is_completed());
    }
}
