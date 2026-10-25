//! A header-only Bitcoin client: admits headers under Bitcoin's rules from a checkpoint, follows
//! the most-work chain, and answers "is this transaction buried under N blocks of it".

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::{
    check_proof_of_work, median_time_past, next_work_required, verify_merkle_inclusion,
    work_from_bits, BlockHeader, Hash, Params, SpvError, MAX_FUTURE_BLOCK_TIME, MEDIAN_TIME_SPAN,
    U256,
};

/// The header a client trusts without checking its ancestry: the trust root.
///
/// It sits on a retarget boundary (`height % 2016 == 0`) so that every later window's difficulty
/// can be derived from headers the client holds; a checkpoint in the middle of a window would leave
/// the next retarget's starting timestamp unknown, and an unknown retarget is one the client could
/// only guess at. `prior_timestamps` are up to ten timestamps of the blocks just before it, oldest
/// first, so median-time-past can be enforced from the first header on; with fewer, the rule is
/// enforced from the first header that has eleven known ancestors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub height: u64,
    pub header: BlockHeader,
    pub prior_timestamps: Vec<u32>,
}

#[derive(Clone, Debug)]
struct Entry {
    header: BlockHeader,
    height: u64,
    chain_work: U256,
}

/// What [`LightClient::submit`] admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Submitted {
    pub hash: Hash,
    pub height: u64,
    /// Whether the header became the most-work tip.
    pub new_tip: bool,
}

/// A proven inclusion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inclusion {
    pub block_hash: Hash,
    pub height: u64,
    pub confirmations: u64,
}

/// Headers admitted under Bitcoin's rules from one checkpoint, with the most-work tip tracked.
#[derive(Clone, Debug)]
pub struct LightClient {
    params: Params,
    entries: BTreeMap<Hash, Entry>,
    checkpoint: Hash,
    prior_timestamps: Vec<u32>,
    tip: Hash,
}

impl LightClient {
    /// Start from a checkpoint. Its own proof of work is checked; its ancestry is trusted.
    pub fn new(params: Params, checkpoint: Checkpoint) -> Result<Self, SpvError> {
        if !params.no_retargeting && !checkpoint.height.is_multiple_of(params.retarget_interval) {
            return Err(SpvError::CheckpointNotAtRetargetBoundary(checkpoint.height));
        }
        if checkpoint.prior_timestamps.len() >= MEDIAN_TIME_SPAN {
            return Err(SpvError::TooManyPriorTimestamps(
                checkpoint.prior_timestamps.len(),
            ));
        }
        check_proof_of_work(&checkpoint.header, &params)?;
        let hash = checkpoint.header.block_hash();
        let mut entries = BTreeMap::new();
        entries.insert(
            hash,
            Entry {
                header: checkpoint.header,
                height: checkpoint.height,
                chain_work: work_from_bits(checkpoint.header.bits)?,
            },
        );
        Ok(Self {
            params,
            entries,
            checkpoint: hash,
            prior_timestamps: checkpoint.prior_timestamps,
            tip: hash,
        })
    }

    /// Admit a header whose parent is already admitted. `now` is the verifier's clock in Unix
    /// seconds (on chain, the block's timestamp), against which Bitcoin's two-hour future limit is
    /// applied.
    pub fn submit(&mut self, header: BlockHeader, now: u32) -> Result<Submitted, SpvError> {
        let hash = header.block_hash();
        if self.entries.contains_key(&hash) {
            return Err(SpvError::DuplicateHeader(hash));
        }
        let parent = self
            .entries
            .get(&header.prev_blockhash)
            .ok_or(SpvError::UnknownParent(header.prev_blockhash))?;
        let height = parent
            .height
            .checked_add(1)
            .ok_or(SpvError::HeightOverflow)?;

        let expected = self.expected_bits(parent, height)?;
        if header.bits != expected {
            return Err(SpvError::UnexpectedDifficulty {
                height,
                expected,
                actual: header.bits,
            });
        }
        check_proof_of_work(&header, &self.params)?;

        let ancestors = self.ancestor_timestamps(&header.prev_blockhash);
        if ancestors.len() == MEDIAN_TIME_SPAN {
            let median = median_time_past(&ancestors).unwrap_or(0);
            if header.time <= median {
                return Err(SpvError::TimestampNotAfterMedian {
                    time: header.time,
                    median,
                });
            }
        }
        if header.time > now.saturating_add(MAX_FUTURE_BLOCK_TIME) {
            return Err(SpvError::TimestampTooFarInFuture {
                time: header.time,
                now,
            });
        }

        let chain_work = parent
            .chain_work
            .saturating_add(&work_from_bits(header.bits)?);
        let new_tip = chain_work > self.entries[&self.tip].chain_work;
        self.entries.insert(
            hash,
            Entry {
                header,
                height,
                chain_work,
            },
        );
        if new_tip {
            self.tip = hash;
        }
        Ok(Submitted {
            hash,
            height,
            new_tip,
        })
    }

