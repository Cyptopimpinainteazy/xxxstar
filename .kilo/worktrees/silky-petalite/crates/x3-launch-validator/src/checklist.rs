//! Launch checklist item definitions from the X3 Launch Spec (vΩ-1.0).

use serde::{Deserialize, Serialize};

/// Phase of the launch process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckPhase {
    PreLaunch,
    LaunchDay,
    PostLaunch30Days,
    FailureConditions,
}

impl std::fmt::Display for CheckPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckPhase::PreLaunch => write!(f, "PRE-LAUNCH"),
            CheckPhase::LaunchDay => write!(f, "LAUNCH DAY"),
            CheckPhase::PostLaunch30Days => write!(f, "POST-LAUNCH (30 DAYS)"),
            CheckPhase::FailureConditions => write!(f, "FAILURE CONDITIONS"),
        }
    }
}

/// Status of an individual check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckResult {
    Pass,
    Fail(String),
    Skipped(String),
}

impl CheckResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, CheckResult::Pass)
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, CheckResult::Fail(_))
    }
}

impl std::fmt::Display for CheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckResult::Pass => write!(f, "PASS"),
            CheckResult::Fail(msg) => write!(f, "FAIL: {msg}"),
            CheckResult::Skipped(msg) => write!(f, "SKIP: {msg}"),
        }
    }
}

/// A single launch checklist item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    /// Unique identifier.
    pub id: &'static str,
    /// Phase this check belongs to.
    pub phase: CheckPhase,
    /// Human-readable description.
    pub description: &'static str,
    /// Whether this check is blocking (failure = halt launch).
    pub blocking: bool,
    /// Result after running.
    pub result: Option<CheckResult>,
}

impl CheckItem {
    pub fn new(
        id: &'static str,
        phase: CheckPhase,
        description: &'static str,
        blocking: bool,
    ) -> Self {
        Self {
            id,
            phase,
            description,
            blocking,
            result: None,
        }
    }

    pub fn passed(&self) -> bool {
        self.result.as_ref().map(|r| r.is_pass()).unwrap_or(false)
    }

    pub fn failed(&self) -> bool {
        self.result.as_ref().map(|r| r.is_fail()).unwrap_or(false)
    }
}

/// The complete launch checklist per spec vΩ-1.0.
pub struct LaunchChecklist {
    pub items: Vec<CheckItem>,
}

impl LaunchChecklist {
    /// Build the canonical checklist from the spec.
    pub fn canonical() -> Self {
        let items = vec![
            // ----------------------------------------------------------------
            // PRE-LAUNCH
            // ----------------------------------------------------------------
            CheckItem::new(
                "PRE-001",
                CheckPhase::PreLaunch,
                "Deterministic builds reproducible on ≥3 independent machines",
                true,
            ),
            CheckItem::new(
                "PRE-002",
                CheckPhase::PreLaunch,
                "Genesis state hashed and notarized",
                true,
            ),
            CheckItem::new(
                "PRE-003",
                CheckPhase::PreLaunch,
                "Constitution proofs verified (canonical constitution hash matches on-chain)",
                true,
            ),
            CheckItem::new(
                "PRE-004",
                CheckPhase::PreLaunch,
                "ZK verifier gas costs bounded (per-block proof cost under protocol limit)",
                true,
            ),
            CheckItem::new(
                "PRE-005",
                CheckPhase::PreLaunch,
                "Kill-switch tested and slashing dry-run executed",
                true,
            ),
            // ----------------------------------------------------------------
            // LAUNCH DAY
            // ----------------------------------------------------------------
            CheckItem::new(
                "LAUNCH-001",
                CheckPhase::LaunchDay,
                "Genesis proof published and anchored on-chain",
                true,
            ),
            CheckItem::new(
                "LAUNCH-002",
                CheckPhase::LaunchDay,
                "Cross-chain verifier contracts deployed",
                true,
            ),
            CheckItem::new(
                "LAUNCH-003",
                CheckPhase::LaunchDay,
                "Monitoring + replay auditors live",
                true,
            ),
            CheckItem::new(
                "LAUNCH-004",
                CheckPhase::LaunchDay,
                "Agent deployment frozen (observation window active)",
                true,
            ),
            // ----------------------------------------------------------------
            // POST-LAUNCH (FIRST 30 DAYS)
            // ----------------------------------------------------------------
            CheckItem::new(
                "POST-001",
                CheckPhase::PostLaunch30Days,
                "Continuous replay verification running",
                false,
            ),
            CheckItem::new(
                "POST-002",
                CheckPhase::PostLaunch30Days,
                "Adversarial invariant fuzzing scheduled and active",
                false,
            ),
            CheckItem::new(
                "POST-003",
                CheckPhase::PostLaunch30Days,
                "Governance proposals disabled (observation window)",
                true,
            ),
            CheckItem::new(
                "POST-004",
                CheckPhase::PostLaunch30Days,
                "Proof latency benchmarks published",
                false,
            ),
            // ----------------------------------------------------------------
            // FAILURE CONDITIONS (IMMEDIATE HALT)
            // ----------------------------------------------------------------
            CheckItem::new(
                "FAIL-001",
                CheckPhase::FailureConditions,
                "No replay mismatch detected (replay hash matches canonical chain)",
                true,
            ),
            CheckItem::new(
                "FAIL-002",
                CheckPhase::FailureConditions,
                "No invalid ZK proof encountered",
                true,
            ),
            CheckItem::new(
                "FAIL-003",
                CheckPhase::FailureConditions,
                "No invariant violation detected",
                true,
            ),
            CheckItem::new(
                "FAIL-004",
                CheckPhase::FailureConditions,
                "No nondeterministic execution detected",
                true,
            ),
        ];
        Self { items }
    }

