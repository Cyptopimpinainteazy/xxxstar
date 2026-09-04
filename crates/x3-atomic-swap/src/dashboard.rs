//! # Atomic Command Center - Dashboard for Atomic Swap Operations
//!
//! Aggregates active intent state, proof ledger data, and watcher alerts
//! into a single snapshot suitable for UI rendering or programmatic consumption.
//!
//! Also provides the Chaos Test Scoreboard for displaying test scenario status.

use crate::adapter::X3VmAdapter;
use crate::intent::AtomicIntent;
use crate::intent::AtomicSwapStatus;
use crate::ledger::ProofLedger;
use crate::registry::{RelayerRegistry, SolverRegistry};
use crate::relayer::{scan_for_alerts, WatcherAlert};
use crate::scoreboard::{AdapterScoreboard, SwapScoreboard};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// A per-step transaction link for a swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLink {
    /// Step name (e.g. "Source Lock", "Destination Lock", "Claim").
    pub step: String,
    /// Chain identifier (e.g. "eth", "sol").
    pub chain: String,
    /// Transaction hash, if known.
    pub tx_hash: Option<String>,
    /// Explorer URL, if available.
    pub url: Option<String>,
}

/// Result of a single chaos test scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosTestResult {
    /// Scenario name (e.g. "Wrong preimage rejected").
    pub name: String,
    /// Whether the test scenario passed.
    /// None means unknown/unrun — no assertion has been recorded.
    pub passed: Option<bool>,
    /// Human-readable description of what is tested.
    pub description: String,
}

/// Chaos Test Scoreboard showing status of all test scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosTestScoreboard {
    /// All test scenarios.
    pub test_scenarios: Vec<ChaosTestResult>,
    /// Overall score percentage (0-100).
    pub total_score: u8,
    /// Summary string (e.g. "Score: 8 passed, 0 failed, 2 unknown / 10").
    pub summary: String,
}

impl ChaosTestScoreboard {
    /// Build a scoreboard from actual scenario results.
    ///
    /// Scenarios whose `passed` is `None` are treated as unknown/unrun.
    /// The summary reflects how many passed, failed, and are unknown.
    pub fn from_results(results: Vec<ChaosTestResult>) -> Self {
        let total_count = results.len();
        let passed_count = results.iter().filter(|s| s.passed == Some(true)).count();
        let failed_count = results.iter().filter(|s| s.passed == Some(false)).count();
        let unknown_count = results.iter().filter(|s| s.passed.is_none()).count();

        // Score = passed / (total - unknown) * 100 when at least one is known.
        // If nothing is known, score is 0.
        let known_count = passed_count + failed_count;
        let total_score = if known_count > 0 {
            ((passed_count as u16 * 100) / known_count as u16) as u8
        } else {
            0
        };

        let summary = if unknown_count > 0 {
            format!(
                "Score: {} passed, {} failed, {} unknown / {}",
                passed_count, failed_count, unknown_count, total_count
            )
        } else {
            format!("Score: {}/{}", passed_count, total_count)
        };

        Self {
            test_scenarios: results,
            total_score,
            summary,
        }
    }

    /// Render the scoreboard as a formatted string matching the spec format.
    pub fn render_scoreboard(&self) -> String {
        let mut lines = Vec::new();
        lines.push("CHAOS TEST SCOREBOARD".into());
        for scenario in &self.test_scenarios {
            let mark: String = match scenario.passed {
                Some(true) => "[✓]".into(),
                Some(false) => "[✗]".into(),
                None => "[?]".into(),
            };
            lines.push(format!("{} {}", mark, scenario.name));
        }
        lines.push(self.summary.clone());
        lines.join("\n")
    }
}

