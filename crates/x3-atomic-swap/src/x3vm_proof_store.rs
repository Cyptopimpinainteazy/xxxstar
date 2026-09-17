//! Durable proof-ledger persistence for finalized native X3VM lifecycle evidence.

use crate::adapter::{ClaimProof, LockProof, RefundProof};
use crate::error::SwapError;
use crate::intent::ChainKind;
use crate::ledger::{ProofEntry, ProofFinalStatus, ProofKind, ProofLedger};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct PersistentX3ProofLedger {
    path: PathBuf,
    ledger: Mutex<ProofLedger>,
}

impl PersistentX3ProofLedger {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SwapError> {
        let path = path.into();
        let ledger = if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|e| SwapError::Internal(format!("read X3 proof ledger {}: {e}", path.display())))?;
            if bytes.is_empty() {
                ProofLedger::new()
            } else {
                serde_json::from_slice(&bytes).map_err(|e| {
                    SwapError::Internal(format!("decode X3 proof ledger {}: {e}", path.display()))
                })?
            }
        } else {
            ProofLedger::new()
        };
        Ok(Self {
            path,
            ledger: Mutex::new(ledger),
        })
    }

    pub fn snapshot(&self) -> Result<ProofLedger, SwapError> {
        self.ledger
            .lock()
            .map_err(|_| SwapError::Internal("X3 proof ledger mutex poisoned".into()))
            .map(|ledger| ledger.clone())
    }

    pub fn record_lock(&self, intent_id: u64, proof: &LockProof) -> Result<(), SwapError> {
        self.record_finalized(
            intent_id,
            ProofKind::SourceLock,
            &proof.tx_id,
            proof.block_number,
            &proof.raw_proof,
            None,
        )
    }

    pub fn record_claim(&self, intent_id: u64, proof: &ClaimProof) -> Result<(), SwapError> {
        self.record_finalized(
            intent_id,
            ProofKind::Claim,
            &proof.tx_id,
            proof.block_number,
            &proof.raw_proof,
            Some(ProofFinalStatus::Completed),
        )
    }

    pub fn record_refund(&self, intent_id: u64, proof: &RefundProof) -> Result<(), SwapError> {
        self.record_finalized(
            intent_id,
            ProofKind::Refund,
            &proof.tx_id,
            proof.block_number,
            &proof.raw_proof,
            Some(ProofFinalStatus::Refunded),
        )
    }

    fn record_finalized(
        &self,
        intent_id: u64,
        kind: ProofKind,
        tx_id: &str,
        block_number: u64,
        raw_proof: &[u8],
        final_status: Option<ProofFinalStatus>,
    ) -> Result<(), SwapError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SwapError::Internal(format!("system clock before unix epoch: {e}")))?
            .as_secs();
        let mut ledger = self
            .ledger
            .lock()
            .map_err(|_| SwapError::Internal("X3 proof ledger mutex poisoned".into()))?;

        if ledger.intent_id.is_none() {
            ledger.intent_id = Some(intent_id);
        }
        let record_id = match ledger.get_latest_for_intent(intent_id) {
            Some(record) => record.record_id,
            None => ledger
                .create_record(intent_id, "x3-native".into(), timestamp)
                .record_id,
        };

        let proof_id = ledger
            .records
            .iter()
            .map(|record| record.entries.len() as u64)
            .sum::<u64>();
        let entry = ProofEntry::new(
            proof_id,
            intent_id,
            kind,
            ChainKind::X3,
            timestamp,
            0,
        )
        .with_tx_hash(tx_id.to_string())
        .with_block(block_number)
        .mark_verified();

        let record = ledger
            .get_record_mut(record_id)
            .ok_or_else(|| SwapError::Internal("X3 proof ledger record disappeared".into()))?;
        match kind {
            ProofKind::SourceLock => record.record_source_lock(tx_id.to_string(), block_number, timestamp),
            ProofKind::Claim => record.record_claim(tx_id.to_string(), block_number, timestamp),
            ProofKind::Refund => record.record_refund(tx_id.to_string(), block_number, timestamp),
            _ => {}
        }
        let mut entry = entry;
        entry.data = Some(raw_proof.to_vec());
        record.entries.push(entry);

        let finality_id = proof_id.saturating_add(1);
        let mut finality = ProofEntry::new(
            finality_id,
            intent_id,
            ProofKind::FinalityVerified,
            ChainKind::X3,
            timestamp,
            0,
        )
        .with_tx_hash(tx_id.to_string())
        .with_block(block_number)
        .mark_verified();
        finality.data = Some(raw_proof.to_vec());
        record.entries.push(finality);
        record.record_finality_verified(true, timestamp);

        if let Some(status) = final_status {
            record.final_status = Some(status);
            ledger.final_status = Some(status);
        }

        Self::persist_atomic(&self.path, &ledger)
    }

    fn persist_atomic(path: &Path, ledger: &ProofLedger) -> Result<(), SwapError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SwapError::Internal(format!("create X3 proof ledger dir {}: {e}", parent.display()))
            })?;
        }
        let tmp = path.with_extension("tmp");
        let bytes = serde_json::to_vec_pretty(ledger)
            .map_err(|e| SwapError::Internal(format!("encode X3 proof ledger: {e}")))?;
        {
            let mut file = File::create(&tmp).map_err(|e| {
                SwapError::Internal(format!("create X3 proof ledger temp {}: {e}", tmp.display()))
            })?;
            file.write_all(&bytes).map_err(|e| {
                SwapError::Internal(format!("write X3 proof ledger temp {}: {e}", tmp.display()))
            })?;
            file.sync_all().map_err(|e| {
                SwapError::Internal(format!("fsync X3 proof ledger temp {}: {e}", tmp.display()))
            })?;
        }
        fs::rename(&tmp, path).map_err(|e| {
            SwapError::Internal(format!("replace X3 proof ledger {}: {e}", path.display()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::VmType;

    #[test]
    fn ledger_survives_reopen_with_finalized_lock() {
        let path = std::env::temp_dir().join(format!(
            "x3-proof-ledger-{}-{}.json",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        let store = PersistentX3ProofLedger::open(&path).unwrap();
        let proof = LockProof {
            tx_id: "0xlock".into(),
            chain_id: "x3-local".into(),
            vm_type: VmType::X3Vm,
            block_number: 7,
            block_hash: "0xblock".into(),
            confirmations: 1,
            lock_address: "x3-native-escrow".into(),
            locked_amount: 10,
            hashlock: [1u8; 32],
            receiver: vec![2u8; 32],
            refund_address: vec![3u8; 32],
            timeout: 100,
            raw_proof: b"finalized-proof".to_vec(),
        };
        store.record_lock(42, &proof).unwrap();
        drop(store);

        let reopened = PersistentX3ProofLedger::open(&path).unwrap();
        let ledger = reopened.snapshot().unwrap();
        assert!(ledger.has_verified_kind_for_intent(42, ProofKind::SourceLock));
        assert!(ledger.has_verified_kind_for_intent(42, ProofKind::FinalityVerified));
        let _ = fs::remove_file(path);
    }
}
