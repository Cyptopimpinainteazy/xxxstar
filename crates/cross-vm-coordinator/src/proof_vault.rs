//! Durable canonical proof-bundle vault and proof-set assembly.
//!
//! Canonical operation results store only proof hashes. The vault preserves the
//! full verified CrossDomainProofBundle addressed by that hash so recovery and
//! settlement can reconstruct the exact evidence later.

use crate::{CanonicalOperationResult, CoordinatorError};

#[cfg(feature = "canonical-proofs")]
use x3_atomic_swap::{AtomicIntent, CrossDomainProofBundle, CrossDomainProofSet};

#[cfg(feature = "canonical-proofs")]
pub trait ProofBundleStore: Send + Sync + 'static {
    fn put_bundle(&self, bundle: &CrossDomainProofBundle) -> Result<(), CoordinatorError>;
    fn get_bundle(
        &self,
        proof_hash: [u8; 32],
    ) -> Result<Option<CrossDomainProofBundle>, CoordinatorError>;
}

#[cfg(feature = "canonical-proofs")]
#[derive(Default)]
pub struct InMemoryProofBundleStore {
    bundles: std::sync::RwLock<
        std::collections::HashMap<[u8; 32], CrossDomainProofBundle>,
    >,
}

#[cfg(feature = "canonical-proofs")]
impl ProofBundleStore for InMemoryProofBundleStore {
    fn put_bundle(&self, bundle: &CrossDomainProofBundle) -> Result<(), CoordinatorError> {
        let mut guard = self
            .bundles
            .write()
            .map_err(|_| CoordinatorError::Internal("proof vault poisoned".into()))?;

        if let Some(existing) = guard.get(&bundle.proof_hash) {
            let existing_bytes = serde_json::to_vec(existing).map_err(|e| {
                CoordinatorError::Internal(format!("existing proof serialization failed: {e}"))
            })?;
            let incoming_bytes = serde_json::to_vec(bundle).map_err(|e| {
                CoordinatorError::Internal(format!("incoming proof serialization failed: {e}"))
            })?;
            if existing_bytes == incoming_bytes {
                return Ok(());
            }
            return Err(CoordinatorError::Internal(
                "proof hash collision/reuse with different bundle contents".into(),
            ));
        }

        guard.insert(bundle.proof_hash, bundle.clone());
        Ok(())
    }

    fn get_bundle(
        &self,
        proof_hash: [u8; 32],
    ) -> Result<Option<CrossDomainProofBundle>, CoordinatorError> {
        let guard = self
            .bundles
            .read()
            .map_err(|_| CoordinatorError::Internal("proof vault poisoned".into()))?;
        Ok(guard.get(&proof_hash).cloned())
    }
}

/// Reconstruct a canonical CrossDomainProofSet exclusively from canonical
/// coordinator results and their content-addressed bundles.
///
/// Every bundle is re-verified against the AtomicIntent. The stored tx id and
/// proof hash must exactly match the canonical operation result.
#[cfg(feature = "canonical-proofs")]
pub fn assemble_proof_set<S: ProofBundleStore>(
    intent: &AtomicIntent,
    runtime_intent_id: [u8; 32],
    canonical_results: &[CanonicalOperationResult],
    store: &S,
) -> Result<CrossDomainProofSet, CoordinatorError> {
    if canonical_results.is_empty() {
        return Err(CoordinatorError::Internal(
            "cannot assemble proof set without canonical operation results".into(),
        ));
    }

    let has_claim = canonical_results.iter().any(|result| {
        matches!(
            result.operation,
            crate::CoordinatorOperation::FastClaim | crate::CoordinatorOperation::SlowClaim
        )
    });
    let has_refund = canonical_results
        .iter()
        .any(|result| result.operation == crate::CoordinatorOperation::RefundBoth);
    if has_claim && has_refund {
        return Err(CoordinatorError::Internal(
            "canonical evidence contains both claim and refund terminal paths".into(),
        ));
    }

    let mut set = CrossDomainProofSet::new(intent, runtime_intent_id);

    for result in canonical_results {
        let bundle = store
            .get_bundle(result.proof_hash)?
            .ok_or_else(|| {
                CoordinatorError::Internal(format!(
                    "canonical proof bundle {} missing from vault",
                    hex::encode(result.proof_hash)
                ))
            })?;

        if bundle.proof_hash != result.proof_hash {
            return Err(CoordinatorError::Internal(
                "proof-vault hash does not match canonical result".into(),
            ));
        }
        if bundle.tx_id != result.tx_id {
            return Err(CoordinatorError::Internal(format!(
                "canonical result tx '{}' does not match proof bundle tx '{}'",
                result.tx_id, bundle.tx_id
            )));
        }

        bundle.verify(intent).map_err(|e| {
            CoordinatorError::Internal(format!(
                "vault proof failed canonical verification: {e}"
            ))
        })?;
        bundle.verify_runtime_binding(runtime_intent_id).map_err(|e| {
            CoordinatorError::Internal(format!(
                "vault proof failed runtime-intent verification: {e}"
            ))
        })?;

        set.push_verified(intent, bundle).map_err(|e| {
            CoordinatorError::Internal(format!(
                "canonical proof-set assembly failed: {e}"
            ))
        })?;
    }

    Ok(set)
}

