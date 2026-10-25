//! # Atomic Swap Scoreboard
//!
//! Every atomic swap produces a scoreboard that proves each step was
//! completed with verifiable on-chain evidence. The scoreboard is a
//! mandatory output - no swap is considered complete without one.
//!
//! ## Scoring Rules
//!
//! | Category | Points | Requirement |
//! |----------|--------|-------------|
//! | Source lock proof | 20 | Transaction hash + block number |
//! | Destination lock proof | 20 | Transaction hash + block number |
//! | Hashlock proof | 10 | Preimage verified against hashlock |
//! | Timeout proof | 10 | Timeout ordering validated |
//! | Finality proof | 10 | Finality reached on both chains |
//! | Reveal proof | 10 | Secret preimage reveal tx hash |
//! | Claim/Refund proof | 10 | Claim tx hash OR refund tx hash |
//! | Relayer quorum | 10 | Minimum relayer attestations met |
//! | **Total** | **100** | |
//!
//! A completed refund path also earns full claim/refund points (10).
//! The scoreboard cannot reach 100 if any proof step is missing its
//! corresponding transaction hash.
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::adapter::{VmType, X3VmAdapter};
use crate::ledger::ProofRecord;
use serde::{Deserialize, Serialize};

/// Score category weights.
pub const SCORE_SOURCE_LOCK: u8 = 20;
pub const SCORE_DESTINATION_LOCK: u8 = 20;
pub const SCORE_HASHLOCK: u8 = 10;
pub const SCORE_TIMEOUT: u8 = 10;
pub const SCORE_FINALITY: u8 = 10;
pub const SCORE_REVEAL: u8 = 10;
pub const SCORE_CLAIM: u8 = 10;
pub const SCORE_RELAYER_QUORUM: u8 = 10;
pub const SCORE_MAX: u8 = 100;

/// A scored proof category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredCategory {
    /// Category name.
    pub name: String,
    /// Maximum possible points.
    pub max_points: u8,
    /// Points earned.
    pub earned: u8,
    /// Whether the proof is complete.
    pub complete: bool,
    /// Evidence description (tx hash, block number, etc.).
    pub evidence: String,
}

/// The complete atomic swap scoreboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapScoreboard {
    /// Intent ID this scoreboard belongs to.
    pub intent_id: u64,
    /// Individual category scores.
    pub categories: Vec<ScoredCategory>,
    /// Total score out of 100.
    pub total_score: u8,
    /// List of proof steps that are missing.
    pub missing_proofs: Vec<String>,
    /// Whether the scoreboard is complete (score == 100).
    pub is_complete: bool,
}

