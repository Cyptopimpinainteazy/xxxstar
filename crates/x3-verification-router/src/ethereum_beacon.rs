//! Native Ethereum beacon-chain light-client verification.
//!
//! This verifies the consensus-spec sync-committee update, finality branch,
//! next-committee proof, domain-separated BLS signature, and execution payload
//! branch. SSZ behavior follows the Snowbridge Apache-2.0 beacon primitives.

use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode};
use primitive_types::H256;
use scale_info::TypeInfo;
use ssz_rs::{
    prelude::{List, Vector},
    Bitvector, Deserialize as SszDeserialize,
};

use self::ssz_types::{
    SSZBeaconBlockHeader, SSZExecutionPayloadHeader, SSZExecutionPayloadHeaderDeneb, SSZForkData,
    SSZSigningData, SSZSyncCommittee,
};

pub const SLOTS_PER_EPOCH: u64 = 32;
pub const EPOCHS_PER_SYNC_COMMITTEE_PERIOD: u64 = 256;
pub const SYNC_COMMITTEE_SIZE: usize = 512;
pub const SYNC_COMMITTEE_BITS_SIZE: usize = SYNC_COMMITTEE_SIZE / 8;
pub const SLOTS_PER_HISTORICAL_ROOT: usize = 8192;
pub const BLOCK_ROOT_AT_INDEX_DEPTH: usize = 13;
pub const DOMAIN_SYNC_COMMITTEE: [u8; 4] = [7, 0, 0, 0];

const PUBKEY_SIZE: usize = 48;
const SIGNATURE_SIZE: usize = 96;
const MAX_EXTRA_DATA_BYTES: usize = 32;
const MAX_LOGS_BLOOM_SIZE: usize = 256;
const FEE_RECIPIENT_SIZE: usize = 20;

