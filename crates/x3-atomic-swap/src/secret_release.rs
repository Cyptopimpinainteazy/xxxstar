//! Secret-release firewall for cross-domain atomic swaps.
//!
//! A preimage is value-moving authority. This module centralizes the policy
//! that decides whether the coordinator is allowed to reveal it. Individual
//! adapter finality flags are necessary but never sufficient on their own.
use alloc::format;

use crate::adapter::{ChainId, FinalityProof, LockProof, VmType};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, AtomicSwapStatus, ChainKind, FinalityLevel, IntentId};
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

/// Length of an authorized RPC provider's Ed25519 public key.
pub const RPC_PROVIDER_PUBLIC_KEY_LEN: usize = 32;
/// Length of an RPC provider's Ed25519 signature.
pub const RPC_PROVIDER_SIGNATURE_LEN: usize = 64;

/// One domain that must be safely locked before the secret may be released.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretReleaseRequirement {
    pub chain_id: ChainId,
    pub vm_type: VmType,
    pub min_confirmations: u64,
    /// How many **distinct** RPC providers the policy requires to have reported
    /// the lock.
    ///
    /// This is a field of the *requirement* on purpose. It used to exist only on
    /// the attestation, and the firewall compared the attestation's
    /// `provider_count` against the attestation's own `required_quorum` — so the
    /// caller supplying the evidence also chose the bar it had to clear, and
    /// `provider_count: 3, required_quorum: 2` written by hand satisfied a
    /// security gate. A bar that the thing being judged gets to set is not a bar.
    /// `0` is refused: "no providers must agree" is not a quorum.
    pub min_providers: u32,
    /// RPC-provider signing keys authorized by policy for this domain.
    ///
    /// The quorum counts distinct keys from this set, never the labels carried
    /// by the evidence. An empty or duplicated set is refused rather than
    /// silently treated as an unauthenticated provider list.
    pub authorized_providers: Vec<[u8; RPC_PROVIDER_PUBLIC_KEY_LEN]>,
}

/// A cryptographically authenticated observation from one RPC provider.
#[derive(Clone, PartialEq, Eq)]
pub struct RpcProviderObservation {
    /// Operator-visible provider identity, bound into the signed statement.
    pub provider_id: String,
    /// Ed25519 public key of the provider.
    pub public_key: [u8; RPC_PROVIDER_PUBLIC_KEY_LEN],
    /// Ed25519 signature over [`rpc_provider_statement_hash`].
    pub signature: [u8; RPC_PROVIDER_SIGNATURE_LEN],
}

/// Canonical statement signed by one RPC provider.
///
/// The statement binds every field that changes the release decision, including
/// the provider identity. Relabelling a valid observation as another provider,
/// moving it to another chain or VM, or changing the transaction, block, or
/// finality result invalidates the signature.
pub fn rpc_provider_statement_hash(
    provider_id: &str,
    chain_id: &str,
    vm_type: VmType,
    tx_id: &str,
    block_hash: &str,
    finalized: bool,
) -> [u8; 32] {
    fn component(hasher: &mut Sha256, bytes: &[u8]) {
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }

    let mut hasher = Sha256::new();
    component(&mut hasher, b"x3-rpc-provider-observation-v1");
    component(&mut hasher, provider_id.as_bytes());
    component(&mut hasher, chain_id.as_bytes());
    component(&mut hasher, vm_type.name().as_bytes());
    component(&mut hasher, tx_id.as_bytes());
    component(&mut hasher, block_hash.as_bytes());
    hasher.update([u8::from(finalized)]);
    hasher.finalize().into()
}

/// Signed quorum attestation produced by the RPC/finality verifier.
#[derive(Clone, PartialEq, Eq)]
pub struct RpcQuorumAttestation {
    pub tx_id: String,
    pub block_hash: String,
    /// Provider observations with per-provider signatures.
    pub providers: Vec<RpcProviderObservation>,
    pub finalized: bool,
}