impl SwapScoreboard {
    /// Compute a scoreboard from a proof record, relayer count, and RPC quorum state.
    ///
    /// `has_rpc_quorum` must be true when at least one agreed `RpcQuorumProof`
    /// exists for this intent in the ledger. Without it the scoreboard cannot
    /// reach 100 - the quorum category depends on actual RPC agreement, not
    /// just relayer count.
    pub fn from_proof_record(
        record: &ProofRecord,
        relayer_quorum_requirement: u32,
        actual_relayer_count: u32,
        has_rpc_quorum: bool,
    ) -> Self {
        let mut categories = Vec::new();
        let mut total_score: u8 = 0;
        let mut missing_proofs = Vec::new();

        // 1. Source lock proof (20 pts)
        let (src_earned, src_complete, src_evidence) = match &record.source_lock_tx {
            Some(tx) => {
                let block = record.source_lock_block.unwrap_or(0);
                total_score += SCORE_SOURCE_LOCK;
                (
                    SCORE_SOURCE_LOCK,
                    true,
                    format!("tx={} block={}", tx, block),
                )
            }
            None => {
                missing_proofs.push("source_lock_tx".to_string());
                (0, false, "missing".into())
            }
        };
        categories.push(ScoredCategory {
            name: "source_lock_proof".into(),
            max_points: SCORE_SOURCE_LOCK,
            earned: src_earned,
            complete: src_complete,
            evidence: src_evidence,
        });

        // 2. Destination lock proof (20 pts)
        let (dst_earned, dst_complete, dst_evidence) = match &record.destination_lock_tx {
            Some(tx) => {
                let block = record.destination_lock_block.unwrap_or(0);
                total_score += SCORE_DESTINATION_LOCK;
                (
                    SCORE_DESTINATION_LOCK,
                    true,
                    format!("tx={} block={}", tx, block),
                )
            }
            None => {
                missing_proofs.push("destination_lock_tx".to_string());
                (0, false, "missing".into())
            }
        };
        categories.push(ScoredCategory {
            name: "destination_lock_proof".into(),
            max_points: SCORE_DESTINATION_LOCK,
            earned: dst_earned,
            complete: dst_complete,
            evidence: dst_evidence,
        });

        // 3. Hashlock proof (10 pts)
        if record.hashlock_match {
            total_score += SCORE_HASHLOCK;
            categories.push(ScoredCategory {
                name: "hashlock_proof".into(),
                max_points: SCORE_HASHLOCK,
                earned: SCORE_HASHLOCK,
                complete: true,
                evidence: "hashlock preimage verified".into(),
            });
        } else {
            missing_proofs.push("hashlock_match".to_string());
            categories.push(ScoredCategory {
                name: "hashlock_proof".into(),
                max_points: SCORE_HASHLOCK,
                earned: 0,
                complete: false,
                evidence: "hashlock not matched".into(),
            });
        }

        // 4. Timeout proof (10 pts)
        if record.timeout_order_valid {
            total_score += SCORE_TIMEOUT;
            categories.push(ScoredCategory {
                name: "timeout_proof".into(),
                max_points: SCORE_TIMEOUT,
                earned: SCORE_TIMEOUT,
                complete: true,
                evidence: "timeout ordering valid (dest < source)".into(),
            });
        } else {
            missing_proofs.push("timeout_order_valid".to_string());
            categories.push(ScoredCategory {
                name: "timeout_proof".into(),
                max_points: SCORE_TIMEOUT,
                earned: 0,
                complete: false,
                evidence: "timeout ordering invalid".into(),
            });
        }

        // 5. Finality proof (10 pts)
        if record.finality_verified {
            total_score += SCORE_FINALITY;
            categories.push(ScoredCategory {
                name: "finality_proof".into(),
                max_points: SCORE_FINALITY,
                earned: SCORE_FINALITY,
                complete: true,
                evidence: "finality verified on both chains".into(),
            });
        } else {
            missing_proofs.push("finality_verified".to_string());
            categories.push(ScoredCategory {
                name: "finality_proof".into(),
                max_points: SCORE_FINALITY,
                earned: 0,
                complete: false,
                evidence: "finality not verified".into(),
            });
        }

        // 6. Reveal proof (10 pts) — only required for claim paths.
        //    A refund terminal state does NOT require secret_reveal_tx
        //    because the preimage was never revealed on-chain.
        let is_refund_path = record.claim_tx.is_none() && record.refund_tx.is_some();
        let (rev_earned, rev_complete, rev_evidence) = match &record.secret_reveal_tx {
            Some(tx) => {
                total_score += SCORE_REVEAL;
                (SCORE_REVEAL, true, format!("reveal tx={}", tx))
            }
            None => {
                if is_refund_path {
                    // Refund path: reveal is not required; award the full
                    // category points so a refund can reach 100/100.
                    total_score += SCORE_REVEAL;
                    (SCORE_REVEAL, true, "not applicable (refund path)".into())
                } else {
                    missing_proofs.push("secret_reveal_tx".to_string());
                    (0, false, "missing".into())
                }
            }
        };
        categories.push(ScoredCategory {
            name: "reveal_proof".into(),
            max_points: SCORE_REVEAL,
            earned: rev_earned,
            complete: rev_complete,
            evidence: rev_evidence,
        });

        // 7. Claim/Refund proof (10 pts - full credit for either claim or refund)
        let (clm_earned, clm_complete, clm_evidence) = if let Some(tx) = &record.claim_tx {
            let block = record.claim_block.unwrap_or(0);
            total_score += SCORE_CLAIM;
            (
                SCORE_CLAIM,
                true,
                format!("claim tx={} block={}", tx, block),
            )
        } else if let Some(tx) = &record.refund_tx {
            let block = record.refund_block.unwrap_or(0);
            total_score += SCORE_CLAIM; // Refund earns full claim/refund points
            (
                SCORE_CLAIM,
                true,
                format!("refund tx={} block={}", tx, block),
            )
        } else {
            missing_proofs.push("claim_tx_or_refund_tx".to_string());
            (0, false, "missing".into())
        };
        categories.push(ScoredCategory {
            name: "claim_refund_proof".into(),
            max_points: SCORE_CLAIM,
            earned: clm_earned,
            complete: clm_complete,
            evidence: clm_evidence,
        });

        // 8. Relayer quorum (10 pts) - requires BOTH relayer count AND RPC quorum agreement.
        if actual_relayer_count >= relayer_quorum_requirement && has_rpc_quorum {
            total_score += SCORE_RELAYER_QUORUM;
            categories.push(ScoredCategory {
                name: "relayer_quorum".into(),
                max_points: SCORE_RELAYER_QUORUM,
                earned: SCORE_RELAYER_QUORUM,
                complete: true,
                evidence: format!(
                    "{}/{} relayers attested",
                    actual_relayer_count, relayer_quorum_requirement
                ),
            });
        } else {
            missing_proofs.push("relayer_quorum".to_string());
            categories.push(ScoredCategory {
                name: "relayer_quorum".into(),
                max_points: SCORE_RELAYER_QUORUM,
                earned: 0,
                complete: false,
                evidence: format!(
                    "only {}/{} relayers",
                    actual_relayer_count, relayer_quorum_requirement
                ),
            });
        }

        Self {
            intent_id: record.intent_id,
            categories,
            total_score,
            missing_proofs,
            is_complete: total_score == SCORE_MAX,
        }
    }

