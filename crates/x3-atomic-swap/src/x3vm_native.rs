//! Safety wrapper for the native X3-node transport.
//!
//! `X3NodeTransport` performs the real RPC submission and captures finalized
//! inclusion evidence. This wrapper additionally binds `finality_status(tx_id)`
//! to transactions this process actually observed in GRANDPA-finalized blocks,
//! so an unrelated finalized head can never be reported as proof for a queried
//! transaction.

use crate::adapter::{
    AdapterReadinessScore, ChainHealth, ChainId, ClaimProof, FeeEstimate, FinalityProof, LockProof,
    RefundProof, TxId,
};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, IntentId};
use crate::x3vm_live::X3VmLiveTransport;
use crate::x3vm_node::{X3ExtrinsicSigner, X3NodeTransport, X3NodeTransportConfig};
use crate::x3vm_proof_store::PersistentX3ProofLedger;
use alloc::collections::BTreeMap;
use core::fmt::Debug;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug)]
pub struct NativeX3NodeTransport<S: X3ExtrinsicSigner> {
    inner: X3NodeTransport<S>,
    finalized: Mutex<BTreeMap<TxId, FinalityProof>>,
    proof_ledger: Option<PersistentX3ProofLedger>,
}

impl<S: X3ExtrinsicSigner> NativeX3NodeTransport<S> {
    pub fn new(config: X3NodeTransportConfig, signer: S) -> Self {
        Self {
            inner: X3NodeTransport::new(config, signer),
            finalized: Mutex::new(BTreeMap::new()),
            proof_ledger: None,
        }
    }

    pub fn new_with_proof_ledger(
        config: X3NodeTransportConfig,
        signer: S,
        path: impl Into<PathBuf>,
    ) -> Result<Self, SwapError> {
        Ok(Self {
            inner: X3NodeTransport::new(config, signer),
            finalized: Mutex::new(BTreeMap::new()),
            proof_ledger: Some(PersistentX3ProofLedger::open(path)?),
        })
    }

    pub fn proof_ledger_snapshot(&self) -> Result<Option<crate::ledger::ProofLedger>, SwapError> {
        self.proof_ledger
            .as_ref()
            .map(PersistentX3ProofLedger::snapshot)
            .transpose()
    }

    fn remember_finality(
        &self,
        chain_id: &ChainId,
        tx_id: &TxId,
        block_number: u64,
        block_hash: &str,
    ) -> Result<(), SwapError> {
        let proof = FinalityProof {
            chain_id: chain_id.clone(),
            vm_type: crate::adapter::VmType::X3Vm,
            tx_id: tx_id.clone(),
            block_number,
            block_hash: block_hash.into(),
            confirmations: 1,
            finalized: true,
            finality_source: "GRANDPA finalized inclusion".into(),
            safe_to_reveal_secret: true,
        };
        self.finalized
            .lock()
            .map_err(|_| SwapError::RpcError("X3 finality cache mutex poisoned".into()))?
            .insert(tx_id.clone(), proof);
        Ok(())
    }
}

impl<S: X3ExtrinsicSigner> X3VmLiveTransport for NativeX3NodeTransport<S> {
    fn lock(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent: &AtomicIntent,
    ) -> Result<LockProof, SwapError> {
        let proof = self.inner.lock(chain_id, escrow_address, intent)?;
        self.remember_finality(chain_id, &proof.tx_id, proof.block_number, &proof.block_hash)?;
        if let Some(ledger) = &self.proof_ledger {
            ledger.record_lock(intent.intent_id, &proof)?;
        }
        Ok(proof)
    }

    fn claim(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent_id: IntentId,
        preimage: [u8; 32],
    ) -> Result<ClaimProof, SwapError> {
        let proof = self
            .inner
            .claim(chain_id, escrow_address, intent_id, preimage)?;
        self.remember_finality(chain_id, &proof.tx_id, proof.block_number, &proof.block_hash)?;
        if let Some(ledger) = &self.proof_ledger {
            ledger.record_claim(intent_id, &proof)?;
        }
        Ok(proof)
    }

