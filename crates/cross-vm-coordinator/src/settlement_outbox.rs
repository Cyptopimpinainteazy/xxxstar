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
pub use x3_release_beacon::{
    release_beacon_statement_hash, ReleaseBeaconAttestation, ReleaseBeaconError,
    ReleaseBeaconEvidence, ReleaseBeaconPolicy, VerifiedReleaseBeacon,
    RELEASE_BEACON_PUBLIC_KEY_LEN, RELEASE_BEACON_SIGNATURE_LEN,
};

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
    /// Monotonic verified X3 beacon floor attached at broadcast time.
    #[serde(default)]
    pub release_beacon_number: Option<u64>,
    #[serde(default)]
    pub release_beacon_hash: Option<[u8; 32]>,
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

    /// Read the atomically advanced X3 release beacon floor.
    fn release_beacon_floor(&self) -> Result<Option<(u64, [u8; 32])>, CoordinatorError> {
        Err(CoordinatorError::Internal(
            "settlement outbox store does not expose an atomic release beacon floor".into(),
        ))
    }

    /// Atomically enforce the monotonic beacon floor and append `next` only
    /// when the current latest record equals `expected`.
    ///
    /// Stores must fail closed when they cannot make the floor check and append
    /// one atomic decision. A separate check followed by `compare_and_append`
    /// lets two submissions race different beacons.
    fn compare_and_append_release(
        &self,
        submission_id: [u8; 32],
        expected: Option<&SettlementOutboxRecord>,
        next: &SettlementOutboxRecord,
        _release_beacon_number: u64,
        _release_beacon_hash: [u8; 32],
    ) -> Result<(), CoordinatorError> {
        let _ = (submission_id, expected, next);
        Err(CoordinatorError::Internal(
            "settlement outbox store cannot atomically append a release beacon".into(),
        ))
    }

    /// Enumerate every submission held by this outbox so release ordering can
    /// be derived from the complete pending set rather than a caller's subset.
    fn submission_ids(&self) -> Result<Vec<[u8; 32]>, CoordinatorError>;
}

#[derive(Default)]
pub struct InMemorySettlementOutboxStore {
    entries: RwLock<HashMap<[u8; 32], Vec<SettlementOutboxRecord>>>,
    release_beacon_floor: RwLock<Option<(u64, [u8; 32])>>,
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

    fn release_beacon_floor(&self) -> Result<Option<(u64, [u8; 32])>, CoordinatorError> {
        self.release_beacon_floor
            .read()
            .map(|floor| *floor)
            .map_err(|_| CoordinatorError::Internal("settlement outbox poisoned".into()))
    }

    fn compare_and_append_release(
        &self,
        submission_id: [u8; 32],
        expected: Option<&SettlementOutboxRecord>,
        next: &SettlementOutboxRecord,
        release_beacon_number: u64,
        release_beacon_hash: [u8; 32],
    ) -> Result<(), CoordinatorError> {
        if next.submission_id != submission_id {
            return Err(CoordinatorError::Internal(
                "settlement outbox key does not match record submission id".into(),
            ));
        }

        // The floor lock is always acquired before the entries lock. No other
        // path acquires them in reverse order.
        let mut floor = self
            .release_beacon_floor
            .write()
            .map_err(|_| CoordinatorError::Internal("settlement outbox poisoned".into()))?;
        let mut entries = self
            .entries
            .write()
            .map_err(|_| CoordinatorError::Internal("settlement outbox poisoned".into()))?;

        enforce_release_beacon_floor(*floor, release_beacon_number, release_beacon_hash)?;

        let history = entries.entry(submission_id).or_default();
        let current = history.last();
        if current != Some(next) {
            if current != expected {
                return Err(CoordinatorError::Internal(
                    "settlement outbox concurrent transition conflict".into(),
                ));
            }
            history.push(next.clone());
        }

        *floor = Some((release_beacon_number, release_beacon_hash));
        Ok(())
    }

    fn submission_ids(&self) -> Result<Vec<[u8; 32]>, CoordinatorError> {
        let guard = self
            .entries
            .read()
            .map_err(|_| CoordinatorError::Internal("settlement outbox poisoned".into()))?;
        Ok(guard.keys().copied().collect())
    }
}