    /// Check if the scoreboard is complete (100/100).
    pub fn is_perfect(&self) -> bool {
        self.is_complete
    }

    /// Get a summary string for the scoreboard.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "Scoreboard for intent {}: {}/100",
            self.intent_id, self.total_score
        ));
        for cat in &self.categories {
            let check = if cat.complete { "✓" } else { "✗" };
            lines.push(format!(
                "  {} {}: {}/{}  ({})",
                check, cat.name, cat.earned, cat.max_points, cat.evidence
            ));
        }
        if !self.missing_proofs.is_empty() {
            lines.push(format!(
                "  Missing proofs: {}",
                self.missing_proofs.join(", ")
            ));
        }
        lines.join("\n")
    }

    /// Format the scoreboard as a CLI display string.
    pub fn format_cli(&self) -> alloc::string::String {
        use alloc::format;
        let mut output = alloc::string::String::new();
        output.push_str("\nSWAP SCOREBOARD\n");
        output.push_str(&format!("{}\n", "=".repeat(50)));
        for category in &self.categories {
            let bar = Self::progress_bar(category.earned as u32);
            output.push_str(&format!(
                "{:<30} {} {:>3}/100\n",
                category.name, bar, category.earned
            ));
        }
        output.push_str(&format!("{}\n", "-".repeat(50)));
        let overall_bar = Self::progress_bar(self.total_score as u32);
        output.push_str(&format!(
            "{:<30} {} {:>3}/100\n",
            "Total Score", overall_bar, self.total_score
        ));
        output
    }

    /// Incorporate adapter readiness scores into the total score.
    /// Caps the original score at 70 and adds a bonus from average adapter readiness.
    pub fn with_adapter_scores(mut self, adapter_scores: &[u32]) -> Self {
        let avg_adapter: u32 = if adapter_scores.is_empty() {
            0
        } else {
            adapter_scores.iter().sum::<u32>() / adapter_scores.len() as u32
        };
        self.total_score = self
            .total_score
            .min(70)
            .saturating_add((avg_adapter / 10) as u8);
        self.is_complete = self.total_score >= SCORE_MAX;
        self
    }

    fn progress_bar(score: u32) -> alloc::string::String {
        let filled = ((score + 5) / 10) as usize;
        let filled = filled.min(10);
        let empty = 10 - filled;
        alloc::format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }
}

