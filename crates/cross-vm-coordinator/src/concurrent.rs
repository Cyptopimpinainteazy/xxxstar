//! Thread-safe coordinator façade.
//!
//! The core SwapCoordinator intentionally owns mutable session state. This
//! wrapper serializes concurrent relayer/task access through one mutex so the
//! persisted idempotency journal from the underlying coordinator is evaluated
//! atomically with each mutation.

use crate::{
    CoordinatorConfig, CoordinatorError, CoordinatorOperation, HtlcRecord, HtlcSecret,
    DistributedLeaseStore, DurableLeaseAuthority, DurableSecretRegistry, InMemoryPersistence,
    SessionLease, SessionLeaseManager, SessionPersistence, SwapCoordinator, SwapPhase, SwapSession,
};
use std::sync::{Arc, Mutex, MutexGuard};


/// Multi-process coordinator façade backed by a shared durable fencing authority.
///
/// Lease acquisition refreshes the latest persisted session/security state.
/// Every value-moving commit revalidates the shared fence before entering the
/// process-local coordinator mutation boundary.
pub struct DistributedConcurrentSwapCoordinator<
    P: SessionPersistence,
    S: DistributedLeaseStore,
> {
    coordinator: ConcurrentSwapCoordinator<P>,
    authority: DurableLeaseAuthority<S>,
    secrets: DurableSecretRegistry<S>,
}

impl<P: SessionPersistence, S: DistributedLeaseStore> Clone
    for DistributedConcurrentSwapCoordinator<P, S>
{
    fn clone(&self) -> Self {
        Self {
            coordinator: self.coordinator.clone(),
            authority: self.authority.clone(),
            secrets: self.secrets.clone(),
        }
    }
}

impl<P: SessionPersistence, S: DistributedLeaseStore>
    DistributedConcurrentSwapCoordinator<P, S>
{
    pub fn new(
        coordinator: ConcurrentSwapCoordinator<P>,
        authority: DurableLeaseAuthority<S>,
    ) -> Self {
        let secrets = DurableSecretRegistry::new(authority.shared_store());
        Self {
            coordinator,
            authority,
            secrets,
        }
    }

    pub fn acquire_session_lease(
        &self,
        session_id: &str,
        owner_id: &str,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        let lease = self
            .authority
            .acquire(session_id, owner_id, now_unix, ttl_secs)?;
        self.coordinator
            .execute(|inner| inner.refresh_from_persistence(session_id))?;
        Ok(lease)
    }

    pub fn renew_session_lease(
        &self,
        lease: &SessionLease,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        self.authority.renew(lease, now_unix, ttl_secs)
    }

    pub fn release_session_lease(
        &self,
        lease: &SessionLease,
    ) -> Result<(), CoordinatorError> {
        self.authority.release(lease)
    }

    fn validate_before_commit(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.authority.validate(lease, now_unix)
    }

    pub fn record_htlc_fast(
        &self,
        lease: &SessionLease,
        record: HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_before_commit(lease, now_unix)?;
        self.coordinator
            .record_htlc_fast(&lease.session_id, record, now_unix)
    }

    pub fn record_htlc_slow(
        &self,
        lease: &SessionLease,
        record: HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_before_commit(lease, now_unix)?;
        self.coordinator
            .record_htlc_slow(&lease.session_id, record, now_unix)
    }

    pub fn record_fast_claim(
        &self,
        lease: &SessionLease,
        secret: HtlcSecret,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_before_commit(lease, now_unix)?;

        let session = self
            .coordinator
            .session(&lease.session_id)?
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: lease.session_id.clone(),
            })?;

        if secret.hash() != session.hash_lock {
            return Err(CoordinatorError::Internal(format!(
                "secret hash mismatch for distributed claim on session '{}'",
                lease.session_id
            )));
        }

        let already_claimed = session.operation_journal.iter().any(|receipt| {
            receipt.operation == CoordinatorOperation::FastClaim
        });
        if session.phase != SwapPhase::ClaimingFast && !already_claimed {
            return Err(CoordinatorError::InvalidPhaseTransition {
                from: session.phase.to_string(),
                to: SwapPhase::ClaimingFast.to_string(),
            });
        }

        let secret_hash = *blake3::hash(secret.as_bytes()).as_bytes();
        self.secrets.claim(secret_hash, &lease.session_id)?;
        self.coordinator
            .record_fast_claim(&lease.session_id, secret, now_unix)
    }

    pub fn record_slow_claim(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_before_commit(lease, now_unix)?;
        self.coordinator
            .record_slow_claim(&lease.session_id, now_unix)
    }

    pub fn record_refunds(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_before_commit(lease, now_unix)?;
        self.coordinator
            .record_refunds(&lease.session_id, now_unix)
    }

    pub fn session(
        &self,
        session_id: &str,
    ) -> Result<Option<SwapSession>, CoordinatorError> {
        self.coordinator.session(session_id)
    }
}

