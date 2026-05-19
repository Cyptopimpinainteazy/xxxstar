use crate::{ConflictClass, StateKey, VmLane};
use codec::{Decode, Encode};
use contention_predictor::{ContentionPredictor, TxMetadata};
use futures::{channel::oneshot, future, FutureExt};
use log::{debug, info, trace, warn};
use rayon::prelude::*;
use sc_block_builder::{BlockBuilderApi, BlockBuilderBuilder};
use sc_client_api::backend;
use sc_telemetry::{telemetry, TelemetryHandle, CONSENSUS_INFO};
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use sp_api::CallApiAt;
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::{ApplyExtrinsicFailed::Validity, Error::ApplyExtrinsicFailed, HeaderBackend};
use sp_consensus::{Environment, ProofRecording, Proposal, Proposer};
use sp_core::traits::SpawnNamed;
use sp_inherents::InherentData;
use sp_runtime::{
    traits::{BlakeTwo256, Block as BlockT, Hash as HashT, Header as HeaderT, SaturatedConversion},
    Digest, Percent,
};
use std::{marker::PhantomData, pin::Pin, sync::Arc, time};
use x3_chain_runtime::{Address, UncheckedExtrinsic};

/// Default block size limit in bytes used by the parallel proposer.
pub const DEFAULT_BLOCK_SIZE_LIMIT: usize = 4 * 1024 * 1024 + 512;

const DEFAULT_SOFT_DEADLINE_PERCENT: Percent = Percent::from_percent(50);

const LOG_TARGET: &str = "parallel-proposer";

/// Substrate parallel proposer factory.
pub struct ParallelProposerFactory<A, B, C, PR> {
    spawn_handle: Box<dyn SpawnNamed>,
    client: Arc<C>,
    transaction_pool: Arc<A>,
    default_block_size_limit: usize,
    soft_deadline_percent: Percent,
    telemetry: Option<TelemetryHandle>,
    include_proof_in_block_size_estimation: bool,
    predictor: Arc<ContentionPredictor>,
    _phantom: PhantomData<(B, PR)>,
}

impl<A, B, C> ParallelProposerFactory<A, B, C, sp_consensus::DisableProofRecording> {
    /// Create a new parallel proposer factory.
    pub fn new(
        spawn_handle: impl SpawnNamed + 'static,
        client: Arc<C>,
        transaction_pool: Arc<A>,
        _prometheus: Option<&prometheus_endpoint::Registry>,
        telemetry: Option<TelemetryHandle>,
        predictor: Arc<ContentionPredictor>,
    ) -> Self {
        ParallelProposerFactory {
            spawn_handle: Box::new(spawn_handle),
            client,
            transaction_pool,
            default_block_size_limit: DEFAULT_BLOCK_SIZE_LIMIT,
            soft_deadline_percent: DEFAULT_SOFT_DEADLINE_PERCENT,
            telemetry,
            include_proof_in_block_size_estimation: false,
            predictor,
            _phantom: PhantomData,
        }
    }
}

