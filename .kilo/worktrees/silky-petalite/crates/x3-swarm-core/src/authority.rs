//! Swarm authority coordinator — wires misconduct ladder → genesis termination.
//!
//! `SwarmAuthority` is the single entry point for recording agent violations.
//! When a violation escalates to [`Sanction::Kill`], the authority
//! automatically terminates the agent's genesis record and emits an audit
//! event. This is the kill path described in AGENT_LAW §3 and the Stage 1
//! exit-gate requirement in `docs/swarm-ops/ROLLOUT_STAGES.md`.
//!
//! # Invariants maintained
//! - A Kill'd agent always has `GenesisRecord::terminated == true`.
//! - Genesis termination is idempotent after the first Kill.
//! - Every Kill event produces a [`AuditCategory::AgentKilled`] entry.

use crate::{
    audit::{AuditCategory, AuditLog},
    genesis::{AgentId, BlockHeight, GenesisError, GenesisStore},
    misconduct::{MisconductEngine, MisconductError, Sanction, ViolationClass},
};

/// Error type returned by [`SwarmAuthority::enforce_violation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityError {
    /// The agent was already in the Kill sanction before this call.
    AgentAlreadyKilled(AgentId),
    /// Genesis termination failed after a Kill sanction was issued.
    TerminationFailed(GenesisError),
}

impl core::fmt::Display for AuthorityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AuthorityError::AgentAlreadyKilled(id) => {
                write!(f, "agent {:?} is already killed", id)
            }
            AuthorityError::TerminationFailed(e) => {
                write!(f, "genesis termination failed: {e}")
            }
        }
    }
}

/// Coordinates misconduct enforcement, genesis lifecycle, and audit logging.
///
/// Callers should prefer this over calling `MisconductEngine` and
/// `GenesisStore` directly so that the kill-path invariant is always enforced.
#[derive(Debug, Default)]
pub struct SwarmAuthority {
    misconduct: MisconductEngine,
    genesis: GenesisStore,
    audit: AuditLog,
}

impl SwarmAuthority {
    pub fn new() -> Self {
        Self::default()
    }

    /// Access the genesis store (for agent creation and queries).
    pub fn genesis(&self) -> &GenesisStore {
        &self.genesis
    }

    /// Mutable access to the genesis store (for agent creation).
    pub fn genesis_mut(&mut self) -> &mut GenesisStore {
        &mut self.genesis
    }

