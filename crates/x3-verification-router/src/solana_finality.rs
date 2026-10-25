//! Stake-weighted Solana finality verification.
//!
//! This module deliberately separates the Solana-specific proof from the
//! generic router. A proof is accepted only when:
//!
//! - it names the configured Solana cluster;
//! - it supplies a finalized slot, blockhash, and bank hash, plus its parent slot
//!   and parent bank hash;
//! - its stake-table digest matches the governance-configured stake table;
//! - distinct authorized validators holding at least the configured
//!   supermajority of stake sign the exact X3 transfer intent and finality
//!   claim; and
//! - the finalized slot is not below the caller-supplied finalized-slot floor.
//!
//! The finalized-slot floor is explicit rather than hidden mutable state. That
//! keeps verification deterministic and lets the runtime persist the floor in
//! consensus storage, where it can survive restarts and reorgs.
//!
//! This verifier proves a stake-weighted finalized blockhash and bank hash, and
//! binds both to the transfer fields in [`ProofEnvelope`]. It does not parse a
//! raw Solana transaction. A production producer must obtain the blockhash,
//! bank hash, and transfer result from a Solana RPC provider quorum and persist
//! the finalized slot before accepting value. Raw transaction-entry and
//! instruction decoding remains a separate adapter boundary.

extern crate alloc;

use alloc::collections::btree_set::BTreeSet;
use alloc::vec::Vec;
use blake2::digest::consts::U32;
use blake2::{Blake2b, Digest};

use crate::{
    ChainKind, ProofEnvelope, VerificationError, VerificationOutcome, VerificationStrategy,
    Verifier,
};

/// Version byte for a stake-weighted Solana finality proof that commits to the
/// finalized blockhash and bank hash separately.
pub const SOLANA_FINALIZED_FORMAT_V3: u8 = 3;

/// Maximum number of vote records accepted in one V3 proof.
///
/// The bound keeps malformed payloads from forcing unbounded verification work.
pub const MAX_SOLANA_FINALITY_VOTES: usize = 1_024;

/// Fixed length of a V2 vote record: Ed25519 public key plus signature.
pub const SOLANA_FINALITY_VOTE_LEN: usize = 96;

/// Fixed header length of a V3 proof.
///
/// ```text
/// version u8 | cluster u8 | finalized_slot u64 | parent_slot u64 |
/// finalized_blockhash [32] | finalized_bank_hash [32] |
/// finalized_bank_hash [32] | parent_bank_hash [32] |
/// stake_table_digest [32] | vote_count u32
/// ```
pub const SOLANA_FINALITY_V3_HEADER_LEN: usize = 150;

const STAKE_TABLE_DOMAIN: &[u8] = b"X3/SOLANA_STAKE_TABLE/V2";
const FINALITY_DOMAIN: &[u8] = b"X3/SOLANA_FINALITY/V3";
const TRANSFER_DOMAIN: &[u8] = b"X3/SOLANA_TRANSFER/V3";

/// Solana cluster bound by a finality proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolanaCluster {
    MainnetBeta = 1,
    Testnet = 2,
    Devnet = 3,
}

impl SolanaCluster {
    fn decode(value: u8) -> Result<Self, VerificationError> {
        match value {
            1 => Ok(Self::MainnetBeta),
            2 => Ok(Self::Testnet),
            3 => Ok(Self::Devnet),
            _ => Err(VerificationError::UnsupportedChain),
        }
    }
}

/// One governance-authorized validator and its effective stake weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolanaValidatorStake {
    pub pubkey: [u8; 32],
    pub stake: u64,
}

/// Why a stake-weighted verifier could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolanaVerifierConfigError {
    EmptyStakeTable,
    ZeroStake,
    DuplicateValidator,
    UnsortedValidatorSet,
    InvalidThreshold,
    StakeOverflow,
}

/// Stake-weighted Solana finality verifier.
pub struct SolanaStakeWeightedVerifier {
    cluster: SolanaCluster,
    validators: Vec<SolanaValidatorStake>,
    total_stake: u64,
    required_stake: u64,
    stake_table_digest: [u8; 32],
    minimum_finalized_slot: u64,
}

