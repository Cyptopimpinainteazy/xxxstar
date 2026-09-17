//! Durable outbox for X3 settlement proof submissions.
//!
//! The outbox closes the crash window between "we built the exact proof-set
//! call" and "we know what happened on-chain". After a broadcast is persisted,
//! recovery MUST query chain state/tx status rather than blindly broadcast a
//! second extrinsic.

use crate::CoordinatorError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::RwLock;

#[cfg(feature = "canonical-proofs")]
use crate::{SettlementProofPurpose, SettlementSubmissionEnvelope};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementOutboxStatus {
    Prepared,
    /// Exact signed extrinsic bytes + tx hash persisted before network send.
    Signed,
    Broadcast,
    Included,
    TerminalObserved,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementOutboxRecovery {
    SignPrepared,
    /// Query tx hash first; if absent, rebroadcast the exact persisted bytes.
    QueryOrRebroadcastExactSignedExtrinsic,
    QueryBroadcastStatus,
    ObserveSettlementState,
    Done,
    ManualHalt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementOutboxRecord {
    pub submission_id: [u8; 32],
    pub session_id: String,
    pub runtime_intent_id: [u8; 32],
    /// 0 = Claim, 1 = Refund.
    pub purpose: u8,
    pub call_index: u8,
    pub call_args_hash: [u8; 32],
    pub proof_hashes: Vec<[u8; 32]>,
    pub status: SettlementOutboxStatus,
    pub tx_id: Option<String>,
    /// Exact signed extrinsic bytes. Persisted before any network send.
    pub signed_extrinsic: Option<Vec<u8>>,
    pub block_number: Option<u64>,
    pub owner_id: Option<String>,
    pub fence: Option<u64>,
    pub prepared_at: u64,
    pub updated_at: u64,
    pub error: Option<String>,
}

pub trait SettlementOutboxStore: Send + Sync + 'static {
    /// Atomically append `next` only when the current latest record exactly
    /// equals `expected`. `expected=None` means the history must be empty.
    /// Re-appending an identical current latest record is idempotent.
    fn compare_and_append(
        &self,
        submission_id: [u8; 32],
        expected: Option<&SettlementOutboxRecord>,
        next: &SettlementOutboxRecord,
    ) -> Result<(), CoordinatorError>;

    fn history(
        &self,
        submission_id: [u8; 32],
    ) -> Result<Vec<SettlementOutboxRecord>, CoordinatorError>;
}

#[derive(Default)]
pub struct InMemorySettlementOutboxStore {
    entries: RwLock<HashMap<[u8; 32], Vec<SettlementOutboxRecord>>>,
}

impl SettlementOutboxStore for InMemorySettlementOutboxStore {
    fn compare_and_append(
        &self,
        submission_id: [u8; 32],
        expected: Option<&SettlementOutboxRecord>,
        next: &SettlementOutboxRecord,
    ) -> Result<(), CoordinatorError> {
        if next.submission_id != submission_id {
            return Err(CoordinatorError::Internal(
                "settlement outbox key does not match record submission id".into(),
            ));
        }

        let mut guard = self
            .entries
            .write()
            .map_err(|_| CoordinatorError::Internal("settlement outbox poisoned".into()))?;
        let history = guard.entry(submission_id).or_default();
        let current = history.last();

        if current == Some(next) {
            return Ok(());
        }
        if current != expected {
            return Err(CoordinatorError::Internal(
                "settlement outbox concurrent transition conflict".into(),
            ));
        }

        history.push(next.clone());
        Ok(())
    }

    fn history(
        &self,
        submission_id: [u8; 32],
    ) -> Result<Vec<SettlementOutboxRecord>, CoordinatorError> {
        let guard = self
            .entries
            .read()
            .map_err(|_| CoordinatorError::Internal("settlement outbox poisoned".into()))?;
        Ok(guard.get(&submission_id).cloned().unwrap_or_default())
    }
}

pub struct SettlementSubmissionOutbox<S: SettlementOutboxStore> {
    store: S,
}

impl<S: SettlementOutboxStore> SettlementSubmissionOutbox<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    #[cfg(feature = "canonical-proofs")]
    pub fn prepare(
        &self,
        session_id: &str,
        envelope: &SettlementSubmissionEnvelope,
        now: u64,
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        let call_args = envelope.scale_call_args();
        let call_args_hash = sha256(&call_args);
        let purpose = match envelope.purpose {
            SettlementProofPurpose::Claim => 0,
            SettlementProofPurpose::Refund => 1,
        };

        let mut id_preimage = Vec::with_capacity(1 + call_args.len());
        id_preimage.push(envelope.call_index());
        id_preimage.extend_from_slice(&call_args);
        let submission_id = sha256(&id_preimage);

        let record = SettlementOutboxRecord {
            submission_id,
            session_id: session_id.to_string(),
            runtime_intent_id: envelope.runtime_intent_id,
            purpose,
            call_index: envelope.call_index(),
            call_args_hash,
            proof_hashes: envelope.proof_hashes(),
            status: SettlementOutboxStatus::Prepared,
            tx_id: None,
            signed_extrinsic: None,
            block_number: None,
            owner_id: None,
            fence: None,
            prepared_at: now,
            updated_at: now,
            error: None,
        };

        if let Some(existing) = self.latest(submission_id)? {
            self.ensure_same_submission_identity(&existing, &record)?;
            return Ok(existing);
        }

        self.store.compare_and_append(submission_id, None, &record)?;
        Ok(record)
    }

    /// Persist the exact signed extrinsic and deterministic tx hash BEFORE
    /// the caller is allowed to send anything to the network.
    pub fn record_signed(
        &self,
        prepared: &SettlementOutboxRecord,
        tx_id: &str,
        signed_extrinsic: Vec<u8>,
        owner_id: &str,
        fence: u64,
        now: u64,
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        if signed_extrinsic.is_empty() {
            return Err(CoordinatorError::Internal(
                "signed settlement extrinsic cannot be empty".into(),
            ));
        }

        let latest = self.require_latest(prepared.submission_id)?;
        self.ensure_same_submission_identity(&latest, prepared)?;

        if latest.status == SettlementOutboxStatus::Signed {
            if latest.tx_id.as_deref() == Some(tx_id)
                && latest.signed_extrinsic.as_deref() == Some(signed_extrinsic.as_slice())
            {
                return Ok(latest);
            }
            return Err(CoordinatorError::Internal(
                "settlement submission already signed with different bytes or tx id".into(),
            ));
        }

        if latest.status != SettlementOutboxStatus::Prepared {
            return Err(CoordinatorError::Internal(format!(
                "cannot sign settlement submission from status {:?}",
                latest.status
            )));
        }

        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::Signed,
            tx_id: Some(tx_id.to_string()),
            signed_extrinsic: Some(signed_extrinsic),
            owner_id: Some(owner_id.to_string()),
            fence: Some(fence),
            updated_at: now,
            ..latest
        };
        self.store
            .compare_and_append(next.submission_id, Some(&latest), &next)
            .or_else(|_| {
                // Another process may have won the transition after our read.
                // Re-read and accept only the exact same signed transaction.
                let current = self.require_latest(prepared.submission_id)?;
                if current.status == SettlementOutboxStatus::Signed
                    && current.tx_id == next.tx_id
                    && current.signed_extrinsic == next.signed_extrinsic
                {
                    Ok(())
                } else {
                    Err(CoordinatorError::Internal(
                        "settlement signing transition conflict".into(),
                    ))
                }
            })?;
        self.require_latest(prepared.submission_id)
    }

    /// Record that the exact persisted signed extrinsic was submitted to RPC.
    /// This transition happens after send; a crash before it leaves status
    /// Signed, which recovery handles by querying/rebroadcasting the SAME bytes.
    pub fn record_broadcast(
        &self,
        signed: &SettlementOutboxRecord,
        now: u64,
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        let latest = self.require_latest(signed.submission_id)?;
        self.ensure_same_submission_identity(&latest, signed)?;

        if latest.status == SettlementOutboxStatus::Broadcast {
            return Ok(latest);
        }
        if latest.status != SettlementOutboxStatus::Signed {
            return Err(CoordinatorError::Internal(format!(
                "cannot mark settlement broadcast from status {:?}",
                latest.status
            )));
        }
        if latest.tx_id.is_none()
            || latest
                .signed_extrinsic
                .as_ref()
                .map_or(true, Vec::is_empty)
        {
            return Err(CoordinatorError::Internal(
                "signed settlement record is missing tx id or extrinsic bytes".into(),
            ));
        }

        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::Broadcast,
            updated_at: now,
            ..latest.clone()
        };
        self.store
            .compare_and_append(next.submission_id, Some(&latest), &next)?;
        Ok(next)
    }

    pub fn record_included(
        &self,
        broadcast: &SettlementOutboxRecord,
        block_number: u64,
        now: u64,
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        let latest = self.require_latest(broadcast.submission_id)?;
        self.ensure_same_submission_identity(&latest, broadcast)?;

        if latest.status == SettlementOutboxStatus::Included {
            if latest.block_number == Some(block_number) {
                return Ok(latest);
            }
            return Err(CoordinatorError::Internal(
                "settlement submission inclusion block changed".into(),
            ));
        }

        if latest.status != SettlementOutboxStatus::Broadcast {
            return Err(CoordinatorError::Internal(format!(
                "cannot mark settlement submission included from status {:?}",
                latest.status
            )));
        }

        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::Included,
            block_number: Some(block_number),
            updated_at: now,
            ..latest
        };
        self.store.compare_and_append(next.submission_id, Some(&latest), &next)?;
        Ok(next)
    }

    pub fn record_terminal_observed(
        &self,
        included: &SettlementOutboxRecord,
        now: u64,
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        let latest = self.require_latest(included.submission_id)?;
        self.ensure_same_submission_identity(&latest, included)?;

        if latest.status == SettlementOutboxStatus::TerminalObserved {
            return Ok(latest);
        }
        if latest.status != SettlementOutboxStatus::Included {
            return Err(CoordinatorError::Internal(format!(
                "cannot mark settlement terminal from status {:?}",
                latest.status
            )));
        }

        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::TerminalObserved,
            updated_at: now,
            ..latest
        };
        self.store.compare_and_append(next.submission_id, Some(&latest), &next)?;
        Ok(next)
    }

    pub fn record_failed(
        &self,
        current: &SettlementOutboxRecord,
        error: &str,
        now: u64,
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        let latest = self.require_latest(current.submission_id)?;
        self.ensure_same_submission_identity(&latest, current)?;
        if matches!(
            latest.status,
            SettlementOutboxStatus::Included | SettlementOutboxStatus::TerminalObserved
        ) {
            return Err(CoordinatorError::Internal(
                "cannot fail an already included/terminal settlement submission".into(),
            ));
        }

        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::Failed,
            error: Some(error.to_string()),
            updated_at: now,
            ..latest
        };
        self.store.compare_and_append(next.submission_id, Some(&latest), &next)?;
        Ok(next)
    }

    pub fn latest(
        &self,
        submission_id: [u8; 32],
    ) -> Result<Option<SettlementOutboxRecord>, CoordinatorError> {
        Ok(self.store.history(submission_id)?.last().cloned())
    }

    pub fn recovery(
        &self,
        submission_id: [u8; 32],
    ) -> Result<Option<SettlementOutboxRecovery>, CoordinatorError> {
        let Some(latest) = self.latest(submission_id)? else {
            return Ok(None);
        };
        let action = match latest.status {
            SettlementOutboxStatus::Prepared => SettlementOutboxRecovery::SignPrepared,
            SettlementOutboxStatus::Signed => {
                SettlementOutboxRecovery::QueryOrRebroadcastExactSignedExtrinsic
            }
            SettlementOutboxStatus::Broadcast => SettlementOutboxRecovery::QueryBroadcastStatus,
            SettlementOutboxStatus::Included => SettlementOutboxRecovery::ObserveSettlementState,
            SettlementOutboxStatus::TerminalObserved => SettlementOutboxRecovery::Done,
            SettlementOutboxStatus::Failed => SettlementOutboxRecovery::ManualHalt,
        };
        Ok(Some(action))
    }

    fn require_latest(
        &self,
        submission_id: [u8; 32],
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        self.latest(submission_id)?.ok_or_else(|| {
            CoordinatorError::Internal("settlement submission is not prepared".into())
        })
    }

    fn ensure_same_submission_identity(
        &self,
        existing: &SettlementOutboxRecord,
        incoming: &SettlementOutboxRecord,
    ) -> Result<(), CoordinatorError> {
        if existing.submission_id != incoming.submission_id
            || existing.session_id != incoming.session_id
            || existing.runtime_intent_id != incoming.runtime_intent_id
            || existing.purpose != incoming.purpose
            || existing.call_index != incoming.call_index
            || existing.call_args_hash != incoming.call_args_hash
            || existing.proof_hashes != incoming.proof_hashes
            || existing.prepared_at != incoming.prepared_at
        {
            return Err(CoordinatorError::Internal(
                "settlement outbox submission identity changed".into(),
            ));
        }
        Ok(())
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared() -> SettlementOutboxRecord {
        SettlementOutboxRecord {
            submission_id: [1u8; 32],
            session_id: "swap-a".into(),
            runtime_intent_id: [2u8; 32],
            purpose: 0,
            call_index: 33,
            call_args_hash: [3u8; 32],
            proof_hashes: vec![[4u8; 32]],
            status: SettlementOutboxStatus::Prepared,
            tx_id: None,
            signed_extrinsic: None,
            block_number: None,
            owner_id: None,
            fence: None,
            prepared_at: 100,
            updated_at: 100,
            error: None,
        }
    }

    #[test]
    fn broadcast_recovery_never_recommends_blind_rebroadcast() {
        let outbox = SettlementSubmissionOutbox::new(
            InMemorySettlementOutboxStore::default(),
        );
        let p = prepared();
        outbox.store.compare_and_append(p.submission_id, None, &p).unwrap();
        let signed = outbox
            .record_signed(&p, "0xabc", vec![1, 2, 3], "worker-a", 7, 101)
            .unwrap();

        assert_eq!(
            outbox.recovery(signed.submission_id).unwrap(),
            Some(SettlementOutboxRecovery::QueryOrRebroadcastExactSignedExtrinsic)
        );

        let b = outbox.record_broadcast(&signed, 102).unwrap();
        assert_eq!(
            outbox.recovery(b.submission_id).unwrap(),
            Some(SettlementOutboxRecovery::QueryBroadcastStatus)
        );
    }

    #[test]
    fn second_different_broadcast_is_rejected() {
        let outbox = SettlementSubmissionOutbox::new(
            InMemorySettlementOutboxStore::default(),
        );
        let p = prepared();
        outbox.store.compare_and_append(p.submission_id, None, &p).unwrap();
        outbox
            .record_signed(&p, "0xabc", vec![1, 2, 3], "worker-a", 7, 101)
            .unwrap();

        assert!(outbox
            .record_signed(&p, "0xdef", vec![4, 5, 6], "worker-b", 8, 102)
            .is_err());
    }

    #[test]
    fn included_submission_cannot_be_failed() {
        let outbox = SettlementSubmissionOutbox::new(
            InMemorySettlementOutboxStore::default(),
        );
        let p = prepared();
        outbox.store.compare_and_append(p.submission_id, None, &p).unwrap();
        let signed = outbox
            .record_signed(&p, "0xabc", vec![1, 2, 3], "worker-a", 7, 101)
            .unwrap();
        let b = outbox.record_broadcast(&signed, 102).unwrap();
        let i = outbox.record_included(&b, 55, 103).unwrap();

        assert!(outbox.record_failed(&i, "late timeout", 103).is_err());
    }
}