/// Which chain an adapter settles on.
///
/// This is the distinction the "cross-chain complete" claim kept losing. X3 *runs* EVM, SVM and
/// X3VM as its own execution domains, so their readiness is X3's own and is proven by X3's
/// validators. Every other adapter settles on a chain X3 does not run, where readiness has to be
/// proven against that chain; a perfect internal score says nothing about it. Pooling the two
/// produced one number that read as "cross-chain complete" no matter which side was unready.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SettlementScope {
    /// A domain X3 executes itself: EVM, SVM or X3VM.
    InternalDomain,
    /// A chain X3 does not run; settlement there needs that chain's own proof.
    ExternalChain,
}

impl Default for SettlementScope {
    /// Fail closed: an adapter nobody classified is *not* counted as one of X3's own domains.
    fn default() -> Self {
        SettlementScope::ExternalChain
    }
}

impl SettlementScope {
    /// The scope of a VM type. Only the three domains X3 itself executes are internal.
    pub fn of(vm_type: VmType) -> Self {
        match vm_type {
            VmType::Evm | VmType::Svm | VmType::X3Vm => SettlementScope::InternalDomain,
            _ => SettlementScope::ExternalChain,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SettlementScope::InternalDomain => "internal domain (X3 runs it)",
            SettlementScope::ExternalChain => "external chain (X3 does not run it)",
        }
    }
}

/// A `(earned, max)` pair as a percentage, or 0 when the bucket is empty.
fn percent(bucket: (u32, u32)) -> u32 {
    if bucket.1 > 0 {
        (bucket.0 * 100) / bucket.1
    } else {
        0
    }
}

/// Per-adapter score entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdapterScoreEntry {
    pub vm_type: VmType,
    /// Whether this adapter is one of X3's own execution domains or an external chain.
    ///
    /// `#[serde(default)]` so a scoreboard serialised before this field existed still loads, and the
    /// default is [`SettlementScope::ExternalChain`]: an entry nobody classified is never silently
    /// counted as an internal domain.
    #[serde(default)]
    pub scope: SettlementScope,
    pub adapter_name: String,
    pub score: u32,
    pub max_score: u32, // always 100
    pub missing_capabilities: Vec<String>,
}

/// Aggregate scoreboard for all VM adapters
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdapterScoreboard {
    pub entries: Vec<AdapterScoreEntry>,
    /// Pooled across both scopes. **Not** a cross-chain-completeness claim — read
    /// [`AdapterScoreboard::internal_score`] and [`AdapterScoreboard::external_score`] for that.
    pub overall_score: u32,
    pub max_overall_score: u32,
    /// X3's own execution domains (EVM/SVM/X3VM) only.
    #[serde(default)]
    pub internal_score: u32,
    #[serde(default)]
    pub internal_max_score: u32,
    /// External chains only.
    #[serde(default)]
    pub external_score: u32,
    #[serde(default)]
    pub external_max_score: u32,
    pub timestamp: u64,
}

impl Default for AdapterScoreboard {
    fn default() -> Self {
        Self::new()
    }
}