impl<A, B, C, PR> ParallelProposerFactory<A, B, C, PR>
where
    A: TransactionPool,
{
    /// Set the default block size limit in bytes.
    pub fn set_default_block_size_limit(&mut self, limit: usize) {
        self.default_block_size_limit = limit;
    }

    /// Set soft deadline percentage.
    pub fn set_soft_deadline(&mut self, percent: Percent) {
        self.soft_deadline_percent = percent;
    }

    fn init_with_now(
        &mut self,
        parent_header: &<A::Block as BlockT>::Header,
        now: Box<dyn Fn() -> time::Instant + Send + Sync>,
    ) -> ParallelProposer<B, A::Block, C, A, PR>
    where
        C: HeaderBackend<A::Block>,
    {
        let parent_hash = parent_header.hash();

        info!(target: LOG_TARGET, "Starting parallel proposer on top of parent {:?}", parent_hash);

        ParallelProposer {
            spawn_handle: self.spawn_handle.clone(),
            client: self.client.clone(),
            parent_hash,
            parent_number: *parent_header.number(),
            transaction_pool: self.transaction_pool.clone(),
            now,
            default_block_size_limit: self.default_block_size_limit,
            include_proof_in_block_size_estimation: self.include_proof_in_block_size_estimation,
            soft_deadline_percent: self.soft_deadline_percent,
            telemetry: self.telemetry.clone(),
            predictor: self.predictor.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<A, B, C, PR> Environment<A::Block> for ParallelProposerFactory<A, B, C, PR>
where
    A: TransactionPool + 'static,
    B: backend::Backend<A::Block> + Send + Sync + 'static,
    C: CallApiAt<A::Block>
        + HeaderBackend<A::Block>
        + ProvideRuntimeApi<A::Block>
        + Send
        + Sync
        + 'static,
    C::Api: ApiExt<A::Block> + BlockBuilderApi<A::Block>,
    PR: ProofRecording,
{
    type Proposer = ParallelProposer<B, A::Block, C, A, PR>;
    type CreateProposer = future::Ready<Result<Self::Proposer, Self::Error>>;
    type Error = sp_blockchain::Error;

    fn init(&mut self, parent_header: &<A::Block as BlockT>::Header) -> Self::CreateProposer {
        future::ready(Ok(
            self.init_with_now(parent_header, Box::new(time::Instant::now))
        ))
    }
}

/// The proposer logic.
pub struct ParallelProposer<B, Block: BlockT, C, A: TransactionPool, PR> {
    spawn_handle: Box<dyn SpawnNamed>,
    client: Arc<C>,
    parent_hash: Block::Hash,
    parent_number: <<Block as BlockT>::Header as HeaderT>::Number,
    transaction_pool: Arc<A>,
    now: Box<dyn Fn() -> time::Instant + Send + Sync>,
    default_block_size_limit: usize,
    include_proof_in_block_size_estimation: bool,
    soft_deadline_percent: Percent,
    telemetry: Option<TelemetryHandle>,
    predictor: Arc<ContentionPredictor>,
    _phantom: PhantomData<(B, PR)>,
}

impl<A, B, Block, C, PR> Proposer<Block> for ParallelProposer<B, Block, C, A, PR>
where
    A: TransactionPool<Block = Block> + 'static,
    B: backend::Backend<Block> + Send + Sync + 'static,
    Block: BlockT,
    C: CallApiAt<Block> + HeaderBackend<Block> + ProvideRuntimeApi<Block> + Send + Sync + 'static,
    C::Api: ApiExt<Block> + BlockBuilderApi<Block>,
    PR: ProofRecording,
{
    type Proposal = Pin<
        Box<dyn futures::Future<Output = Result<Proposal<Block, PR::Proof>, Self::Error>> + Send>,
    >;
    type Error = sp_blockchain::Error;
    type ProofRecording = PR;
    type Proof = PR::Proof;

    fn propose(
        self,
        inherent_data: InherentData,
        inherent_digests: Digest,
        max_duration: time::Duration,
        block_size_limit: Option<usize>,
    ) -> Self::Proposal {
        let (tx, rx) = oneshot::channel();
        let spawn_handle = self.spawn_handle.clone();

        spawn_handle.spawn_blocking(
            "parallel-proposer",
            None,
            Box::pin(async move {
                let deadline = (self.now)() + max_duration - max_duration / 3;
                let res = self
                    .propose_with(inherent_data, inherent_digests, deadline, block_size_limit)
                    .await;
                if tx.send(res).is_err() {
                    trace!(target: LOG_TARGET, "Could not send block production result to proposer");
                }
            }),
        );

        async move { rx.await? }.boxed()
    }
}

impl<A, B, Block, C, PR> ParallelProposer<B, Block, C, A, PR>
where
    A: TransactionPool<Block = Block>,
    B: backend::Backend<Block> + Send + Sync + 'static,
    Block: BlockT,
    C: CallApiAt<Block> + HeaderBackend<Block> + ProvideRuntimeApi<Block> + Send + Sync + 'static,
    C::Api: ApiExt<Block> + BlockBuilderApi<Block>,
    PR: ProofRecording,
{
    async fn propose_with(
        self,
        inherent_data: InherentData,
        inherent_digests: Digest,
        deadline: time::Instant,
        block_size_limit: Option<usize>,
    ) -> Result<Proposal<Block, PR::Proof>, sp_blockchain::Error> {
        let mut block_builder = BlockBuilderBuilder::new(&*self.client)
            .on_parent_block(self.parent_hash)
            .fetch_parent_block_number(&*self.client)?
            .with_proof_recording(PR::ENABLED)
            .with_inherent_digests(inherent_digests)
            .build()?;

        self.apply_inherents(&mut block_builder, inherent_data)?;
        self.apply_extrinsics_parallel(&mut block_builder, deadline, block_size_limit)
            .await?;

        let (block, storage_changes, proof) = block_builder.build()?.into_inner();
        let proof =
            PR::into_proof(proof).map_err(|e| sp_blockchain::Error::Application(Box::new(e)))?;

        info!(
            target: LOG_TARGET,
            "Prepared parallel proposal for block #{} on parent #{}",
            block.header().number(),
            self.parent_number,
        );
        telemetry!(
            self.telemetry;
            CONSENSUS_INFO;
            "prepared_parallel_block";
            "number" => ?block.header().number(),
            "hash" => ?<Block as BlockT>::Hash::from(block.header().hash()),
        );

        Ok(Proposal {
            block,
            proof,
            storage_changes,
        })
    }

    fn apply_inherents(
        &self,
        block_builder: &mut sc_block_builder::BlockBuilder<'_, Block, C>,
        inherent_data: InherentData,
    ) -> Result<(), sp_blockchain::Error> {
        let inherents = block_builder.create_inherents(inherent_data)?;

        for inherent in inherents {
            match block_builder.push(inherent) {
                Err(ApplyExtrinsicFailed(Validity(e))) if e.exhausted_resources() => {
                    warn!(target: LOG_TARGET, "Dropping non-mandatory inherent from overweight block")
                }
                Err(ApplyExtrinsicFailed(Validity(e))) if e.was_mandatory() => {
                    return Err(ApplyExtrinsicFailed(Validity(e)))
                }
                Err(e) => {
                    warn!(target: LOG_TARGET, "Inherent extrinsic error: {}. Dropping.", e);
                }
                Ok(_) => {}
            }
        }
        Ok(())
    }

    async fn apply_extrinsics_parallel(
        &self,
        block_builder: &mut sc_block_builder::BlockBuilder<'_, Block, C>,
        deadline: time::Instant,
        block_size_limit: Option<usize>,
    ) -> Result<(), sp_blockchain::Error> {
        let block_size_limit = block_size_limit.unwrap_or(self.default_block_size_limit);
        let soft_deadline = {
            let now = (self.now)();
            let left = deadline.saturating_duration_since(now);
            let left_micros: u64 = left.as_micros().saturated_into();
            now + time::Duration::from_micros(self.soft_deadline_percent.mul_floor(left_micros))
        };

        let mut pending: Vec<PendingTx<Block, A::Hash>> = Vec::new();
        let mut total_size = 0usize;
        let pending_iter = self.transaction_pool.ready();

        for pending_tx in pending_iter {
            let data = pending_tx.data().clone();
            let size = data.encoded_size();
            if total_size + size > block_size_limit {
                continue;
            }

            let pool_hash = pending_tx.hash().clone();
            total_size = total_size.saturating_add(size);
            let tx_hash = BlakeTwo256::hash_of(&data);
            let metadata = extract_tx_metadata(data.as_ref(), hash_to_bytes(tx_hash.as_ref()));
            pending.push(PendingTx {
                pool_hash,
                data: Some(data),
                size,
                metadata,
            });
        }

        if pending.is_empty() {
            return Ok(());
        }

        let tx_metadata: Vec<TxMetadata> = pending.iter().map(|tx| tx.metadata.clone()).collect();
        let shard_groups = match self.predictor.predict_and_shard(&tx_metadata).await {
            Ok(groups) => groups,
            Err(e) => {
                warn!(target: LOG_TARGET, "contention predictor error: {e}");
                Vec::new()
            }
        };

        let mut shard_groups: Vec<(u32, Vec<usize>)> = shard_groups
            .par_iter()
            .map(|group| (group.shard_id, group.tx_indices.clone()))
            .collect();
        shard_groups.sort_by_key(|(shard_id, _)| *shard_id);
        let mut execution_order: Vec<usize> = Vec::with_capacity(pending.len());
        let mut seen = vec![false; pending.len()];

        if shard_groups.len() > 1 {
            for (_, indices) in &shard_groups {
                for &idx in indices {
                    if idx < pending.len() && !seen[idx] {
                        execution_order.push(idx);
                        seen[idx] = true;
                    }
                }
            }
        }

        for idx in 0..pending.len() {
            if !seen[idx] {
                execution_order.push(idx);
            }
        }

        let mut skipped = 0usize;
        let mut invalid_hashes = Vec::new();

        for idx in execution_order {
            let now = (self.now)();
            if now > deadline {
                debug!(target: LOG_TARGET, "consensus deadline reached; proposing with current txs");
                break;
            }

            let pending_tx = match pending.get_mut(idx) {
                Some(tx) => tx,
                None => continue,
            };
            let block_size =
                block_builder.estimate_block_size(self.include_proof_in_block_size_estimation);
            if block_size + pending_tx.size > block_size_limit {
                if skipped < MAX_SKIPPED_TRANSACTIONS || now < soft_deadline {
                    skipped += 1;
                    continue;
                }
                break;
            }

            let data = match pending_tx.data.take() {
                Some(data) => data,
                None => continue,
            };
            trace!(target: LOG_TARGET, "Pushing tx {:?} to block", pending_tx.pool_hash);
            match block_builder.push((*data).clone()) {
                Ok(()) => {}
                Err(ApplyExtrinsicFailed(Validity(e))) if e.exhausted_resources() => {
                    if skipped < MAX_SKIPPED_TRANSACTIONS || (self.now)() < soft_deadline {
                        skipped += 1;
                        continue;
                    }
                    break;
                }
                Err(e) => {
                    invalid_hashes.push(pending_tx.pool_hash.clone());
                    debug!(
                        target: LOG_TARGET,
                        "Invalid transaction {:?}: {}",
                        pending_tx.pool_hash,
                        e
                    );
                }
            }
        }

        if !invalid_hashes.is_empty() {
            // NOTE: remove_invalid was removed in stable2512; pool manages invalid tx cleanup internally
            let _ = invalid_hashes;
        }

        Ok(())
    }
}

const MAX_SKIPPED_TRANSACTIONS: usize = 8;

struct PendingTx<Block: BlockT, Hash> {
    pool_hash: Hash,
    data: Option<Arc<Block::Extrinsic>>,
    size: usize,
    metadata: TxMetadata,
}

fn hash_to_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes[..32.min(bytes.len())]);
    out
}

fn address_to_account(address: Address) -> Option<x3_chain_runtime::AccountId> {
    match address {
        sp_runtime::MultiAddress::Id(id) => Some(id),
        _ => None,
    }
}

/// Extract transaction metadata from an opaque extrinsic.
pub fn extract_tx_metadata<E: Encode>(extrinsic: &E, tx_hash: [u8; 32]) -> TxMetadata {
    let bytes = extrinsic.encode();
    if let Ok(decoded) = UncheckedExtrinsic::decode(&mut &bytes[..]) {
        let mut sender = tx_hash;
        let mut nonce = 0u64;

        if let sp_runtime::generic::Preamble::Signed(address, _signature, extra) = decoded.preamble
        {
            if let Some(account) = address_to_account(address) {
                sender.copy_from_slice(account.as_ref());
            }

            let (_, _, _, _, _, check_nonce, _, _, _) = extra;
            nonce = check_nonce.0.saturated_into::<u64>();
        }

        let call_bytes = decoded.function.encode();
        let selector = if call_bytes.len() >= 4 {
            Some([call_bytes[0], call_bytes[1], call_bytes[2], call_bytes[3]])
        } else {
            None
        };

        return TxMetadata {
            tx_hash,
            sender,
            target: None,
            selector,
            gas_limit: 0,
            value: 0,
            calldata_len: 0,
            nonce,
            timestamp: 0,
        };
    }

    TxMetadata {
        tx_hash,
        sender: tx_hash,
        target: None,
        selector: None,
        gas_limit: 0,
        value: 0,
        calldata_len: 0,
        nonce: 0,
        timestamp: 0,
    }
}

pub fn state_keys_from_metadata(metadata: &TxMetadata) -> Vec<StateKey> {
    let mut keys = Vec::new();

    keys.push(StateKey::new(
        VmLane::System,
        ConflictClass::Account,
        "sender",
        hex::encode(metadata.sender),
    ));

    if let Some(target) = metadata.target {
        keys.push(StateKey::new(
            VmLane::System,
            ConflictClass::Account,
            "target",
            hex::encode(target),
        ));
    }

    if let Some(selector) = metadata.selector {
        keys.push(StateKey::new(
            VmLane::System,
            ConflictClass::StorageSlot,
            "call",
            hex::encode(selector),
        ));
    }

    keys.push(StateKey::new(
        VmLane::System,
        ConflictClass::Global,
        "nonce",
        metadata.nonce.to_string(),
    ));

    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_key_projection_is_deterministic() {
        let metadata = TxMetadata {
            tx_hash: [1u8; 32],
            sender: [2u8; 32],
            target: Some([3u8; 32]),
            selector: Some([0xaa, 0xbb, 0xcc, 0xdd]),
            gas_limit: 0,
            value: 0,
            calldata_len: 0,
            nonce: 7,
            timestamp: 0,
        };

        let a = state_keys_from_metadata(&metadata);
        let b = state_keys_from_metadata(&metadata);

        assert_eq!(a, b);
        assert!(a.iter().any(|key| key.domain == "sender"));
        assert!(a.iter().any(|key| key.domain == "target"));
        assert!(a.iter().any(|key| key.domain == "call"));
        assert!(a.iter().any(|key| key.domain == "nonce"));
    }
}
