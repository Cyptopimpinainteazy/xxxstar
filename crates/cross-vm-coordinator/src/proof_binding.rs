//! Binding between canonical cross-domain proof bundles and coordinator attempts.

use crate::{CoordinatorError, CoordinatorOperation, OperationAttempt};

#[cfg(feature = "canonical-proofs")]
use x3_atomic_swap::{
    AtomicIntent, CrossDomainOperation, CrossDomainProofBundle,
};

#[cfg(feature = "canonical-proofs")]
fn expected_operation(operation: CoordinatorOperation) -> CrossDomainOperation {
    match operation {
        CoordinatorOperation::FastHtlcLock | CoordinatorOperation::SlowHtlcLock => {
            CrossDomainOperation::Lock
        }
        CoordinatorOperation::FastClaim | CoordinatorOperation::SlowClaim => {
            CrossDomainOperation::Claim
        }
        CoordinatorOperation::RefundBoth => CrossDomainOperation::Refund,
    }
}

/// Verify that one canonical proof bundle is the exact finalized evidence for
/// the fenced coordinator attempt.
#[cfg(feature = "canonical-proofs")]
pub fn verify_bundle_for_attempt(
    prior: &OperationAttempt,
    intent: &AtomicIntent,
    runtime_intent_id: [u8; 32],
    bundle: &CrossDomainProofBundle,
) -> Result<[u8; 32], CoordinatorError> {
    bundle.verify(intent).map_err(|e| {
        CoordinatorError::Internal(format!(
            "canonical cross-domain proof verification failed: {e}"
        ))
    })?;
    bundle.verify_runtime_binding(runtime_intent_id).map_err(|e| {
        CoordinatorError::Internal(format!(
            "canonical runtime-intent proof binding failed: {e}"
        ))
    })?;

    let tx_id = prior.tx_id.as_deref().ok_or_else(|| {
        CoordinatorError::Internal(
            "attempt must have a broadcast tx/signature before proof finalization".into(),
        )
    })?;

    if tx_id != bundle.tx_id {
        return Err(CoordinatorError::Internal(format!(
            "proof tx mismatch: attempt '{tx_id}' vs bundle '{}'",
            bundle.tx_id
        )));
    }

    if prior.domain != bundle.chain_id {
        return Err(CoordinatorError::Internal(format!(
            "proof domain mismatch: attempt '{}' vs bundle '{}'",
            prior.domain, bundle.chain_id
        )));
    }

    let expected = expected_operation(prior.operation);
    if bundle.operation != expected {
        return Err(CoordinatorError::Internal(format!(
            "proof operation mismatch: attempt {:?} requires {:?}, bundle is {:?}",
            prior.operation, expected, bundle.operation
        )));
    }

    Ok(bundle.proof_hash)
}

#[cfg(all(test, feature = "canonical-proofs"))]
mod tests {
    use super::*;
    use crate::OperationAttemptStatus;
    use x3_atomic_swap::{
        adapter::{FinalityProof, VmType},
        intent::{AtomicIntentBuilder, ChainKind, RefundPath},
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
            .hashlock([7u8; 32])
            .source_timeout(2000)
            .destination_timeout(1000)
            .refund_path(RefundPath {
                chain: ChainKind::X3,
                address: "refund".into(),
                asset: None,
            })
            .build(77)
            .unwrap()
    }

    fn attempt() -> OperationAttempt {
        OperationAttempt {
            session_id: "swap-a".into(),
            operation: CoordinatorOperation::FastClaim,
            attempt_id: "attempt-1#broadcast".into(),
            owner_id: "worker-a".into(),
            fence: 9,
            domain: "ethereum-mainnet".into(),
            status: OperationAttemptStatus::Broadcast,
            tx_id: Some("0xclaim".into()),
            proof_hash: None,
            started_at: 100,
            updated_at: 101,
            error: None,
        }
    }

    fn bundle(intent: &AtomicIntent) -> CrossDomainProofBundle {
        let block_hash = "0xblock".to_string();
        CrossDomainProofBundle::new(
            intent,
            [0xabu8; 32],
            "ethereum-mainnet".into(),
            VmType::Evm,
            CrossDomainOperation::Claim,
            "0xclaim".into(),
            55,
            block_hash.clone(),
            vec![1, 2, 3],
            FinalityProof {
                chain_id: "ethereum-mainnet".into(),
                vm_type: VmType::Evm,
                tx_id: "0xclaim".into(),
                block_number: 55,
                block_hash,
                confirmations: 12,
                finalized: true,
                finality_source: "test".into(),
                safe_to_reveal_secret: true,
            },
        )
        .unwrap()
    }

    #[test]
    fn verified_bundle_returns_its_canonical_hash() {
        let intent = intent();
        let proof = bundle(&intent);
        assert_eq!(
            verify_bundle_for_attempt(&attempt(), &intent, [0xabu8; 32], &proof).unwrap(),
            proof.proof_hash
        );
    }

    #[test]
    fn tx_domain_operation_and_integrity_mismatches_fail_closed() {
        let intent = intent();

        let mut wrong_tx = bundle(&intent);
        wrong_tx.tx_id = "0xother".into();
        assert!(verify_bundle_for_attempt(&attempt(), &intent, [0xabu8; 32], &wrong_tx).is_err());

        let mut wrong_domain = attempt();
        wrong_domain.domain = "base-mainnet".into();
        assert!(verify_bundle_for_attempt(&wrong_domain, &intent, [0xabu8; 32], &bundle(&intent)).is_err());

        let mut wrong_operation = attempt();
        wrong_operation.operation = CoordinatorOperation::RefundBoth;
        assert!(verify_bundle_for_attempt(&wrong_operation, &intent, [0xabu8; 32], &bundle(&intent)).is_err());

        let mut tampered = bundle(&intent);
        tampered.block_number += 1;
        assert!(verify_bundle_for_attempt(&attempt(), &intent, [0xabu8; 32], &tampered).is_err());
    }
}