impl SolanaStakeWeightedVerifier {
    /// Construct a verifier over a canonical validator stake table.
    ///
    /// Validators must be sorted by public key and unique. Requiring a canonical
    /// ordering means the stake-table digest is independent of JSON/RPC ordering
    /// and cannot silently differ across operators.
    pub fn new(
        cluster: SolanaCluster,
        validators: Vec<SolanaValidatorStake>,
        threshold_numerator: u32,
        threshold_denominator: u32,
        minimum_finalized_slot: u64,
    ) -> Result<Self, SolanaVerifierConfigError> {
        if validators.is_empty() {
            return Err(SolanaVerifierConfigError::EmptyStakeTable);
        }
        if threshold_numerator == 0
            || threshold_denominator == 0
            || threshold_numerator > threshold_denominator
        {
            return Err(SolanaVerifierConfigError::InvalidThreshold);
        }

        let mut total_stake = 0u64;
        let mut previous: Option<[u8; 32]> = None;
        for validator in &validators {
            if validator.stake == 0 {
                return Err(SolanaVerifierConfigError::ZeroStake);
            }
            if let Some(previous) = previous {
                if validator.pubkey == previous {
                    return Err(SolanaVerifierConfigError::DuplicateValidator);
                }
                if validator.pubkey < previous {
                    return Err(SolanaVerifierConfigError::UnsortedValidatorSet);
                }
            }
            previous = Some(validator.pubkey);
            total_stake = total_stake
                .checked_add(validator.stake)
                .ok_or(SolanaVerifierConfigError::StakeOverflow)?;
        }

        let required_stake =
            required_stake(total_stake, threshold_numerator, threshold_denominator)
                .ok_or(SolanaVerifierConfigError::StakeOverflow)?;
        let stake_table_digest = solana_stake_table_digest(&validators);

        Ok(Self {
            cluster,
            validators,
            total_stake,
            required_stake,
            stake_table_digest,
            minimum_finalized_slot,
        })
    }

    /// A verifier with no trusted stake table. Every proof fails closed.
    pub fn empty(cluster: SolanaCluster) -> Self {
        Self {
            cluster,
            validators: Vec::new(),
            total_stake: 0,
            required_stake: 1,
            stake_table_digest: [0u8; 32],
            minimum_finalized_slot: 0,
        }
    }

    pub fn cluster(&self) -> SolanaCluster {
        self.cluster
    }

    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }

    pub fn required_stake(&self) -> u64 {
        self.required_stake
    }

    pub fn stake_table_digest(&self) -> [u8; 32] {
        self.stake_table_digest
    }

    /// Verify with an explicit persisted finalized-slot floor.
    ///
    /// The runtime must pass the highest finalized slot it has durably accepted
    /// for this cluster. The verifier remains stateless so the same proof and
    /// floor are deterministic on every node and after restart.
    pub fn verify_with_finalized_floor(
        &self,
        proof: &ProofEnvelope,
        finalized_floor: u64,
    ) -> Result<VerificationOutcome, VerificationError> {
        if proof.strategy != VerificationStrategy::SolanaFinalizedProof {
            return Err(VerificationError::InvalidStrategy);
        }
        if proof.source_chain != ChainKind::Solana || proof.destination_chain != ChainKind::X3 {
            return Err(VerificationError::UnsupportedChain);
        }
        if self.validators.is_empty()
            || self.total_stake == 0
            || self.required_stake == 0
            || self.required_stake > self.total_stake
        {
            return Err(VerificationError::NoAuthorizedValidators);
        }

        let decoded = decode_v3(proof)?;
        if decoded.cluster != self.cluster {
            return Err(VerificationError::WrongCluster);
        }
        if decoded.parent_slot >= decoded.finalized_slot {
            return Err(VerificationError::MalformedProof);
        }
        if decoded.stake_table_digest != self.stake_table_digest {
            return Err(VerificationError::StakeTableMismatch);
        }

        let floor = core::cmp::max(finalized_floor, self.minimum_finalized_slot);
        if decoded.finalized_slot < floor {
            return Err(VerificationError::StaleFinality);
        }

        for vote in decoded.votes() {
            if find_validator(&self.validators, &vote.pubkey).is_none() {
                return Err(VerificationError::UnauthorizedSvmValidator);
            }
        }

        let message = solana_finality_message_v3(
            decoded.cluster,
            decoded.finalized_slot,
            decoded.parent_slot,
            &decoded.finalized_blockhash,
            &decoded.finalized_bank_hash,
            &decoded.parent_bank_hash,
            &decoded.stake_table_digest,
            proof,
        )?;

        let mut counted = BTreeSet::new();
        let mut valid_stake = 0u64;
        for vote in decoded.votes() {
            if !counted.insert(vote.pubkey) {
                continue;
            }
            let validator = find_validator(&self.validators, &vote.pubkey)
                .ok_or(VerificationError::UnauthorizedSvmValidator)?;
            if verify_ed25519(&vote.pubkey, &vote.signature, &message) {
                valid_stake = valid_stake
                    .checked_add(validator.stake)
                    .ok_or(VerificationError::StakeWeightOverflow)?;
            }
        }

        if valid_stake < self.required_stake {
            return Err(VerificationError::InsufficientStake);
        }

        Ok(VerificationOutcome {
            accepted: true,
            reason: "solana_stake_weighted_bank_state_verified",
            verified_at_height: Some(decoded.finalized_slot),
        })
    }
}