#[cfg(all(test, feature = "canonical-proofs"))]
mod tests {
    use super::*;
    use crate::{CanonicalOperationResult, CoordinatorOperation};
    use x3_atomic_swap::{
        adapter::{FinalityProof, VmType},
        intent::{AtomicIntentBuilder, ChainKind, RefundPath},
        CrossDomainOperation,
    };

    fn intent() -> AtomicIntent {
        AtomicIntentBuilder::new()
            .source_chain(ChainKind::X3)
            .destination_chain(ChainKind::Ethereum)
            .source_asset("X3")
            .destination_asset("ETH")
            .amount_in(1000)
            .min_amount_out(900)
            .receiver("receiver")
            .hashlock([9u8; 32])
            .source_timeout(2000)
            .destination_timeout(1000)
            .refund_path(RefundPath {
                chain: ChainKind::X3,
                address: "refund".into(),
                asset: None,
            })
            .build(88)
            .unwrap()
    }

    fn bundle(
        intent: &AtomicIntent,
        chain: &str,
        vm: VmType,
        op: CrossDomainOperation,
        tx: &str,
        block: u64,
    ) -> CrossDomainProofBundle {
        let block_hash = format!("block-{block}");
        CrossDomainProofBundle::new(
            intent,
            [0xabu8; 32],
            chain.into(),
            vm,
            op,
            tx.into(),
            block,
            block_hash.clone(),
            vec![1, 2, 3, block as u8],
            FinalityProof {
                chain_id: chain.into(),
                vm_type: vm,
                tx_id: tx.into(),
                block_number: block,
                block_hash,
                confirmations: 12,
                finalized: true,
                finality_source: "test".into(),
                safe_to_reveal_secret: true,
            },
        )
        .unwrap()
    }

    fn result(
        operation: CoordinatorOperation,
        attempt_id: &str,
        bundle: &CrossDomainProofBundle,
    ) -> CanonicalOperationResult {
        CanonicalOperationResult {
            session_id: "swap-vault".into(),
            operation,
            attempt_id: attempt_id.into(),
            tx_id: bundle.tx_id.clone(),
            proof_hash: bundle.proof_hash,
            finalized_at: 100,
        }
    }

    #[test]
    fn reconstructs_verified_set_from_content_addressed_bundles() {
        let intent = intent();
        let store = InMemoryProofBundleStore::default();

        let x3 = bundle(
            &intent,
            "x3-local",
            VmType::X3Vm,
            CrossDomainOperation::Claim,
            "x3-claim",
            10,
        );
        let eth = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Claim,
            "eth-claim",
            11,
        );
        store.put_bundle(&x3).unwrap();
        store.put_bundle(&eth).unwrap();

        let results = vec![
            result(CoordinatorOperation::FastClaim, "a", &x3),
            result(CoordinatorOperation::SlowClaim, "b", &eth),
        ];
        let set = assemble_proof_set(&intent, [0xabu8; 32], &results, &store).unwrap();

        set.verify_claim_set(
            &intent,
            &[
                ("x3-local".into(), VmType::X3Vm),
                ("eth-mainnet".into(), VmType::Evm),
            ],
        )
        .unwrap();
    }

    #[test]
    fn missing_bundle_fails_closed() {
        let intent = intent();
        let store = InMemoryProofBundleStore::default();
        let proof = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Claim,
            "eth-claim",
            11,
        );
        let results = vec![result(CoordinatorOperation::FastClaim, "a", &proof)];
        assert!(assemble_proof_set(&intent, [0xabu8; 32], &results, &store).is_err());
    }

    #[test]
    fn canonical_tx_mismatch_fails_closed() {
        let intent = intent();
        let store = InMemoryProofBundleStore::default();
        let proof = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Claim,
            "eth-claim",
            11,
        );
        store.put_bundle(&proof).unwrap();

        let mut canonical = result(CoordinatorOperation::FastClaim, "a", &proof);
        canonical.tx_id = "different-tx".into();

        assert!(assemble_proof_set(&intent, [0xabu8; 32], &[canonical], &store).is_err());
    }

    #[test]
    fn claim_and_refund_canonical_results_cannot_share_a_proof_set() {
        let intent = intent();
        let store = InMemoryProofBundleStore::default();

        let claim = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Claim,
            "eth-claim",
            11,
        );
        let refund = bundle(
            &intent,
            "x3-local",
            VmType::X3Vm,
            CrossDomainOperation::Refund,
            "x3-refund",
            12,
        );
        store.put_bundle(&claim).unwrap();
        store.put_bundle(&refund).unwrap();

        let results = vec![
            result(CoordinatorOperation::FastClaim, "claim", &claim),
            result(CoordinatorOperation::RefundBoth, "refund", &refund),
        ];

        assert!(
            assemble_proof_set(&intent, [0xabu8; 32], &results, &store).is_err()
        );
    }

    #[test]
    fn runtime_intent_mismatch_fails_closed() {
        let intent = intent();
        let store = InMemoryProofBundleStore::default();
        let proof = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Claim,
            "eth-claim",
            11,
        );
        store.put_bundle(&proof).unwrap();
        let results = vec![result(CoordinatorOperation::FastClaim, "a", &proof)];

        assert!(
            assemble_proof_set(&intent, [0xcdu8; 32], &results, &store).is_err()
        );
    }

}