impl AdapterScoreboard {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            overall_score: 0,
            max_overall_score: 100,
            internal_score: 0,
            internal_max_score: 0,
            external_score: 0,
            external_max_score: 0,
            timestamp: 0,
        }
    }

    /// Compute adapter scores from readiness scores of actual adapters
    pub fn from_adapters(adapters: &[&dyn X3VmAdapter], timestamp: u64) -> Self {
        let entries: Vec<AdapterScoreEntry> = adapters
            .iter()
            .map(|a| {
                let rs = a.readiness_score();
                let vm_type = a.vm_type();
                AdapterScoreEntry {
                    vm_type,
                    scope: SettlementScope::of(vm_type),
                    adapter_name: a.adapter_name().to_string(),
                    score: rs.score(),
                    max_score: 100,
                    missing_capabilities: rs
                        .missing_items()
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                }
            })
            .collect();

        let mut board = Self {
            entries,
            overall_score: 0,
            max_overall_score: 100,
            internal_score: 0,
            internal_max_score: 0,
            external_score: 0,
            external_max_score: 0,
            timestamp,
        };
        board.recompute();
        board
    }

    /// Recompute every aggregate from [`Self::entries`].
    ///
    /// The pooled figure stays for compatibility, but it is not the claim: a chain with three
    /// perfect internal domains and twelve 60%-ready external adapters pools to something that
    /// reads as "mostly ready". `internal_score` and `external_score` are the two numbers a claim
    /// of cross-chain completeness has to cite.
    fn recompute(&mut self) {
        let total: u32 = self.entries.iter().map(|e| e.score).sum();
        let max_total: u32 = (self.entries.len() as u32) * 100;
        self.overall_score = if max_total > 0 {
            (total * 100) / max_total
        } else {
            0
        };

        let mut internal = (0u32, 0u32);
        let mut external = (0u32, 0u32);
        for entry in &self.entries {
            let bucket = match entry.scope {
                SettlementScope::InternalDomain => &mut internal,
                SettlementScope::ExternalChain => &mut external,
            };
            bucket.0 = bucket.0.saturating_add(entry.score);
            bucket.1 = bucket.1.saturating_add(entry.max_score);
        }
        self.internal_score = percent(internal);
        self.internal_max_score = internal.1;
        self.external_score = percent(external);
        self.external_max_score = external.1;
    }

    /// Whether every adapter X3 runs itself is fully ready. Says nothing about external chains.
    pub fn internal_is_ready(&self) -> bool {
        self.internal_max_score > 0 && self.internal_score == 100
    }

    /// Whether every external-chain adapter is fully ready — the bar "cross-chain complete" needs.
    pub fn external_is_ready(&self) -> bool {
        self.external_max_score > 0 && self.external_score == 100
    }

    /// The chain's default adapter view, **derived from the adapters themselves**.
    ///
    /// This used to be a hand-written table of 16 entries, and by 2026-09-27 it had drifted into
    /// advertising capabilities the adapters no longer declare: `x3-adapter-substrate` at 80,
    /// `x3-adapter-x3vm` at 100 and `x3-adapter-bitcoin` at 80, while those adapters'
    /// `readiness_score()` had been corrected to 60/70 with the missing capabilities named. Nothing
    /// tied the table to the code, so nothing could make it disagree — the same "the scoreboard
    /// feeds the claim" defect this row is about, one layer up. There is one source now:
    /// [`crate::adapter::all_adapters`], the same inventory the cross-adapter proof tests use.
    pub fn default_scoreboard() -> Self {
        let adapters = crate::adapter::all_adapters();
        let refs: Vec<&dyn X3VmAdapter> = adapters.iter().map(|a| a.as_ref()).collect();
        Self::from_adapters(&refs, 0)
    }

    /// Format the scoreboard as a display string (CLI output)
    pub fn format_cli(&self) -> alloc::string::String {
        use alloc::format;
        let mut output = alloc::string::String::new();
        output.push_str("\nX3 CROSS-VM ADAPTER SCOREBOARD\n");
        output.push_str(&format!("{}\n", "=".repeat(60)));
        for entry in &self.entries {
            let bar = Self::progress_bar(entry.score);
            output.push_str(&format!(
                "{:<20} {} {:>3}/100\n",
                entry.adapter_name, bar, entry.score
            ));
            if !entry.missing_capabilities.is_empty() {
                output.push_str(&format!(
                    "  Missing: {}\n",
                    entry.missing_capabilities.join(", ")
                ));
            }
        }
        output.push_str(&format!("{}\n", "-".repeat(60)));
        let overall_bar = Self::progress_bar(self.overall_score);
        output.push_str(&format!(
            "{:<20} {} {:>3}/100\n",
            "Overall (pooled)", overall_bar, self.overall_score
        ));
        // The pooled line is not a cross-chain claim; these two are. Printing only "Overall" is how
        // a board with unready external chains still read as complete.
        let internal_bar = Self::progress_bar(self.internal_score);
        output.push_str(&format!(
            "{:<20} {} {:>3}/100  internal domains\n",
            "X3's own domains", internal_bar, self.internal_score
        ));
        let external_bar = Self::progress_bar(self.external_score);
        output.push_str(&format!(
            "{:<20} {} {:>3}/100  external chains\n",
            "External chains", external_bar, self.external_score
        ));
        output
    }

    pub(crate) fn progress_bar(score: u32) -> alloc::string::String {
        let filled = ((score + 5) / 10) as usize;
        let filled = filled.min(10);
        let empty = 10 - filled;
        use alloc::format;
        format!("{}{}", "█".repeat(filled), "░".repeat(empty))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{ProofFinalStatus, ProofRecord};

    fn make_complete_record(intent_id: u64, record_id: u64) -> ProofRecord {
        let mut record = ProofRecord::new(record_id, intent_id, "relayer-1".into(), 1000);
        record.source_lock_tx = Some("0xsource".into());
        record.source_lock_block = Some(100);
        record.destination_lock_tx = Some("0xdest".into());
        record.destination_lock_block = Some(200);
        record.hashlock_match = true;
        record.timeout_order_valid = true;
        record.finality_verified = true;
        record.secret_reveal_tx = Some("0xreveal".into());
        record.claim_tx = Some("0xclaim".into());
        record.claim_block = Some(300);
        record.final_status = Some(ProofFinalStatus::Completed);
        record
    }

    fn make_partial_record(intent_id: u64, record_id: u64) -> ProofRecord {
        let mut record = ProofRecord::new(record_id, intent_id, "relayer-1".into(), 1000);
        record.source_lock_tx = Some("0xsource".into());
        record.source_lock_block = Some(100);
        // Missing: destination_lock, hashlock_match, timeout, finality, reveal, claim
        record
    }

    #[test]
    fn test_scoreboard_complete_claim_100() {
        let record = make_complete_record(1, 1);
        let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 3, true);
        assert!(
            scoreboard.is_perfect(),
            "complete record should score 100: {}",
            scoreboard.total_score
        );
        assert_eq!(scoreboard.total_score, 100);
        assert!(scoreboard.missing_proofs.is_empty());
    }

    #[test]
    fn test_scoreboard_complete_refund_100() {
        let mut record = make_complete_record(1, 1);
        record.claim_tx = None; // Remove claim
        record.secret_reveal_tx = None; // Refund path doesn't need reveal
        record.refund_tx = Some("0xrefund".into());
        record.refund_block = Some(400);
        let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 3, true);
        // Source(20) + Dest(20) + Hashlock(10) + Timeout(10) + Finality(10)
        // + Refund(10) + Reveal acked as not-applicable(10) + Quorum(10) = 100
        assert_eq!(scoreboard.total_score, 100);
        assert!(scoreboard.is_perfect());
        assert!(!scoreboard
            .missing_proofs
            .contains(&"secret_reveal_tx".to_string()));
    }

    #[test]
    fn test_scoreboard_partial_no_dest_lock() {
        let record = make_partial_record(1, 1);
        let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 0, false);
        // Only source lock (20) should be earned
        assert_eq!(scoreboard.total_score, 20);
        assert!(!scoreboard.is_perfect());
        assert!(!scoreboard.missing_proofs.is_empty());
    }

    #[test]
    fn test_scoreboard_missing_claim_with_refund_without_tx() {
        let mut record = make_complete_record(1, 1);
        record.claim_tx = None;
        record.refund_tx = None; // Neither claim nor refund has tx hash
        let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 3, true);
        // Should have missing_proofs containing claim_tx_or_refund_tx
        assert!(scoreboard
            .missing_proofs
            .contains(&"claim_tx_or_refund_tx".to_string()));
        assert!(!scoreboard.is_perfect());
    }

    #[test]
    fn test_scoreboard_relayer_quorum_not_met() {
        let record = make_complete_record(1, 1);
        let scoreboard = SwapScoreboard::from_proof_record(&record, 5, 3, false);
        assert!(!scoreboard.is_perfect());
        assert_eq!(scoreboard.total_score, 90); // Missing relayer_quorum (10)
        assert!(scoreboard
            .missing_proofs
            .contains(&"relayer_quorum".to_string()));
    }

    #[test]
    fn test_scoreboard_relayer_count_ok_but_no_rpc_quorum() {
        // Regression: max score without RPC quorum must be 90 even with enough relayers.
        let record = make_complete_record(1, 1);
        let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 5, false);
        assert!(!scoreboard.is_perfect());
        assert_eq!(scoreboard.total_score, 90);
        assert!(scoreboard
            .missing_proofs
            .contains(&"relayer_quorum".to_string()));
    }

    #[test]
    fn test_scoreboard_summary_format() {
        let record = make_complete_record(1, 1);
        let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 3, true);
        let summary = scoreboard.summary();
        assert!(summary.contains("100/100"));
        assert!(summary.contains("✓"));
    }

    #[test]
    fn test_scoreboard_cannot_reach_100_without_tx_proof() {
        // Intentional test: scoreboard must have tx hash for every required step
        let mut record = make_complete_record(1, 1);
        record.source_lock_tx = None; // Remove source lock tx
        let scoreboard = SwapScoreboard::from_proof_record(&record, 3, 3, true);
        // Max possible: 100 - 20 (source lock) = 80
        assert_eq!(scoreboard.total_score, 80);
        assert!(!scoreboard.is_perfect());
        assert!(scoreboard
            .missing_proofs
            .contains(&"source_lock_tx".to_string()));
    }

    // ── Internal domains versus external chains ─────────────────────────────
    //
    // X3-CLAIM-003's blocker: "cross-chain complete" pooled the three domains X3 runs itself with
    // every external-chain adapter, so a board with unready external chains still read as complete.

    #[test]
    fn only_the_three_domains_x3_runs_are_internal() {
        for vm_type in [VmType::Evm, VmType::Svm, VmType::X3Vm] {
            assert_eq!(
                SettlementScope::of(vm_type),
                SettlementScope::InternalDomain,
                "{vm_type:?} is a domain X3 executes"
            );
        }
        for vm_type in [
            VmType::Substrate,
            VmType::BitcoinScript,
            VmType::MoveVm,
            VmType::CosmWasm,
            VmType::CairoVm,
            VmType::PlutusEutxo,
            VmType::TonTvm,
            VmType::FuelVm,
            VmType::NearWasm,
            VmType::SorobanWasm,
            VmType::PolkadotPvm,
            VmType::InkWasm,
            VmType::WasmL1,
            VmType::ZkVm,
        ] {
            assert_eq!(
                SettlementScope::of(vm_type),
                SettlementScope::ExternalChain,
                "{vm_type:?} settles on a chain X3 does not run"
            );
        }
    }

    /// The load-bearing one: adding an external adapter must not move the internal score, and vice
    /// versa. Numbers come from the adapters' own declarations, not from this test.
    #[test]
    fn an_external_adapter_never_moves_the_internal_score() {
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let btc =
            crate::bitcoin_htlc::BtcHtlcAdapter::new(crate::bitcoin_htlc::BitcoinNetwork::Mainnet);

        let internal_only = AdapterScoreboard::from_adapters(&[&evm], 1);
        let with_external = AdapterScoreboard::from_adapters(&[&evm, &btc], 1);

        assert_eq!(internal_only.internal_score, evm.readiness_score().score());
        assert_eq!(internal_only.internal_max_score, 100);
        assert_eq!(
            internal_only.external_max_score, 0,
            "no external adapter yet"
        );

        assert_eq!(
            with_external.internal_score, internal_only.internal_score,
            "an external adapter's readiness must not raise or lower X3's own domains"
        );
        assert_eq!(with_external.internal_max_score, 100);
        assert_eq!(with_external.external_max_score, 100);
        assert_eq!(
            with_external.external_score,
            btc.readiness_score().score(),
            "the external bucket holds exactly the external adapter"
        );
    }

    #[test]
    fn an_unclassified_entry_is_external_not_internal() {
        // The serde default is what a scoreboard written before the field existed deserialises to,
        // and it must fail closed.
        assert_eq!(
            SettlementScope::default(),
            SettlementScope::ExternalChain,
            "an entry nobody classified must not be counted as one of X3's own domains"
        );
        let json = r#"{
            "entries": [{
                "vm_type": "X3Vm",
                "adapter_name": "an-old-entry",
                "score": 100,
                "max_score": 100,
                "missing_capabilities": []
            }],
            "overall_score": 100,
            "max_overall_score": 100,
            "timestamp": 0
        }"#;
        let board: AdapterScoreboard = serde_json::from_str(json).expect("old shape must load");
        assert_eq!(board.entries[0].scope, SettlementScope::ExternalChain);
        assert_eq!(
            board.internal_max_score, 0,
            "a legacy entry must not populate the internal bucket just because its vm_type is X3VM"
        );
    }

    /// The default view cannot disagree with the adapters, because it *is* the adapters' view.
    #[test]
    fn the_default_scoreboard_agrees_with_every_adapters_own_declaration() {
        let board = AdapterScoreboard::default_scoreboard();
        let adapters = crate::adapter::all_adapters();
        assert_eq!(
            board.entries.len(),
            adapters.len(),
            "one entry per adapter, no hand-written extras"
        );

        for entry in &board.entries {
            let adapter = adapters
                .iter()
                .find(|a| a.adapter_name() == entry.adapter_name)
                .unwrap_or_else(|| panic!("{} is not an adapter", entry.adapter_name));
            let declared = adapter.readiness_score();
            assert_eq!(
                entry.score,
                declared.score(),
                "{} advertises {} but declares {}",
                entry.adapter_name,
                entry.score,
                declared.score()
            );
            assert_eq!(entry.vm_type, adapter.vm_type());
            assert_eq!(entry.scope, SettlementScope::of(adapter.vm_type()));
            assert_eq!(
                entry.missing_capabilities.len(),
                declared.missing_items().len(),
                "{} lists different missing capabilities than it declares",
                entry.adapter_name
            );
        }

        // Three internal domains (EVM, SVM, X3VM) and thirteen external chains.
        assert_eq!(board.internal_max_score, 300);
        assert_eq!(board.external_max_score, 1_300);
        assert!(
            !board.external_is_ready(),
            "no external-chain adapter is fully ready, so the board must not claim it"
        );
        assert!(
            !board.internal_is_ready(),
            "and X3's own domains are not fully ready either"
        );
        // Measured 2026-09-27, and the opposite of what this test first assumed: internal 56,
        // external 60. X3's own three domains are *not* the strong side of this board — the
        // corrected external adapters declare 60-80 while EVM/SVM/X3VM declare the same kind of
        // gaps. The assertion is the shape (both sides reported, neither ready), not the numbers,
        // because those belong to the adapters.
        assert!(
            board.internal_score < 100 && board.external_score < 100,
            "internal {} and external {} must both be reported and both be short of 100",
            board.internal_score,
            board.external_score
        );
    }

    /// The pooled figure is exactly the thing the claim must not cite on its own.
    #[test]
    fn the_pooled_score_is_a_blend_that_hides_whichever_side_is_unready() {
        let evm = crate::evm_htlc::EvmAdapter::at_address([0x01u8; 20]);
        let btc =
            crate::bitcoin_htlc::BtcHtlcAdapter::new(crate::bitcoin_htlc::BitcoinNetwork::Mainnet);
        let board = AdapterScoreboard::from_adapters(&[&evm, &btc], 1);

        let internal = board.internal_score;
        let external = board.external_score;
        let pooled = board.overall_score;

        assert_eq!(internal, evm.readiness_score().score());
        assert_eq!(external, btc.readiness_score().score());
        assert!(
            pooled >= internal.min(external) && pooled <= internal.max(external),
            "the pooled figure must sit between the two sides it blends: pooled {pooled}, \
             internal {internal}, external {external}"
        );
        if internal != external {
            assert!(
                pooled != external && pooled != internal,
                "pooling the two sides must not reproduce either one: pooled {pooled}, \
                 internal {internal}, external {external}"
            );
        }
        assert!(
            !board.external_is_ready(),
            "the external side is not ready, and the pooled line cannot say otherwise"
        );
    }
}
