//! Lightweight integration helpers for the parallel proposer.

use crate::{DeclaredAccess, ParallelProposer, ProposalConfig, ProposalResult, TransactionMeta};
use anyhow::Result;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct IntegrationConfig {
    pub proposer_config: ProposalConfig,
}

#[derive(Debug, Clone, Default)]
pub struct IntegrationMetrics {
    pub total_submissions: u64,
    pub total_proposals: u64,
    pub failed_proposals: u64,
}

pub struct IntegrationContext {
    proposer: Arc<ParallelProposer>,
    metrics: tokio::sync::Mutex<IntegrationMetrics>,
}

impl IntegrationContext {
    pub fn new(config: IntegrationConfig) -> Result<Self> {
        Ok(Self {
            proposer: Arc::new(ParallelProposer::new(config.proposer_config)),
            metrics: tokio::sync::Mutex::new(IntegrationMetrics::default()),
        })
    }

    pub fn proposer(&self) -> Arc<ParallelProposer> {
        self.proposer.clone()
    }

    pub async fn submit_transaction(
        &self,
        tx: TransactionMeta,
        declared_access: Option<DeclaredAccess>,
    ) -> Result<()> {
        self.proposer
            .submit_transaction_with_access(tx, declared_access)
            .await?;

        let mut metrics = self.metrics.lock().await;
        metrics.total_submissions = metrics.total_submissions.saturating_add(1);
        Ok(())
    }

    pub async fn build_next_proposal(&self) -> Result<ProposalResult> {
        let result = self.proposer.create_proposal().await;

        let mut metrics = self.metrics.lock().await;
        match &result {
            Ok(_) => {
                metrics.total_proposals = metrics.total_proposals.saturating_add(1);
            }
            Err(_) => {
                metrics.failed_proposals = metrics.failed_proposals.saturating_add(1);
            }
        }

        result
    }

    pub async fn get_metrics(&self) -> IntegrationMetrics {
        self.metrics.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConflictClass, VmLane};
    use x3_common::signing::{Ed25519Signer, Signer};

    fn sample_tx(id: &str) -> TransactionMeta {
        // A real (public key, message, signature) triple: `create_proposal` verifies it.
        let signer = Ed25519Signer::from_seed(&[0x7Au8; 32]);
        let message = blake3::hash(id.as_bytes());
        let signature = signer.sign(message.as_bytes());
        let public_key = signer.public_key();
        TransactionMeta {
            tx_hash: hex::encode(message.as_bytes()),
            sender: hex::encode(public_key.as_bytes()),
            receiver: "0x2".to_string(),
            value: 1,
            gas_limit: 21_000,
            gas_price: 1,
            nonce: 0,
            signature: hex::encode(signature.as_bytes()),
            contract_address: Some("0xabc".to_string()),
            timestamp: 1,
        }
    }

    #[tokio::test]
    async fn integration_tracks_submissions_and_proposals() {
        let ctx = IntegrationContext::new(IntegrationConfig::default()).unwrap();
        ctx.submit_transaction(
            sample_tx("tx-1"),
            Some(
                DeclaredAccess::new(VmLane::System, ConflictClass::Global)
                    .with_reads(["r:1"])
                    .with_writes(["w:1"]),
            ),
        )
        .await
        .unwrap();

        let _ = ctx.build_next_proposal().await.unwrap();

        let metrics = ctx.get_metrics().await;
        assert_eq!(metrics.total_submissions, 1);
        assert_eq!(metrics.total_proposals, 1);
        assert_eq!(metrics.failed_proposals, 0);
    }
}
