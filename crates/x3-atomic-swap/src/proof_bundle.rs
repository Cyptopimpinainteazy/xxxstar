//! Canonical cross-domain proof bundle.
//!
//! Settlement and recovery should consume one domain-bound artifact rather than
//! unrelated tx hashes, block fields, receipt bytes, and finality flags.

use crate::adapter::{ChainId, FinalityProof, VmType};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, IntentId};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Value-moving operation proven by this bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossDomainOperation {
    Lock,
    Claim,
    Refund,
}

/// Canonical evidence package for one operation on one execution domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainProofBundle {
    pub version: u32,
    pub intent_id: IntentId,
    pub intent_hash: [u8; 32],
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub operation: CrossDomainOperation,
    pub tx_id: String,
    pub block_number: u64,
    pub block_hash: String,
    /// Receipt/event/account-state/inclusion evidence produced by the domain adapter.
    pub execution_evidence: Vec<u8>,
    /// Domain-specific proof that the transaction/block is final.
    pub finality: FinalityProof,
    /// Hash of all canonical fields above. Detects mutation/rebinding.
    pub proof_hash: [u8; 32],
}

impl CrossDomainProofBundle {
    pub const VERSION: u32 = 1;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        intent: &AtomicIntent,
        chain_id: ChainId,
        vm_type: VmType,
        operation: CrossDomainOperation,
        tx_id: String,
        block_number: u64,
        block_hash: String,
        execution_evidence: Vec<u8>,
        finality: FinalityProof,
    ) -> Result<Self, SwapError> {
        let mut bundle = Self {
            version: Self::VERSION,
            intent_id: intent.intent_id,
            intent_hash: intent.intent_hash,
            chain_id,
            vm_type,
            operation,
            tx_id,
            block_number,
            block_hash,
            execution_evidence,
            finality,
            proof_hash: [0u8; 32],
        };
        bundle.validate_bindings()?;
        bundle.proof_hash = bundle.compute_hash()?;
        Ok(bundle)
    }

    /// Recompute the canonical bundle hash with proof_hash zeroed.
    pub fn compute_hash(&self) -> Result<[u8; 32], SwapError> {
        let mut canonical = self.clone();
        canonical.proof_hash = [0u8; 32];
        let bytes = serde_json::to_vec(&canonical).map_err(|e| {
            SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof bundle",
                reason: alloc::format!("canonical serialization failed: {e}"),
            }
        })?;
        let digest = Sha256::digest(bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        Ok(out)
    }

    /// Validate domain, transaction, block, finality and integrity bindings.
    pub fn verify(&self, expected_intent: &AtomicIntent) -> Result<(), SwapError> {
        if self.version != Self::VERSION {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof bundle",
                reason: alloc::format!("unsupported bundle version {}", self.version),
            });
        }
        if self.intent_id != expected_intent.intent_id
            || self.intent_hash != expected_intent.intent_hash
        {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof intent binding",
                reason: "bundle belongs to a different intent".into(),
            });
        }
        self.validate_bindings()?;
        let expected_hash = self.compute_hash()?;
        if expected_hash != self.proof_hash {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof integrity",
                reason: "proof_hash mismatch".into(),
            });
        }
        Ok(())
    }

    fn validate_bindings(&self) -> Result<(), SwapError> {
        if self.tx_id.is_empty() || self.block_hash.is_empty() || self.execution_evidence.is_empty() {
            return Err(SwapError::MissingProof {
                proof_name: "cross-domain transaction/block/execution evidence",
            });
        }

        if self.finality.chain_id != self.chain_id || self.finality.vm_type != self.vm_type {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof domain binding",
                reason: "finality domain does not match execution domain".into(),
            });
        }

        if self.finality.tx_id != self.tx_id {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof transaction binding",
                reason: "finality transaction does not match execution transaction".into(),
            });
        }

        if self.finality.block_number != self.block_number
            || self.finality.block_hash != self.block_hash
        {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof block binding",
                reason: "finality block does not match execution block".into(),
            });
        }

        if !self.finality.finalized {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof finality",
                reason: "operation is not finalized".into(),
            });
        }

        Ok(())
    }
}