impl Verifier for SolanaStakeWeightedVerifier {
    fn strategy(&self) -> VerificationStrategy {
        VerificationStrategy::SolanaFinalizedProof
    }

    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        self.verify_with_finalized_floor(proof, 0)
    }
}

/// Canonical digest of a sorted validator stake table.
pub fn solana_stake_table_digest(validators: &[SolanaValidatorStake]) -> [u8; 32] {
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(STAKE_TABLE_DOMAIN);
    hasher.update((validators.len() as u32).to_le_bytes());
    for validator in validators {
        hasher.update(validator.pubkey);
        hasher.update(validator.stake.to_le_bytes());
    }
    digest(hasher)
}

/// Encode a V3 proof. Validators must already be sorted by public key.
#[allow(clippy::too_many_arguments)]
pub fn encode_solana_finality_v3(
    cluster: SolanaCluster,
    finalized_slot: u64,
    parent_slot: u64,
    finalized_blockhash: [u8; 32],
    finalized_bank_hash: [u8; 32],
    parent_bank_hash: [u8; 32],
    stake_table_digest: [u8; 32],
    votes: &[([u8; 32], [u8; 64])],
) -> Result<Vec<u8>, VerificationError> {
    if votes.len() > MAX_SOLANA_FINALITY_VOTES {
        return Err(VerificationError::MalformedProof);
    }
    let body_len = votes
        .len()
        .checked_mul(SOLANA_FINALITY_VOTE_LEN)
        .ok_or(VerificationError::MalformedProof)?;
    let payload_len = SOLANA_FINALITY_V3_HEADER_LEN
        .checked_add(body_len)
        .ok_or(VerificationError::MalformedProof)?;

    let mut payload = Vec::with_capacity(payload_len);
    payload.push(SOLANA_FINALIZED_FORMAT_V3);
    payload.push(cluster as u8);
    payload.extend_from_slice(&finalized_slot.to_le_bytes());
    payload.extend_from_slice(&parent_slot.to_le_bytes());
    payload.extend_from_slice(&finalized_blockhash);
    payload.extend_from_slice(&finalized_bank_hash);
    payload.extend_from_slice(&parent_bank_hash);
    payload.extend_from_slice(&stake_table_digest);
    payload.extend_from_slice(&(votes.len() as u32).to_le_bytes());
    for (pubkey, signature) in votes {
        payload.extend_from_slice(pubkey);
        payload.extend_from_slice(signature);
    }
    Ok(payload)
}

#[derive(Debug, Clone, Copy)]
struct SolanaVote {
    pubkey: [u8; 32],
    signature: [u8; 64],
}

#[derive(Debug)]
struct DecodedSolanaFinalityV3<'a> {
    cluster: SolanaCluster,
    finalized_slot: u64,
    parent_slot: u64,
    finalized_blockhash: [u8; 32],
    finalized_bank_hash: [u8; 32],
    parent_bank_hash: [u8; 32],
    stake_table_digest: [u8; 32],
    votes: &'a [u8],
}