    fn refund(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent_id: IntentId,
    ) -> Result<RefundProof, SwapError> {
        let proof = self.inner.refund(chain_id, escrow_address, intent_id)?;
        self.remember_finality(chain_id, &proof.tx_id, proof.block_number, &proof.block_hash)?;
        if let Some(ledger) = &self.proof_ledger {
            ledger.record_refund(intent_id, &proof)?;
        }
        Ok(proof)
    }

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        self.inner.verify_lock(proof)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        self.inner.verify_claim(proof)
    }

    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError> {
        self.inner.verify_refund(proof)
    }

    fn estimate_fee(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent: &AtomicIntent,
    ) -> Result<FeeEstimate, SwapError> {
        self.inner.estimate_fee(chain_id, escrow_address, intent)
    }

    fn finality_status(
        &self,
        chain_id: &ChainId,
        tx_id: &TxId,
    ) -> Result<FinalityProof, SwapError> {
        let proof = self
            .finalized
            .lock()
            .map_err(|_| SwapError::RpcError("X3 finality cache mutex poisoned".into()))?
            .get(tx_id)
            .cloned()
            .ok_or_else(|| {
                SwapError::RpcError(alloc::format!(
                    "no finalized inclusion evidence recorded for X3 transaction {}",
                    tx_id
                ))
            })?;
        if &proof.chain_id != chain_id {
            return Err(SwapError::RpcError(alloc::format!(
                "X3 finality chain mismatch: proof {}, requested {}",
                proof.chain_id,
                chain_id
            )));
        }
        Ok(proof)
    }

    fn chain_health(&self, chain_id: &ChainId) -> Result<ChainHealth, SwapError> {
        self.inner.chain_health(chain_id)
    }

    fn readiness_score(&self) -> AdapterReadinessScore {
        let mut score = self.inner.readiness_score();
        // Real node/RPC and finalized-inclusion paths are implemented. Promotion
        // to production is still controlled by LiveX3VmAdapter's fail-closed
        // `is_simulated()` default until live-node tests and proof-ledger wiring pass.
        score.finality_proof = true;
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[derive(Debug)]
    struct NeverSigner;

    impl X3ExtrinsicSigner for NeverSigner {
        fn sign_lock_escrow(
            &self,
            _chain_id: &ChainId,
            _escrow_address: &[u8],
            _intent: &AtomicIntent,
        ) -> Result<String, SwapError> {
            panic!("signer should not be called by unknown-finality test")
        }

        fn sign_claim_settlement(
            &self,
            _chain_id: &ChainId,
            _intent_id: IntentId,
            _preimage: [u8; 32],
        ) -> Result<String, SwapError> {
            panic!("signer should not be called by unknown-finality test")
        }

        fn sign_refund_settlement(
            &self,
            _chain_id: &ChainId,
            _intent_id: IntentId,
        ) -> Result<String, SwapError> {
            panic!("signer should not be called by unknown-finality test")
        }
    }

    #[test]
    fn unknown_transaction_never_inherits_unrelated_finalized_head() {
        let transport = NativeX3NodeTransport::new(
            X3NodeTransportConfig::local("http://127.0.0.1:9944".into()),
            NeverSigner,
        );
        let err = transport
            .finality_status(&"x3-local".into(), &"0xunknown".into())
            .expect_err("unknown tx must fail closed");
        assert!(err.to_string().contains("no finalized inclusion evidence"));
    }

    #[test]
    fn remembered_transaction_returns_exact_finalized_block() {
        let transport = NativeX3NodeTransport::new(
            X3NodeTransportConfig::local("http://127.0.0.1:9944".into()),
            NeverSigner,
        );
        transport
            .remember_finality(&"x3-local".into(), &"0xtx".into(), 42, "0xblock42")
            .unwrap();
        let proof = transport
            .finality_status(&"x3-local".into(), &"0xtx".into())
            .unwrap();
        assert_eq!(proof.tx_id, "0xtx");
        assert_eq!(proof.block_number, 42);
        assert_eq!(proof.block_hash, "0xblock42");
        assert!(proof.finalized);
        assert!(proof.safe_to_reveal_secret);
    }
}
