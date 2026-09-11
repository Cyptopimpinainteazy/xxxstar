//! Secret-release firewall for cross-domain atomic swaps.
//!
//! A preimage is value-moving authority. This module centralizes the policy
//! that decides whether the coordinator is allowed to reveal it. Individual
//! adapter finality flags are necessary but never sufficient on their own.

use crate::adapter::{ChainId, FinalityProof, LockProof, VmType};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, IntentId};
use alloc::string::String;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

/// One domain that must be safely locked before the secret may be released.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretReleaseRequirement {
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub min_confirmations: u64,
}

/// Evidence presented for one required domain.
#[derive(Debug, Clone)]
pub struct SecretReleaseEvidence {
    pub lock: LockProof,
    pub finality: FinalityProof,
    /// Independent RPC/quorum layer agrees on the transaction/finality state.
    pub rpc_quorum_agreed: bool,
    /// A refund has been observed or finalized for this lock.
    pub refunded: bool,
}

/// Capability returned only after all required domains pass the firewall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretReleasePermit {
    intent_id: IntentId,
    evidence_domains: usize,
    preimage: [u8; 32],
}

impl SecretReleasePermit {
    pub fn intent_id(&self) -> IntentId {
        self.intent_id
    }

    pub fn evidence_domains(&self) -> usize {
        self.evidence_domains
    }

    pub fn preimage(&self) -> [u8; 32] {
        self.preimage
    }
}

/// Fail-closed secret release policy.
pub struct SecretReleaseFirewall;

