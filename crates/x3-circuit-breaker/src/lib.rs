use std::collections::HashMap;

pub type ScopeId = [u8; 32];
pub type BlockNumber = u64;

/// Maximum blocks a Tripped breaker may remain active before the engine
/// auto-expires it (INV-R-006: privileged action expiry). Callers must
/// invoke [`CircuitBreakerEngine::tick`] each block to enforce this.
pub const DEFAULT_MAX_TRIP_BLOCKS: BlockNumber = 72;

/// Maximum blocks a Degraded scope may remain active before requiring
/// re-evaluation by an operator.
pub const DEFAULT_MAX_DEGRADE_BLOCKS: BlockNumber = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CircuitBreakerScope {
    Asset(ScopeId),
    Route(ScopeId),
    Gateway(ScopeId),
    DexPool(ScopeId),
    Verifier(ScopeId),
}

/// Full status of a scope including the new Degraded tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerStatus {
    /// Normal operation.
    Armed,
    /// Scope is halted. Auto-expires after `max_trip_blocks`.
    Tripped,
    /// Scope is in reduced-capacity safe-mode. Requires re-evaluation before
    /// full re-enable.
    Degraded,
    /// Breaker was tripped but the expiry window elapsed without a manual
    /// reset. Scope is re-armed automatically (INV-R-006).
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitBreakerRecord {
    pub scope: CircuitBreakerScope,
    pub status: CircuitBreakerStatus,
    pub reason: String,
    pub tripped_at_block: Option<BlockNumber>,
    pub reset_at_block: Option<BlockNumber>,
    /// Block at which this record expires if not manually reset (INV-R-006).
    /// `None` means no automatic expiry (use with care — governance-only resets).
    pub expires_at_block: Option<BlockNumber>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerError {
    GovernanceRequired,
    /// The scope is currently in Degraded state and disallows the requested
    /// operation. Operator must re-evaluate before full service is restored.
    ServiceDegraded,
}

#[derive(Debug, Default)]
pub struct CircuitBreakerEngine {
    records: HashMap<CircuitBreakerScope, CircuitBreakerRecord>,
    events: Vec<CircuitBreakerRecord>,
    /// Configurable maximum blocks a trip may remain active. Defaults to
    /// [`DEFAULT_MAX_TRIP_BLOCKS`].
    max_trip_blocks: Option<BlockNumber>,
}

impl CircuitBreakerEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an engine with a custom trip expiry window.
    pub fn with_max_trip_blocks(max_trip_blocks: BlockNumber) -> Self {
        Self {
            max_trip_blocks: Some(max_trip_blocks),
            ..Self::default()
        }
    }

    fn effective_max_trip_blocks(&self) -> BlockNumber {
        self.max_trip_blocks.unwrap_or(DEFAULT_MAX_TRIP_BLOCKS)
    }

    // ── Trip ──────────────────────────────────────────────────────────────

    pub fn trip_circuit_breaker(
        &mut self,
        scope: CircuitBreakerScope,
        reason: impl Into<String>,
        now: BlockNumber,
    ) -> CircuitBreakerRecord {
        let expires = now.saturating_add(self.effective_max_trip_blocks());
        let record = CircuitBreakerRecord {
            scope,
            status: CircuitBreakerStatus::Tripped,
            reason: reason.into(),
            tripped_at_block: Some(now),
            reset_at_block: None,
            expires_at_block: Some(expires),
        };
        self.records.insert(scope, record.clone());
        self.events.push(record.clone());
        record
    }

    // ── Degrade ───────────────────────────────────────────────────────────

    /// Put a scope into safe-mode (Degraded).
    ///
    /// Degraded scopes reject operations that require full capacity (enforced
    /// by individual `enforce_*` methods). They do NOT auto-expire; an operator
    /// must explicitly call [`Self::undegrage`] after re-evaluation.
    pub fn degrade(
        &mut self,
        scope: CircuitBreakerScope,
        reason: impl Into<String>,
        now: BlockNumber,
    ) -> CircuitBreakerRecord {
        let expires = now.saturating_add(DEFAULT_MAX_DEGRADE_BLOCKS);
        let record = CircuitBreakerRecord {
            scope,
            status: CircuitBreakerStatus::Degraded,
            reason: reason.into(),
            tripped_at_block: Some(now),
            reset_at_block: None,
            expires_at_block: Some(expires),
        };
        self.records.insert(scope, record.clone());
        self.events.push(record.clone());
        record
    }

    /// Restore a Degraded scope to Armed. Requires privileged origin.
    pub fn undegrage(
        &mut self,
        scope: CircuitBreakerScope,
        privileged_origin: bool,
        now: BlockNumber,
    ) -> Result<CircuitBreakerRecord, CircuitBreakerError> {
        if !privileged_origin {
            return Err(CircuitBreakerError::GovernanceRequired);
        }
        let record = CircuitBreakerRecord {
            scope,
            status: CircuitBreakerStatus::Armed,
            reason: "undegraded".to_string(),
            tripped_at_block: None,
            reset_at_block: Some(now),
            expires_at_block: None,
        };
        self.records.insert(scope, record.clone());
        self.events.push(record.clone());
        Ok(record)
    }

    // ── Reset ─────────────────────────────────────────────────────────────

    pub fn reset_circuit_breaker(
        &mut self,
        scope: CircuitBreakerScope,
        privileged_origin: bool,
        now: BlockNumber,
    ) -> Result<CircuitBreakerRecord, CircuitBreakerError> {
        if !privileged_origin {
            return Err(CircuitBreakerError::GovernanceRequired);
        }
        let record = CircuitBreakerRecord {
            scope,
            status: CircuitBreakerStatus::Armed,
            reason: "reset".to_string(),
            tripped_at_block: None,
            reset_at_block: Some(now),
            expires_at_block: None,
        };
        self.records.insert(scope, record.clone());
        self.events.push(record.clone());
        Ok(record)
    }

    // ── Tick (expiry enforcement — INV-R-006) ─────────────────────────────

    /// Must be called once per block. Expires any Tripped or Degraded records
    /// whose `expires_at_block` has been reached, re-arming them automatically.
    ///
    /// Returns the list of scopes that were auto-expired this tick.
    pub fn tick(&mut self, now: BlockNumber) -> Vec<CircuitBreakerScope> {
        let mut expired = Vec::new();
        for (scope, record) in self.records.iter_mut() {
            if matches!(
                record.status,
                CircuitBreakerStatus::Tripped | CircuitBreakerStatus::Degraded
            ) {
                if let Some(exp) = record.expires_at_block {
                    if now >= exp {
                        expired.push(*scope);
                    }
                }
            }
        }
        for scope in &expired {
            let exp_record = CircuitBreakerRecord {
                scope: *scope,
                status: CircuitBreakerStatus::Expired,
                reason: "auto-expired".to_string(),
                tripped_at_block: None,
                reset_at_block: Some(now),
                expires_at_block: None,
            };
            self.records.insert(*scope, exp_record.clone());
            self.events.push(exp_record);
        }
        expired
    }

    // ── Status helpers ────────────────────────────────────────────────────

    pub fn is_circuit_breaker_tripped(&self, scope: CircuitBreakerScope) -> bool {
        matches!(
            self.records.get(&scope).map(|r| r.status),
            Some(CircuitBreakerStatus::Tripped)
        )
    }

    pub fn is_degraded(&self, scope: CircuitBreakerScope) -> bool {
        matches!(
            self.records.get(&scope).map(|r| r.status),
            Some(CircuitBreakerStatus::Degraded)
        )
    }

    pub fn get_circuit_breaker_status(&self, scope: CircuitBreakerScope) -> CircuitBreakerStatus {
        self.records
            .get(&scope)
            .map(|record| record.status)
            .unwrap_or(CircuitBreakerStatus::Armed)
    }

    // ── Enforce helpers ───────────────────────────────────────────────────

    pub fn enforce_deposit_allowed(&self, route_id: ScopeId) -> Result<(), CircuitBreakerScope> {
        let scope = CircuitBreakerScope::Route(route_id);
        if self.is_circuit_breaker_tripped(scope) || self.is_degraded(scope) {
            return Err(scope);
        }
        Ok(())
    }

    pub fn enforce_proof_acceptance_allowed(
        &self,
        verifier_id: ScopeId,
    ) -> Result<(), CircuitBreakerScope> {
        let scope = CircuitBreakerScope::Verifier(verifier_id);
        if self.is_circuit_breaker_tripped(scope) || self.is_degraded(scope) {
            return Err(scope);
        }
        Ok(())
    }

    pub fn enforce_swap_allowed(&self, pool_id: ScopeId) -> Result<(), CircuitBreakerScope> {
        let scope = CircuitBreakerScope::DexPool(pool_id);
        if self.is_circuit_breaker_tripped(scope) || self.is_degraded(scope) {
            return Err(scope);
        }
        Ok(())
    }

    pub fn events(&self) -> &[CircuitBreakerRecord] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_breaker_blocks_deposits() {
        let mut engine = CircuitBreakerEngine::new();
        engine.trip_circuit_breaker(CircuitBreakerScope::Route([1; 32]), "failure_spike", 10);

        assert!(engine.enforce_deposit_allowed([1; 32]).is_err());
    }

    #[test]
    fn verifier_breaker_blocks_proof_acceptance() {
        let mut engine = CircuitBreakerEngine::new();
        engine.trip_circuit_breaker(CircuitBreakerScope::Verifier([2; 32]), "quorum_failure", 10);

        assert!(engine.enforce_proof_acceptance_allowed([2; 32]).is_err());
    }

    #[test]
    fn pool_breaker_blocks_swaps() {
        let mut engine = CircuitBreakerEngine::new();
        engine.trip_circuit_breaker(
            CircuitBreakerScope::DexPool([3; 32]),
            "reserve_mismatch",
            10,
        );

        assert!(engine.enforce_swap_allowed([3; 32]).is_err());
    }

    #[test]
    fn reset_requires_governance() {
        let mut engine = CircuitBreakerEngine::new();
        engine.trip_circuit_breaker(CircuitBreakerScope::Route([1; 32]), "failure_spike", 10);

        assert!(matches!(
            engine.reset_circuit_breaker(CircuitBreakerScope::Route([1; 32]), false, 11),
            Err(CircuitBreakerError::GovernanceRequired)
        ));
        engine
            .reset_circuit_breaker(CircuitBreakerScope::Route([1; 32]), true, 12)
            .unwrap();
        assert_eq!(
            engine.get_circuit_breaker_status(CircuitBreakerScope::Route([1; 32])),
            CircuitBreakerStatus::Armed
        );
    }

    #[test]
    fn trip_event_is_recorded_without_supply_mutation() {
        let mut engine = CircuitBreakerEngine::new();
        engine.trip_circuit_breaker(
            CircuitBreakerScope::Gateway([4; 32]),
            "collateral_mismatch",
            7,
        );

        assert_eq!(engine.events().len(), 1);
        assert_eq!(engine.events()[0].status, CircuitBreakerStatus::Tripped);
    }

    // ── INV-R-006 expiry tests ────────────────────────────────────────────

    #[test]
    fn trip_has_expiry_set() {
        let mut engine = CircuitBreakerEngine::with_max_trip_blocks(50);
        let record = engine.trip_circuit_breaker(CircuitBreakerScope::Asset([5; 32]), "test", 100);
        assert_eq!(record.expires_at_block, Some(150));
    }

    #[test]
    fn tick_auto_expires_tripped_breaker() {
        let mut engine = CircuitBreakerEngine::with_max_trip_blocks(10);
        let scope = CircuitBreakerScope::Route([6; 32]);
        engine.trip_circuit_breaker(scope, "test", 100);
        // Before expiry — still blocked.
        assert!(engine.is_circuit_breaker_tripped(scope));
        // At expiry block — should expire.
        let expired = engine.tick(110);
        assert_eq!(expired, vec![scope]);
        assert_eq!(
            engine.get_circuit_breaker_status(scope),
            CircuitBreakerStatus::Expired
        );
        // After expiry — deposits should be allowed (Expired != Tripped/Degraded).
        assert!(engine.enforce_deposit_allowed([6; 32]).is_ok());
    }

    #[test]
    fn tick_before_expiry_does_not_expire() {
        let mut engine = CircuitBreakerEngine::with_max_trip_blocks(10);
        let scope = CircuitBreakerScope::Route([7; 32]);
        engine.trip_circuit_breaker(scope, "test", 100);
        let expired = engine.tick(109);
        assert!(expired.is_empty());
        assert!(engine.is_circuit_breaker_tripped(scope));
    }

    // ── Degrade state machine tests ───────────────────────────────────────

    #[test]
    fn degrade_blocks_deposits() {
        let mut engine = CircuitBreakerEngine::new();
        let scope = CircuitBreakerScope::Route([8; 32]);
        engine.degrade(scope, "capacity_warning", 50);
        assert!(engine.is_degraded(scope));
        assert!(engine.enforce_deposit_allowed([8; 32]).is_err());
    }

    #[test]
    fn undegrage_requires_governance() {
        let mut engine = CircuitBreakerEngine::new();
        let scope = CircuitBreakerScope::DexPool([9; 32]);
        engine.degrade(scope, "vol_spike", 60);
        assert!(matches!(
            engine.undegrage(scope, false, 70),
            Err(CircuitBreakerError::GovernanceRequired)
        ));
        engine.undegrage(scope, true, 75).unwrap();
        assert_eq!(
            engine.get_circuit_breaker_status(scope),
            CircuitBreakerStatus::Armed
        );
    }

    #[test]
    fn degraded_scope_auto_expires_via_tick() {
        let mut engine = CircuitBreakerEngine::new();
        let scope = CircuitBreakerScope::Gateway([10; 32]);
        engine.degrade(scope, "reduced_capacity", 0);
        let expired = engine.tick(DEFAULT_MAX_DEGRADE_BLOCKS);
        assert!(expired.contains(&scope));
        assert_eq!(
            engine.get_circuit_breaker_status(scope),
            CircuitBreakerStatus::Expired
        );
    }
}