/// Shared coordinator handle for concurrent relayers/workers.
///
/// This provides process-local linearization. Cross-process duplicate delivery
/// is still resolved by the coordinator's persisted idempotency journal.
pub struct ConcurrentSwapCoordinator<P: SessionPersistence = InMemoryPersistence> {
    inner: Arc<Mutex<SwapCoordinator<P>>>,
    leases: SessionLeaseManager,
}

impl<P: SessionPersistence> Clone for ConcurrentSwapCoordinator<P> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            leases: self.leases.clone(),
        }
    }
}

impl<P: SessionPersistence> ConcurrentSwapCoordinator<P> {
    pub fn new(coordinator: SwapCoordinator<P>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(coordinator)),
            leases: SessionLeaseManager::default(),
        }
    }

    pub fn with_persistence(config: CoordinatorConfig, persistence: Arc<P>) -> Self {
        Self::new(SwapCoordinator::with_persistence(config, persistence))
    }

    pub fn lease_manager(&self) -> &SessionLeaseManager {
        &self.leases
    }

    pub fn acquire_session_lease(
        &self,
        session_id: &str,
        owner_id: &str,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        self.leases
            .acquire(session_id, owner_id, now_unix, ttl_secs)
    }

    fn validate_lease(
        &self,
        lease: &SessionLease,
        session_id: &str,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        if lease.session_id != session_id {
            return Err(CoordinatorError::Internal(format!(
                "lease/session mismatch: lease for '{}' used on '{session_id}'",
                lease.session_id
            )));
        }
        self.leases.validate(lease, now_unix)
    }

    fn lock(&self) -> Result<MutexGuard<'_, SwapCoordinator<P>>, CoordinatorError> {
        self.inner.lock().map_err(|_| {
            CoordinatorError::Internal(
                "concurrent coordinator mutex poisoned; refusing mutation".to_string(),
            )
        })
    }

    /// Execute an arbitrary coordinator mutation while holding the global
    /// process-local serialization lock.
    pub fn execute<R>(
        &self,
        f: impl FnOnce(&mut SwapCoordinator<P>) -> Result<R, CoordinatorError>,
    ) -> Result<R, CoordinatorError> {
        let mut coordinator = self.lock()?;
        f(&mut coordinator)
    }

    /// Snapshot one session while holding the same serialization boundary used
    /// by mutations.
    pub fn session(&self, session_id: &str) -> Result<Option<SwapSession>, CoordinatorError> {
        let coordinator = self.lock()?;
        Ok(coordinator.get_session(session_id).cloned())
    }

    pub fn record_htlc_fast(
        &self,
        session_id: &str,
        record: HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.execute(|coordinator| coordinator.record_htlc_fast(session_id, record, now_unix))
    }

    pub fn record_htlc_slow(
        &self,
        session_id: &str,
        record: HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.execute(|coordinator| coordinator.record_htlc_slow(session_id, record, now_unix))
    }

    pub fn record_fast_claim(
        &self,
        session_id: &str,
        secret: HtlcSecret,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.execute(|coordinator| coordinator.record_fast_claim(session_id, secret, now_unix))
    }

    pub fn record_slow_claim(
        &self,
        session_id: &str,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.execute(|coordinator| coordinator.record_slow_claim(session_id, now_unix))
    }

    pub fn record_refunds(
        &self,
        session_id: &str,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.execute(|coordinator| coordinator.record_refunds(session_id, now_unix))
    }

    pub fn record_htlc_fast_with_lease(
        &self,
        lease: &SessionLease,
        record: HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_lease(lease, &lease.session_id, now_unix)?;
        self.record_htlc_fast(&lease.session_id, record, now_unix)
    }

    pub fn record_htlc_slow_with_lease(
        &self,
        lease: &SessionLease,
        record: HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_lease(lease, &lease.session_id, now_unix)?;
        self.record_htlc_slow(&lease.session_id, record, now_unix)
    }

    pub fn record_fast_claim_with_lease(
        &self,
        lease: &SessionLease,
        secret: HtlcSecret,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_lease(lease, &lease.session_id, now_unix)?;
        self.record_fast_claim(&lease.session_id, secret, now_unix)
    }

    pub fn record_slow_claim_with_lease(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_lease(lease, &lease.session_id, now_unix)?;
        self.record_slow_claim(&lease.session_id, now_unix)
    }

    pub fn record_refunds_with_lease(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate_lease(lease, &lease.session_id, now_unix)?;
        self.record_refunds(&lease.session_id, now_unix)
    }

    /// Number of journal entries for one operation. Useful for telemetry and
    /// proving duplicate delivery collapsed into one semantic mutation.
    pub fn operation_count(
        &self,
        session_id: &str,
        operation: CoordinatorOperation,
    ) -> Result<usize, CoordinatorError> {
        let coordinator = self.lock()?;
        let session = coordinator
            .get_session(session_id)
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?;
        Ok(session
            .operation_journal
            .iter()
            .filter(|receipt| receipt.operation == operation)
            .count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HtlcCreateParams, HtlcHash, HtlcId, HtlcStatus, SwapPhase, VmTarget,
    };
    use std::sync::{Barrier, RwLock};
    use std::thread;

    fn record(id: u8, hash_lock: HtlcHash, now: u64) -> HtlcRecord {
        HtlcRecord {
            id: HtlcId(vec![id; 32]),
            params: HtlcCreateParams {
                vm: VmTarget::Svm,
                recipient: vec![1; 32],
                hash_lock,
                timelock: now + 3_600,
                asset: vec![2; 32],
                amount: 1_000,
            },
            status: HtlcStatus::Funded,
            created_at_block: 100,
            confirmations_required: 1,
            confirmations: 1,
            params_hash: [id; 32],
        }
    }

    fn seeded(
        id: &str,
        phase: SwapPhase,
        secret: &HtlcSecret,
        now: u64,
    ) -> (Arc<InMemoryPersistence>, ConcurrentSwapCoordinator<InMemoryPersistence>) {
        let persistence = Arc::new(InMemoryPersistence::new());
        let session = SwapSession {
            session_id: id.to_string(),
            hash_lock: secret.hash(),
            htlc_fast: None,
            htlc_slow: None,
            flash_legs: vec![],
            leg_outcomes: vec![],
            phase,
            timelock_fast: now + 3_600,
            timelock_slow: now + 7_200,
            created_at: now,
            updated_at: now,
            operation_journal: vec![],
            requires_merkle_verification: false,
        };
        persistence.save(&session);
        let coordinator = ConcurrentSwapCoordinator::with_persistence(
            CoordinatorConfig::default(),
            persistence.clone(),
        );
        (persistence, coordinator)
    }

    #[test]
    fn distributed_secret_replay_is_rejected_across_sessions() {
        let now = 1_700_000_000;
        let secret = HtlcSecret([0x57; 32]);
        let persistence = Arc::new(InMemoryPersistence::new());

        for id in ["secret-a", "secret-b"] {
            let mut session = SwapSession {
                session_id: id.to_string(),
                hash_lock: secret.hash(),
                htlc_fast: Some(record(13, secret.hash(), now)),
                htlc_slow: None,
                flash_legs: vec![],
                leg_outcomes: vec![],
                phase: SwapPhase::ClaimingFast,
                timelock_fast: now + 3_600,
                timelock_slow: now + 7_200,
                created_at: now,
                updated_at: now,
                operation_journal: vec![],
                requires_merkle_verification: false,
            };
            persistence.save(&session);
        }

        let store = Arc::new(crate::InMemoryDistributedLeaseStore::default());
        let proc_a = DistributedConcurrentSwapCoordinator::new(
            ConcurrentSwapCoordinator::with_persistence(
                CoordinatorConfig::default(),
                persistence.clone(),
            ),
            crate::DurableLeaseAuthority::new(store.clone()),
        );
        let proc_b = DistributedConcurrentSwapCoordinator::new(
            ConcurrentSwapCoordinator::with_persistence(
                CoordinatorConfig::default(),
                persistence,
            ),
            crate::DurableLeaseAuthority::new(store),
        );

        let lease_a = proc_a
            .acquire_session_lease("secret-a", "proc-a", now, 30)
            .unwrap();
        let lease_b = proc_b
            .acquire_session_lease("secret-b", "proc-b", now, 30)
            .unwrap();

        proc_a
            .record_fast_claim(&lease_a, secret.clone(), now + 1)
            .expect("first distributed secret owner");
        assert!(proc_b
            .record_fast_claim(&lease_b, secret, now + 1)
            .is_err());
    }

    #[test]
    fn distributed_takeover_refreshes_state_and_rejects_stale_commit() {
        let now = 1_700_000_000;
        let secret = HtlcSecret([0x56; 32]);
        let persistence = Arc::new(InMemoryPersistence::new());
        let session = SwapSession {
            session_id: "dist-refresh".to_string(),
            hash_lock: secret.hash(),
            htlc_fast: None,
            htlc_slow: None,
            flash_legs: vec![],
            leg_outcomes: vec![],
            phase: SwapPhase::Setup,
            timelock_fast: now + 3_600,
            timelock_slow: now + 7_200,
            created_at: now,
            updated_at: now,
            operation_journal: vec![],
            requires_merkle_verification: false,
        };
        persistence.save(&session);

        let lease_store = Arc::new(crate::InMemoryDistributedLeaseStore::default());
        let authority_a = crate::DurableLeaseAuthority::new(lease_store.clone());
        let authority_b = crate::DurableLeaseAuthority::new(lease_store);

        let proc_a = DistributedConcurrentSwapCoordinator::new(
            ConcurrentSwapCoordinator::with_persistence(
                CoordinatorConfig::default(),
                persistence.clone(),
            ),
            authority_a,
        );
        let proc_b = DistributedConcurrentSwapCoordinator::new(
            ConcurrentSwapCoordinator::with_persistence(
                CoordinatorConfig::default(),
                persistence.clone(),
            ),
            authority_b,
        );

        let lease_a = proc_a
            .acquire_session_lease("dist-refresh", "proc-a", now, 5)
            .unwrap();
        proc_a
            .record_htlc_fast(&lease_a, record(11, secret.hash(), now), now + 1)
            .unwrap();

        let lease_b = proc_b
            .acquire_session_lease("dist-refresh", "proc-b", now + 5, 30)
            .unwrap();

        // proc-b was constructed before proc-a committed, so this assertion
        // proves lease acquisition refreshed the persisted session.
        let refreshed = proc_b.session("dist-refresh").unwrap().unwrap();
        assert!(refreshed.htlc_fast.is_some());

        assert!(proc_a
            .record_htlc_slow(&lease_a, record(12, secret.hash(), now), now + 6)
            .is_err());
        proc_b
            .record_htlc_slow(&lease_b, record(12, secret.hash(), now), now + 6)
            .unwrap();

        let final_state = persistence.load("dist-refresh").unwrap();
        assert!(final_state.htlc_fast.is_some());
        assert!(final_state.htlc_slow.is_some());
    }

    #[test]
    fn identical_concurrent_lock_observations_collapse_to_one_mutation() {
        let now = 1_700_000_000;
        let secret = HtlcSecret([0x51; 32]);
        let (_persistence, coordinator) = seeded("race-lock", SwapPhase::Setup, &secret, now);
        let barrier = Arc::new(Barrier::new(3));
        let result_a = Arc::new(RwLock::new(None));
        let result_b = Arc::new(RwLock::new(None));
        let lock = record(1, secret.hash(), now);

        let a = {
            let coordinator = coordinator.clone();
            let barrier = barrier.clone();
            let result = result_a.clone();
            let lock = lock.clone();
            thread::spawn(move || {
                barrier.wait();
                *result.write().unwrap() =
                    Some(coordinator.record_htlc_fast("race-lock", lock, now));
            })
        };
        let b = {
            let coordinator = coordinator.clone();
            let barrier = barrier.clone();
            let result = result_b.clone();
            let lock = lock.clone();
            thread::spawn(move || {
                barrier.wait();
                *result.write().unwrap() =
                    Some(coordinator.record_htlc_fast("race-lock", lock, now + 1));
            })
        };

        barrier.wait();
        a.join().unwrap();
        b.join().unwrap();

        assert!(result_a.write().unwrap().take().unwrap().is_ok());
        assert!(result_b.write().unwrap().take().unwrap().is_ok());
        assert_eq!(
            coordinator
                .operation_count("race-lock", CoordinatorOperation::FastHtlcLock)
                .unwrap(),
            1
        );
    }

    #[test]
    fn conflicting_concurrent_lock_observations_yield_one_winner_one_conflict() {
        let now = 1_700_000_000;
        let secret = HtlcSecret([0x52; 32]);
        let (_persistence, coordinator) =
            seeded("race-conflict", SwapPhase::Setup, &secret, now);
        let barrier = Arc::new(Barrier::new(3));

        let handles: Vec<_> = [record(1, secret.hash(), now), record(2, secret.hash(), now)]
            .into_iter()
            .map(|lock| {
                let coordinator = coordinator.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    coordinator.record_htlc_fast("race-conflict", lock, now)
                })
            })
            .collect();

        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(
                    result,
                    Err(CoordinatorError::IdempotencyConflict {
                        operation: CoordinatorOperation::FastHtlcLock,
                        ..
                    })
                ))
                .count(),
            1
        );
        assert_eq!(
            coordinator
                .operation_count("race-conflict", CoordinatorOperation::FastHtlcLock)
                .unwrap(),
            1
        );
    }

    #[test]
    fn stale_fence_cannot_commit_after_takeover() {
        let now = 1_700_000_000;
        let secret = HtlcSecret([0x54; 32]);
        let (_persistence, coordinator) =
            seeded("lease-race", SwapPhase::Setup, &secret, now);
        let first = coordinator
            .acquire_session_lease("lease-race", "relayer-a", now, 5)
            .unwrap();
        let second = coordinator
            .acquire_session_lease("lease-race", "relayer-b", now + 5, 30)
            .unwrap();

        let lock = record(9, secret.hash(), now);
        assert!(coordinator
            .record_htlc_fast_with_lease(&first, lock.clone(), now + 6)
            .is_err());
        coordinator
            .record_htlc_fast_with_lease(&second, lock, now + 6)
            .expect("current fence may commit");
        assert_eq!(
            coordinator
                .operation_count("lease-race", CoordinatorOperation::FastHtlcLock)
                .unwrap(),
            1
        );
    }

    #[test]
    fn lease_for_one_session_cannot_mutate_another() {
        let now = 1_700_000_000;
        let secret = HtlcSecret([0x55; 32]);
        let (_persistence, coordinator) =
            seeded("lease-a", SwapPhase::Setup, &secret, now);
        let lease = coordinator
            .acquire_session_lease("lease-a", "relayer-a", now, 30)
            .unwrap();

        assert_eq!(lease.session_id, "lease-a");
        assert!(coordinator
            .validate_lease(&lease, "lease-b", now + 1)
            .is_err());
    }

    #[test]
    fn concurrent_fast_claim_retries_reveal_once() {
        let now = 1_700_000_000;
        let secret = HtlcSecret([0x53; 32]);
        let (_persistence, coordinator) =
            seeded("race-claim", SwapPhase::ClaimingFast, &secret, now);
        coordinator
            .execute(|inner| {
                inner
                    .get_session_mut("race-claim")
                    .expect("session")
                    .htlc_fast = Some(record(3, secret.hash(), now));
                Ok(())
            })
            .unwrap();

        let barrier = Arc::new(Barrier::new(3));
        let handles: Vec<_> = (0..2)
            .map(|offset| {
                let coordinator = coordinator.clone();
                let barrier = barrier.clone();
                let secret = secret.clone();
                thread::spawn(move || {
                    barrier.wait();
                    coordinator.record_fast_claim("race-claim", secret, now + offset)
                })
            })
            .collect();

        barrier.wait();
        for handle in handles {
            handle.join().unwrap().expect("idempotent fast claim");
        }

        assert_eq!(
            coordinator
                .operation_count("race-claim", CoordinatorOperation::FastClaim)
                .unwrap(),
            1
        );
        assert_eq!(
            coordinator.session("race-claim").unwrap().unwrap().phase,
            SwapPhase::ClaimingSlow
        );
    }
}