fn decode_v3(proof: &ProofEnvelope) -> Result<DecodedSolanaFinalityV3<'_>, VerificationError> {
    let payload = proof.payload.as_slice();
    if payload.len() < SOLANA_FINALITY_V3_HEADER_LEN {
        return Err(VerificationError::MalformedProof);
    }
    if payload[0] != SOLANA_FINALIZED_FORMAT_V3 {
        return Err(VerificationError::UnsupportedProofFormat);
    }

    let cluster = SolanaCluster::decode(payload[1])?;
    let finalized_slot = u64::from_le_bytes(
        payload[2..10]
            .try_into()
            .map_err(|_| VerificationError::MalformedProof)?,
    );
    let parent_slot = u64::from_le_bytes(
        payload[10..18]
            .try_into()
            .map_err(|_| VerificationError::MalformedProof)?,
    );

    let mut finalized_blockhash = [0u8; 32];
    finalized_blockhash.copy_from_slice(&payload[18..50]);
    let mut finalized_bank_hash = [0u8; 32];
    finalized_bank_hash.copy_from_slice(&payload[50..82]);
    let mut parent_bank_hash = [0u8; 32];
    parent_bank_hash.copy_from_slice(&payload[82..114]);
    let mut stake_table_digest = [0u8; 32];
    stake_table_digest.copy_from_slice(&payload[114..146]);

    let count = u32::from_le_bytes(
        payload[146..150]
            .try_into()
            .map_err(|_| VerificationError::MalformedProof)?,
    ) as usize;
    if count == 0 || count > MAX_SOLANA_FINALITY_VOTES {
        return Err(VerificationError::MalformedProof);
    }
    let vote_bytes = count
        .checked_mul(SOLANA_FINALITY_VOTE_LEN)
        .ok_or(VerificationError::MalformedProof)?;
    let expected_len = SOLANA_FINALITY_V3_HEADER_LEN
        .checked_add(vote_bytes)
        .ok_or(VerificationError::MalformedProof)?;
    if payload.len() != expected_len {
        return Err(VerificationError::MalformedProof);
    }

    Ok(DecodedSolanaFinalityV3 {
        cluster,
        finalized_slot,
        parent_slot,
        finalized_blockhash,
        finalized_bank_hash,
        parent_bank_hash,
        stake_table_digest,
        votes: &payload[SOLANA_FINALITY_V3_HEADER_LEN..],
    })
}

impl DecodedSolanaFinalityV3<'_> {
    fn votes(&self) -> impl Iterator<Item = SolanaVote> + '_ {
        self.votes
            .chunks_exact(SOLANA_FINALITY_VOTE_LEN)
            .map(|vote| {
                let mut pubkey = [0u8; 32];
                pubkey.copy_from_slice(&vote[..32]);
                let mut signature = [0u8; 64];
                signature.copy_from_slice(&vote[32..]);
                SolanaVote { pubkey, signature }
            })
    }
}

fn required_stake(total: u64, numerator: u32, denominator: u32) -> Option<u64> {
    if denominator == 0 || numerator == 0 || numerator > denominator {
        return None;
    }
    let product = u128::from(total).checked_mul(u128::from(numerator))?;
    let denominator = u128::from(denominator);
    let rounded_up = product.checked_add(denominator.checked_sub(1)?)? / denominator;
    u64::try_from(rounded_up).ok()
}

fn find_validator<'a>(
    validators: &'a [SolanaValidatorStake],
    pubkey: &[u8; 32],
) -> Option<&'a SolanaValidatorStake> {
    validators
        .binary_search_by_key(pubkey, |validator| validator.pubkey)
        .ok()
        .map(|index| &validators[index])
}