impl SecretReleaseFirewall {
    pub fn authorize(
        intent: &AtomicIntent,
        preimage: [u8; 32],
        requirements: &[SecretReleaseRequirement],
        evidence: &[SecretReleaseEvidence],
    ) -> Result<SecretReleasePermit, SwapError> {
        let computed = Sha256::digest(preimage);
        if computed.as_slice() != intent.hashlock {
            return Err(SwapError::HashlockMismatch);
        }

        if requirements.is_empty() {
            return Err(SwapError::MissingProof {
                proof_name: "secret-release requirements",
            });
        }

        let mut seen_required_domains: Vec<(String, VmType)> = Vec::new();
        for required in requirements {
            let domain = (required.chain_id.clone(), required.vm_type);
            if seen_required_domains.contains(&domain) {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release requirements",
                    reason: alloc::format!(
                        "duplicate required domain {} / {}",
                        required.chain_id,
                        required.vm_type.name()
                    ),
                });
            }
            seen_required_domains.push(domain);
        }

        let mut consumed_evidence: Vec<usize> = Vec::new();
        let mut seen_tx_ids: Vec<String> = Vec::new();

        for required in requirements {
            let Some((index, item)) = evidence.iter().enumerate().find(|(index, item)| {
                !consumed_evidence.contains(index)
                    && item.lock.chain_id == required.chain_id
                    && item.lock.vm_type == required.vm_type
            }) else {
                return Err(SwapError::MissingProof {
                    proof_name: "required destination lock/finality evidence",
                });
            };

            if item.refunded {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release finality",
                    reason: alloc::format!(
                        "{} / {} lock has already been refunded",
                        required.chain_id,
                        required.vm_type.name()
                    ),
                });
            }

            if !item.rpc_quorum_agreed {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release RPC quorum",
                    reason: alloc::format!(
                        "RPC providers disagree for {} / {}",
                        required.chain_id,
                        required.vm_type.name()
                    ),
                });
            }

            if item.lock.hashlock != intent.hashlock {
                return Err(SwapError::HashlockMismatch);
            }

            if item.lock.tx_id.is_empty()
                || item.lock.block_hash.is_empty()
                || item.finality.tx_id.is_empty()
                || item.finality.block_hash.is_empty()
            {
                return Err(SwapError::MissingTxHash {
                    step: "secret-release evidence".into(),
                    chain: required.chain_id.clone(),
                });
            }

            if item.finality.chain_id != required.chain_id
                || item.finality.vm_type != required.vm_type
            {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release domain binding",
                    reason: alloc::format!(
                        "finality proof belongs to {} / {}, expected {} / {}",
                        item.finality.chain_id,
                        item.finality.vm_type.name(),
                        required.chain_id,
                        required.vm_type.name()
                    ),
                });
            }

            if item.finality.tx_id != item.lock.tx_id {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release transaction binding",
                    reason: "finality proof does not bind to the lock transaction".into(),
                });
            }

            if item.finality.block_hash != item.lock.block_hash
                || item.finality.block_number != item.lock.block_number
            {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release block binding",
                    reason: "finality proof does not bind to the lock block".into(),
                });
            }

            if !item.finality.finalized || !item.finality.safe_to_reveal_secret {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release finality",
                    reason: alloc::format!(
                        "{} / {} is not finalized and safe for reveal",
                        required.chain_id,
                        required.vm_type.name()
                    ),
                });
            }

            if item.finality.confirmations < required.min_confirmations {
                return Err(SwapError::FinalityNotMet {
                    chain: required.chain_id.clone(),
                    required: required.min_confirmations.min(u32::MAX as u64) as u32,
                    current: item.finality.confirmations.min(u32::MAX as u64) as u32,
                });
            }

            if seen_tx_ids.contains(&item.lock.tx_id) {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release replay",
                    reason: "same lock transaction reused for multiple required domains".into(),
                });
            }

            seen_tx_ids.push(item.lock.tx_id.clone());
            consumed_evidence.push(index);
        }

        Ok(SecretReleasePermit {
            intent_id: intent.intent_id,
            evidence_domains: requirements.len(),
            preimage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{
        AtomicSwapStatus, ChainKind, FinalityLevel, FinalityRequirement, RefundPath, RouteMode,
    };

    fn intent(preimage: [u8; 32]) -> AtomicIntent {
        let hashlock = Sha256::digest(preimage);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hashlock);
        AtomicIntent {
            intent_id: 7001,
            source_chain: ChainKind::X3,
            destination_chain: ChainKind::Ethereum,
            source_asset: "X3".into(),
            destination_asset: "ETH".into(),
            amount_in: 1_000,
            min_amount_out: 1,
            receiver: "receiver".into(),
            hashlock: hash,
            source_timeout: 2_000,
            destination_timeout: 1_000,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::Ethereum,
                level: FinalityLevel::Confirmations(12),
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "refund".into(),
                asset: None,
            },
            route_mode: RouteMode::DirectHtlc,
            max_slippage_bps: 100,
            relayer_quorum_requirement: 2,
            status: AtomicSwapStatus::FinalityPending,
            intent_hash: [0u8; 32],
        }
    }

    fn evidence(
        chain: &str,
        vm: VmType,
        tx: &str,
        block: u64,
        hashlock: [u8; 32],
        confirmations: u64,
    ) -> SecretReleaseEvidence {
        let block_hash = alloc::format!("0xblock{block}");
        SecretReleaseEvidence {
            lock: LockProof {
                tx_id: tx.into(),
                chain_id: chain.into(),
                vm_type: vm,
                block_number: block,
                block_hash: block_hash.clone(),
                confirmations,
                lock_address: "escrow".into(),
                locked_amount: 100,
                hashlock,
                receiver: vec![1],
                refund_address: vec![2],
                timeout: 1_000,
                raw_proof: vec![3],
            },
            finality: FinalityProof {
                chain_id: chain.into(),
                vm_type: vm,
                tx_id: tx.into(),
                block_number: block,
                block_hash,
                confirmations,
                finalized: true,
                finality_source: "test-finality".into(),
                safe_to_reveal_secret: true,
            },
            rpc_quorum_agreed: true,
            refunded: false,
        }
    }

    fn req(chain: &str, vm: VmType, min_confirmations: u64) -> SecretReleaseRequirement {
        SecretReleaseRequirement {
            chain_id: chain.into(),
            vm_type: vm,
            min_confirmations,
        }
    }

    #[test]
    fn authorizes_only_when_all_required_domains_are_finalized() {
        let preimage = [0x11u8; 32];
        let intent = intent(preimage);
        let evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        let svm = evidence("solana-mainnet", VmType::Svm, "solsig", 20, intent.hashlock, 1);
        let requirements = [
            req("eth-mainnet", VmType::Evm, 12),
            req("solana-mainnet", VmType::Svm, 1),
        ];

        let permit =
            SecretReleaseFirewall::authorize(&intent, preimage, &requirements, &[evm, svm])
                .expect("all domains are safely finalized");
        assert_eq!(permit.intent_id(), intent.intent_id);
        assert_eq!(permit.evidence_domains(), 2);
        assert_eq!(permit.preimage(), preimage);
    }

    #[test]
    fn rejects_missing_destination_domain() {
        let preimage = [0x12u8; 32];
        let intent = intent(preimage);
        let evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        let requirements = [
            req("eth-mainnet", VmType::Evm, 12),
            req("solana-mainnet", VmType::Svm, 1),
        ];
        assert!(SecretReleaseFirewall::authorize(&intent, preimage, &requirements, &[evm]).is_err());
    }

    #[test]
    fn rejects_included_but_not_finalized_transaction() {
        let preimage = [0x13u8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.finality.finalized = false;
        evm.finality.safe_to_reveal_secret = false;
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());
    }

    #[test]
    fn rejects_insufficient_confirmations() {
        let preimage = [0x14u8; 32];
        let intent = intent(preimage);
        let evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 3);
        assert!(matches!(
            SecretReleaseFirewall::authorize(
                &intent,
                preimage,
                &[req("eth-mainnet", VmType::Evm, 12)],
                &[evm]
            ),
            Err(SwapError::FinalityNotMet { .. })
        ));
    }

    #[test]
    fn rejects_rpc_disagreement() {
        let preimage = [0x15u8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.rpc_quorum_agreed = false;
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());
    }

    #[test]
    fn rejects_wrong_chain_or_vm_binding() {
        let preimage = [0x16u8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.finality.chain_id = "base-mainnet".into();
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());
    }

    #[test]
    fn rejects_finality_for_different_transaction_or_block() {
        let preimage = [0x17u8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.finality.tx_id = "0xother".into();
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());

        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.finality.block_hash = "0xotherblock".into();
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());
    }

    #[test]
    fn rejects_refunded_destination() {
        let preimage = [0x18u8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.refunded = true;
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());
    }

    #[test]
    fn rejects_wrong_preimage() {
        let preimage = [0x19u8; 32];
        let intent = intent(preimage);
        let evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        assert!(matches!(
            SecretReleaseFirewall::authorize(
                &intent,
                [0x99u8; 32],
                &[req("eth-mainnet", VmType::Evm, 12)],
                &[evm]
            ),
            Err(SwapError::HashlockMismatch)
        ));
    }

    #[test]
    fn rejects_reused_proof_across_required_domains() {
        let preimage = [0x20u8; 32];
        let intent = intent(preimage);
        let evm_a = evidence("eth-mainnet", VmType::Evm, "same-tx", 10, intent.hashlock, 12);
        let mut evm_b = evidence("base-mainnet", VmType::Evm, "same-tx", 20, intent.hashlock, 12);
        evm_b.finality.chain_id = "base-mainnet".into();
        let requirements = [
            req("eth-mainnet", VmType::Evm, 12),
            req("base-mainnet", VmType::Evm, 12),
        ];
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &requirements,
            &[evm_a, evm_b]
        )
        .is_err());
    }
}