const ALTAIR_BLOCK_ROOTS_INDEX: usize = 37;
const ALTAIR_FINALIZED_ROOT_INDEX: usize = 105;
const ALTAIR_CURRENT_SYNC_COMMITTEE_INDEX: usize = 54;
const ALTAIR_NEXT_SYNC_COMMITTEE_INDEX: usize = 55;
const EXECUTION_HEADER_INDEX: usize = 25;
const ELECTRA_BLOCK_ROOTS_INDEX: usize = 69;
const ELECTRA_FINALIZED_ROOT_INDEX: usize = 169;
const ELECTRA_CURRENT_SYNC_COMMITTEE_INDEX: usize = 86;
const ELECTRA_NEXT_SYNC_COMMITTEE_INDEX: usize = 87;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct Fork {
    pub version: [u8; 4],
    pub epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct ForkVersions {
    pub genesis: Fork,
    pub altair: Fork,
    pub bellatrix: Fork,
    pub capella: Fork,
    pub deneb: Fork,
    pub electra: Fork,
    pub fulu: Fork,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumBeaconHeader {
    pub slot: u64,
    pub proposer_index: u64,
    pub parent_root: H256,
    pub state_root: H256,
    pub body_root: H256,
}

impl EthereumBeaconHeader {
    pub fn hash_tree_root(&self) -> Result<H256, EthereumBeaconError> {
        hash_tree_root(SSZBeaconBlockHeader {
            slot: self.slot,
            proposer_index: self.proposer_index,
            parent_root: self.parent_root.to_fixed_bytes(),
            state_root: self.state_root.to_fixed_bytes(),
            body_root: self.body_root.to_fixed_bytes(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumSyncCommittee {
    pub pubkeys: Vec<[u8; PUBKEY_SIZE]>,
    pub aggregate_pubkey: [u8; PUBKEY_SIZE],
}

impl EthereumSyncCommittee {
    fn hash_tree_root(&self) -> Result<H256, EthereumBeaconError> {
        if self.pubkeys.len() != SYNC_COMMITTEE_SIZE {
            return Err(EthereumBeaconError::SyncCommitteeRoot);
        }
        let pubkeys = self
            .pubkeys
            .iter()
            .map(|key| Vector::<u8, PUBKEY_SIZE>::try_from(key.to_vec()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| EthereumBeaconError::SyncCommitteeRoot)?;
        let pubkeys = Vector::<Vector<u8, PUBKEY_SIZE>, SYNC_COMMITTEE_SIZE>::try_from(pubkeys)
            .map_err(|_| EthereumBeaconError::SyncCommitteeRoot)?;
        let aggregate_pubkey = Vector::<u8, PUBKEY_SIZE>::try_from(self.aggregate_pubkey.to_vec())
            .map_err(|_| EthereumBeaconError::SyncCommitteeRoot)?;
        hash_tree_root(SSZSyncCommittee {
            pubkeys,
            aggregate_pubkey,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumSyncAggregate {
    pub sync_committee_bits: [u8; SYNC_COMMITTEE_BITS_SIZE],
    pub sync_committee_signature: [u8; SIGNATURE_SIZE],
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumNextSyncCommitteeUpdate {
    pub next_sync_committee: EthereumSyncCommittee,
    pub next_sync_committee_branch: Vec<H256>,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumCheckpointUpdate {
    pub header: EthereumBeaconHeader,
    pub current_sync_committee: EthereumSyncCommittee,
    pub current_sync_committee_branch: Vec<H256>,
    pub validators_root: H256,
    pub block_roots_root: H256,
    pub block_roots_branch: Vec<H256>,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumUpdate {
    pub attested_header: EthereumBeaconHeader,
    pub sync_aggregate: EthereumSyncAggregate,
    pub signature_slot: u64,
    pub next_sync_committee_update: Option<EthereumNextSyncCommitteeUpdate>,
    pub finalized_header: EthereumBeaconHeader,
    pub finality_branch: Vec<H256>,
    pub block_roots_root: H256,
    pub block_roots_branch: Vec<H256>,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumExecutionPayloadHeader {
    pub parent_hash: H256,
    pub fee_recipient: [u8; FEE_RECIPIENT_SIZE],
    pub state_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Vec<u8>,
    pub prev_randao: H256,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub base_fee_per_gas: [u8; 32],
    pub block_hash: H256,
    pub transactions_root: H256,
    pub withdrawals_root: H256,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum EthereumExecutionHeader {
    Capella(EthereumExecutionPayloadHeader),
    Deneb(EthereumExecutionPayloadHeader),
}

impl EthereumExecutionHeader {
    fn hash_tree_root(&self) -> Result<H256, EthereumBeaconError> {
        let (header, deneb) = match self {
            EthereumExecutionHeader::Capella(header) => (header, false),
            EthereumExecutionHeader::Deneb(header) => (header, true),
        };
        let base = ssz_execution_header(header)?;
        if deneb {
            hash_tree_root(SSZExecutionPayloadHeaderDeneb {
                parent_hash: base.parent_hash,
                fee_recipient: base.fee_recipient,
                state_root: base.state_root,
                receipts_root: base.receipts_root,
                logs_bloom: base.logs_bloom,
                prev_randao: base.prev_randao,
                block_number: base.block_number,
                gas_limit: base.gas_limit,
                gas_used: base.gas_used,
                timestamp: base.timestamp,
                extra_data: base.extra_data,
                base_fee_per_gas: base.base_fee_per_gas,
                block_hash: base.block_hash,
                transactions_root: base.transactions_root,
                withdrawals_root: base.withdrawals_root,
                blob_gas_used: header.blob_gas_used,
                excess_blob_gas: header.excess_blob_gas,
            })
        } else {
            hash_tree_root(base)
        }
    }

    fn block_number(&self) -> u64 {
        match self {
            EthereumExecutionHeader::Capella(header) => header.block_number,
            EthereumExecutionHeader::Deneb(header) => header.block_number,
        }
    }

    fn block_hash(&self) -> H256 {
        match self {
            EthereumExecutionHeader::Capella(header) => header.block_hash,
            EthereumExecutionHeader::Deneb(header) => header.block_hash,
        }
    }

    fn state_root(&self) -> H256 {
        match self {
            EthereumExecutionHeader::Capella(header) => header.state_root,
            EthereumExecutionHeader::Deneb(header) => header.state_root,
        }
    }

    fn receipts_root(&self) -> H256 {
        match self {
            EthereumExecutionHeader::Capella(header) => header.receipts_root,
            EthereumExecutionHeader::Deneb(header) => header.receipts_root,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumAncestryProof {
    pub header_branch: Vec<H256>,
    pub finalized_block_root: H256,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumExecutionProof {
    pub header: EthereumBeaconHeader,
    pub ancestry_proof: Option<EthereumAncestryProof>,
    pub execution_header: EthereumExecutionHeader,
    pub execution_branch: Vec<H256>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthereumBeaconError {
    InvalidSyncCommitteeProof,
    InvalidNextSyncCommitteeProof,
    InvalidBlockRootsProof,
    InvalidFinalityProof,
    InvalidAncestryProof,
    InvalidExecutionProof,
    ParticipationBelowSupermajority,
    InvalidUpdateOrder,
    SkippedSyncCommitteePeriod,
    IrrelevantUpdate,
    InvalidFinalizedHeaderGap,
    SyncCommitteeUpdateRequired,
    InvalidSyncCommitteeUpdate,
    HeaderNotFinalized,
    ExpectedFinalizedHeaderNotStored,
    HeaderRoot,
    ExecutionHeaderRoot,
    SyncCommitteeRoot,
    ForkDataRoot,
    SigningRoot,
    InvalidBlsPublicKey,
    BlsVerificationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumBeaconState {
    pub finalized_header: EthereumBeaconHeader,
    pub block_roots_root: H256,
    pub validators_root: H256,
    pub current_sync_committee: EthereumSyncCommittee,
    pub next_sync_committee: Option<EthereumSyncCommittee>,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EthereumFinalizedCheckpoint {
    pub slot: u64,
    pub block_roots_root: H256,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedEthereumExecutionHeader {
    pub beacon_slot: u64,
    pub block_number: u64,
    pub block_hash: H256,
    pub state_root: H256,
    pub receipts_root: H256,
}

pub fn initialize_ethereum_beacon(
    checkpoint: &EthereumCheckpointUpdate,
    fork_versions: &ForkVersions,
) -> Result<EthereumBeaconState, EthereumBeaconError> {
    let committee_root = checkpoint.current_sync_committee.hash_tree_root()?;
    let committee_gindex =
        current_sync_committee_gindex_at_slot(checkpoint.header.slot, fork_versions);
    if !verify_merkle_branch(
        committee_root,
        &checkpoint.current_sync_committee_branch,
        subtree_index(committee_gindex),
        generalized_index_length(committee_gindex),
        checkpoint.header.state_root,
    ) {
        return Err(EthereumBeaconError::InvalidSyncCommitteeProof);
    }
    let block_roots_gindex = block_roots_gindex_at_slot(checkpoint.header.slot, fork_versions);
    if !verify_merkle_branch(
        checkpoint.block_roots_root,
        &checkpoint.block_roots_branch,
        subtree_index(block_roots_gindex),
        generalized_index_length(block_roots_gindex),
        checkpoint.header.state_root,
    ) {
        return Err(EthereumBeaconError::InvalidBlockRootsProof);
    }
    Ok(EthereumBeaconState {
        finalized_header: checkpoint.header.clone(),
        block_roots_root: checkpoint.block_roots_root,
        validators_root: checkpoint.validators_root,
        current_sync_committee: checkpoint.current_sync_committee.clone(),
        next_sync_committee: None,
    })
}

pub fn apply_ethereum_finality_update(
    state: &EthereumBeaconState,
    update: &EthereumUpdate,
    fork_versions: &ForkVersions,
) -> Result<EthereumBeaconState, EthereumBeaconError> {
    let participation: [u8; SYNC_COMMITTEE_SIZE] =
        decompress_bits(update.sync_aggregate.sync_committee_bits);
    if !sync_committee_participation_is_supermajority(&participation) {
        return Err(EthereumBeaconError::ParticipationBelowSupermajority);
    }
    if update.signature_slot <= update.attested_header.slot
        || update.attested_header.slot < update.finalized_header.slot
    {
        return Err(EthereumBeaconError::InvalidUpdateOrder);
    }

    let store_period = compute_period(state.finalized_header.slot);
    let signature_period = compute_period(update.signature_slot);
    if state.next_sync_committee.is_some() {
        if !(store_period..=store_period.saturating_add(1)).contains(&signature_period) {
            return Err(EthereumBeaconError::SkippedSyncCommitteePeriod);
        }
    } else if signature_period != store_period {
        return Err(EthereumBeaconError::SkippedSyncCommitteePeriod);
    }

    let attested_period = compute_period(update.attested_header.slot);
    let finalized_period = compute_period(update.finalized_header.slot);
    let has_next_committee = state.next_sync_committee.is_none()
        && update.next_sync_committee_update.is_some()
        && attested_period == store_period;
    if update.attested_header.slot <= state.finalized_header.slot && !has_next_committee {
        return Err(EthereumBeaconError::IrrelevantUpdate);
    }
    if state
        .finalized_header
        .slot
        .saturating_add(SLOTS_PER_HISTORICAL_ROOT as u64)
        < update.finalized_header.slot
    {
        return Err(EthereumBeaconError::InvalidFinalizedHeaderGap);
    }

    let finalized_root = update.finalized_header.hash_tree_root()?;
    let finalized_gindex =
        finalized_root_gindex_at_slot(update.attested_header.slot, fork_versions);
    if !verify_merkle_branch(
        finalized_root,
        &update.finality_branch,
        subtree_index(finalized_gindex),
        generalized_index_length(finalized_gindex),
        update.attested_header.state_root,
    ) {
        return Err(EthereumBeaconError::InvalidFinalityProof);
    }
    let block_roots_gindex =
        block_roots_gindex_at_slot(update.finalized_header.slot, fork_versions);
    if !verify_merkle_branch(
        update.block_roots_root,
        &update.block_roots_branch,
        subtree_index(block_roots_gindex),
        generalized_index_length(block_roots_gindex),
        update.finalized_header.state_root,
    ) {
        return Err(EthereumBeaconError::InvalidBlockRootsProof);
    }

    if let Some(next_update) = &update.next_sync_committee_update {
        let next_root = next_update.next_sync_committee.hash_tree_root()?;
        if attested_period == store_period {
            if let Some(existing) = &state.next_sync_committee {
                if existing.hash_tree_root()? != next_root {
                    return Err(EthereumBeaconError::InvalidSyncCommitteeUpdate);
                }
            }
        }
        let next_gindex =
            next_sync_committee_gindex_at_slot(update.attested_header.slot, fork_versions);
        if !verify_merkle_branch(
            next_root,
            &next_update.next_sync_committee_branch,
            subtree_index(next_gindex),
            generalized_index_length(next_gindex),
            update.attested_header.state_root,
        ) {
            return Err(EthereumBeaconError::InvalidNextSyncCommitteeProof);
        }
    } else if finalized_period != store_period {
        return Err(EthereumBeaconError::SyncCommitteeUpdateRequired);
    }

    let signing_committee = if signature_period == store_period {
        &state.current_sync_committee
    } else {
        state
            .next_sync_committee
            .as_ref()
            .ok_or(EthereumBeaconError::SkippedSyncCommitteePeriod)?
    };
    verify_sync_committee_signature(
        signing_committee,
        &participation,
        &update.sync_aggregate.sync_committee_signature,
        &signing_root(
            &update.attested_header,
            state.validators_root,
            update.signature_slot,
            fork_versions,
        )?,
    )?;

    let mut next_state = state.clone();
    if let Some(next_update) = &update.next_sync_committee_update {
        if state.next_sync_committee.is_none() {
            if finalized_period != store_period {
                return Err(EthereumBeaconError::InvalidSyncCommitteeUpdate);
            }
            next_state.next_sync_committee = Some(next_update.next_sync_committee.clone());
        } else if finalized_period == store_period.saturating_add(1) {
            next_state.current_sync_committee = state
                .next_sync_committee
                .clone()
                .ok_or(EthereumBeaconError::SkippedSyncCommitteePeriod)?;
            next_state.next_sync_committee = Some(next_update.next_sync_committee.clone());
        }
    }
    if update.finalized_header.slot > state.finalized_header.slot {
        next_state.finalized_header = update.finalized_header.clone();
        next_state.block_roots_root = update.block_roots_root;
    }
    Ok(next_state)
}

pub fn verify_ethereum_execution_proof<F>(
    trusted_root: H256,
    trusted: &EthereumFinalizedCheckpoint,
    proof: &EthereumExecutionProof,
    known_finalized: F,
) -> Result<VerifiedEthereumExecutionHeader, EthereumBeaconError>
where
    F: Fn(H256) -> Option<EthereumFinalizedCheckpoint>,
{
    if proof.header.slot > trusted.slot {
        return Err(EthereumBeaconError::HeaderNotFinalized);
    }
    let beacon_root = proof.header.hash_tree_root()?;
    match &proof.ancestry_proof {
        Some(ancestry) => {
            let anchor = if ancestry.finalized_block_root == trusted_root {
                trusted.clone()
            } else {
                known_finalized(ancestry.finalized_block_root)
                    .ok_or(EthereumBeaconError::ExpectedFinalizedHeaderNotStored)?
            };
            if proof.header.slot >= anchor.slot {
                return Err(EthereumBeaconError::HeaderNotFinalized);
            }
            let leaf_index = SLOTS_PER_HISTORICAL_ROOT as u64
                + proof.header.slot % SLOTS_PER_HISTORICAL_ROOT as u64;
            if !verify_merkle_branch(
                beacon_root,
                &ancestry.header_branch,
                leaf_index as usize,
                BLOCK_ROOT_AT_INDEX_DEPTH,
                anchor.block_roots_root,
            ) {
                return Err(EthereumBeaconError::InvalidAncestryProof);
            }
        }
        None => {
            let anchor = if beacon_root == trusted_root {
                trusted.clone()
            } else {
                known_finalized(beacon_root)
                    .ok_or(EthereumBeaconError::ExpectedFinalizedHeaderNotStored)?
            };
            if proof.header.slot != anchor.slot {
                return Err(EthereumBeaconError::ExpectedFinalizedHeaderNotStored);
            }
        }
    }

    let execution_root = proof.execution_header.hash_tree_root()?;
    if !verify_merkle_branch(
        execution_root,
        &proof.execution_branch,
        subtree_index(EXECUTION_HEADER_INDEX),
        generalized_index_length(EXECUTION_HEADER_INDEX),
        proof.header.body_root,
    ) {
        return Err(EthereumBeaconError::InvalidExecutionProof);
    }
    Ok(VerifiedEthereumExecutionHeader {
        beacon_slot: proof.header.slot,
        block_number: proof.execution_header.block_number(),
        block_hash: proof.execution_header.block_hash(),
        state_root: proof.execution_header.state_root(),
        receipts_root: proof.execution_header.receipts_root(),
    })
}

fn verify_sync_committee_signature(
    committee: &EthereumSyncCommittee,
    participation: &[u8],
    signature: &[u8; SIGNATURE_SIZE],
    message: &H256,
) -> Result<(), EthereumBeaconError> {
    use snowbridge_milagro_bls::{AggregatePublicKey, AggregateSignature, PublicKey, Signature};

    if committee.pubkeys.len() != SYNC_COMMITTEE_SIZE || participation.len() != SYNC_COMMITTEE_SIZE
    {
        return Err(EthereumBeaconError::InvalidBlsPublicKey);
    }
    let absent = participation
        .iter()
        .zip(committee.pubkeys.iter())
        .filter_map(|(participating, key)| {
            if *participating == 0 {
                Some(PublicKey::from_bytes_unchecked(key))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| EthereumBeaconError::InvalidBlsPublicKey)?;
    let aggregate = PublicKey::from_bytes_unchecked(&committee.aggregate_pubkey)
        .map_err(|_| EthereumBeaconError::InvalidBlsPublicKey)?;
    let mut signing_key = AggregatePublicKey::from_public_key(&aggregate);
    if !absent.is_empty() {
        let absent_aggregate = AggregatePublicKey::into_aggregate(&absent)
            .map_err(|_| EthereumBeaconError::InvalidBlsPublicKey)?;
        signing_key.point.sub(&absent_aggregate.point);
    }

    let signature =
        Signature::from_bytes(signature).map_err(|_| EthereumBeaconError::BlsVerificationFailed)?;
    let signature = AggregateSignature::from_signature(&signature);
    if signature.fast_aggregate_verify_pre_aggregated(&message[..], &signing_key) {
        Ok(())
    } else {
        Err(EthereumBeaconError::BlsVerificationFailed)
    }
}

fn signing_root(
    header: &EthereumBeaconHeader,
    validators_root: H256,
    signature_slot: u64,
    fork_versions: &ForkVersions,
) -> Result<H256, EthereumBeaconError> {
    let fork_version_slot = signature_slot.saturating_sub(1);
    let fork_version = select_fork_version(fork_versions, compute_epoch(fork_version_slot));
    let fork_data_root = hash_tree_root(SSZForkData {
        current_version: fork_version,
        genesis_validators_root: validators_root.to_fixed_bytes(),
    })?;
    let mut domain = [0u8; 32];
    domain[..4].copy_from_slice(&DOMAIN_SYNC_COMMITTEE);
    domain[4..].copy_from_slice(&fork_data_root.to_fixed_bytes()[..28]);
    hash_tree_root(SSZSigningData {
        object_root: header.hash_tree_root()?.to_fixed_bytes(),
        domain,
    })
}

fn ssz_execution_header(
    header: &EthereumExecutionPayloadHeader,
) -> Result<SSZExecutionPayloadHeader, EthereumBeaconError> {
    use ssz_rs::U256;

    Ok(SSZExecutionPayloadHeader {
        parent_hash: header.parent_hash.to_fixed_bytes(),
        fee_recipient: Vector::<u8, FEE_RECIPIENT_SIZE>::try_from(header.fee_recipient.to_vec())
            .map_err(|_| EthereumBeaconError::ExecutionHeaderRoot)?,
        state_root: header.state_root.to_fixed_bytes(),
        receipts_root: header.receipts_root.to_fixed_bytes(),
        logs_bloom: Vector::<u8, MAX_LOGS_BLOOM_SIZE>::try_from(header.logs_bloom.clone())
            .map_err(|_| EthereumBeaconError::ExecutionHeaderRoot)?,
        prev_randao: header.prev_randao.to_fixed_bytes(),
        block_number: header.block_number,
        gas_limit: header.gas_limit,
        gas_used: header.gas_used,
        timestamp: header.timestamp,
        extra_data: List::<u8, MAX_EXTRA_DATA_BYTES>::try_from(header.extra_data.clone())
            .map_err(|_| EthereumBeaconError::ExecutionHeaderRoot)?,
        base_fee_per_gas: U256::from_bytes_le(header.base_fee_per_gas),
        block_hash: header.block_hash.to_fixed_bytes(),
        transactions_root: header.transactions_root.to_fixed_bytes(),
        withdrawals_root: header.withdrawals_root.to_fixed_bytes(),
    })
}

fn hash_tree_root<T: ssz_rs::SimpleSerialize>(mut value: T) -> Result<H256, EthereumBeaconError> {
    let root = value
        .hash_tree_root()
        .map_err(|_| EthereumBeaconError::SigningRoot)?;
    let bytes: [u8; 32] = root
        .as_ref()
        .try_into()
        .map_err(|_| EthereumBeaconError::SigningRoot)?;
    Ok(H256::from(bytes))
}

fn verify_merkle_branch(
    leaf: H256,
    branch: &[H256],
    index: usize,
    depth: usize,
    root: H256,
) -> bool {
    if branch.len() != depth {
        return false;
    }
    let mut value = leaf.to_fixed_bytes();
    for (level, sibling) in branch.iter().enumerate() {
        let mut pair = [0u8; 64];
        if index & (1 << level) == 0 {
            pair[..32].copy_from_slice(&value);
            pair[32..].copy_from_slice(&sibling.to_fixed_bytes());
        } else {
            pair[..32].copy_from_slice(&sibling.to_fixed_bytes());
            pair[32..].copy_from_slice(&value);
        }
        value = sha2_256(&pair);
    }
    H256::from(value) == root
}

fn sha2_256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).into()
}

fn subtree_index(generalized_index: usize) -> usize {
    generalized_index % (1 << generalized_index_length(generalized_index))
}

fn generalized_index_length(generalized_index: usize) -> usize {
    generalized_index.ilog2() as usize
}

fn decompress_bits<const N: usize, const B: usize>(input: [u8; B]) -> [u8; N] {
    Bitvector::<N>::deserialize(&input)
        .expect("checked statically; qed")
        .iter()
        .map(|bit| u8::from(*bit))
        .collect::<Vec<_>>()
        .try_into()
        .expect("checked statically; qed")
}

fn sync_committee_participation_is_supermajority(participation: &[u8]) -> bool {
    let participants = participation
        .iter()
        .map(|bit| u64::from(*bit != 0))
        .sum::<u64>();
    participants.saturating_mul(3) >= (participation.len() as u64).saturating_mul(2)
}

fn compute_epoch(slot: u64) -> u64 {
    slot / SLOTS_PER_EPOCH
}

fn compute_period(slot: u64) -> u64 {
    slot / (SLOTS_PER_EPOCH * EPOCHS_PER_SYNC_COMMITTEE_PERIOD)
}

fn select_fork_version(fork_versions: &ForkVersions, epoch: u64) -> [u8; 4] {
    if epoch >= fork_versions.fulu.epoch {
        fork_versions.fulu.version
    } else if epoch >= fork_versions.electra.epoch {
        fork_versions.electra.version
    } else if epoch >= fork_versions.deneb.epoch {
        fork_versions.deneb.version
    } else if epoch >= fork_versions.capella.epoch {
        fork_versions.capella.version
    } else if epoch >= fork_versions.bellatrix.epoch {
        fork_versions.bellatrix.version
    } else if epoch >= fork_versions.altair.epoch {
        fork_versions.altair.version
    } else {
        fork_versions.genesis.version
    }
}

fn finalized_root_gindex_at_slot(slot: u64, fork_versions: &ForkVersions) -> usize {
    if compute_epoch(slot) >= fork_versions.electra.epoch {
        ELECTRA_FINALIZED_ROOT_INDEX
    } else {
        ALTAIR_FINALIZED_ROOT_INDEX
    }
}

fn current_sync_committee_gindex_at_slot(slot: u64, fork_versions: &ForkVersions) -> usize {
    if compute_epoch(slot) >= fork_versions.electra.epoch {
        ELECTRA_CURRENT_SYNC_COMMITTEE_INDEX
    } else {
        ALTAIR_CURRENT_SYNC_COMMITTEE_INDEX
    }
}

fn next_sync_committee_gindex_at_slot(slot: u64, fork_versions: &ForkVersions) -> usize {
    if compute_epoch(slot) >= fork_versions.electra.epoch {
        ELECTRA_NEXT_SYNC_COMMITTEE_INDEX
    } else {
        ALTAIR_NEXT_SYNC_COMMITTEE_INDEX
    }
}

fn block_roots_gindex_at_slot(slot: u64, fork_versions: &ForkVersions) -> usize {
    if compute_epoch(slot) >= fork_versions.electra.epoch {
        ELECTRA_BLOCK_ROOTS_INDEX
    } else {
        ALTAIR_BLOCK_ROOTS_INDEX
    }
}

mod ssz_types {
    use super::{
        FEE_RECIPIENT_SIZE, MAX_EXTRA_DATA_BYTES, MAX_LOGS_BLOOM_SIZE, PUBKEY_SIZE,
        SYNC_COMMITTEE_SIZE,
    };
    use ssz_rs::{
        prelude::{List, Vector},
        Deserialize as SszDeserialize, DeserializeError, Sized,
    };
    use ssz_rs_derive::SimpleSerialize as SimpleSerializeDerive;

    #[derive(Default, SimpleSerializeDerive, Clone, Debug)]
    pub(super) struct SSZBeaconBlockHeader {
        pub(super) slot: u64,
        pub(super) proposer_index: u64,
        pub(super) parent_root: [u8; 32],
        pub(super) state_root: [u8; 32],
        pub(super) body_root: [u8; 32],
    }

    #[derive(Default, SimpleSerializeDerive, Clone)]
    pub(super) struct SSZSyncCommittee {
        pub(super) pubkeys: Vector<Vector<u8, PUBKEY_SIZE>, SYNC_COMMITTEE_SIZE>,
        pub(super) aggregate_pubkey: Vector<u8, PUBKEY_SIZE>,
    }

    #[derive(Default, SimpleSerializeDerive, Clone)]
    pub(super) struct SSZForkData {
        pub(super) current_version: [u8; 4],
        pub(super) genesis_validators_root: [u8; 32],
    }

    #[derive(Default, SimpleSerializeDerive, Clone)]
    pub(super) struct SSZSigningData {
        pub(super) object_root: [u8; 32],
        pub(super) domain: [u8; 32],
    }

    #[derive(Default, SimpleSerializeDerive, Clone)]
    pub(super) struct SSZExecutionPayloadHeader {
        pub(super) parent_hash: [u8; 32],
        pub(super) fee_recipient: Vector<u8, FEE_RECIPIENT_SIZE>,
        pub(super) state_root: [u8; 32],
        pub(super) receipts_root: [u8; 32],
        pub(super) logs_bloom: Vector<u8, MAX_LOGS_BLOOM_SIZE>,
        pub(super) prev_randao: [u8; 32],
        pub(super) block_number: u64,
        pub(super) gas_limit: u64,
        pub(super) gas_used: u64,
        pub(super) timestamp: u64,
        pub(super) extra_data: List<u8, MAX_EXTRA_DATA_BYTES>,
        pub(super) base_fee_per_gas: ssz_rs::U256,
        pub(super) block_hash: [u8; 32],
        pub(super) transactions_root: [u8; 32],
        pub(super) withdrawals_root: [u8; 32],
    }

    #[derive(Default, SimpleSerializeDerive, Clone)]
    pub(super) struct SSZExecutionPayloadHeaderDeneb {
        pub(super) parent_hash: [u8; 32],
        pub(super) fee_recipient: Vector<u8, FEE_RECIPIENT_SIZE>,
        pub(super) state_root: [u8; 32],
        pub(super) receipts_root: [u8; 32],
        pub(super) logs_bloom: Vector<u8, MAX_LOGS_BLOOM_SIZE>,
        pub(super) prev_randao: [u8; 32],
        pub(super) block_number: u64,
        pub(super) gas_limit: u64,
        pub(super) gas_used: u64,
        pub(super) timestamp: u64,
        pub(super) extra_data: List<u8, MAX_EXTRA_DATA_BYTES>,
        pub(super) base_fee_per_gas: ssz_rs::U256,
        pub(super) block_hash: [u8; 32],
        pub(super) transactions_root: [u8; 32],
        pub(super) withdrawals_root: [u8; 32],
        pub(super) blob_gas_used: u64,
        pub(super) excess_blob_gas: u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct HeaderJson {
        slot: u64,
        proposer_index: u64,
        parent_root: String,
        state_root: String,
        body_root: String,
    }

    #[derive(Deserialize)]
    struct SyncCommitteeJson {
        pubkeys: Vec<String>,
        aggregate_pubkey: String,
    }

    #[derive(Deserialize)]
    struct CheckpointJson {
        header: HeaderJson,
        current_sync_committee: SyncCommitteeJson,
        current_sync_committee_branch: Vec<String>,
        validators_root: String,
        block_roots_root: String,
        block_roots_branch: Vec<String>,
    }

    #[derive(Deserialize)]
    struct SyncAggregateJson {
        sync_committee_bits: String,
        sync_committee_signature: String,
    }

    #[derive(Deserialize)]
    struct NextSyncCommitteeJson {
        next_sync_committee: SyncCommitteeJson,
        next_sync_committee_branch: Vec<String>,
    }

    #[derive(Deserialize)]
    struct UpdateJson {
        attested_header: HeaderJson,
        sync_aggregate: SyncAggregateJson,
        signature_slot: u64,
        next_sync_committee_update: Option<NextSyncCommitteeJson>,
        finalized_header: HeaderJson,
        finality_branch: Vec<String>,
        block_roots_root: String,
        block_roots_branch: Vec<String>,
    }

    #[derive(Deserialize)]
    struct ExecutionProofJson {
        header: HeaderJson,
        ancestry_proof: Option<AncestryProofJson>,
        execution_header: ExecutionHeaderJson,
        execution_branch: Vec<String>,
    }

    #[derive(Deserialize)]
    struct AncestryProofJson {
        header_branch: Vec<String>,
        finalized_block_root: String,
    }

    #[derive(Deserialize)]
    enum ExecutionHeaderJson {
        Capella(ExecutionHeaderFieldsJson),
        Deneb(ExecutionHeaderFieldsJson),
    }

    #[derive(Deserialize)]
    struct ExecutionHeaderFieldsJson {
        parent_hash: String,
        fee_recipient: String,
        state_root: String,
        receipts_root: String,
        logs_bloom: String,
        prev_randao: String,
        block_number: u64,
        gas_limit: u64,
        gas_used: u64,
        timestamp: u64,
        extra_data: String,
        base_fee_per_gas: u64,
        block_hash: String,
        transactions_root: String,
        withdrawals_root: String,
        blob_gas_used: Option<u64>,
        excess_blob_gas: Option<u64>,
    }

    fn h256(value: &str) -> H256 {
        let bytes = hex::decode(value.trim_start_matches("0x")).expect("hex H256");
        H256::from_slice(&bytes)
    }

    fn bytes<const N: usize>(value: &str) -> [u8; N] {
        let bytes = hex::decode(value.trim_start_matches("0x")).expect("hex bytes");
        bytes.try_into().expect("fixed byte length")
    }

    fn header(value: HeaderJson) -> EthereumBeaconHeader {
        EthereumBeaconHeader {
            slot: value.slot,
            proposer_index: value.proposer_index,
            parent_root: h256(&value.parent_root),
            state_root: h256(&value.state_root),
            body_root: h256(&value.body_root),
        }
    }

    fn committee(value: SyncCommitteeJson) -> EthereumSyncCommittee {
        EthereumSyncCommittee {
            pubkeys: value
                .pubkeys
                .iter()
                .map(|key| bytes::<PUBKEY_SIZE>(key))
                .collect(),
            aggregate_pubkey: bytes::<PUBKEY_SIZE>(&value.aggregate_pubkey),
        }
    }

    fn checkpoint() -> EthereumCheckpointUpdate {
        let value: CheckpointJson = serde_json::from_str(include_str!(
            "../tests/fixtures/ethereum/initial-checkpoint.json"
        ))
        .expect("checkpoint fixture");
        EthereumCheckpointUpdate {
            header: header(value.header),
            current_sync_committee: committee(value.current_sync_committee),
            current_sync_committee_branch: value
                .current_sync_committee_branch
                .iter()
                .map(|value| h256(value))
                .collect(),
            validators_root: h256(&value.validators_root),
            block_roots_root: h256(&value.block_roots_root),
            block_roots_branch: value
                .block_roots_branch
                .iter()
                .map(|value| h256(value))
                .collect(),
        }
    }

    fn update(path: &str) -> EthereumUpdate {
        let value: UpdateJson = serde_json::from_str(path).expect("beacon update fixture");
        EthereumUpdate {
            attested_header: header(value.attested_header),
            sync_aggregate: EthereumSyncAggregate {
                sync_committee_bits: bytes(&value.sync_aggregate.sync_committee_bits),
                sync_committee_signature: bytes(&value.sync_aggregate.sync_committee_signature),
            },
            signature_slot: value.signature_slot,
            next_sync_committee_update: value.next_sync_committee_update.map(|update| {
                EthereumNextSyncCommitteeUpdate {
                    next_sync_committee: committee(update.next_sync_committee),
                    next_sync_committee_branch: update
                        .next_sync_committee_branch
                        .iter()
                        .map(|value| h256(value))
                        .collect(),
                }
            }),
            finalized_header: header(value.finalized_header),
            finality_branch: value
                .finality_branch
                .iter()
                .map(|value| h256(value))
                .collect(),
            block_roots_root: h256(&value.block_roots_root),
            block_roots_branch: value
                .block_roots_branch
                .iter()
                .map(|value| h256(value))
                .collect(),
        }
    }

    fn execution_header(value: ExecutionHeaderFieldsJson) -> EthereumExecutionPayloadHeader {
        let mut base_fee_per_gas = [0u8; 32];
        base_fee_per_gas[..8].copy_from_slice(&value.base_fee_per_gas.to_le_bytes());
        EthereumExecutionPayloadHeader {
            parent_hash: h256(&value.parent_hash),
            fee_recipient: bytes(&value.fee_recipient),
            state_root: h256(&value.state_root),
            receipts_root: h256(&value.receipts_root),
            logs_bloom: hex::decode(value.logs_bloom.trim_start_matches("0x")).expect("logs bloom"),
            prev_randao: h256(&value.prev_randao),
            block_number: value.block_number,
            gas_limit: value.gas_limit,
            gas_used: value.gas_used,
            timestamp: value.timestamp,
            extra_data: hex::decode(value.extra_data.trim_start_matches("0x")).expect("extra data"),
            base_fee_per_gas,
            block_hash: h256(&value.block_hash),
            transactions_root: h256(&value.transactions_root),
            withdrawals_root: h256(&value.withdrawals_root),
            blob_gas_used: value.blob_gas_used.unwrap_or(0),
            excess_blob_gas: value.excess_blob_gas.unwrap_or(0),
        }
    }

    fn execution_proof() -> EthereumExecutionProof {
        let value: ExecutionProofJson = serde_json::from_str(include_str!(
            "../tests/fixtures/ethereum/execution-proof.json"
        ))
        .expect("execution proof fixture");
        EthereumExecutionProof {
            header: header(value.header),
            ancestry_proof: value.ancestry_proof.map(|proof| EthereumAncestryProof {
                header_branch: proof
                    .header_branch
                    .iter()
                    .map(|value| h256(value))
                    .collect(),
                finalized_block_root: h256(&proof.finalized_block_root),
            }),
            execution_header: match value.execution_header {
                ExecutionHeaderJson::Capella(header) => {
                    EthereumExecutionHeader::Capella(execution_header(header))
                }
                ExecutionHeaderJson::Deneb(header) => {
                    EthereumExecutionHeader::Deneb(execution_header(header))
                }
            },
            execution_branch: value
                .execution_branch
                .iter()
                .map(|value| h256(value))
                .collect(),
        }
    }

    fn fork_versions() -> ForkVersions {
        ForkVersions {
            genesis: Fork {
                version: [0, 0, 0, 0],
                epoch: 0,
            },
            altair: Fork {
                version: [1, 0, 0, 0],
                epoch: 0,
            },
            bellatrix: Fork {
                version: [2, 0, 0, 0],
                epoch: 0,
            },
            capella: Fork {
                version: [3, 0, 0, 0],
                epoch: 0,
            },
            deneb: Fork {
                version: [4, 0, 0, 0],
                epoch: 0,
            },
            electra: Fork {
                version: [5, 0, 0, 0],
                epoch: 0,
            },
            fulu: Fork {
                version: [6, 0, 0, 0],
                epoch: 100_000_000,
            },
        }
    }

    fn finalized_state() -> (EthereumBeaconState, EthereumFinalizedCheckpoint, H256) {
        let forks = fork_versions();
        let checkpoint = checkpoint();
        let initial = initialize_ethereum_beacon(&checkpoint, &forks).unwrap();
        let initial_root = initial.finalized_header.hash_tree_root().unwrap();
        let initial_anchor = EthereumFinalizedCheckpoint {
            slot: initial.finalized_header.slot,
            block_roots_root: initial.block_roots_root,
        };
        let state = apply_ethereum_finality_update(
            &initial,
            &update(include_str!(
                "../tests/fixtures/ethereum/sync-committee-update.json"
            )),
            &forks,
        )
        .unwrap();
        let state = apply_ethereum_finality_update(
            &state,
            &update(include_str!(
                "../tests/fixtures/ethereum/finalized-header-update.json"
            )),
            &forks,
        )
        .unwrap();
        assert_eq!(state.finalized_header.slot, 800);
        (state, initial_anchor, initial_root)
    }

    #[test]
    fn official_checkpoint_and_two_finality_updates_verify() {
        let forks = fork_versions();
        let state = initialize_ethereum_beacon(&checkpoint(), &forks).unwrap();
        assert_eq!(state.finalized_header.slot, 64);

        let state = apply_ethereum_finality_update(
            &state,
            &update(include_str!(
                "../tests/fixtures/ethereum/sync-committee-update.json"
            )),
            &forks,
        )
        .unwrap();
        assert_eq!(state.finalized_header.slot, 64);
        assert!(state.next_sync_committee.is_some());

        let state = apply_ethereum_finality_update(
            &state,
            &update(include_str!(
                "../tests/fixtures/ethereum/finalized-header-update.json"
            )),
            &forks,
        )
        .unwrap();
        assert_eq!(state.finalized_header.slot, 800);
        assert_ne!(state.block_roots_root, H256::zero());
    }

    #[test]
    fn official_execution_proof_binds_execution_fields_to_a_finalized_beacon_block() {
        let (state, initial_anchor, initial_root) = finalized_state();
        let root = state.finalized_header.hash_tree_root().unwrap();
        let trusted = EthereumFinalizedCheckpoint {
            slot: state.finalized_header.slot,
            block_roots_root: state.block_roots_root,
        };
        let verified =
            verify_ethereum_execution_proof(root, &trusted, &execution_proof(), |candidate| {
                (candidate == initial_root).then_some(initial_anchor.clone())
            })
            .unwrap();

        assert_eq!(verified.beacon_slot, 182);
        assert_eq!(verified.block_number, 182);
        assert_eq!(
            verified.receipts_root,
            H256::from(hex_literal::hex!(
                "2a2c82ec07ed78ee7495aef5ee3b2c3fd4959cfd1f16bac86edff12ae03defc4"
            ))
        );
    }

    #[test]
    fn tampered_signature_finality_and_execution_branches_fail_closed() {
        let forks = fork_versions();
        let state = initialize_ethereum_beacon(&checkpoint(), &forks).unwrap();

        let mut bad_signature = update(include_str!(
            "../tests/fixtures/ethereum/sync-committee-update.json"
        ));
        bad_signature.sync_aggregate.sync_committee_signature[0] ^= 1;
        assert_eq!(
            apply_ethereum_finality_update(&state, &bad_signature, &forks),
            Err(EthereumBeaconError::BlsVerificationFailed)
        );

        let mut bad_finality = update(include_str!(
            "../tests/fixtures/ethereum/sync-committee-update.json"
        ));
        bad_finality.finality_branch[0] = H256::repeat_byte(0x42);
        assert_eq!(
            apply_ethereum_finality_update(&state, &bad_finality, &forks),
            Err(EthereumBeaconError::InvalidFinalityProof)
        );

        let (state, initial_anchor, initial_root) = finalized_state();
        let root = state.finalized_header.hash_tree_root().unwrap();
        let trusted = EthereumFinalizedCheckpoint {
            slot: state.finalized_header.slot,
            block_roots_root: state.block_roots_root,
        };
        let mut bad_ancestry = execution_proof();
        bad_ancestry
            .ancestry_proof
            .as_mut()
            .expect("fixture ancestry")
            .header_branch[0] = H256::repeat_byte(0x43);
        assert_eq!(
            verify_ethereum_execution_proof(root, &trusted, &bad_ancestry, |candidate| (candidate
                == initial_root)
                .then_some(initial_anchor.clone()),),
            Err(EthereumBeaconError::InvalidAncestryProof)
        );

        let mut bad_execution = execution_proof();
        bad_execution.execution_branch[0] = H256::repeat_byte(0x44);
        assert_eq!(
            verify_ethereum_execution_proof(root, &trusted, &bad_execution, |candidate| (candidate
                == initial_root)
                .then_some(initial_anchor.clone()),),
            Err(EthereumBeaconError::InvalidExecutionProof)
        );
    }
}
