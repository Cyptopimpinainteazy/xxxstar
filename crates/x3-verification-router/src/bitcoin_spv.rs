//! Bitcoin deposit proofs, verified against this chain's own view of Bitcoin.
//!
//! A deposit proof claims: *this transaction, in a block on Bitcoin's best chain buried under N
//! blocks, pays `amount` satoshis to the vault, and names this beneficiary*. Every part is checked
//! against something the prover does not control:
//!
//! - **the block** must be one this chain's header store has admitted onto its best Bitcoin chain
//!   ([`BtcHeaderAnchor`]); the header in the proof only has to hash to it. The previous verifier
//!   took the header chain *and* the chain tip from the payload, checked proof of work against
//!   whatever target the header named, and counted confirmations as `tip - tx_index` — a 32-byte
//!   all-zero "txid" in an all-zero header with an easy target passed with 101 confirmations;
//! - **inclusion** is Bitcoin's Merkle rule with position ([`x3_btc_spv`]); the old one sorted each
//!   pair, which is not Bitcoin's tree;
//! - **the payment** is read from the transaction itself: the output at `vout` must pay
//!   `expected_amount` to `expected_recipient` (the vault's `scriptPubKey`);
//! - **the beneficiary** must be committed in the transaction by an `OP_RETURN` output
//!   ([`deposit_commitment_script`]), so a watcher who sees someone else's deposit cannot claim it;
//! - **uniqueness**: the envelope's `proof_id` must be [`bitcoin_deposit_id`] of the outpoint, so
//!   the gateway's replay set holds one entry per real deposit rather than one per chosen id.

use alloc::vec::Vec;
use sha2::{Digest, Sha256};
use x3_btc_spv::{
    check_proof_of_work, transaction_outputs, verify_transaction, BlockHeader, Hash, Params,
    HEADER_LEN, MAX_MERKLE_DEPTH,
};

use crate::{
    ChainKind, ProofEnvelope, VerificationError, VerificationOutcome, VerificationStrategy,
    Verifier,
};

/// The payload format this verifier reads.
pub const BTC_PROOF_FORMAT_V1: u8 = 1;
/// The `OP_RETURN` tag that opens a deposit's beneficiary commitment.
pub const DEPOSIT_COMMITMENT_TAG: &[u8; 6] = b"X3DEP1";

/// This chain's view of Bitcoin: which blocks are on its best chain, and how tall that chain is.
///
/// Implemented by the chain's header store (the settlement engine's SPV client on X3). The
/// verifier never takes a block's position from the proof.
pub trait BtcHeaderAnchor {
    /// The height of `block_hash` (internal byte order) if it is on the best chain.
    fn best_chain_height(block_hash: &Hash) -> Option<u64>;
    /// The best chain's height, `None` until the store holds a chain.
    fn best_height() -> Option<u64>;
}

/// No header store: every proof is refused.
pub struct NoBtcHeaderAnchor;

impl BtcHeaderAnchor for NoBtcHeaderAnchor {
    fn best_chain_height(_block_hash: &Hash) -> Option<u64> {
        None
    }
    fn best_height() -> Option<u64> {
        None
    }
}

#[derive(Clone, Copy)]
struct AnchorFns {
    height: fn(&Hash) -> Option<u64>,
    best: fn() -> Option<u64>,
}

/// The id a Bitcoin deposit is known by: one per outpoint, whoever submits it.
pub fn bitcoin_deposit_id(txid: &Hash, vout: u32) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"x3-btc-deposit-v1");
    hasher.update(txid);
    hasher.update(vout.to_le_bytes());
    hasher.finalize().into()
}

/// The `OP_RETURN` script a deposit carries to name its beneficiary on X3:
/// `OP_RETURN PUSH38 "X3DEP1" sha256(beneficiary)`.
pub fn deposit_commitment_script(beneficiary: &[u8]) -> Vec<u8> {
    let mut script = Vec::with_capacity(40);
    script.push(0x6a);
    script.push(38);
    script.extend_from_slice(DEPOSIT_COMMITMENT_TAG);
    script.extend_from_slice(&Sha256::digest(beneficiary));
    script
}

/// Encode a v1 payload: `version | header(80) | tx_index u32 | n u32 | n×32 path | vout u32 |
/// tx_len u32 | raw transaction (legacy serialization)`.
pub fn encode_btc_proof_v1(
    header: &BlockHeader,
    tx_index: u32,
    path: &[Hash],
    vout: u32,
    raw_tx: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + HEADER_LEN + 12 + path.len() * 32 + 4 + raw_tx.len());
    out.push(BTC_PROOF_FORMAT_V1);
    out.extend_from_slice(&header.encode());
    out.extend_from_slice(&tx_index.to_le_bytes());
    out.extend_from_slice(&(path.len() as u32).to_le_bytes());
    for node in path {
        out.extend_from_slice(node);
    }
    out.extend_from_slice(&vout.to_le_bytes());
    out.extend_from_slice(&(raw_tx.len() as u32).to_le_bytes());
    out.extend_from_slice(raw_tx);
    out
}

struct Payload<'a> {
    header: BlockHeader,
    tx_index: u32,
    path: Vec<Hash>,
    vout: u32,
    raw_tx: &'a [u8],
}