fn enforce_release_beacon_floor(
    floor: Option<(u64, [u8; 32])>,
    block_number: u64,
    block_hash: [u8; 32],
) -> Result<(), CoordinatorError> {
    if let Some((number, hash)) = floor {
        if block_number < number {
            return Err(CoordinatorError::Internal(format!(
                "refusing stale release beacon {}:{}; persisted beacon floor is {number}:{}",
                block_number,
                hex::encode(block_hash),
                hex::encode(hash)
            )));
        }
        if block_number == number && block_hash != hash {
            return Err(CoordinatorError::Internal(format!(
                "refusing conflicting release beacon {}:{}; persisted beacon is {}:{number}",
                block_number,
                hex::encode(block_hash),
                hex::encode(hash)
            )));
        }
    }
    Ok(())
}

pub struct SettlementSubmissionOutbox<S: SettlementOutboxStore> {
    store: S,
}

impl<S: SettlementOutboxStore> SettlementSubmissionOutbox<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Canonical release key for one submission under a finalized X3 block hash.
    ///
    /// The block hash is the cross-domain ordering beacon. Binding it before the
    /// submission id makes the key unpredictable before finality and independent
    /// of relayer arrival order.
    pub fn release_order_key(x3_block_hash: [u8; 32], submission_id: [u8; 32]) -> [u8; 32] {
        let mut preimage = Vec::with_capacity(64);
        preimage.extend_from_slice(b"X3:CROSS_DOMAIN_RELEASE_ORDER:V1");
        preimage.extend_from_slice(&x3_block_hash);
        preimage.extend_from_slice(&submission_id);
        sha256(&preimage)
    }

    /// Every submission that still needs a release decision, in canonical order.
    ///
    /// Prepared records participate so a relayer cannot let a later submission
    /// overtake one it has already accepted but not yet signed. Broadcast and
    /// terminal records have had their decision and no longer block the next.
    pub fn canonical_release_order(
        &self,
        beacon: &VerifiedReleaseBeacon,
    ) -> Result<Vec<[u8; 32]>, CoordinatorError> {
        let mut candidates = Vec::new();
        for submission_id in self.store.submission_ids()? {
            let Some(latest) = self.latest(submission_id)? else {
                continue;
            };
            if matches!(
                latest.status,
                SettlementOutboxStatus::Prepared | SettlementOutboxStatus::Signed
            ) {
                candidates.push((
                    Self::release_order_key(beacon.block_hash(), submission_id),
                    submission_id,
                ));
            }
        }
        candidates.sort();
        Ok(candidates
            .into_iter()
            .map(|(_, submission_id)| submission_id)
            .collect())
    }

    /// Broadcast only the canonical next settlement for this finalized X3 block.
    pub fn record_broadcast_ordered(
        &self,
        signed: &SettlementOutboxRecord,
        beacon: &VerifiedReleaseBeacon,
        now: u64,
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        enforce_release_beacon_floor(
            self.store.release_beacon_floor()?,
            beacon.block_number(),
            beacon.block_hash(),
        )?;
        let order = self.canonical_release_order(beacon)?;
        let next = order.first().copied().ok_or_else(|| {
            CoordinatorError::Internal("settlement release order is empty at broadcast time".into())
        })?;
        if next != signed.submission_id {
            return Err(CoordinatorError::Internal(format!(
                "refusing out-of-order settlement release: canonical next submission is {}, \
                 attempted {}",
                hex::encode(next),
                hex::encode(signed.submission_id)
            )));
        }
        let result = self.record_broadcast_transition(signed, Some(beacon), now);
        if result.is_err() {
            // Map a concurrent floor winner to the same explicit refusal a
            // sequential caller receives. The store still makes the final,
            // atomic decision; this only improves the surfaced error.
            enforce_release_beacon_floor(
                self.store.release_beacon_floor()?,
                beacon.block_number(),
                beacon.block_hash(),
            )?;
        }
        result
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
            release_beacon_number: None,
            release_beacon_hash: None,
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

        let expected = latest.clone();
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
            .compare_and_append(next.submission_id, Some(&expected), &next)
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
    pub(crate) fn record_broadcast(
        &self,
        signed: &SettlementOutboxRecord,
        now: u64,
    ) -> Result<SettlementOutboxRecord, CoordinatorError> {
        self.record_broadcast_transition(signed, None, now)
    }

    fn record_broadcast_transition(
        &self,
        signed: &SettlementOutboxRecord,
        beacon: Option<&VerifiedReleaseBeacon>,
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
                .is_none_or(Vec::is_empty)
        {
            return Err(CoordinatorError::Internal(
                "signed settlement record is missing tx id or extrinsic bytes".into(),
            ));
        }

        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::Broadcast,
            release_beacon_number: beacon.map(VerifiedReleaseBeacon::block_number),
            release_beacon_hash: beacon.map(VerifiedReleaseBeacon::block_hash),
            updated_at: now,
            ..latest.clone()
        };
        if let Some(beacon) = beacon {
            self.store.compare_and_append_release(
                next.submission_id,
                Some(&latest),
                &next,
                beacon.block_number(),
                beacon.block_hash(),
            )?;
        } else {
            self.store
                .compare_and_append(next.submission_id, Some(&latest), &next)?;
        }
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

        let expected = latest.clone();
        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::Included,
            block_number: Some(block_number),
            updated_at: now,
            ..latest
        };
        self.store.compare_and_append(next.submission_id, Some(&expected), &next)?;
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

        let expected = latest.clone();
        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::TerminalObserved,
            updated_at: now,
            ..latest
        };
        self.store.compare_and_append(next.submission_id, Some(&expected), &next)?;
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

        let expected = latest.clone();
        let next = SettlementOutboxRecord {
            status: SettlementOutboxStatus::Failed,
            error: Some(error.to_string()),
            updated_at: now,
            ..latest
        };
        self.store.compare_and_append(next.submission_id, Some(&expected), &next)?;
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
    use ed25519_dalek::{Signer, SigningKey};
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn attestor_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; RELEASE_BEACON_PUBLIC_KEY_LEN])
    }

    fn attestation(
        attestor_id: &str,
        seed: u8,
        block_number: u64,
        block_hash: [u8; 32],
    ) -> ReleaseBeaconAttestation {
        let key = attestor_key(seed);
        let statement = release_beacon_statement_hash(
            attestor_id,
            block_number,
            block_hash,
        );
        ReleaseBeaconAttestation {
            attestor_id: attestor_id.into(),
            public_key: key.verifying_key().to_bytes(),
            signature: key.sign(&statement).to_bytes().to_vec(),
        }
    }

    fn verified_beacon(
        block_number: u64,
        block_hash: [u8; 32],
        required: u32,
    ) -> VerifiedReleaseBeacon {
        let policy = ReleaseBeaconPolicy::new(
            required,
            [1u8, 2, 3]
                .into_iter()
                .map(|seed| attestor_key(seed).verifying_key().to_bytes())
                .collect(),
        )
        .unwrap();
        policy
            .verify(&ReleaseBeaconEvidence {
                block_number,
                block_hash,
                attestations: vec![
                    attestation("validator-a", 1, block_number, block_hash),
                    attestation("validator-b", 2, block_number, block_hash),
                    attestation("validator-c", 3, block_number, block_hash),
                ],
            })
            .unwrap()
    }

    fn prepared() -> SettlementOutboxRecord {
        prepared_with([1u8; 32], [2u8; 32])
    }

    fn prepared_with(
        submission_id: [u8; 32],
        runtime_intent_id: [u8; 32],
    ) -> SettlementOutboxRecord {
        SettlementOutboxRecord {
            submission_id,
            session_id: "swap-a".into(),
            runtime_intent_id,
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
            release_beacon_number: None,
            release_beacon_hash: None,
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

    #[test]
    fn cross_domain_release_order_refuses_a_later_relayer_choice() {
        let outbox =
            SettlementSubmissionOutbox::new(InMemorySettlementOutboxStore::default());
        let first = prepared_with([1u8; 32], [11u8; 32]);
        let second = prepared_with([2u8; 32], [22u8; 32]);
        outbox
            .store
            .compare_and_append(first.submission_id, None, &first)
            .unwrap();
        outbox
            .store
            .compare_and_append(second.submission_id, None, &second)
            .unwrap();
        let first_signed = outbox
            .record_signed(&first, "0xfirst", vec![1, 2, 3], "worker-a", 7, 101)
            .unwrap();
        let second_signed = outbox
            .record_signed(&second, "0xsecond", vec![4, 5, 6], "worker-a", 7, 102)
            .unwrap();

        let beacon = verified_beacon(77, [0x91; 32], 2);
        let order = outbox.canonical_release_order(&beacon).unwrap();
        assert_eq!(order.len(), 2);
        let canonical_first = if order[0] == first.submission_id {
            &first_signed
        } else {
            &second_signed
        };
        let canonical_second = if order[1] == second.submission_id {
            &second_signed
        } else {
            &first_signed
        };

        let error = outbox
            .record_broadcast_ordered(canonical_second, &beacon, 103)
            .expect_err("the later release must be refused");
        assert!(error
            .to_string()
            .contains("out-of-order settlement release"));

        let first_broadcast = outbox
            .record_broadcast_ordered(canonical_first, &beacon, 104)
            .expect("the canonical first release");
        assert_eq!(first_broadcast.release_beacon_number, Some(77));
        assert_eq!(first_broadcast.release_beacon_hash, Some([0x91; 32]));

        let stale = verified_beacon(76, [0x90; 32], 2);
        let error = outbox
            .record_broadcast_ordered(canonical_second, &stale, 105)
            .expect_err("an older validly signed beacon must not reorder releases");
        assert!(error.to_string().contains("stale release beacon"));

        let conflicting = verified_beacon(77, [0x92; 32], 2);
        let error = outbox
            .record_broadcast_ordered(canonical_second, &conflicting, 105)
            .expect_err("two hashes at one height must not be interchangeable");
        assert!(error.to_string().contains("conflicting release beacon"));

        outbox
            .record_broadcast_ordered(canonical_second, &beacon, 106)
            .expect("the second release becomes canonical after the first");
    }

    #[test]
    fn concurrent_release_floor_claim_is_atomic() {
        let first = prepared_with([1u8; 32], [11u8; 32]);
        let second = prepared_with([2u8; 32], [22u8; 32]);
        let store = Arc::new(InMemorySettlementOutboxStore::default());
        let barrier = Arc::new(Barrier::new(3));

        let contenders = [
            (first.clone(), [0x91; 32]),
            (second.clone(), [0x92; 32]),
        ]
        .into_iter()
        .map(|(record, hash)| {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                store.compare_and_append_release(
                    record.submission_id,
                    None,
                    &record,
                    77,
                    hash,
                )
            })
        })
        .collect::<Vec<_>>();
        barrier.wait();

        let accepted = contenders
            .into_iter()
            .map(|worker| worker.join().unwrap().is_ok())
            .filter(|accepted| *accepted)
            .count();
        assert_eq!(accepted, 1, "one conflicting beacon must win atomically");
        assert!(store.release_beacon_floor().unwrap().is_some());
    }

    #[test]
    fn release_beacon_requires_valid_authorized_threshold_signatures() {
        let block_hash = [0x91; 32];
        let policy = ReleaseBeaconPolicy::new(
            2,
            [1u8, 2]
                .into_iter()
                .map(|seed| attestor_key(seed).verifying_key().to_bytes())
                .collect(),
        )
        .unwrap();

        let forged = ReleaseBeaconEvidence {
            block_number: 77,
            block_hash,
            attestations: vec![attestation("validator-a", 1, 77, block_hash)],
        };
        assert!(policy.verify(&forged).is_err());

        let mut outsider = attestation("validator-outside", 9, 77, block_hash);
        outsider.public_key = attestor_key(9).verifying_key().to_bytes();
        let unauthorized = ReleaseBeaconEvidence {
            block_number: 77,
            block_hash,
            attestations: vec![
                attestation("validator-a", 1, 77, block_hash),
                outsider,
            ],
        };
        assert!(policy.verify(&unauthorized).is_err());
    }

    #[test]
    fn release_beacon_signatures_bind_height_hash_and_attestor_identity() {
        let block_hash = [0x91; 32];
        let policy = ReleaseBeaconPolicy::new(
            1,
            vec![attestor_key(1).verifying_key().to_bytes()],
        )
        .unwrap();

        let mut wrong_height = attestation("validator-a", 1, 77, block_hash);
        wrong_height.attestor_id = "validator-b".into();
        let evidence = ReleaseBeaconEvidence {
            block_number: 78,
            block_hash,
            attestations: vec![wrong_height],
        };
        assert!(policy.verify(&evidence).is_err());

        let evidence = ReleaseBeaconEvidence {
            block_number: 77,
            block_hash: [0x92; 32],
            attestations: vec![attestation("validator-a", 1, 77, block_hash)],
        };
        assert!(policy.verify(&evidence).is_err());
    }

    #[test]
    fn release_beacon_policy_counts_distinct_keys_once() {
        let repeated = attestor_key(1).verifying_key().to_bytes();
        assert!(ReleaseBeaconPolicy::new(2, vec![repeated, repeated]).is_err());
    }

    #[test]
    fn release_order_key_changes_with_the_finalized_beacon() {
        let submission_id = [7u8; 32];
        assert_ne!(
            SettlementSubmissionOutbox::<InMemorySettlementOutboxStore>::release_order_key(
                [1u8; 32],
                submission_id
            ),
            SettlementSubmissionOutbox::<InMemorySettlementOutboxStore>::release_order_key(
                [2u8; 32],
                submission_id
            )
        );
    }
}
