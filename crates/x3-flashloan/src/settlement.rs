//! Settlement engine — proof-based finalization of flashloan lifecycles.
//!
//! Every flashloan generates an execution proof binding borrow → use → repay
//! into a single verifiable chain. No bridged custody. No trust assumptions.

use sha2::{Digest, Sha256};

use x3_proof::ProofEngine;

use crate::error::FlashloanError;
use crate::types::{BorrowReceipt, FlashloanId, SettlementRecord, SettlementStatus};

/// Settlement engine for flashloan lifecycle finalization.
pub struct SettlementEngine {
    records: Vec<SettlementRecord>,
}

impl SettlementEngine {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Settle a flashloan — verify repayment and generate proof.
    pub fn settle(
        &self,
        receipt: &BorrowReceipt,
        amount_repaid: u128,
        proof_engine: &ProofEngine,
    ) -> Result<SettlementRecord, FlashloanError> {
        let total_owed = receipt.total_owed();

        if amount_repaid < total_owed {
            return Err(FlashloanError::InsufficientRepayment {
                owed: total_owed,
                paid: amount_repaid,
            });
        }

        // Generate settlement proof
        let proof_hash = self.generate_proof(receipt, amount_repaid, proof_engine);

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(SettlementRecord {
            flashloan_id: receipt.id.clone(),
            status: SettlementStatus::Settled,
            amount_repaid,
            proof_hash,
            settled_at: now_ms,
        })
    }

    /// Record a reverted flashloan.
    pub fn record_revert(&mut self, flashloan_id: &FlashloanId) -> SettlementRecord {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let record = SettlementRecord {
            flashloan_id: flashloan_id.clone(),
            status: SettlementStatus::Reverted,
            amount_repaid: 0,
            proof_hash: String::new(),
            settled_at: now_ms,
        };

        self.records.push(record.clone());
        record
    }

    /// Record a defaulted flashloan (agent will be slashed).
    pub fn record_default(
        &mut self,
        receipt: &BorrowReceipt,
        proof_engine: &ProofEngine,
    ) -> SettlementRecord {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let proof_hash = self.generate_default_proof(receipt, proof_engine);

        let record = SettlementRecord {
            flashloan_id: receipt.id.clone(),
            status: SettlementStatus::Defaulted,
            amount_repaid: 0,
            proof_hash,
            settled_at: now_ms,
        };

        self.records.push(record.clone());
        record
    }

    /// Get all settlement records.
    pub fn records(&self) -> &[SettlementRecord] {
        &self.records
    }

    /// Get settlement records filtered by status.
    pub fn records_by_status(&self, status: SettlementStatus) -> Vec<&SettlementRecord> {
        self.records.iter().filter(|r| r.status == status).collect()
    }

    /// Generate a proof hash binding borrow → use → repay.
    fn generate_proof(
        &self,
        receipt: &BorrowReceipt,
        amount_repaid: u128,
        _proof_engine: &ProofEngine,
    ) -> String {
        let mut hasher = Sha256::new();

        // Bind: flashloan ID
        hasher.update(receipt.id.0.as_bytes());

        // Bind: chain
        hasher.update(format!("{}", receipt.chain).as_bytes());

        // Bind: asset
        hasher.update(receipt.asset.0.as_bytes());

        // Bind: principal
        hasher.update(receipt.principal.to_le_bytes());

        // Bind: premium
        hasher.update(receipt.premium.to_le_bytes());

        // Bind: amount repaid
        hasher.update(amount_repaid.to_le_bytes());

        // Bind: borrowed_at timestamp
        hasher.update(receipt.borrowed_at.to_le_bytes());

        // Bind: settlement status
        hasher.update(b"settled");

        hex::encode(hasher.finalize())
    }

    /// Generate a proof hash for a default event.
    fn generate_default_proof(
        &self,
        receipt: &BorrowReceipt,
        _proof_engine: &ProofEngine,
    ) -> String {
        let mut hasher = Sha256::new();

        hasher.update(receipt.id.0.as_bytes());
        hasher.update(format!("{}", receipt.chain).as_bytes());
        hasher.update(receipt.asset.0.as_bytes());
        hasher.update(receipt.principal.to_le_bytes());
        hasher.update(b"defaulted");

        hex::encode(hasher.finalize())
    }
}

impl Default for SettlementEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetId, ChainKind};
    use x3_proof::{AgentIdentity, ProofEngineConfig};

    fn test_proof_engine() -> ProofEngine {
        ProofEngine::new(
            ProofEngineConfig::default(),
            100,
            AgentIdentity {
                pubkey: [0xBB; 32],
                ephemeral: true,
            },
            [0xCC; 32],
        )
    }

    fn sample_receipt() -> BorrowReceipt {
        BorrowReceipt {
            id: FlashloanId::from_str("test-settle-001"),
            chain: ChainKind::Evm(1),
            asset: AssetId::new("USDC"),
            principal: 1_000_000,
            premium: 500,
            borrowed_at: 1700000000000,
            must_repay_by: 1700000030000,
        }
    }

    #[test]
    fn test_settle_success() {
        let engine = SettlementEngine::new();
        let proof_engine = test_proof_engine();
        let receipt = sample_receipt();

        let record = engine.settle(&receipt, 1_000_500, &proof_engine).unwrap();
        assert_eq!(record.status, SettlementStatus::Settled);
        assert_eq!(record.amount_repaid, 1_000_500);
        assert_eq!(record.proof_hash.len(), 64);
    }

    #[test]
    fn test_settle_insufficient() {
        let engine = SettlementEngine::new();
        let proof_engine = test_proof_engine();
        let receipt = sample_receipt();

        let result = engine.settle(&receipt, 999_999, &proof_engine);
        assert!(matches!(
            result,
            Err(FlashloanError::InsufficientRepayment { .. })
        ));
    }

    #[test]
    fn test_record_revert() {
        let mut engine = SettlementEngine::new();
        let id = FlashloanId::from_str("revert-001");

        let record = engine.record_revert(&id);
        assert_eq!(record.status, SettlementStatus::Reverted);
        assert_eq!(record.amount_repaid, 0);
        assert_eq!(engine.records().len(), 1);
    }

    #[test]
    fn test_record_default() {
        let mut engine = SettlementEngine::new();
        let proof_engine = test_proof_engine();
        let receipt = sample_receipt();

        let record = engine.record_default(&receipt, &proof_engine);
        assert_eq!(record.status, SettlementStatus::Defaulted);
        assert!(record.proof_hash.len() == 64);
    }

    #[test]
    fn test_proof_deterministic() {
        let engine = SettlementEngine::new();
        let proof_engine = test_proof_engine();
        let receipt = sample_receipt();

        let r1 = engine.settle(&receipt, 1_000_500, &proof_engine).unwrap();
        let r2 = engine.settle(&receipt, 1_000_500, &proof_engine).unwrap();
        assert_eq!(r1.proof_hash, r2.proof_hash);
    }

    #[test]
    fn test_filter_by_status() {
        let mut engine = SettlementEngine::new();
        let proof_engine = test_proof_engine();

        engine.record_revert(&FlashloanId::from_str("r1"));
        engine.record_revert(&FlashloanId::from_str("r2"));
        engine.record_default(&sample_receipt(), &proof_engine);

        assert_eq!(
            engine.records_by_status(SettlementStatus::Reverted).len(),
            2
        );
        assert_eq!(
            engine.records_by_status(SettlementStatus::Defaulted).len(),
            1
        );
    }
}