/// A detail view for a single atomic swap intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapDetail {
    /// Intent identifier.
    pub intent_id: u64,
    /// Human-readable route (e.g. "eth.USDC -> sol.SOL").
    pub route: String,
    /// Amount in source asset base units.
    pub amount: u128,
    /// Current lifecycle status.
    pub status: AtomicSwapStatus,
    /// Seconds remaining until source timeout (negative = expired).
    pub timeout_countdown_secs: i64,
    /// Current proof score (0-100).
    pub proof_score: u8,
    /// List of missing proof names.
    pub missing_proofs: Vec<String>,
    /// Per-step transaction links.
    pub tx_links: Vec<TxLink>,
    /// Relayer IDs assigned to this swap.
    pub relayers_assigned: Vec<String>,
    /// Solver IDs assigned to this swap.
    pub solvers_assigned: Vec<String>,
    /// Whether the claim button should be enabled (true when status is Claimable).
    pub claim_button_enabled: bool,
    /// Whether the refund button should be enabled (true when status is Refundable or Expired).
    pub refund_button_enabled: bool,
}

impl SwapDetail {
    /// Create a new SwapDetail from parts.
    pub fn new(
        intent_id: u64,
        route: String,
        amount: u128,
        status: AtomicSwapStatus,
        timeout_countdown_secs: i64,
        proof_score: u8,
        missing_proofs: Vec<String>,
        tx_links: Vec<TxLink>,
        relayers_assigned: Vec<String>,
        solvers_assigned: Vec<String>,
    ) -> Self {
        let claim_button_enabled = status == AtomicSwapStatus::Claimable;
        let refund_button_enabled = matches!(
            status,
            AtomicSwapStatus::Refundable | AtomicSwapStatus::Expired
        );
        Self {
            intent_id,
            route,
            amount,
            status,
            timeout_countdown_secs,
            proof_score,
            missing_proofs,
            tx_links,
            relayers_assigned,
            solvers_assigned,
            claim_button_enabled,
            refund_button_enabled,
        }
    }
}

/// An aggregated snapshot of all active atomic swaps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapDashboardSnapshot {
    /// Number of swaps currently in a non-terminal state.
    pub active_swaps: u64,
    /// Number of swaps that completed (Claimed).
    pub completed_swaps: u64,
    /// Number of intents with a MissingClaim or MissingDestinationLock alert.
    pub stuck_swaps: u64,
    /// Number of intents in a refundable state.
    pub refundable_swaps: u64,
    /// Number of failed intents.
    pub failed_swaps: u64,
    /// Average time from creation to completion (seconds).
    pub average_fill_time_secs: f64,
    /// Total notional volume across all swaps in source-asset base units.
    /// These are raw units (USDC, SOL, BTC, etc.) — not USD.
    /// USD-normalized totals require injected pricing data.
    pub total_volume_notional: u128,
    /// Relayer uptime percentage (None = not yet measured).
    pub relayer_uptime_pct: Option<f64>,
    /// Solver failure rate percentage (None = not yet measured).
    pub solver_failure_rate_pct: Option<f64>,
    /// Insurance fund value (None = not yet tracked).
    pub insurance_fund_usd: Option<u128>,
    /// Active watcher alerts.
    pub alerts: Vec<WatcherAlert>,
    /// Per-swap detail entries.
    pub details: Vec<SwapDetail>,
}

impl SwapDashboardSnapshot {
    /// Return a formatted dashboard summary string matching the spec format.
    pub fn dashboard_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push("X3 Atomic Command Center".into());
        lines.push(format!("Active swaps: {}", self.active_swaps));
        lines.push(format!("Completed swaps: {}", self.completed_swaps));
        lines.push(format!("Stuck swaps: {}", self.stuck_swaps));
        lines.push(format!("Refundable swaps: {}", self.refundable_swaps));
        lines.push(format!("Failed swaps: {}", self.failed_swaps));
        if self.average_fill_time_secs > 0.0 {
            lines.push(format!(
                "Average fill time: {:.0}s",
                self.average_fill_time_secs
            ));
        } else {
            lines.push("Average fill time: N/A".into());
        }
        if let Some(uptime) = self.relayer_uptime_pct {
            lines.push(format!("Relayer uptime: {:.1}%", uptime));
        } else {
            lines.push("Relayer uptime: N/A".into());
        }
        if let Some(solver_fail) = self.solver_failure_rate_pct {
            lines.push(format!("Solver failure rate: {:.1}%", solver_fail));
        } else {
            lines.push("Solver failure rate: N/A".into());
        }
        // total_volume_notional is raw source-asset units, NOT USD.
        // Label as "notional" to avoid misleading consumers.
        lines.push(format!(
            "Total notional volume (raw): {}",
            self.total_volume_notional
        ));
        if let Some(insurance) = self.insurance_fund_usd {
            lines.push(format!("Insurance fund: ${}", insurance));
        } else {
            lines.push("Insurance fund: N/A".into());
        }
        lines.join("\n")
    }
}