    /// Returns all items for a given phase.
    pub fn phase(&self, phase: &CheckPhase) -> Vec<&CheckItem> {
        self.items.iter().filter(|i| &i.phase == phase).collect()
    }

    /// Returns `true` if all blocking items have passed.
    pub fn all_blocking_passed(&self) -> bool {
        self.items.iter().filter(|i| i.blocking).all(|i| i.passed())
    }

    /// Returns `true` if any blocking item has failed.
    pub fn any_blocking_failed(&self) -> bool {
        self.items.iter().filter(|i| i.blocking).any(|i| i.failed())
    }

    /// Returns `true` if any blocking item is not explicitly passed.
    ///
    /// This treats `Fail`, `Skipped`, and `None` (not executed) as unmet.
    pub fn any_blocking_unmet(&self) -> bool {
        self.items
            .iter()
            .filter(|i| i.blocking)
            .any(|i| !i.passed())
    }

    /// Count of blocking checks that are not explicitly passed.
    pub fn blocking_unmet_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.blocking)
            .filter(|i| !i.passed())
            .count()
    }

    /// Count of (pass, fail, skip) across all items.
    pub fn summary(&self) -> (usize, usize, usize) {
        let pass = self.items.iter().filter(|i| i.passed()).count();
        let fail = self
            .items
            .iter()
            .filter(|i| i.result.as_ref().map(|r| r.is_fail()).unwrap_or(false))
            .count();
        let skip = self
            .items
            .iter()
            .filter(|i| matches!(i.result, Some(CheckResult::Skipped(_))) || i.result.is_none())
            .count();
        (pass, fail, skip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_checklist_has_all_phases() {
        let cl = LaunchChecklist::canonical();
        assert!(cl.phase(&CheckPhase::PreLaunch).len() >= 5);
        assert!(cl.phase(&CheckPhase::LaunchDay).len() >= 4);
        assert!(cl.phase(&CheckPhase::PostLaunch30Days).len() >= 4);
        assert!(cl.phase(&CheckPhase::FailureConditions).len() >= 4);
    }

    #[test]
    fn all_blocking_items_are_present() {
        let cl = LaunchChecklist::canonical();
        let blocking_count = cl.items.iter().filter(|i| i.blocking).count();
        assert!(
            blocking_count >= 12,
            "expected ≥12 blocking checks, got {}",
            blocking_count
        );
    }

    #[test]
    fn blocking_unmet_includes_skipped_and_pending() {
        let mut cl = LaunchChecklist::canonical();
        // Mark one blocking item pass and leave the rest pending.
        if let Some(item) = cl.items.iter_mut().find(|i| i.blocking) {
            item.result = Some(CheckResult::Pass);
        }

        assert!(cl.any_blocking_unmet());
        assert!(cl.blocking_unmet_count() > 0);
    }
}
