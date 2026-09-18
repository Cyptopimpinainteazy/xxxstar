//! Immutable operation-attempt ledger for cross-domain execution.
//!
//! Idempotency answers "did this semantic operation already apply?".
//! This ledger answers "what actually happened on every try?" and binds one
//! canonical finalized result to a proof hash.

use crate::{CoordinatorError, CoordinatorOperation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationAttemptStatus {
    Started,
    Broadcast,
    Finalized,
    Failed,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationAttempt {
    pub session_id: String,
    pub operation: CoordinatorOperation,
    pub attempt_id: String,
    pub owner_id: String,
    pub fence: u64,
    pub domain: String,
    pub status: OperationAttemptStatus,
    pub tx_id: Option<String>,
    pub proof_hash: Option<[u8; 32]>,
    pub started_at: u64,
    pub updated_at: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalOperationResult {
    pub session_id: String,
    pub operation: CoordinatorOperation,
    pub attempt_id: String,
    pub tx_id: String,
    pub proof_hash: [u8; 32],
    pub finalized_at: u64,
}

/// Storage contract for immutable attempts and one canonical result per
/// semantic operation.
///
/// Implementations must reject duplicate attempt IDs with different contents
/// and must provide atomic "set canonical if absent or identical" semantics.
pub trait AttemptStore: Send + Sync + 'static {
    fn append_attempt(&self, attempt: &OperationAttempt) -> Result<(), CoordinatorError>;
    fn attempts(&self, session_id: &str) -> Result<Vec<OperationAttempt>, CoordinatorError>;
    fn canonical(
        &self,
        session_id: &str,
        operation: CoordinatorOperation,
    ) -> Result<Option<CanonicalOperationResult>, CoordinatorError>;
    fn set_canonical(
        &self,
        result: &CanonicalOperationResult,
    ) -> Result<(), CoordinatorError>;
}

#[derive(Default)]
pub struct InMemoryAttemptStore {
    attempts: RwLock<HashMap<String, Vec<OperationAttempt>>>,
    canonical: RwLock<HashMap<(String, CoordinatorOperation), CanonicalOperationResult>>,
}

impl AttemptStore for InMemoryAttemptStore {
    fn append_attempt(&self, attempt: &OperationAttempt) -> Result<(), CoordinatorError> {
        let mut guard = self
            .attempts
            .write()
            .map_err(|_| CoordinatorError::Internal("attempt ledger poisoned".into()))?;
        let entries = guard.entry(attempt.session_id.clone()).or_default();

        if let Some(existing) = entries
            .iter()
            .find(|entry| entry.attempt_id == attempt.attempt_id)
        {
            if existing == attempt {
                return Ok(());
            }
            return Err(CoordinatorError::Internal(format!(
                "attempt id '{}' reused with different immutable contents",
                attempt.attempt_id
            )));
        }

        entries.push(attempt.clone());
        Ok(())
    }

    fn attempts(&self, session_id: &str) -> Result<Vec<OperationAttempt>, CoordinatorError> {
        let guard = self
            .attempts
            .read()
            .map_err(|_| CoordinatorError::Internal("attempt ledger poisoned".into()))?;
        Ok(guard.get(session_id).cloned().unwrap_or_default())
    }

    fn canonical(
        &self,
        session_id: &str,
        operation: CoordinatorOperation,
    ) -> Result<Option<CanonicalOperationResult>, CoordinatorError> {
        let guard = self
            .canonical
            .read()
            .map_err(|_| CoordinatorError::Internal("canonical ledger poisoned".into()))?;
        Ok(guard.get(&(session_id.to_string(), operation)).cloned())
    }

    fn set_canonical(
        &self,
        result: &CanonicalOperationResult,
    ) -> Result<(), CoordinatorError> {
        let mut guard = self
            .canonical
            .write()
            .map_err(|_| CoordinatorError::Internal("canonical ledger poisoned".into()))?;
        let key = (result.session_id.clone(), result.operation);

        if let Some(existing) = guard.get(&key) {
            if existing == result {
                return Ok(());
            }
            return Err(CoordinatorError::IdempotencyConflict {
                operation: result.operation,
                existing: hex::encode(existing.proof_hash),
                incoming: hex::encode(result.proof_hash),
            });
        }

        guard.insert(key, result.clone());
        Ok(())
    }
}

/// Append-only coordinator-facing API.
///
/// Updates are represented by a new attempt snapshot rather than mutating the
/// prior record. Consumers reconstruct the latest state per attempt ID.
pub struct OperationAttemptLedger<S: AttemptStore> {
    store: S,
}

impl<S: AttemptStore> OperationAttemptLedger<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn record_started(
        &self,
        session_id: &str,
        operation: CoordinatorOperation,
        attempt_id: &str,
        owner_id: &str,
        fence: u64,
        domain: &str,
        now: u64,
    ) -> Result<OperationAttempt, CoordinatorError> {
        let attempt = OperationAttempt {
            session_id: session_id.to_string(),
            operation,
            attempt_id: attempt_id.to_string(),
            owner_id: owner_id.to_string(),
            fence,
            domain: domain.to_string(),
            status: OperationAttemptStatus::Started,
            tx_id: None,
            proof_hash: None,
            started_at: now,
            updated_at: now,
            error: None,
        };
        self.store.append_attempt(&attempt)?;
        Ok(attempt)
    }

    pub fn record_broadcast(
        &self,
        started: &OperationAttempt,
        tx_id: &str,
        now: u64,
    ) -> Result<OperationAttempt, CoordinatorError> {
        let attempt = OperationAttempt {
            status: OperationAttemptStatus::Broadcast,
            tx_id: Some(tx_id.to_string()),
            updated_at: now,
            ..started.clone()
        };
        self.append_snapshot(&attempt)?;
        Ok(attempt)
    }

    pub fn record_failed(
        &self,
        prior: &OperationAttempt,
        error: &str,
        now: u64,
    ) -> Result<OperationAttempt, CoordinatorError> {
        let attempt = OperationAttempt {
            status: OperationAttemptStatus::Failed,
            updated_at: now,
            error: Some(error.to_string()),
            ..prior.clone()
        };
        self.append_snapshot(&attempt)?;
        Ok(attempt)
    }

    pub fn record_finalized(
        &self,
        prior: &OperationAttempt,
        proof_hash: [u8; 32],
        now: u64,
    ) -> Result<CanonicalOperationResult, CoordinatorError> {
        let tx_id = prior.tx_id.clone().ok_or_else(|| {
            CoordinatorError::Internal("cannot finalize attempt before broadcast tx id".into())
        })?;

        let finalized = OperationAttempt {
            status: OperationAttemptStatus::Finalized,
            proof_hash: Some(proof_hash),
            updated_at: now,
            error: None,
            ..prior.clone()
        };
        self.append_snapshot(&finalized)?;

        let canonical = CanonicalOperationResult {
            session_id: finalized.session_id.clone(),
            operation: finalized.operation,
            attempt_id: finalized.attempt_id.clone(),
            tx_id,
            proof_hash,
            finalized_at: now,
        };
        self.store.set_canonical(&canonical)?;
        Ok(canonical)
    }

    pub fn history(
        &self,
        session_id: &str,
    ) -> Result<Vec<OperationAttempt>, CoordinatorError> {
        self.store.attempts(session_id)
    }

    pub fn canonical(
        &self,
        session_id: &str,
        operation: CoordinatorOperation,
    ) -> Result<Option<CanonicalOperationResult>, CoordinatorError> {
        self.store.canonical(session_id, operation)
    }

    fn append_snapshot(&self, next: &OperationAttempt) -> Result<(), CoordinatorError> {
        let history = self.store.attempts(&next.session_id)?;
        let previous = history
            .iter()
            .rev()
            .find(|entry| entry.attempt_id == next.attempt_id)
            .ok_or_else(|| {
                CoordinatorError::Internal(format!(
                    "attempt '{}' has no STARTED record",
                    next.attempt_id
                ))
            })?;

        if previous.operation != next.operation
            || previous.owner_id != next.owner_id
            || previous.fence != next.fence
            || previous.domain != next.domain
            || previous.started_at != next.started_at
        {
            return Err(CoordinatorError::Internal(format!(
                "attempt '{}' immutable identity changed",
                next.attempt_id
            )));
        }

        // Snapshot records need unique append IDs while retaining the semantic
        // attempt ID. Store implementation compares full records, so append
        // through a derived record ID is handled by preserving history here.
        //
        // InMemoryAttemptStore stores snapshots with the same attempt ID only
        // when identical, therefore we directly append via a synthetic clone
        // whose attempt_id is phase-qualified.
        let mut snapshot = next.clone();
        snapshot.attempt_id = format!(
            "{}#{}",
            next.attempt_id,
            match next.status {
                OperationAttemptStatus::Started => "started",
                OperationAttemptStatus::Broadcast => "broadcast",
                OperationAttemptStatus::Finalized => "finalized",
                OperationAttemptStatus::Failed => "failed",
                OperationAttemptStatus::Superseded => "superseded",
            }
        );
        self.store.append_attempt(&snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger() -> OperationAttemptLedger<InMemoryAttemptStore> {
        OperationAttemptLedger::new(InMemoryAttemptStore::default())
    }

    #[test]
    fn attempt_history_records_started_broadcast_and_finalized() {
        let ledger = ledger();
        let started = ledger
            .record_started(
                "swap-a",
                CoordinatorOperation::FastClaim,
                "attempt-1",
                "relayer-a",
                7,
                "ethereum",
                100,
            )
            .unwrap();
        let broadcast = ledger.record_broadcast(&started, "0xabc", 101).unwrap();
        let canonical = ledger
            .record_finalized(&broadcast, [0x11; 32], 102)
            .unwrap();

        assert_eq!(canonical.proof_hash, [0x11; 32]);
        assert_eq!(canonical.tx_id, "0xabc");
        assert_eq!(
            ledger
                .canonical("swap-a", CoordinatorOperation::FastClaim)
                .unwrap()
                .unwrap()
                .proof_hash,
            [0x11; 32]
        );
        assert_eq!(ledger.history("swap-a").unwrap().len(), 3);
    }

    #[test]
    fn canonical_proof_hash_is_immutable() {
        let ledger = ledger();
        let first = ledger
            .record_started(
                "swap-a",
                CoordinatorOperation::FastClaim,
                "attempt-1",
                "relayer-a",
                7,
                "ethereum",
                100,
            )
            .unwrap();
        let first = ledger.record_broadcast(&first, "0xaaa", 101).unwrap();
        ledger.record_finalized(&first, [0x22; 32], 102).unwrap();

        let second = ledger
            .record_started(
                "swap-a",
                CoordinatorOperation::FastClaim,
                "attempt-2",
                "relayer-b",
                8,
                "ethereum",
                103,
            )
            .unwrap();
        let second = ledger.record_broadcast(&second, "0xbbb", 104).unwrap();
        let err = ledger
            .record_finalized(&second, [0x33; 32], 105)
            .expect_err("second canonical proof must conflict");

        assert!(matches!(
            err,
            CoordinatorError::IdempotencyConflict {
                operation: CoordinatorOperation::FastClaim,
                ..
            }
        ));
    }

    #[test]
    fn failed_attempt_is_retained_without_becoming_canonical() {
        let ledger = ledger();
        let started = ledger
            .record_started(
                "swap-a",
                CoordinatorOperation::RefundBoth,
                "attempt-1",
                "relayer-a",
                4,
                "solana",
                100,
            )
            .unwrap();
        ledger
            .record_failed(&started, "rpc timeout", 101)
            .unwrap();

        assert!(ledger
            .canonical("swap-a", CoordinatorOperation::RefundBoth)
            .unwrap()
            .is_none());
        let history = ledger.history("swap-a").unwrap();
        assert_eq!(history.len(), 2);
        assert!(history
            .iter()
            .any(|a| a.status == OperationAttemptStatus::Failed));
    }
}