/// The Atomic Command Center builds [`SwapDashboardSnapshot`] from
/// a set of intents and the proof ledger.
///
/// Also provides adapter scoreboard and status report output.
pub struct AtomicCommandCenter {
    /// All tracked atomic swap intents.
    pub intents: Vec<AtomicIntent>,
    /// The proof ledger (shared across all intents).
    pub ledger: ProofLedger,
    /// Registered solvers.
    pub solver_registry: SolverRegistry,
    /// Registered relayers.
    pub relayer_registry: RelayerRegistry,
    /// Live VM adapters for readiness scoring.
    pub adapters: Vec<Box<dyn X3VmAdapter>>,
}

impl Default for AtomicCommandCenter {
    fn default() -> Self {
        Self {
            intents: Vec::new(),
            ledger: ProofLedger::new(),
            solver_registry: SolverRegistry::new(),
            relayer_registry: RelayerRegistry::new(),
            adapters: Vec::new(),
        }
    }
}

impl AtomicCommandCenter {
    /// Build a dashboard snapshot from the current intent set and ledger.
    ///
    /// # Parameters
    /// - `intents` - all tracked atomic swap intents
    /// - `ledger` - the proof ledger (shared across all intents)
    /// - `now` - current unix timestamp
    pub fn build(
        intents: &[AtomicIntent],
        ledger: &ProofLedger,
        now: u64,
    ) -> SwapDashboardSnapshot {
        let mut active_swaps: u64 = 0;
        let mut completed_swaps: u64 = 0;
        let mut refundable_swaps: u64 = 0;
        let mut failed_swaps: u64 = 0;
        let mut total_volume: u128 = 0;

        let alerts = scan_for_alerts(intents, ledger, now, 300); // 5 min warning

        let mut stuck_swaps: u64 = 0;
        for alert in &alerts {
            match alert {
                WatcherAlert::MissingClaim { .. } | WatcherAlert::MissingDestinationLock { .. } => {
                    stuck_swaps += 1;
                }
                _ => {}
            }
        }

        let mut details = Vec::new();

        for intent in intents {
            let status = intent.status;
            // Accumulate raw source-asset notional volume (not USD).
            // Pricing/normalization must be injected by the consumer.
            total_volume += intent.amount_in;

            match status {
                AtomicSwapStatus::Claimed | AtomicSwapStatus::Completed => {
                    completed_swaps += 1;
                }
                AtomicSwapStatus::Failed => {
                    failed_swaps += 1;
                }
                _ if status.is_terminal() => {
                    // Refunded is terminal but not failed
                }
                _ => {
                    active_swaps += 1;

                    if status == AtomicSwapStatus::Refundable
                        || status == AtomicSwapStatus::Refunding
                    {
                        refundable_swaps += 1;
                    }
                }
            }

            let timeout_countdown = if intent.source_timeout > now {
                (intent.source_timeout - now) as i64
            } else {
                -((now - intent.source_timeout) as i64)
            };

            // Build an honest per-intent scoreboard from the latest proof record.
            let (proof_score, missing_proofs) =
                if let Some(latest) = ledger.get_latest_for_intent(intent.intent_id) {
                    // Count unique relayers for this intent
                    let relayers_count = ledger
                        .get_records_for_intent(intent.intent_id)
                        .iter()
                        .map(|r| r.relayer_id.as_str())
                        .collect::<std::collections::HashSet<&str>>()
                        .len() as u32;
                    let has_rpc_quorum = ledger.has_rpc_quorum_for_intent(intent.intent_id);
                    let sb = SwapScoreboard::from_proof_record(
                        latest,
                        intent.relayer_quorum_requirement,
                        relayers_count,
                        has_rpc_quorum,
                    );
                    let missing: Vec<String> = sb.missing_proofs.to_vec();
                    (sb.total_score, missing)
                } else {
                    // No records yet - everything is missing
                    let missing: Vec<String> = crate::ledger::ProofKind::required_for_success()
                        .iter()
                        .map(|k| k.display_name().to_string())
                        .collect();
                    (0u8, missing)
                };

            let route = format!(
                "{}.{} -> {}.{}",
                intent.source_chain.as_str(),
                intent.source_asset,
                intent.destination_chain.as_str(),
                intent.destination_asset,
            );

            // Build tx_links from ledger records for this intent
            let mut tx_links = Vec::new();
            for record in &ledger.records {
                if record.intent_id == intent.intent_id {
                    if let Some(ref tx) = record.source_lock_tx {
                        tx_links.push(TxLink {
                            step: "Source Lock".into(),
                            chain: intent.source_chain.as_str().into(),
                            tx_hash: Some(tx.clone()),
                            url: None,
                        });
                    }
                    if let Some(ref tx) = record.destination_lock_tx {
                        tx_links.push(TxLink {
                            step: "Destination Lock".into(),
                            chain: intent.destination_chain.as_str().into(),
                            tx_hash: Some(tx.clone()),
                            url: None,
                        });
                    }
                    if let Some(ref tx) = record.secret_reveal_tx {
                        tx_links.push(TxLink {
                            step: "Secret Reveal".into(),
                            chain: intent.source_chain.as_str().into(),
                            tx_hash: Some(tx.clone()),
                            url: None,
                        });
                    }
                    if let Some(ref tx) = record.claim_tx {
                        tx_links.push(TxLink {
                            step: "Claim".into(),
                            chain: intent.destination_chain.as_str().into(),
                            tx_hash: Some(tx.clone()),
                            url: None,
                        });
                    }
                    if let Some(ref tx) = record.refund_tx {
                        tx_links.push(TxLink {
                            step: "Refund".into(),
                            chain: intent.source_chain.as_str().into(),
                            tx_hash: Some(tx.clone()),
                            url: None,
                        });
                    }
                }
            }

            details.push(SwapDetail::new(
                intent.intent_id,
                route,
                intent.amount_in,
                status,
                timeout_countdown,
                proof_score,
                missing_proofs,
                tx_links,
                Vec::new(), // relayers_assigned - populated externally
                Vec::new(), // solvers_assigned - populated externally
            ));
        }

        SwapDashboardSnapshot {
            active_swaps,
            completed_swaps,
            stuck_swaps,
            refundable_swaps,
            failed_swaps,
            average_fill_time_secs: ledger.compute_average_fill_time_secs(),
            total_volume_notional: total_volume,
            // Derive relayer uptime from the registry (or leave unavailable).
            relayer_uptime_pct: None, // TODO: populate from relayer-registry heartbeat data
            solver_failure_rate_pct: None, // TODO: aggregate from solver-registry history
            insurance_fund_usd: None, // TODO: wire to on-chain insurance pool balance
            alerts,
            details,
        }
    }

