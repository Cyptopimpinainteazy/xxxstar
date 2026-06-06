//! Misconduct ladder — enforces AGENT_LAW §3 (violation escalation).
//!
//! Each violation against an agent is classified A–D by severity and recorded.
//! The ladder advances the agent's sanction level deterministically. The engine
//! is append-only; sanctions may only escalate (never silently reset).
//!
//! Sanction progression (per AGENT_LAW §3.2):
//!   A-class  → Warning
//!   B-class  → Strike1
//!   C-class  → Strike2 → BondSlash → Quarantine (on repeat)
//!   D-class  → Quarantine → ForcedDowngrade → Suspension → Kill
//!
//! The kill sanction is terminal; a Kill'd agent cannot recover and must not
//! be re-spawned under the same `agent_id`.

use crate::genesis::AgentId;
use serde::{Deserialize, Serialize};

/// Severity class of a protocol violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ViolationClass {
    /// Minor policy breach, no financial impact (e.g. exceeded rate limit).
    A,
    /// Significant policy breach, partial impact (e.g. unauthorized read).
    B,
    /// Serious breach, potential financial impact (e.g. path guard bypass attempt).
    C,
    /// Critical breach, safety risk (e.g. attempted mainnet key access).
    D,
}

/// A recorded violation event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViolationRecord {
    pub agent_id: AgentId,
    pub class: ViolationClass,
    /// Free-form evidence text (capped to 512 bytes at ingestion).
    pub evidence: String,
    /// Block at which the violation was recorded.
    pub block: u64,
}

/// Current sanction level for an agent.
///
/// Levels are ordered: each represents a stricter sanction. The only allowed
/// direction of change is upward (escalation). Downgraded/reset sanctions
/// require explicit governance action and are out of scope for this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Sanction {
    /// No recorded violations; normal operation.
    Clean,
    /// First minor violation on record.
    Warning,
    /// First significant strike.
    Strike1,
    /// Second significant strike; bond may be slashed next.
    Strike2,
    /// Bonded stake partially or fully slashed.
    BondSlash,
    /// Agent is quarantined; all actions suspended pending review.
    Quarantine,
    /// Agent demoted to lower permission tier.
    ForcedDowngrade,
    /// Agent fully suspended; no actions permitted.
    Suspension,
    /// Terminal sanction. Agent must not be re-spawned under same id.
    Kill,
}

impl Sanction {
    /// Returns `true` if the agent is no longer permitted to act.
    pub fn is_halted(&self) -> bool {
        matches!(
            self,
            Sanction::Quarantine | Sanction::Suspension | Sanction::Kill
        )
    }

    /// Returns `true` if the sanction is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Sanction::Kill)
    }
}

/// An agent's full misconduct history and current sanction level.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MisconductRecord {
    pub agent_id: AgentId,
    pub sanction: Sanction,
    pub violations: Vec<ViolationRecord>,
}

impl MisconductRecord {
    fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            sanction: Sanction::Clean,
            violations: vec![],
        }
    }

    /// Count violations of a given class in this record.
    fn count(&self, class: ViolationClass) -> usize {
        self.violations.iter().filter(|v| v.class == class).count()
    }
}

/// Error returned by the misconduct engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MisconductError {
    /// Cannot record additional violations for a Kill'd agent.
    AgentKilled(AgentId),
}

/// The misconduct ladder engine.
///
/// Maintains one [`MisconductRecord`] per agent. Call
/// [`MisconductEngine::record`] on every confirmed violation; the engine
/// advances the sanction deterministically and returns the new level.
#[derive(Debug, Default)]
pub struct MisconductEngine {
    records: std::collections::HashMap<AgentId, MisconductRecord>,
}