    /// Access the audit log.
    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }

    /// Access the misconduct engine (for direct queries).
    pub fn misconduct(&self) -> &MisconductEngine {
        &self.misconduct
    }

    /// Record a violation for `agent_id` and enforce the resulting sanction.
    ///
    /// # Kill path
    ///
    /// If the violation escalates (or already was at) [`Sanction::Kill`]:
    /// 1. The genesis record is marked as terminated via
    ///    [`GenesisStore::terminate`].
    /// 2. An [`AuditCategory::AgentKilled`] event is appended.
    /// 3. [`Sanction::Kill`] is returned.
    ///
    /// If the agent was already killed before this call, returns
    /// [`AuthorityError::AgentAlreadyKilled`] — no new violation is recorded.
    ///
    /// # Other sanctions
    ///
    /// For non-Kill sanctions (Warning through Suspension), an audit event
    /// with the sanction description is recorded and the sanction is returned.
    pub fn enforce_violation(
        &mut self,
        agent_id: AgentId,
        class: ViolationClass,
        evidence: impl Into<String>,
        block: BlockHeight,
    ) -> Result<Sanction, AuthorityError> {
        let evidence_str: String = evidence.into();

        let sanction = self
            .misconduct
            .record(agent_id, class, evidence_str.clone(), block)
            .map_err(|e| match e {
                MisconductError::AgentKilled(id) => AuthorityError::AgentAlreadyKilled(id),
            })?;

        if sanction == Sanction::Kill {
            // Terminate the genesis record. If the record doesn't exist
            // (e.g. agent was registered outside this authority), we still
            // emit the audit event — the caller is responsible for ensuring
            // genesis records exist for all agents.
            if let Err(e) = self.genesis.terminate(&agent_id) {
                // Only propagate if it's a genuine failure (not NotFound —
                // agents registered externally may not be in this store).
                if !matches!(e, GenesisError::NotFound(_)) {
                    return Err(AuthorityError::TerminationFailed(e));
                }
            }
            let summary = format!(
                "Kill sanction issued. Last evidence: {}",
                &evidence_str[..evidence_str.len().min(128)]
            );
            self.audit.record(block, AuditCategory::AgentKilled, Some(agent_id), summary, None);
        } else {
            let summary = format!("Sanction advanced to {:?} (class {:?})", sanction, class);
            self.audit.record(block, AuditCategory::ViolationRecorded, Some(agent_id), summary, None);
        }

        Ok(sanction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        genesis::GenesisRecord,
        AgentKind, AgentPermissionTier,
    };

    fn make_id(b: u8) -> AgentId {
        [b; 32]
    }

    fn register_agent(authority: &mut SwarmAuthority, id: u8) {
        let agent_id = make_id(id);
        let rec = GenesisRecord::new(
            agent_id,
            make_id(0),
            "test agent",
            AgentKind::Auditor,
            AgentPermissionTier::DocsTestsReports,
            vec![],
            1,
        );
        authority.genesis_mut().create(rec).unwrap();
    }

    #[test]
    fn kill_terminates_genesis_record() {
        let mut auth = SwarmAuthority::new();
        let id = make_id(1);
        register_agent(&mut auth, 1);

        // Escalate to Kill via three D-class violations.
        for i in 0..3u64 {
            let _ = auth.enforce_violation(id, ViolationClass::D, "critical breach", i + 1);
        }

        let record = auth.genesis().get(&id).unwrap();
        assert!(record.terminated, "genesis record must be terminated after Kill");
    }

    #[test]
    fn kill_emits_audit_event() {
        let mut auth = SwarmAuthority::new();
        let id = make_id(2);
        register_agent(&mut auth, 2);

        for i in 0..3u64 {
            let _ = auth.enforce_violation(id, ViolationClass::D, "breach", i + 1);
        }

        let entries = auth.audit().entries_for_agent(&id);
        assert!(
            entries.iter().any(|e| e.category == AuditCategory::AgentKilled),
            "audit log must contain AgentKilled entry"
        );
    }

    #[test]
    fn already_killed_returns_error() {
        let mut auth = SwarmAuthority::new();
        let id = make_id(3);
        register_agent(&mut auth, 3);

        for i in 0..3u64 {
            let _ = auth.enforce_violation(id, ViolationClass::D, "breach", i + 1);
        }

        // Fourth call should be rejected.
        let result = auth.enforce_violation(id, ViolationClass::A, "minor", 99);
        assert_eq!(result, Err(AuthorityError::AgentAlreadyKilled(id)));
    }

    #[test]
    fn non_kill_sanction_records_audit_but_not_terminated() {
        let mut auth = SwarmAuthority::new();
        let id = make_id(4);
        register_agent(&mut auth, 4);

        let sanction = auth
            .enforce_violation(id, ViolationClass::B, "unauthorized read", 10)
            .unwrap();

        assert_ne!(sanction, Sanction::Kill);
        let record = auth.genesis().get(&id).unwrap();
        assert!(!record.terminated, "agent should not be terminated on non-Kill sanction");

        let entries = auth.audit().entries_for_agent(&id);
        assert!(
            entries.iter().any(|e| e.category == AuditCategory::ViolationRecorded),
            "violation audit entry expected"
        );
    }

    #[test]
    fn kill_without_genesis_record_still_succeeds() {
        // Agent has misconduct record but no genesis record in this store.
        let mut auth = SwarmAuthority::new();
        let id = make_id(5);
        // Do NOT register — no genesis record.

        for i in 0..3u64 {
            let result = auth.enforce_violation(id, ViolationClass::D, "breach", i + 1);
            // Should succeed even though genesis is absent (returns Kill, not error).
            if let Ok(s) = result {
                if s == Sanction::Kill {
                    break;
                }
            }
        }

        // No panic, no TerminationFailed error — NotFound is silently tolerated.
        let entries = auth.audit().entries_for_agent(&id);
        assert!(entries.iter().any(|e| e.category == AuditCategory::AgentKilled));
    }
}