/// A set of domain proofs for one atomic intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainProofSet {
    pub intent_id: IntentId,
    pub intent_hash: [u8; 32],
    pub bundles: Vec<CrossDomainProofBundle>,
}

impl CrossDomainProofSet {
    pub fn new(intent: &AtomicIntent) -> Self {
        Self {
            intent_id: intent.intent_id,
            intent_hash: intent.intent_hash,
            bundles: Vec::new(),
        }
    }

    pub fn push_verified(
        &mut self,
        intent: &AtomicIntent,
        bundle: CrossDomainProofBundle,
    ) -> Result<(), SwapError> {
        bundle.verify(intent)?;
        if self.intent_id != intent.intent_id || self.intent_hash != intent.intent_hash {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof set",
                reason: "proof set belongs to a different intent".into(),
            });
        }
        if self.bundles.iter().any(|existing| {
            existing.chain_id == bundle.chain_id
                && existing.vm_type == bundle.vm_type
                && existing.operation == bundle.operation
        }) {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "cross-domain proof replay",
                reason: "duplicate domain operation proof".into(),
            });
        }
        self.bundles.push(bundle);
        Ok(())
    }

    pub fn require_operation(
        &self,
        chain_id: &str,
        vm_type: VmType,
        operation: CrossDomainOperation,
    ) -> Result<&CrossDomainProofBundle, SwapError> {
        self.bundles
            .iter()
            .find(|bundle| {
                bundle.chain_id == chain_id
                    && bundle.vm_type == vm_type
                    && bundle.operation == operation
            })
            .ok_or(SwapError::MissingProof {
                proof_name: "required cross-domain operation proof",
            })
    }

    /// Settlement success requires a finalized claim proof for every required domain.
    pub fn verify_claim_set(
        &self,
        intent: &AtomicIntent,
        required_domains: &[(ChainId, VmType)],
    ) -> Result<(), SwapError> {
        if required_domains.is_empty() {
            return Err(SwapError::MissingProof {
                proof_name: "required settlement domains",
            });
        }

        for (chain_id, vm_type) in required_domains {
            let bundle =
                self.require_operation(chain_id, *vm_type, CrossDomainOperation::Claim)?;
            bundle.verify(intent)?;
        }
        Ok(())
    }

    /// Refund completion requires a finalized refund proof for every required domain.
    pub fn verify_refund_set(
        &self,
        intent: &AtomicIntent,
        required_domains: &[(ChainId, VmType)],
    ) -> Result<(), SwapError> {
        if required_domains.is_empty() {
            return Err(SwapError::MissingProof {
                proof_name: "required refund domains",
            });
        }

        for (chain_id, vm_type) in required_domains {
            let bundle =
                self.require_operation(chain_id, *vm_type, CrossDomainOperation::Refund)?;
            bundle.verify(intent)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{AtomicIntentBuilder, ChainKind, RefundPath};

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
            .build(9001)
            .expect("intent")
    }

    fn finality(chain: &str, vm: VmType, tx: &str, block: u64, hash: &str) -> FinalityProof {
        FinalityProof {
            chain_id: chain.into(),
            vm_type: vm,
            tx_id: tx.into(),
            block_number: block,
            block_hash: hash.into(),
            confirmations: 12,
            finalized: true,
            finality_source: "test".into(),
            safe_to_reveal_secret: true,
        }
    }

    fn bundle(
        intent: &AtomicIntent,
        chain: &str,
        vm: VmType,
        operation: CrossDomainOperation,
        tx: &str,
        block: u64,
    ) -> CrossDomainProofBundle {
        let block_hash = alloc::format!("0xblock{block}");
        CrossDomainProofBundle::new(
            intent,
            chain.into(),
            vm,
            operation,
            tx.into(),
            block,
            block_hash.clone(),
            vec![1, 2, 3],
            finality(chain, vm, tx, block, &block_hash),
        )
        .expect("bundle")
    }

    #[test]
    fn canonical_bundle_verifies() {
        let intent = intent();
        bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Lock,
            "0xlock",
            10,
        )
        .verify(&intent)
        .expect("valid bundle");
    }

    #[test]
    fn mutation_breaks_proof_hash() {
        let intent = intent();
        let mut proof = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Claim,
            "0xclaim",
            11,
        );
        proof.block_number = 12;
        assert!(proof.verify(&intent).is_err());
    }

    #[test]
    fn rejects_wrong_finality_domain_transaction_and_block() {
        let intent = intent();
        let mut wrong_domain = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Lock,
            "0xlock",
            10,
        );
        wrong_domain.finality.chain_id = "base-mainnet".into();
        assert!(wrong_domain.verify(&intent).is_err());

        let mut wrong_tx = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Lock,
            "0xlock",
            10,
        );
        wrong_tx.finality.tx_id = "0xother".into();
        assert!(wrong_tx.verify(&intent).is_err());

        let mut wrong_block = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Lock,
            "0xlock",
            10,
        );
        wrong_block.finality.block_hash = "0xother".into();
        assert!(wrong_block.verify(&intent).is_err());
    }

    #[test]
    fn rejects_unfinalized_or_empty_execution_evidence() {
        let intent = intent();
        let mut proof = bundle(
            &intent,
            "solana-mainnet",
            VmType::Svm,
            CrossDomainOperation::Lock,
            "sig",
            20,
        );
        proof.finality.finalized = false;
        assert!(proof.verify(&intent).is_err());

        let mut proof = bundle(
            &intent,
            "solana-mainnet",
            VmType::Svm,
            CrossDomainOperation::Lock,
            "sig",
            20,
        );
        proof.execution_evidence.clear();
        assert!(proof.verify(&intent).is_err());
    }

    #[test]
    fn proof_set_rejects_duplicate_domain_operation() {
        let intent = intent();
        let mut set = CrossDomainProofSet::new(&intent);
        let first = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Claim,
            "0xclaim1",
            11,
        );
        let second = bundle(
            &intent,
            "eth-mainnet",
            VmType::Evm,
            CrossDomainOperation::Claim,
            "0xclaim2",
            12,
        );
        set.push_verified(&intent, first).expect("first");
        assert!(set.push_verified(&intent, second).is_err());
    }

    #[test]
    fn settlement_requires_claim_from_every_required_domain() {
        let intent = intent();
        let mut set = CrossDomainProofSet::new(&intent);
        set.push_verified(
            &intent,
            bundle(
                &intent,
                "x3-local",
                VmType::X3Vm,
                CrossDomainOperation::Claim,
                "0xx3claim",
                30,
            ),
        )
        .unwrap();

        let required = vec![
            ("x3-local".into(), VmType::X3Vm),
            ("eth-mainnet".into(), VmType::Evm),
        ];
        assert!(set.verify_claim_set(&intent, &required).is_err());

        set.push_verified(
            &intent,
            bundle(
                &intent,
                "eth-mainnet",
                VmType::Evm,
                CrossDomainOperation::Claim,
                "0xethclaim",
                31,
            ),
        )
        .unwrap();
        set.verify_claim_set(&intent, &required)
            .expect("all domains claimed");
    }

    #[test]
    fn refund_set_requires_refund_from_every_required_domain() {
        let intent = intent();
        let mut set = CrossDomainProofSet::new(&intent);
        let required = vec![
            ("x3-local".into(), VmType::X3Vm),
            ("solana-mainnet".into(), VmType::Svm),
        ];

        for (chain, vm, tx, block) in [
            ("x3-local", VmType::X3Vm, "0xx3refund", 40),
            ("solana-mainnet", VmType::Svm, "solrefund", 41),
        ] {
            set.push_verified(
                &intent,
                bundle(
                    &intent,
                    chain,
                    vm,
                    CrossDomainOperation::Refund,
                    tx,
                    block,
                ),
            )
            .unwrap();
        }

        set.verify_refund_set(&intent, &required)
            .expect("all domains refunded");
    }
}