impl MisconductEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a violation for `agent_id` and advance the sanction ladder.
    ///
    /// Returns the new (possibly unchanged) [`Sanction`] level.
    ///
    /// Returns [`MisconductError::AgentKilled`] if the agent is already in the
    /// Kill sanction — no further records are written.
    pub fn record(
        &mut self,
        agent_id: AgentId,
        class: ViolationClass,
        evidence: impl Into<String>,
        block: u64,
    ) -> Result<Sanction, MisconductError> {
        let rec = self
            .records
            .entry(agent_id)
            .or_insert_with(|| MisconductRecord::new(agent_id));

        if rec.sanction.is_terminal() {
            return Err(MisconductError::AgentKilled(agent_id));
        }

        let evidence_str = {
            let s: String = evidence.into();
            if s.len() > 512 {
                s[..512].to_string()
            } else {
                s
            }
        };

        rec.violations.push(ViolationRecord {
            agent_id,
            class,
            evidence: evidence_str,
            block,
        });

        rec.sanction = Self::next_sanction(rec);
        Ok(rec.sanction)
    }

    /// Retrieve the current sanction for an agent (returns `Clean` if
    /// no record exists).
    pub fn current_sanction(&self, agent_id: &AgentId) -> Sanction {
        self.records
            .get(agent_id)
            .map(|r| r.sanction)
            .unwrap_or(Sanction::Clean)
    }

    /// Full misconduct history for an agent, if any.
    pub fn get_record(&self, agent_id: &AgentId) -> Option<&MisconductRecord> {
        self.records.get(agent_id)
    }

    /// Returns `true` if the agent is halted (quarantine / suspension / kill).
    pub fn is_halted(&self, agent_id: &AgentId) -> bool {
        self.current_sanction(agent_id).is_halted()
    }

    /// Deterministic sanction advancement logic. Separate from the mutable
    /// engine state so it's pure and unit-testable.
    fn next_sanction(rec: &MisconductRecord) -> Sanction {
        let current = rec.sanction;
        let a = rec.count(ViolationClass::A);
        let b = rec.count(ViolationClass::B);
        let c = rec.count(ViolationClass::C);
        let d = rec.count(ViolationClass::D);

        // D-class violations escalate fastest.
        if d >= 3 {
            return Sanction::Kill;
        }
        if d >= 2 {
            return Sanction::Suspension.max(current);
        }
        if d >= 1 {
            return Sanction::Quarantine.max(current);
        }

        // C-class violations.
        if c >= 4 {
            return Sanction::Kill;
        }
        if c >= 3 {
            return Sanction::Quarantine.max(current);
        }
        if c >= 2 {
            return Sanction::BondSlash.max(current);
        }
        if c >= 1 {
            return Sanction::Strike2.max(current);
        }

        // B-class violations.
        if b >= 3 {
            return Sanction::Strike2.max(current);
        }
        if b >= 1 {
            return Sanction::Strike1.max(current);
        }

        // A-class violations.
        if a >= 1 {
            return Sanction::Warning.max(current);
        }

        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> AgentId {
        [n; 32]
    }

    #[test]
    fn clean_agent_has_clean_sanction() {
        let engine = MisconductEngine::new();
        assert_eq!(engine.current_sanction(&id(1)), Sanction::Clean);
    }

    #[test]
    fn a_class_gives_warning() {
        let mut engine = MisconductEngine::new();
        let s = engine
            .record(id(1), ViolationClass::A, "rate limit", 1)
            .unwrap();
        assert_eq!(s, Sanction::Warning);
    }

    #[test]
    fn b_class_gives_strike1() {
        let mut engine = MisconductEngine::new();
        let s = engine
            .record(id(2), ViolationClass::B, "unauth read", 1)
            .unwrap();
        assert_eq!(s, Sanction::Strike1);
    }

    #[test]
    fn c_class_escalates_to_strike2_then_bond_slash() {
        let mut engine = MisconductEngine::new();
        engine
            .record(id(3), ViolationClass::C, "bypass attempt", 1)
            .unwrap();
        assert_eq!(engine.current_sanction(&id(3)), Sanction::Strike2);
        let s = engine
            .record(id(3), ViolationClass::C, "bypass attempt 2", 2)
            .unwrap();
        assert_eq!(s, Sanction::BondSlash);
    }

    #[test]
    fn d_class_immediate_quarantine() {
        let mut engine = MisconductEngine::new();
        let s = engine
            .record(id(4), ViolationClass::D, "mainnet key access", 1)
            .unwrap();
        assert_eq!(s, Sanction::Quarantine);
        assert!(engine.is_halted(&id(4)));
    }

    #[test]
    fn two_d_class_gives_suspension() {
        let mut engine = MisconductEngine::new();
        engine.record(id(5), ViolationClass::D, "v1", 1).unwrap();
        let s = engine.record(id(5), ViolationClass::D, "v2", 2).unwrap();
        assert_eq!(s, Sanction::Suspension);
    }

    #[test]
    fn three_d_class_kills() {
        let mut engine = MisconductEngine::new();
        engine.record(id(6), ViolationClass::D, "v1", 1).unwrap();
        engine.record(id(6), ViolationClass::D, "v2", 2).unwrap();
        let s = engine.record(id(6), ViolationClass::D, "v3", 3).unwrap();
        assert_eq!(s, Sanction::Kill);
        assert!(engine.current_sanction(&id(6)).is_terminal());
    }

    #[test]
    fn killed_agent_rejects_further_violations() {
        let mut engine = MisconductEngine::new();
        for i in 0..3 {
            let _ = engine.record(id(7), ViolationClass::D, "severe", i as u64);
        }
        let err = engine
            .record(id(7), ViolationClass::A, "minor", 10)
            .unwrap_err();
        assert_eq!(err, MisconductError::AgentKilled(id(7)));
    }

    #[test]
    fn sanction_never_decreases() {
        let mut engine = MisconductEngine::new();
        // D-class first → Quarantine
        engine
            .record(id(8), ViolationClass::D, "severe", 1)
            .unwrap();
        // Subsequent A-class should not drop below Quarantine.
        let s = engine.record(id(8), ViolationClass::A, "minor", 2).unwrap();
        assert!(s >= Sanction::Quarantine);
    }

    #[test]
    fn evidence_truncated_at_512_bytes() {
        let mut engine = MisconductEngine::new();
        let long = "x".repeat(1000);
        engine.record(id(9), ViolationClass::A, long, 1).unwrap();
        let rec = engine.get_record(&id(9)).unwrap();
        assert_eq!(rec.violations[0].evidence.len(), 512);
    }
}
