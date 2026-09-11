//! Runtime settlement submission envelope.
//!
//! This module prepares the exact typed arguments for pallet
//! `submit_cross_domain_proof_set(intent_id, proof_set)`.
//! It intentionally does NOT construct a signed extrinsic because the runtime
//! pallet index, signed extensions, nonce, era, and signer belong to the node
//! client/runtime metadata layer.

use crate::CoordinatorError;

#[cfg(feature = "canonical-proofs")]
use codec::Encode;
#[cfg(feature = "canonical-proofs")]
use x3_atomic_swap::{
    AtomicIntent, ChainId, CrossDomainProofSet, VmType,
};

/// Pallet call index declared by x3-settlement-engine for
/// `submit_cross_domain_proof_set`.
pub const SUBMIT_CROSS_DOMAIN_PROOF_SET_CALL_INDEX: u8 = 33;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementProofPurpose {
    Claim,
    Refund,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostProofAction {
    /// The pallet can auto-finalize when local claim legs are already complete;
    /// otherwise the caller observes/continues the normal claim path.
    ObserveClaimFinalization,
    /// Refund proof submission does not itself execute the refund. Once timeout
    /// and proof requirements hold, call `refund_settlement(intent_id)`.
    CallRefundSettlement,
}

#[cfg(feature = "canonical-proofs")]
#[derive(Debug, Clone)]
pub struct SettlementSubmissionEnvelope {
    pub runtime_intent_id: [u8; 32],
    pub purpose: SettlementProofPurpose,
    pub proof_set: CrossDomainProofSet,
}

#[cfg(feature = "canonical-proofs")]
impl SettlementSubmissionEnvelope {
    pub fn for_claim(
        intent: &AtomicIntent,
        runtime_intent_id: [u8; 32],
        proof_set: CrossDomainProofSet,
        required_domains: &[(ChainId, VmType)],
    ) -> Result<Self, CoordinatorError> {
        proof_set
            .verify_runtime_binding(runtime_intent_id)
            .map_err(|e| {
                CoordinatorError::Internal(format!(
                    "claim submission runtime binding failed: {e}"
                ))
            })?;
        proof_set
            .verify_claim_set(intent, required_domains)
            .map_err(|e| {
                CoordinatorError::Internal(format!(
                    "claim submission proof set is incomplete or invalid: {e}"
                ))
            })?;

        Ok(Self {
            runtime_intent_id,
            purpose: SettlementProofPurpose::Claim,
            proof_set,
        })
    }

    pub fn for_refund(
        intent: &AtomicIntent,
        runtime_intent_id: [u8; 32],
        proof_set: CrossDomainProofSet,
        required_domains: &[(ChainId, VmType)],
    ) -> Result<Self, CoordinatorError> {
        proof_set
            .verify_runtime_binding(runtime_intent_id)
            .map_err(|e| {
                CoordinatorError::Internal(format!(
                    "refund submission runtime binding failed: {e}"
                ))
            })?;
        proof_set
            .verify_refund_set(intent, required_domains)
            .map_err(|e| {
                CoordinatorError::Internal(format!(
                    "refund submission proof set is incomplete or invalid: {e}"
                ))
            })?;

        Ok(Self {
            runtime_intent_id,
            purpose: SettlementProofPurpose::Refund,
            proof_set,
        })
    }

    pub const fn call_index(&self) -> u8 {
        SUBMIT_CROSS_DOMAIN_PROOF_SET_CALL_INDEX
    }

    /// SCALE encoding of the two pallet call arguments:
    ///
    /// `(intent_id: H256, proof_set: CrossDomainProofSet)`
    ///
    /// H256 and [u8; 32] have the same fixed 32-byte SCALE representation.
    /// This byte vector intentionally excludes pallet index, call index,
    /// signature, nonce, era, tip, signed extensions and metadata hash.
    pub fn scale_call_args(&self) -> Vec<u8> {
        (self.runtime_intent_id, self.proof_set.clone()).encode()
    }

    pub const fn requires_intent_party_signature(&self) -> bool {
        true
    }

    pub const fn post_proof_action(&self) -> PostProofAction {
        match self.purpose {
            SettlementProofPurpose::Claim => PostProofAction::ObserveClaimFinalization,
            SettlementProofPurpose::Refund => PostProofAction::CallRefundSettlement,
        }
    }

    pub fn proof_hashes(&self) -> Vec<[u8; 32]> {
        self.proof_set
            .bundles
            .iter()
            .map(|bundle| bundle.proof_hash)
            .collect()
    }
}

#[cfg(all(test, feature = "canonical-proofs"))]
mod tests {
    use super::*;
    use x3_atomic_swap::{
        adapter::FinalityProof,
        intent::{AtomicIntentBuilder, ChainKind, RefundPath},
        CrossDomainOperation, CrossDomainProofBundle,
    };

    fn intent() -> AtomicIntent {
        AtomicIntentBuilder::new()
            .source_chain(ChainKind::X3)
            .destination_chain(ChainKind::Ethereum)
            .source_asset("X3")
            .destination_asset("ETH")
            .amount_in(1_000)
            .min_amount_out(900)
            .receiver("receiver")
            .hashlock([7u8; 32])
            .source_timeout(2_000)
            .destination_timeout(1_000)
            .refund_path(RefundPath {
                chain: ChainKind::X3,
                address: "refund".into(),
                asset: None,
            })
            .build(99)
            .unwrap()
    }

    fn bundle(
        intent: &AtomicIntent,
        runtime_intent_id: [u8; 32],
        chain: &str,
        vm: VmType,
        operation: CrossDomainOperation,
        tx: &str,
    ) -> CrossDomainProofBundle {
        let block_hash = format!("{chain}-block");
        CrossDomainProofBundle::new(
            intent,
            runtime_intent_id,
            chain.into(),
            vm,
            operation,
            tx.into(),
            10,
            block_hash.clone(),
            vec![1, 2, 3],
            FinalityProof {
                chain_id: chain.into(),
                vm_type: vm,
                tx_id: tx.into(),
                block_number: 10,
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
    fn claim_envelope_encodes_runtime_call_arguments() {
        let intent = intent();
        let runtime = [0xabu8; 32];
        let mut set = CrossDomainProofSet::new(&intent, runtime);
        set.push_verified(
            &intent,
            bundle(
                &intent,
                runtime,
                "eth-mainnet",
                VmType::Evm,
                CrossDomainOperation::Claim,
                "0xclaim",
            ),
        )
        .unwrap();

        let envelope = SettlementSubmissionEnvelope::for_claim(
            &intent,
            runtime,
            set,
            &[("eth-mainnet".into(), VmType::Evm)],
        )
        .unwrap();

        assert_eq!(envelope.call_index(), 33);
        assert!(envelope.requires_intent_party_signature());
        assert_eq!(
            envelope.post_proof_action(),
            PostProofAction::ObserveClaimFinalization
        );

        let encoded = envelope.scale_call_args();
        assert!(encoded.starts_with(&runtime));
        assert!(encoded.len() > 32);
    }

    #[test]
    fn refund_envelope_requires_refund_proofs() {
        let intent = intent();
        let runtime = [0xacu8; 32];
        let mut claim_set = CrossDomainProofSet::new(&intent, runtime);
        claim_set
            .push_verified(
                &intent,
                bundle(
                    &intent,
                    runtime,
                    "eth-mainnet",
                    VmType::Evm,
                    CrossDomainOperation::Claim,
                    "0xclaim",
                ),
            )
            .unwrap();

        assert!(
            SettlementSubmissionEnvelope::for_refund(
                &intent,
                runtime,
                claim_set,
                &[("eth-mainnet".into(), VmType::Evm)],
            )
            .is_err()
        );
    }

    #[test]
    fn wrong_runtime_intent_is_rejected() {
        let intent = intent();
        let runtime = [0xadu8; 32];
        let mut set = CrossDomainProofSet::new(&intent, runtime);
        set.push_verified(
            &intent,
            bundle(
                &intent,
                runtime,
                "eth-mainnet",
                VmType::Evm,
                CrossDomainOperation::Claim,
                "0xclaim",
            ),
        )
        .unwrap();

        assert!(
            SettlementSubmissionEnvelope::for_claim(
                &intent,
                [0xeeu8; 32],
                set,
                &[("eth-mainnet".into(), VmType::Evm)],
            )
            .is_err()
        );
    }
}