fn decode(payload: &[u8]) -> Result<Payload<'_>, VerificationError> {
    let malformed = VerificationError::MalformedProof;
    let (&version, rest) = payload.split_first().ok_or(malformed.clone())?;
    if version != BTC_PROOF_FORMAT_V1 {
        return Err(VerificationError::UnsupportedProofFormat);
    }
    let mut at = 0usize;
    let mut take = |n: usize| -> Result<&[u8], VerificationError> {
        let end = at.checked_add(n).ok_or(VerificationError::MalformedProof)?;
        let slice = rest.get(at..end).ok_or(VerificationError::MalformedProof)?;
        at = end;
        Ok(slice)
    };
    let u32_at = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let header = BlockHeader::decode(take(HEADER_LEN)?).map_err(|_| malformed.clone())?;
    let tx_index = u32_at(take(4)?);
    let depth = u32_at(take(4)?) as usize;
    if depth > MAX_MERKLE_DEPTH {
        return Err(malformed);
    }
    let mut path = Vec::with_capacity(depth);
    for _ in 0..depth {
        let mut node = [0u8; 32];
        node.copy_from_slice(take(32)?);
        path.push(node);
    }
    let vout = u32_at(take(4)?);
    let tx_len = u32_at(take(4)?) as usize;
    let raw_tx = take(tx_len)?;
    if at != rest.len() {
        return Err(VerificationError::MalformedProof);
    }
    Ok(Payload {
        header,
        tx_index,
        path,
        vout,
        raw_tx,
    })
}

/// Verifies Bitcoin deposit proofs against a [`BtcHeaderAnchor`].
///
/// There is no constructor without an anchor: the question a Bitcoin proof answers is *which
/// block, how deep*, and a verifier that took the answer from the proof would be checking the
/// prover against themselves.
///
/// Vault signer approvals are not consulted here; SPV confirmation and signer quorum stay two
/// independent gates (see `x3-bitcoin-vault`).
pub struct BitcoinSpvVerifier {
    pub min_confirmations: u64,
    params: Params,
    anchor: AnchorFns,
}

impl BitcoinSpvVerifier {
    /// A verifier for mainnet headers, anchored to the store `A`.
    pub fn anchored_by<A: BtcHeaderAnchor>(min_confirmations: u64) -> Self {
        Self {
            min_confirmations,
            params: Params::mainnet(),
            anchor: AnchorFns {
                height: A::best_chain_height,
                best: A::best_height,
            },
        }
    }

    /// Anchored to `A`, with the confirmation policy from `x3-bitcoin-vault` (or a route's
    /// stricter requirement).
    pub fn from_vault_defaults<A: BtcHeaderAnchor>(route_requirement: u64) -> Self {
        Self::anchored_by::<A>(route_requirement.max(x3_bitcoin_vault::MIN_BITCOIN_CONFIRMATIONS))
    }

    /// Adopt the vault's confirmation policy, never weakening the current one.
    pub fn with_vault_config(mut self, config: &x3_bitcoin_vault::BtcVaultConfig) -> Self {
        self.min_confirmations = self.min_confirmations.max(config.min_confirmations);
        self
    }

    /// Check headers' own proof of work under other network rules (regtest, for tests). The
    /// anchor still decides which blocks count.
    pub fn with_params(mut self, params: Params) -> Self {
        self.params = params;
        self
    }
}

impl Verifier for BitcoinSpvVerifier {
    fn strategy(&self) -> VerificationStrategy {
        VerificationStrategy::BitcoinSpvProof
    }

    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        if proof.source_chain != ChainKind::Bitcoin {
            return Err(VerificationError::UnsupportedChain);
        }
        let p = decode(&proof.payload)?;

        // The header is checked on its own first (cheap, and refuses junk before storage reads),
        // then it has to be a block this chain already holds on its best chain.
        check_proof_of_work(&p.header, &self.params)
            .map_err(|_| VerificationError::MalformedProof)?;
        let block_hash = p.header.block_hash();
        let height = (self.anchor.height)(&block_hash).ok_or(VerificationError::UnanchoredBlock)?;
        let best = (self.anchor.best)().ok_or(VerificationError::UnanchoredBlock)?;
        let confirmations = best
            .checked_sub(height)
            .and_then(|d| d.checked_add(1))
            .ok_or(VerificationError::UnanchoredBlock)?;
        if confirmations < self.min_confirmations {
            return Err(VerificationError::InsufficientConfirmations);
        }

        let txid = verify_transaction(p.raw_tx, p.tx_index, &p.path, &p.header.merkle_root)
            .map_err(|_| VerificationError::MalformedProof)?;
        if proof.proof_id != bitcoin_deposit_id(&txid, p.vout) {
            return Err(VerificationError::DepositMismatch);
        }
        let outputs =
            transaction_outputs(p.raw_tx).map_err(|_| VerificationError::MalformedProof)?;
        let paid = outputs
            .get(p.vout as usize)
            .ok_or(VerificationError::DepositMismatch)?;
        if paid.script_pubkey != proof.expected_recipient
            || u128::from(paid.value) != proof.expected_amount
        {
            return Err(VerificationError::DepositMismatch);
        }
        let commitment = deposit_commitment_script(&proof.expected_beneficiary);
        if proof.expected_beneficiary.is_empty()
            || !outputs.iter().any(|o| o.script_pubkey == commitment)
        {
            return Err(VerificationError::DepositMismatch);
        }

        Ok(VerificationOutcome {
            accepted: true,
            reason: "bitcoin_spv_proof_verified",
            verified_at_height: Some(height),
        })
    }
}