impl RpcQuorumAttestation {
    /// Distinct authorized provider keys with a valid statement signature.
    fn verified_providers(
        &self,
        chain_id: &str,
        vm_type: VmType,
        authorized_providers: &[[u8; RPC_PROVIDER_PUBLIC_KEY_LEN]],
    ) -> BTreeSet<[u8; RPC_PROVIDER_PUBLIC_KEY_LEN]> {
        self.providers
            .iter()
            .filter(|provider| !provider.provider_id.trim().is_empty())
            .filter(|provider| authorized_providers.contains(&provider.public_key))
            .filter(|provider| {
                let Ok(verifying_key) = VerifyingKey::from_bytes(&provider.public_key) else {
                    return false;
                };
                let statement = rpc_provider_statement_hash(
                    provider.provider_id.trim(),
                    chain_id,
                    vm_type,
                    &self.tx_id,
                    &self.block_hash,
                    self.finalized,
                );
                let signature = Signature::from_bytes(&provider.signature);
                verifying_key.verify_strict(&statement, &signature).is_ok()
            })
            .map(|provider| provider.public_key)
            .collect()
    }
}

/// Refund observation produced by the chain observer.
#[derive(Clone, PartialEq, Eq)]
pub struct RefundObservation {
    pub tx_id: String,
    pub block_hash: String,
    pub refunded: bool,
}

/// Evidence presented for one required domain.
#[derive(Clone)]
pub struct SecretReleaseEvidence {
    pub lock: LockProof,
    pub finality: FinalityProof,
    pub rpc_quorum: RpcQuorumAttestation,
    pub refund: RefundObservation,
}

/// Capability returned only after all required domains pass the firewall.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretReleasePermit {
    pub intent_id: IntentId,
    pub evidence_domains: usize,
    pub preimage: [u8; 32],
}

impl core::fmt::Debug for SecretReleasePermit {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("SecretReleasePermit")
            .field("intent_id", &self.intent_id)
            .field("evidence_domains", &self.evidence_domains)
            .field("preimage", &"[REDACTED]")
            .finish()
    }
}

impl SecretReleasePermit {
    /// Identifier of the intent this permit authorizes.
    pub fn intent_id(&self) -> IntentId {
        self.intent_id
    }

    /// Number of domains whose evidence unlocked this permit.
    pub fn evidence_domains(&self) -> usize {
        self.evidence_domains
    }

    /// The release preimage authorized by this permit.
    ///
    /// Callers must only reach this after the firewall issued the permit; the
    /// value stays out of `Debug` output on purpose.
    pub fn preimage(&self) -> [u8; 32] {
        self.preimage
    }
}

fn chain_matches_kind(chain_id: &str, kind: ChainKind) -> bool {
    let normalized = chain_id.to_ascii_lowercase();
    let aliases = match kind {
        ChainKind::Ethereum => &["eth", "ethereum"][..],
        ChainKind::Solana => &["sol", "solana"][..],
        ChainKind::Bitcoin => &["btc", "bitcoin"][..],
        ChainKind::X3 => &["x3"][..],
        ChainKind::Base => &["base"][..],
        ChainKind::Arbitrum => &["arb", "arbitrum"][..],
        ChainKind::Optimism => &["op", "optimism"][..],
        ChainKind::Bsc => &["bsc"][..],
        ChainKind::Polygon => &["poly", "polygon"][..],
        ChainKind::Avalanche => &["avax", "avalanche"][..],
        ChainKind::Cosmos => &["cosmos"][..],
    };
    aliases
        .iter()
        .any(|alias| normalized == *alias || normalized.starts_with(&format!("{alias}-")))
}

fn vm_matches_kind(vm_type: VmType, kind: ChainKind) -> bool {
    matches!(
        (vm_type, kind),
        (VmType::Evm, ChainKind::Ethereum)
            | (VmType::Evm, ChainKind::Base)
            | (VmType::Evm, ChainKind::Arbitrum)
            | (VmType::Evm, ChainKind::Optimism)
            | (VmType::Evm, ChainKind::Bsc)
            | (VmType::Evm, ChainKind::Polygon)
            | (VmType::Evm, ChainKind::Avalanche)
            | (VmType::Svm, ChainKind::Solana)
            | (VmType::X3Vm, ChainKind::X3)
            | (VmType::BitcoinScript, ChainKind::Bitcoin)
            | (VmType::CosmWasm, ChainKind::Cosmos)
    )
}