    /// The most-work tip: its hash and height.
    pub fn tip(&self) -> (Hash, u64) {
        (self.tip, self.entries[&self.tip].height)
    }

    /// Cumulative work from the checkpoint to the tip, inclusive.
    pub fn tip_work(&self) -> U256 {
        self.entries[&self.tip].chain_work
    }

    /// An admitted header and its height.
    pub fn header(&self, hash: &Hash) -> Option<(BlockHeader, u64)> {
        self.entries.get(hash).map(|e| (e.header, e.height))
    }

    /// Confirmations of a block on the most-work chain: 1 for the tip. A block on a lighter branch
    /// is refused, not counted.
    pub fn confirmations(&self, block: &Hash) -> Result<u64, SpvError> {
        let height = self
            .entries
            .get(block)
            .ok_or(SpvError::UnknownBlock(*block))?
            .height;
        let (tip, tip_height) = self.tip();
        let mut cursor = tip;
        for _ in height..tip_height {
            cursor = self.entries[&cursor].header.prev_blockhash;
        }
        if cursor != *block {
            return Err(SpvError::NotInBestChain(*block));
        }
        Ok(tip_height - height + 1)
    }

    /// Prove `txid` is at `index` in `block` with at least `min_confirmations` on the most-work
    /// chain.
    pub fn verify_inclusion(
        &self,
        txid: &Hash,
        index: u32,
        path: &[Hash],
        block: &Hash,
        min_confirmations: u64,
    ) -> Result<Inclusion, SpvError> {
        let confirmations = self.confirmations(block)?;
        if confirmations < min_confirmations {
            return Err(SpvError::InsufficientConfirmations {
                required: min_confirmations,
                actual: confirmations,
            });
        }
        let entry = &self.entries[block];
        verify_merkle_inclusion(txid, index, path, &entry.header.merkle_root)?;
        Ok(Inclusion {
            block_hash: *block,
            height: entry.height,
            confirmations,
        })
    }

    /// The `nBits` Bitcoin requires for the block at `height` whose parent is `parent`.
    fn expected_bits(&self, parent: &Entry, height: u64) -> Result<u32, SpvError> {
        if self.params.no_retargeting || !height.is_multiple_of(self.params.retarget_interval) {
            return Ok(parent.header.bits);
        }
        // The window's first block is `interval - 1` blocks behind the parent. The checkpoint is on
        // a boundary, so it is at most the checkpoint itself and always held.
        let mut first = parent;
        for _ in 0..self.params.retarget_interval - 1 {
            first = self
                .entries
                .get(&first.header.prev_blockhash)
                .ok_or(SpvError::UnknownParent(first.header.prev_blockhash))?;
        }
        Ok(next_work_required(
            &self.params,
            parent.header.bits,
            parent.header.time,
            first.header.time,
        ))
    }

    /// Timestamps of up to eleven blocks ending at `from`, oldest first, continuing into the
    /// checkpoint's prior timestamps.
    fn ancestor_timestamps(&self, from: &Hash) -> Vec<u32> {
        let mut newest_first = Vec::with_capacity(MEDIAN_TIME_SPAN);
        let mut cursor = *from;
        while newest_first.len() < MEDIAN_TIME_SPAN {
            let entry = &self.entries[&cursor];
            newest_first.push(entry.header.time);
            if cursor == self.checkpoint {
                for time in self.prior_timestamps.iter().rev() {
                    if newest_first.len() == MEDIAN_TIME_SPAN {
                        break;
                    }
                    newest_first.push(*time);
                }
                break;
            }
            cursor = entry.header.prev_blockhash;
        }
        newest_first.reverse();
        newest_first
    }
}