fn verify_ed25519(pubkey: &[u8; 32], signature: &[u8; 64], message: &[u8; 32]) -> bool {
    ed25519_dalek::VerifyingKey::from_bytes(pubkey)
        .ok()
        .and_then(|key| {
            ed25519_dalek::Signature::from_slice(signature)
                .ok()
                .map(|signature| key.verify_strict(message, &signature).is_ok())
        })
        .unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
pub fn solana_finality_message_v3(
    cluster: SolanaCluster,
    finalized_slot: u64,
    parent_slot: u64,
    finalized_blockhash: &[u8; 32],
    finalized_bank_hash: &[u8; 32],
    parent_bank_hash: &[u8; 32],
    stake_table_digest: &[u8; 32],
    proof: &ProofEnvelope,
) -> Result<[u8; 32], VerificationError> {
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(FINALITY_DOMAIN);
    hasher.update(proof.proof_id);
    hasher.update([cluster as u8]);
    hasher.update(finalized_slot.to_le_bytes());
    hasher.update(parent_slot.to_le_bytes());
    hasher.update(finalized_blockhash);
    hasher.update(finalized_bank_hash);
    hasher.update(parent_bank_hash);
    hasher.update(stake_table_digest);

    hasher.update(TRANSFER_DOMAIN);
    hasher.update(proof.expected_asset_id);
    hasher.update(proof.expected_amount.to_le_bytes());
    update_vec(&mut hasher, &proof.expected_sender)?;
    update_vec(&mut hasher, &proof.expected_recipient)?;
    update_vec(&mut hasher, &proof.expected_beneficiary)?;
    Ok(digest(hasher))
}

fn update_vec(hasher: &mut Blake2b<U32>, value: &[u8]) -> Result<(), VerificationError> {
    let len = u32::try_from(value.len()).map_err(|_| VerificationError::MalformedProof)?;
    hasher.update(len.to_le_bytes());
    hasher.update(value);
    Ok(())
}

fn digest(hasher: Blake2b<U32>) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VerificationRouter;
    use alloc::sync::Arc;
    use alloc::vec;
    use ed25519_dalek::Signer;

    fn signing_key(seed: u8) -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
    }

    fn stakes(keys: &[&ed25519_dalek::SigningKey], weights: &[u64]) -> Vec<SolanaValidatorStake> {
        let mut entries = keys
            .iter()
            .zip(weights)
            .map(|(key, stake)| SolanaValidatorStake {
                pubkey: key.verifying_key().to_bytes(),
                stake: *stake,
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.pubkey);
        entries
    }

    struct ProofInput<'a> {
        cluster: SolanaCluster,
        finalized_slot: u64,
        parent_slot: u64,
        finalized_blockhash: [u8; 32],
        finalized_bank_hash: [u8; 32],
        parent_bank_hash: [u8; 32],
        stake_table_digest: [u8; 32],
        keys: &'a [&'a ed25519_dalek::SigningKey],
        sender: &'a [u8],
        recipient: &'a [u8],
    }

    fn proof(input: ProofInput<'_>) -> ProofEnvelope {
        let mut envelope = ProofEnvelope {
            proof_id: [7u8; 32],
            strategy: VerificationStrategy::SolanaFinalizedProof,
            source_chain: ChainKind::Solana,
            destination_chain: ChainKind::X3,
            payload: Vec::new(),
            expected_asset_id: [4u8; 32],
            expected_amount: 5_000,
            expected_sender: input.sender.to_vec(),
            expected_recipient: input.recipient.to_vec(),
            expected_beneficiary: vec![9u8; 20],
        };
        let message = solana_finality_message_v3(
            input.cluster,
            input.finalized_slot,
            input.parent_slot,
            &input.finalized_blockhash,
            &input.finalized_bank_hash,
            &input.parent_bank_hash,
            &input.stake_table_digest,
            &envelope,
        )
        .expect("test intent has bounded fields");
        let votes = input
            .keys
            .iter()
            .map(|key| {
                (
                    key.verifying_key().to_bytes(),
                    key.sign(&message).to_bytes(),
                )
            })
            .collect::<Vec<_>>();
        envelope.payload = encode_solana_finality_v3(
            input.cluster,
            input.finalized_slot,
            input.parent_slot,
            input.finalized_blockhash,
            input.finalized_bank_hash,
            input.parent_bank_hash,
            input.stake_table_digest,
            &votes,
        )
        .expect("test proof fits");
        envelope
    }

    #[test]
    fn supermajority_stake_accepts_bound_bank_hash() {
        let (a, b, c) = (signing_key(1), signing_key(2), signing_key(3));
        let entries = stakes(&[&a, &b, &c], &[50, 30, 20]);
        let verifier =
            SolanaStakeWeightedVerifier::new(SolanaCluster::MainnetBeta, entries.clone(), 2, 3, 0)
                .expect("valid stake table");
        let envelope = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a, &b],
            sender: b"solana-sender",
            recipient: b"x3-recipient",
        });

        let outcome = verifier
            .verify_with_finalized_floor(&envelope, 390)
            .expect("80 of 100 stake must satisfy two thirds");
        assert_eq!(outcome.verified_at_height, Some(400));
        assert_eq!(outcome.reason, "solana_stake_weighted_bank_state_verified");
    }

    #[test]
    fn router_dispatches_the_stake_weighted_verifier() {
        let (a, b) = (signing_key(1), signing_key(2));
        let verifier = SolanaStakeWeightedVerifier::new(
            SolanaCluster::MainnetBeta,
            stakes(&[&a, &b], &[50, 50]),
            2,
            3,
            0,
        )
        .expect("valid stake table");
        let envelope = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a, &b],
            sender: b"sender",
            recipient: b"recipient",
        });
        let mut router = VerificationRouter::new();
        router.register_verifier(Arc::new(verifier));

        assert!(router.route(&envelope).expect("router dispatch").accepted);
    }

    #[test]
    fn insufficient_stake_is_rejected_even_with_multiple_signers() {
        let (a, b, c) = (signing_key(1), signing_key(2), signing_key(3));
        let entries = stakes(&[&a, &b, &c], &[70, 20, 10]);
        let verifier =
            SolanaStakeWeightedVerifier::new(SolanaCluster::MainnetBeta, entries, 2, 3, 0)
                .expect("valid stake table");
        let envelope = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&b, &c],
            sender: b"sender",
            recipient: b"recipient",
        });

        assert_eq!(
            verifier.verify_with_finalized_floor(&envelope, 0),
            Err(VerificationError::InsufficientStake)
        );
    }

    #[test]
    fn duplicate_vote_counts_stake_once() {
        let (a, b) = (signing_key(1), signing_key(2));
        let entries = stakes(&[&a, &b], &[60, 40]);
        let verifier =
            SolanaStakeWeightedVerifier::new(SolanaCluster::MainnetBeta, entries, 2, 3, 0)
                .expect("valid stake table");
        let mut envelope = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a],
            sender: b"sender",
            recipient: b"recipient",
        });
        let decoded = decode_v3(&envelope).expect("well formed proof");
        let vote = decoded.votes().next().expect("one vote");
        let duplicated = encode_solana_finality_v3(
            SolanaCluster::MainnetBeta,
            400,
            399,
            [6u8; 32],
            [8u8; 32],
            [7u8; 32],
            verifier.stake_table_digest(),
            &[(vote.pubkey, vote.signature), (vote.pubkey, vote.signature)],
        )
        .expect("fits");
        envelope.payload = duplicated;

        assert_eq!(
            verifier.verify_with_finalized_floor(&envelope, 0),
            Err(VerificationError::InsufficientStake)
        );
    }

    #[test]
    fn wrong_cluster_is_rejected() {
        let (a, b) = (signing_key(1), signing_key(2));
        let verifier = SolanaStakeWeightedVerifier::new(
            SolanaCluster::MainnetBeta,
            stakes(&[&a, &b], &[50, 50]),
            2,
            3,
            0,
        )
        .expect("valid stake table");
        let devnet_envelope = proof(ProofInput {
            cluster: SolanaCluster::Devnet,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a, &b],
            sender: b"sender",
            recipient: b"recipient",
        });
        assert_eq!(
            verifier.verify_with_finalized_floor(&devnet_envelope, 0),
            Err(VerificationError::WrongCluster)
        );
    }

    #[test]
    fn stale_finalized_slot_is_rejected_by_explicit_floor() {
        let (a, b) = (signing_key(1), signing_key(2));
        let verifier = SolanaStakeWeightedVerifier::new(
            SolanaCluster::MainnetBeta,
            stakes(&[&a, &b], &[50, 50]),
            2,
            3,
            0,
        )
        .expect("valid stake table");
        let envelope = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a, &b],
            sender: b"sender",
            recipient: b"recipient",
        });
        assert_eq!(
            verifier.verify_with_finalized_floor(&envelope, 401),
            Err(VerificationError::StaleFinality)
        );
    }

    #[test]
    fn tampered_blockhash_bank_hash_parent_and_stake_digest_are_rejected() {
        let (a, b) = (signing_key(1), signing_key(2));
        let verifier = SolanaStakeWeightedVerifier::new(
            SolanaCluster::MainnetBeta,
            stakes(&[&a, &b], &[50, 50]),
            2,
            3,
            0,
        )
        .expect("valid stake table");
        let valid = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a, &b],
            sender: b"sender",
            recipient: b"recipient",
        });

        let mut blockhash_tampered = valid.clone();
        blockhash_tampered.payload[18] ^= 1;
        assert_eq!(
            verifier.verify_with_finalized_floor(&blockhash_tampered, 0),
            Err(VerificationError::InsufficientStake)
        );

        let mut bank_hash_tampered = valid.clone();
        bank_hash_tampered.payload[50] ^= 1;
        assert_eq!(
            verifier.verify_with_finalized_floor(&bank_hash_tampered, 0),
            Err(VerificationError::InsufficientStake)
        );

        let mut parent_tampered = valid.clone();
        parent_tampered.payload[82] ^= 1;
        assert_eq!(
            verifier.verify_with_finalized_floor(&parent_tampered, 0),
            Err(VerificationError::InsufficientStake)
        );

        let mut digest_tampered = valid;
        digest_tampered.payload[114] ^= 1;
        assert_eq!(
            verifier.verify_with_finalized_floor(&digest_tampered, 0),
            Err(VerificationError::StakeTableMismatch)
        );
    }

    #[test]
    fn unauthorized_vote_key_is_rejected() {
        let (a, rogue) = (signing_key(1), signing_key(9));
        let verifier = SolanaStakeWeightedVerifier::new(
            SolanaCluster::MainnetBeta,
            stakes(&[&a], &[100]),
            2,
            3,
            0,
        )
        .expect("valid stake table");
        let envelope = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a, &rogue],
            sender: b"sender",
            recipient: b"recipient",
        });
        assert_eq!(
            verifier.verify_with_finalized_floor(&envelope, 0),
            Err(VerificationError::UnauthorizedSvmValidator)
        );
    }

    #[test]
    fn changing_transfer_fields_breaks_the_stake_signatures() {
        let (a, b) = (signing_key(1), signing_key(2));
        let entries = stakes(&[&a, &b], &[50, 50]);
        let verifier =
            SolanaStakeWeightedVerifier::new(SolanaCluster::MainnetBeta, entries, 2, 3, 0)
                .expect("valid stake table");
        let mut envelope = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a, &b],
            sender: b"sender",
            recipient: b"recipient",
        });
        envelope.expected_amount += 1;

        assert_eq!(
            verifier.verify_with_finalized_floor(&envelope, 0),
            Err(VerificationError::InsufficientStake)
        );

        let mut proof_id_changed = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: verifier.stake_table_digest(),
            keys: &[&a, &b],
            sender: b"sender",
            recipient: b"recipient",
        });
        proof_id_changed.proof_id = [8u8; 32];
        assert_eq!(
            verifier.verify_with_finalized_floor(&proof_id_changed, 0),
            Err(VerificationError::InsufficientStake)
        );
    }

    #[test]
    fn malformed_and_unconfigured_proofs_fail_closed() {
        let a = signing_key(1);
        let unconfigured = SolanaStakeWeightedVerifier::empty(SolanaCluster::MainnetBeta);
        let mut envelope = proof(ProofInput {
            cluster: SolanaCluster::MainnetBeta,
            finalized_slot: 400,
            parent_slot: 399,
            finalized_blockhash: [6u8; 32],
            finalized_bank_hash: [8u8; 32],
            parent_bank_hash: [7u8; 32],
            stake_table_digest: [0u8; 32],
            keys: &[&a],
            sender: b"sender",
            recipient: b"recipient",
        });
        assert_eq!(
            unconfigured.verify(&envelope),
            Err(VerificationError::NoAuthorizedValidators)
        );

        let configured = SolanaStakeWeightedVerifier::new(
            SolanaCluster::MainnetBeta,
            stakes(&[&a], &[100]),
            2,
            3,
            0,
        )
        .expect("valid stake table");
        envelope.payload.truncate(envelope.payload.len() - 1);
        assert_eq!(
            configured.verify(&envelope),
            Err(VerificationError::MalformedProof)
        );
    }
}