fn vm_type_for_chain(kind: ChainKind) -> VmType {
    match kind {
        ChainKind::Ethereum
        | ChainKind::Base
        | ChainKind::Arbitrum
        | ChainKind::Optimism
        | ChainKind::Bsc
        | ChainKind::Polygon
        | ChainKind::Avalanche => VmType::Evm,
        ChainKind::Solana => VmType::Svm,
        ChainKind::Bitcoin => VmType::BitcoinScript,
        ChainKind::X3 => VmType::X3Vm,
        ChainKind::Cosmos => VmType::CosmWasm,
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
        if !intent.verify_hash() {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "secret-release intent integrity",
                reason: "intent hash does not match its fields".into(),
            });
        }
        if !matches!(
            intent.status,
            AtomicSwapStatus::BothLocked
                | AtomicSwapStatus::FinalityPending
                | AtomicSwapStatus::Claimable
                | AtomicSwapStatus::PreimageRevealed
                | AtomicSwapStatus::ClaimSubmitted
        ) {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "secret-release lifecycle",
                reason: "intent is not in a releasable lifecycle state".into(),
            });
        }
        let computed = Sha256::digest(preimage);
        if computed.as_slice() != intent.hashlock {
            return Err(SwapError::HashlockMismatch);
        }

        if requirements.is_empty() {
            return Err(SwapError::MissingProof {
                proof_name: "secret-release requirements",
            });
        }

        let mut seen_required_domains: BTreeSet<(ChainId, String)> = BTreeSet::new();
        for required in requirements {
            let domain = (required.chain_id.clone(), required.vm_type.name().into());
            if !seen_required_domains.insert(domain) {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release requirements",
                    reason: alloc::format!(
                        "duplicate required domain {} / {}",
                        required.chain_id,
                        required.vm_type.name()
                    ),
                });
            }
            let distinct_authorized: BTreeSet<_> =
                required.authorized_providers.iter().copied().collect();
            if required.min_providers == 0
                || distinct_authorized.len() != required.authorized_providers.len()
                || required
                    .authorized_providers
                    .iter()
                    .any(|key| key.iter().all(|b| *b == 0))
                || distinct_authorized.len() < required.min_providers as usize
            {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release RPC provider policy",
                    reason: alloc::format!(
                        "provider policy for {} / {} is empty, duplicated, malformed, or smaller \
                         than its quorum",
                        required.chain_id,
                        required.vm_type.name()
                    ),
                });
            }
            let Some(policy) = intent.finality_requirements.iter().find(|policy| {
                chain_matches_kind(&required.chain_id, policy.chain)
                    && vm_matches_kind(required.vm_type, policy.chain)
            }) else {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release canonical requirements",
                    reason: "requirement is not declared by the intent".into(),
                });
            };
            let expected_confirmations = match policy.level {
                FinalityLevel::Confirmations(count) => count as u64,
                FinalityLevel::Finalized | FinalityLevel::Confirmed | FinalityLevel::Bft => 0,
            };
            if required.min_confirmations != expected_confirmations {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release canonical requirements",
                    reason: "caller requirement does not match intent policy".into(),
                });
            }
        }

        if seen_required_domains.len() != intent.finality_requirements.len()
            || !intent.finality_requirements.iter().all(|policy| {
                seen_required_domains.iter().any(|(chain_id, vm_name)| {
                    chain_matches_kind(chain_id, policy.chain)
                        && vm_name == vm_type_for_chain(policy.chain).name()
                })
            })
            || !intent
                .finality_requirements
                .iter()
                .any(|policy| policy.chain == intent.destination_chain)
        {
            return Err(SwapError::ProofVerificationFailed {
                proof_name: "secret-release canonical requirements",
                reason: "requirements do not cover the complete intent policy".into(),
            });
        }

        let mut consumed_evidence = BTreeSet::new();
        let mut seen_transactions: BTreeSet<(ChainId, String, String)> = BTreeSet::new();

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

            if item.refund.refunded {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release finality",
                    reason: alloc::format!(
                        "{} / {} lock has already been refunded",
                        required.chain_id,
                        required.vm_type.name()
                    ),
                });
            }

            // The bar and authorized key set are the requirement's. The count is
            // the firewall's own count of distinct authorized keys that produced
            // a valid signature over this exact domain, transaction, block, and
            // finality result.
            let verified_providers = item.rpc_quorum.verified_providers(
                &required.chain_id,
                required.vm_type,
                &required.authorized_providers,
            );
            let distinct_providers = verified_providers.len();
            if distinct_providers < required.min_providers as usize
                || !item.rpc_quorum.finalized
                || item.rpc_quorum.tx_id != item.lock.tx_id
                || item.rpc_quorum.block_hash != item.lock.block_hash
            {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release RPC quorum",
                    reason: alloc::format!(
                        "RPC quorum attestation is invalid for {} / {}: {} distinct provider(s) \
                         reported, the policy requires {}",
                        required.chain_id,
                        required.vm_type.name(),
                        distinct_providers,
                        required.min_providers
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
                        "{} / {} is not finalized or safe for reveal",
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

            if item.refund.tx_id != item.lock.tx_id
                || item.refund.block_hash != item.lock.block_hash
            {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release refund binding",
                    reason: "refund observation does not bind to the lock".into(),
                });
            }

            let transaction_key = (
                required.chain_id.clone(),
                required.vm_type.name().into(),
                item.lock.tx_id.clone(),
            );
            if !seen_transactions.insert(transaction_key) {
                return Err(SwapError::ProofVerificationFailed {
                    proof_name: "secret-release replay",
                    reason: "same lock transaction reused for multiple required domains".into(),
                });
            }

            consumed_evidence.insert(index);
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
    use crate::intent::{AtomicSwapStatus, ChainKind, FinalityRequirement, RefundPath, RouteMode};
    use alloc::vec;
    use ed25519_dalek::{Signer, SigningKey};

    const PROVIDER_SEEDS: [u8; 3] = [1, 2, 3];

    fn provider_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; RPC_PROVIDER_PUBLIC_KEY_LEN])
    }

    fn provider_public_key(seed: u8) -> [u8; RPC_PROVIDER_PUBLIC_KEY_LEN] {
        provider_key(seed).verifying_key().to_bytes()
    }

    fn provider_observation(
        provider_id: &str,
        seed: u8,
        chain: &str,
        vm: VmType,
        tx: &str,
        block_hash: &str,
        finalized: bool,
    ) -> RpcProviderObservation {
        let key = provider_key(seed);
        let statement =
            rpc_provider_statement_hash(provider_id.trim(), chain, vm, tx, block_hash, finalized);
        RpcProviderObservation {
            provider_id: provider_id.into(),
            public_key: key.verifying_key().to_bytes(),
            signature: key.sign(&statement).to_bytes(),
        }
    }

    fn all_provider_observations(
        chain: &str,
        vm: VmType,
        tx: &str,
        block_hash: &str,
        finalized: bool,
    ) -> Vec<RpcProviderObservation> {
        PROVIDER_SEEDS
            .iter()
            .enumerate()
            .map(|(index, seed)| {
                provider_observation(
                    &alloc::format!("provider-{}", (b'a' + index as u8) as char),
                    *seed,
                    chain,
                    vm,
                    tx,
                    block_hash,
                    finalized,
                )
            })
            .collect()
    }

    fn intent(preimage: [u8; 32]) -> AtomicIntent {
        let hashlock = Sha256::digest(preimage);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hashlock);
        let mut intent = AtomicIntent {
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
        };
        intent.intent_hash = intent.compute_hash();
        intent
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
                block_hash: block_hash.clone(),
                confirmations,
                finalized: true,
                finality_source: "test-finality".into(),
                safe_to_reveal_secret: true,
            },
            rpc_quorum: RpcQuorumAttestation {
                tx_id: tx.into(),
                block_hash: block_hash.clone(),
                providers: all_provider_observations(chain, vm, tx, &block_hash, true),
                finalized: true,
            },
            refund: RefundObservation {
                tx_id: tx.into(),
                block_hash,
                refunded: false,
            },
        }
    }

    fn req(chain: &str, vm: VmType, min_confirmations: u64) -> SecretReleaseRequirement {
        SecretReleaseRequirement {
            chain_id: chain.into(),
            vm_type: vm,
            min_confirmations,
            min_providers: 2,
            authorized_providers: PROVIDER_SEEDS
                .iter()
                .map(|seed| provider_public_key(*seed))
                .collect(),
        }
    }

    #[test]
    fn authorizes_only_when_all_required_domains_are_finalized() {
        let preimage = [0x11u8; 32];
        let intent = intent(preimage);
        let evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        let requirements = [req("eth-mainnet", VmType::Evm, 12)];

        let permit = SecretReleaseFirewall::authorize(&intent, preimage, &requirements, &[evm])
            .expect("all domains are safely finalized");
        assert_eq!(permit.intent_id, intent.intent_id);
        assert_eq!(permit.evidence_domains, 1);
        assert_eq!(permit.preimage, preimage);
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
        assert!(
            SecretReleaseFirewall::authorize(&intent, preimage, &requirements, &[evm]).is_err()
        );
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
        // One provider reported; the policy requires two.
        evm.rpc_quorum.providers.truncate(1);
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());
    }

    #[test]
    fn the_evidence_cannot_set_its_own_quorum_bar() {
        // The shape this replaced carried `provider_count` *and* `required_quorum`
        // in the attestation, and the firewall compared them to each other — so a
        // caller who wrote `provider_count: 3, required_quorum: 2` satisfied the
        // gate. Now the bar is the requirement's, and three providers cannot clear
        // a four-provider policy however the evidence describes itself.
        let preimage = [0x17u8; 32];
        let intent = intent(preimage);
        let evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        let strict = SecretReleaseRequirement {
            min_providers: 4,
            ..req("eth-mainnet", VmType::Evm, 12)
        };
        let strict_evidence = [evm.clone()];
        assert!(
            matches!(
                SecretReleaseFirewall::authorize(&intent, preimage, &[strict], &strict_evidence),
                Err(SwapError::ProofVerificationFailed { .. })
            ),
            "three providers must not clear a four-provider policy"
        );

        // The same evidence clears the policy it does meet, so the refusal above
        // is the bar and not something else about it.
        let met = SecretReleaseRequirement {
            min_providers: 3,
            ..req("eth-mainnet", VmType::Evm, 12)
        };
        assert!(SecretReleaseFirewall::authorize(&intent, preimage, &[met], &[evm]).is_ok());
    }

    #[test]
    fn one_provider_listed_twice_is_one_provider() {
        let preimage = [0x18u8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        let first = evm.rpc_quorum.providers[0].clone();
        evm.rpc_quorum.providers = vec![first.clone(), first];
        assert!(
            SecretReleaseFirewall::authorize(
                &intent,
                preimage,
                &[req("eth-mainnet", VmType::Evm, 12)],
                &[evm]
            )
            .is_err(),
            "a repeated id is not a second provider"
        );
    }

    #[test]
    fn an_unnamed_provider_does_not_count() {
        let preimage = [0x19u8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        let first = evm.rpc_quorum.providers[0].clone();
        evm.rpc_quorum.providers = vec![
            first,
            provider_observation(
                "   ",
                2,
                "eth-mainnet",
                VmType::Evm,
                "0xevm",
                "0xblock10",
                true,
            ),
        ];
        assert!(
            SecretReleaseFirewall::authorize(
                &intent,
                preimage,
                &[req("eth-mainnet", VmType::Evm, 12)],
                &[evm]
            )
            .is_err(),
            "a blank id is not a provider that reported"
        );
    }

    #[test]
    fn a_zero_provider_policy_is_refused() {
        let preimage = [0x1au8; 32];
        let intent = intent(preimage);
        let evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        let nonsense = SecretReleaseRequirement {
            min_providers: 0,
            ..req("eth-mainnet", VmType::Evm, 12)
        };
        assert!(
            SecretReleaseFirewall::authorize(&intent, preimage, &[nonsense], &[evm]).is_err(),
            "\"no provider has to agree\" is not a quorum"
        );
    }

    #[test]
    fn rejects_a_forged_provider_signature() {
        let preimage = [0x1bu8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.rpc_quorum.providers.truncate(1);
        evm.rpc_quorum.providers[0].signature[0] ^= 1;
        assert!(
            SecretReleaseFirewall::authorize(
                &intent,
                preimage,
                &[req("eth-mainnet", VmType::Evm, 12)],
                &[evm]
            )
            .is_err(),
            "a provider label without a valid signature is not a provider observation"
        );
    }

    #[test]
    fn rejects_a_valid_signature_from_an_unauthorized_provider() {
        let preimage = [0x1cu8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.rpc_quorum.providers[2] = provider_observation(
            "provider-outsider",
            9,
            "eth-mainnet",
            VmType::Evm,
            "0xevm",
            "0xblock10",
            true,
        );
        let strict = SecretReleaseRequirement {
            min_providers: 3,
            ..req("eth-mainnet", VmType::Evm, 12)
        };
        assert!(
            SecretReleaseFirewall::authorize(&intent, preimage, &[strict], &[evm]).is_err(),
            "a valid signature proves key control, not authorization"
        );
    }

    #[test]
    fn provider_identity_and_observation_bind_the_signature() {
        let preimage = [0x1du8; 32];
        let intent = intent(preimage);
        let mut evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        evm.rpc_quorum.providers[2].provider_id = "provider-relabelled".into();
        let strict = SecretReleaseRequirement {
            min_providers: 3,
            ..req("eth-mainnet", VmType::Evm, 12)
        };
        assert!(
            SecretReleaseFirewall::authorize(&intent, preimage, &[strict], &[evm]).is_err(),
            "relabelling a signed provider observation must invalidate it"
        );
    }

    #[test]
    fn a_duplicated_or_empty_authorized_provider_set_is_refused() {
        let preimage = [0x1eu8; 32];
        let intent = intent(preimage);
        let evm = evidence("eth-mainnet", VmType::Evm, "0xevm", 10, intent.hashlock, 12);
        let duplicated = SecretReleaseRequirement {
            authorized_providers: vec![provider_public_key(1), provider_public_key(1)],
            ..req("eth-mainnet", VmType::Evm, 12)
        };
        assert!(SecretReleaseFirewall::authorize(
            &intent,
            preimage,
            &[duplicated],
            core::slice::from_ref(&evm),
        )
        .is_err());

        let empty = SecretReleaseRequirement {
            authorized_providers: Vec::new(),
            ..req("eth-mainnet", VmType::Evm, 12)
        };
        assert!(SecretReleaseFirewall::authorize(&intent, preimage, &[empty], &[evm]).is_err());
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
        evm.refund.refunded = true;
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
        let evm_a = evidence(
            "eth-mainnet",
            VmType::Evm,
            "same-tx",
            10,
            intent.hashlock,
            12,
        );
        let mut evm_b = evidence(
            "base-mainnet",
            VmType::Evm,
            "same-tx",
            20,
            intent.hashlock,
            12,
        );
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

    #[test]
    fn rejects_stale_intent_hash_and_terminal_status() {
        let preimage = [0x21u8; 32];
        let mut stale_intent = intent(preimage);
        stale_intent.receiver = "tampered".into();
        let evm = evidence(
            "eth-mainnet",
            VmType::Evm,
            "0xevm",
            10,
            stale_intent.hashlock,
            12,
        );
        assert!(SecretReleaseFirewall::authorize(
            &stale_intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());

        let mut terminal_intent = intent(preimage);
        terminal_intent.status = AtomicSwapStatus::Claimed;
        terminal_intent.intent_hash = terminal_intent.compute_hash();
        let evm = evidence(
            "eth-mainnet",
            VmType::Evm,
            "0xevm",
            10,
            terminal_intent.hashlock,
            12,
        );
        assert!(SecretReleaseFirewall::authorize(
            &terminal_intent,
            preimage,
            &[req("eth-mainnet", VmType::Evm, 12)],
            &[evm]
        )
        .is_err());
    }
}