    /// Generate the full adapter scoreboard output
    pub fn adapter_scoreboard(&self) -> alloc::string::String {
        let scoreboard = if self.adapters.is_empty() {
            AdapterScoreboard::default()
        } else {
            let refs: Vec<&dyn X3VmAdapter> = self.adapters.iter().map(|b| b.as_ref()).collect();
            AdapterScoreboard::from_adapters(&refs, 0)
        };
        scoreboard.format_cli()
    }

    /// Format a human-readable adapter scoreboard with live vs default indicator.
    pub fn format_adapter_scoreboard(&self) -> alloc::string::String {
        use alloc::format;
        let mut output = alloc::string::String::new();
        output.push_str("\nX3 ADAPTER READINESS SCOREBOARD\n");
        output.push_str(&format!("{}\n", "=".repeat(60)));

        if self.adapters.is_empty() {
            output.push_str("  Using DEFAULT scores (no live adapters)\n");
            let scoreboard = AdapterScoreboard::default();
            for entry in &scoreboard.entries {
                let bar = AdapterScoreboard::progress_bar(entry.score);
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
            let overall_bar = AdapterScoreboard::progress_bar(scoreboard.overall_score);
            output.push_str(&format!(
                "{:<20} {} {:>3}/100 (DEFAULT)\n",
                "Overall", overall_bar, scoreboard.overall_score
            ));
        } else {
            output.push_str(&format!(
                "  LIVE adapter scores ({})\n",
                self.adapters.len()
            ));
            let refs: Vec<&dyn X3VmAdapter> = self.adapters.iter().map(|b| b.as_ref()).collect();
            let scoreboard = AdapterScoreboard::from_adapters(&refs, 0);
            for entry in &scoreboard.entries {
                let bar = AdapterScoreboard::progress_bar(entry.score);
                let live = if entry.score >= 80 { "✓" } else { "⚠" };
                output.push_str(&format!(
                    "{} {:<20} {} {:>3}/100\n",
                    live, entry.adapter_name, bar, entry.score
                ));
                if !entry.missing_capabilities.is_empty() {
                    output.push_str(&format!(
                        "  Missing: {}\n",
                        entry.missing_capabilities.join(", ")
                    ));
                }
            }
            output.push_str(&format!("{}\n", "-".repeat(60)));
            let overall_bar = AdapterScoreboard::progress_bar(scoreboard.overall_score);
            output.push_str(&format!(
                "{:<20} {} {:>3}/100 (LIVE)\n",
                "Overall", overall_bar, scoreboard.overall_score
            ));
        }
        output
    }

    /// Create an AtomicCommandCenter with custom adapters.
    pub fn with_adapters(adapters: Vec<Box<dyn X3VmAdapter>>) -> Self {
        Self {
            intents: Vec::new(),
            ledger: ProofLedger::new(),
            solver_registry: SolverRegistry::new(),
            relayer_registry: RelayerRegistry::new(),
            adapters,
        }
    }

    /// Generate a complete status report
    pub fn status_report(&self) -> alloc::string::String {
        use alloc::format;
        let mut report = alloc::string::String::new();
        report.push_str("X3 ATOMIC ENGINE STATUS REPORT\n");
        report.push_str(&format!("{}\n", "=".repeat(60)));
        report.push_str(&format!("Active Intents: {}\n", self.intents.len()));
        report.push_str(&format!("Ledger Records: {}\n", self.ledger.records.len()));
        report.push_str(&format!(
            "Solvers: {}\n",
            self.solver_registry.solvers.len()
        ));
        report.push_str(&format!(
            "Relayers: {}\n",
            self.relayer_registry.relayers.len()
        ));

        // Count intents by status
        let mut status_counts: BTreeMap<String, usize> = BTreeMap::new();
        for intent in &self.intents {
            let key = format!("{:?}", intent.status);
            *status_counts.entry(key).or_insert(0) += 1;
        }
        report.push_str("\nIntent Status Distribution:\n");
        for (status, count) in &status_counts {
            report.push_str(&format!("  {}: {}\n", status, count));
        }

        report.push_str(&format!("\n{}", self.adapter_scoreboard()));
        report
    }
}

/// Return a default ChaosTestScoreboard with all 10 spec scenarios
/// marked as unknown (unrun). This is the honest default — it never
/// fabricates a passing score.
///
/// Consumers that have actual chaos/integration test results should use
/// [`ChaosTestScoreboard::from_results`] instead.
pub fn chaos_scoreboard_default() -> ChaosTestScoreboard {
    let scenarios = vec![
        ChaosTestResult {
            name: "Wrong preimage rejected".into(),
            passed: None,
            description: "Swap claim must reject invalid preimage on both EVM and SVM".into(),
        },
        ChaosTestResult {
            name: "Late secret rejected".into(),
            passed: None,
            description: "Claim after timeout must fail; refund must succeed".into(),
        },
        ChaosTestResult {
            name: "Expired refund works".into(),
            passed: None,
            description: "After timeout, refund must succeed and subsequent claim must fail".into(),
        },
        ChaosTestResult {
            name: "Relayer offline fallback works".into(),
            passed: None,
            description:
                "Scoreboard reports missing relayer quorum when insufficient relayers attest".into(),
        },
        ChaosTestResult {
            name: "Solver disappears".into(),
            passed: None,
            description: "Swap must handle solver disappearance gracefully with timeout escalation"
                .into(),
        },
        ChaosTestResult {
            name: "Source chain reorg".into(),
            passed: None,
            description: "Reorg on source chain must be detected and scored accordingly".into(),
        },
        ChaosTestResult {
            name: "Destination tx fails".into(),
            passed: None,
            description: "Destination chain transaction failure must trigger refund path".into(),
        },
        ChaosTestResult {
            name: "RPC disagreement detected".into(),
            passed: None,
            description: "RPC quorum proof reports not agreed when agreement < required quorum"
                .into(),
        },
        ChaosTestResult {
            name: "Gas spikes".into(),
            passed: None,
            description: "Swap must handle gas price spikes without failing".into(),
        },
        ChaosTestResult {
            name: "Claim front-run attempt".into(),
            passed: None,
            description: "Front-running claim must be rejected and scored".into(),
        },
    ];

    ChaosTestScoreboard::from_results(scenarios)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{AtomicIntentBuilder, AtomicSwapStatus, ChainKind, RefundPath};
    use sha2::{Digest, Sha256};

    fn make_hashlock(preimage: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(preimage);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    fn make_intent(id: u64) -> AtomicIntent {
        AtomicIntentBuilder::new()
            .source_chain(ChainKind::Ethereum)
            .destination_chain(ChainKind::Solana)
            .source_asset("USDC")
            .destination_asset("SOL")
            .amount_in(1000)
            .min_amount_out(950)
            .receiver("sol_receiver")
            .hashlock(make_hashlock(b"preimage"))
            .source_timeout(6000)
            .destination_timeout(5000)
            .refund_path(RefundPath {
                chain: ChainKind::Ethereum,
                address: "0xrefund".into(),
                asset: None,
            })
            .relayer_quorum(3)
            .build(id)
            .expect("intent should build")
    }

    #[test]
    fn test_dashboard_snapshot_basic() {
        let intents = vec![make_intent(1), make_intent(2)];
        let ledger = ProofLedger::new();
        let now = 1000;

        let snapshot = AtomicCommandCenter::build(&intents, &ledger, now);
        assert_eq!(snapshot.active_swaps, 2);
        assert_eq!(snapshot.completed_swaps, 0);
        assert_eq!(snapshot.details.len(), 2);
        assert!(snapshot.details[0].timeout_countdown_secs > 0);
        assert!(!snapshot.details[0].claim_button_enabled);
        assert!(!snapshot.details[0].refund_button_enabled);
    }

    #[test]
    fn test_dashboard_snapshot_mixed_statuses() {
        let mut intents = vec![make_intent(1), make_intent(2), make_intent(3)];
        // Mark intent 2 as claimed
        intents[1].status = AtomicSwapStatus::Claimed;
        // Mark intent 3 as failed
        intents[2].status = AtomicSwapStatus::Failed;

        let ledger = ProofLedger::new();
        let snapshot = AtomicCommandCenter::build(&intents, &ledger, 1000);

        assert_eq!(snapshot.active_swaps, 1);
        assert_eq!(snapshot.completed_swaps, 1);
        assert_eq!(snapshot.failed_swaps, 1);
    }

    #[test]
    fn test_claim_button_enabled_only_for_claimable() {
        assert!(
            SwapDetail::new(
                1,
                "eth.USDC->sol.SOL".into(),
                1000,
                AtomicSwapStatus::Claimable,
                100,
                50,
                vec![],
                vec![],
                vec![],
                vec![]
            )
            .claim_button_enabled
        );
        assert!(
            !SwapDetail::new(
                1,
                "eth.USDC->sol.SOL".into(),
                1000,
                AtomicSwapStatus::SourceLocked,
                100,
                50,
                vec![],
                vec![],
                vec![],
                vec![]
            )
            .claim_button_enabled
        );
    }

    #[test]
    fn test_refund_button_enabled_for_refundable_or_expired() {
        let refundable = SwapDetail::new(
            1,
            "".into(),
            0,
            AtomicSwapStatus::Refundable,
            0,
            0,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        assert!(refundable.refund_button_enabled);

        let expired = SwapDetail::new(
            1,
            "".into(),
            0,
            AtomicSwapStatus::Expired,
            0,
            0,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        assert!(expired.refund_button_enabled);

        let pending = SwapDetail::new(
            1,
            "".into(),
            0,
            AtomicSwapStatus::Pending,
            0,
            0,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        assert!(!pending.refund_button_enabled);
    }

    #[test]
    fn test_chaos_scoreboard_rendering() {
        let scoreboard = chaos_scoreboard_default();
        let rendered = scoreboard.render_scoreboard();
        assert!(rendered.contains("CHAOS TEST SCOREBOARD"));
        assert!(rendered.contains("Wrong preimage rejected"));
        assert!(rendered.contains("Expired refund works"));
        assert!(rendered.contains("RPC disagreement detected"));
        assert!(rendered.contains("Score:"));
        // Default is 0 passed, 0 failed, all 10 unknown — all marked [?]
        assert!(rendered.contains("[?] Wrong preimage rejected"));
        assert!(rendered.contains("[?] Solver disappears"));
    }

    #[test]
    fn test_chaos_scoreboard_with_real_results() {
        let results = vec![
            ChaosTestResult {
                name: "Scenario A".into(),
                passed: Some(true),
                description: "works".into(),
            },
            ChaosTestResult {
                name: "Scenario B".into(),
                passed: Some(false),
                description: "broken".into(),
            },
            ChaosTestResult {
                name: "Scenario C".into(),
                passed: None,
                description: "not run".into(),
            },
        ];
        let scoreboard = ChaosTestScoreboard::from_results(results);
        assert_eq!(scoreboard.total_score, 50); // 1 passed / 2 known = 50%
        assert!(scoreboard
            .summary
            .contains("1 passed, 1 failed, 1 unknown / 3"));

        let rendered = scoreboard.render_scoreboard();
        assert!(rendered.contains("[✓] Scenario A"));
        assert!(rendered.contains("[✗] Scenario B"));
        assert!(rendered.contains("[?] Scenario C"));
    }

    #[test]
    fn test_chaos_scoreboard_default_has_all_10_scenarios() {
        let scoreboard = chaos_scoreboard_default();
        assert_eq!(scoreboard.test_scenarios.len(), 10);
        // All 10 scenarios are unknown/unrun — score is 0 (no evidence).
        assert_eq!(scoreboard.total_score, 0);
        assert!(scoreboard
            .summary
            .contains("0 passed, 0 failed, 10 unknown / 10"));
    }

    #[test]
    fn test_timeout_countdown_calculation() {
        let now = 5000u64;
        let intents = vec![make_intent(1)]; // source_timeout = 6000
        let ledger = ProofLedger::new();
        let snapshot = AtomicCommandCenter::build(&intents, &ledger, now);
        assert_eq!(snapshot.details[0].timeout_countdown_secs, 1000); // 6000 - 5000

        // After timeout
        let later = 7000u64;
        let snapshot2 = AtomicCommandCenter::build(&intents, &ledger, later);
        assert_eq!(snapshot2.details[0].timeout_countdown_secs, -1000); // 6000 - 7000 = -1000
    }

    #[test]
    fn test_dashboard_summary_formatting() {
        let intents = vec![make_intent(1)];
        let ledger = ProofLedger::new();
        let snapshot = AtomicCommandCenter::build(&intents, &ledger, 1000);
        let summary = snapshot.dashboard_summary();
        assert!(summary.contains("X3 Atomic Command Center"));
        assert!(summary.contains("Active swaps: 1"));
        assert!(summary.contains("Completed swaps: 0"));
        // No "$" prefix on notional volume — it's raw asset units, not USD.
        assert!(summary.contains("Total notional volume (raw): 1000"));
        assert!(!summary.contains("Total volume: $"));
        assert!(summary.contains("Relayer uptime:"));
        assert!(summary.contains("Solver failure rate:"));
        assert!(summary.contains("Insurance fund:"));
    }

    #[test]
    fn test_two_intents_different_proof_completeness() {
        use crate::ledger::ProofLedger;

        // Intent 1: complete with all proofs
        let mut intent1 = make_intent(101);
        intent1.set_status(AtomicSwapStatus::SourceLocked).unwrap();
        intent1.set_status(AtomicSwapStatus::BothLocked).unwrap();
        intent1
            .set_status(AtomicSwapStatus::FinalityPending)
            .unwrap();
        intent1.set_status(AtomicSwapStatus::Claimable).unwrap();
        intent1.set_status(AtomicSwapStatus::Claimed).unwrap();

        // Intent 2: only source lock
        let mut intent2 = make_intent(102);
        intent2.set_status(AtomicSwapStatus::SourceLocked).unwrap();

        let mut ledger = ProofLedger::new();

        // Record full proof chain for intent 1
        {
            let rec = ledger.create_record(101, "r1".into(), 1000);
            rec.source_lock_tx = Some("0xsrc1".into());
            rec.source_lock_block = Some(100);
            rec.destination_lock_tx = Some("0xdest1".into());
            rec.destination_lock_block = Some(200);
            rec.hashlock_match = true;
            rec.timeout_order_valid = true;
            rec.finality_verified = true;
            rec.secret_reveal_tx = Some("0xreveal1".into());
            rec.claim_tx = Some("0xclaim1".into());
            rec.claim_block = Some(300);
        }

        // Record only source lock for intent 2
        {
            let rec = ledger.create_record(102, "r2".into(), 1000);
            rec.source_lock_tx = Some("0xsrc2".into());
            rec.source_lock_block = Some(100);
            // nothing else
        }

        let intents = vec![intent1, intent2];
        let snapshot = AtomicCommandCenter::build(&intents, &ledger, 1000);

        assert_eq!(snapshot.details.len(), 2);

        let detail1 = snapshot
            .details
            .iter()
            .find(|d| d.intent_id == 101)
            .expect("detail for 101");
        let detail2 = snapshot
            .details
            .iter()
            .find(|d| d.intent_id == 102)
            .expect("detail for 102");

        // Intent 1: complete - high score, few or no missing proofs
        assert!(
            detail1.proof_score >= 70,
            "complete intent should score >= 70, got {}",
            detail1.proof_score
        );

        // Intent 2: only source lock - low score, many missing proofs
        assert!(
            detail2.proof_score < 50,
            "partial intent should score < 50, got {}",
            detail2.proof_score
        );

        // Missing proofs must differ between the two
        assert_ne!(
            detail1.missing_proofs.len(),
            detail2.missing_proofs.len(),
            "missing proofs must differ: intent1={:?}, intent2={:?}",
            detail1.missing_proofs,
            detail2.missing_proofs,
        );
    }

    #[test]
    fn test_dashboard_total_volume_is_notional_not_usd() {
        // The field is `total_volume_notional` and the summary must NOT
        // claim the number is in USD when pricing isn't injected.
        let intents = vec![make_intent(1), make_intent(2)]; // 1000 each
        let ledger = ProofLedger::new();
        let snapshot = AtomicCommandCenter::build(&intents, &ledger, 1000);
        // Two intents, 1000 each → 2000 notional
        assert_eq!(snapshot.total_volume_notional, 2000);
        let summary = snapshot.dashboard_summary();
        assert!(summary.contains("Total notional volume (raw): 2000"));
        assert!(
            !summary.contains("$2000"),
            "must not label raw units as USD"
        );
    }
}
