//! Thread-safe coordinator façade.
//!
//! The core SwapCoordinator intentionally owns mutable session state. This
//! wrapper serializes concurrent relayer/task access through one mutex so the
//! persisted idempotency journal from the underlying coordinator is evaluated
//! atomically with each mutation.

use crate::{
    CoordinatorConfig, CoordinatorError, CoordinatorOperation, HtlcRecord, HtlcSecret,
    InMemoryPersistence, SessionPersistence, SwapCoordinator, SwapSession,
};
use std::sync::{Arc, Mutex, MutexGuard};

/// Shared coordinator handle for concurrent relayers/workers.
///
/// This provides process-local linearization. Cross-process duplicate delivery
/// is still resolved by the coordinator's persisted idempotency journal.
pub struct ConcurrentSwapCoordinator<P: SessionPersistence = InMemoryPersistence> {
    inner: Arc<Mutex<SwapCoordinator<P>>>,
}

impl<P: SessionPersistence> Clone for ConcurrentSwapCoordinator<P> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<P: SessionPersistence> ConcurrentSwapCoordinator<P> {
    pub fn new(coordinator: SwapCoordinator<P>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(coordinator)),
        }
    }

    pub fn with_persistence(config: CoordinatorConfig, persistence: Arc<P>) -> Self {
        Self::new(SwapCoordinator::with_persistence(config, persistence))
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
