//! Bitcoin deposit proofs: what the gateway accepts and the forgeries it refuses.

use std::cell::RefCell;
use std::collections::BTreeMap;

use x3_btc_spv::{check_proof_of_work, merkle_root_from_path, sha256d, BlockHeader, Hash, Params};
use x3_verification_router::{
    bitcoin_deposit_id, deposit_commitment_script, encode_btc_proof_v1, BitcoinSpvVerifier,
    BtcHeaderAnchor, ChainKind, NoBtcHeaderAnchor, ProofEnvelope, VerificationError,
    VerificationStrategy, Verifier,
};

thread_local! {
    static CHAIN: RefCell<(BTreeMap<Hash, u64>, Option<u64>)> =
        const { RefCell::new((BTreeMap::new(), None)) };
}

struct TestAnchor;

impl BtcHeaderAnchor for TestAnchor {
    fn best_chain_height(block_hash: &Hash) -> Option<u64> {
        CHAIN.with(|c| c.borrow().0.get(block_hash).copied())
    }
    fn best_height() -> Option<u64> {
        CHAIN.with(|c| c.borrow().1)
    }
}

fn anchor(block: Hash, height: u64, best: u64) {
    CHAIN.with(|c| {
        let mut c = c.borrow_mut();
        c.0.insert(block, height);
        c.1 = Some(best);
    });
}

const VAULT: [u8; 22] = [
    0x00, 0x14, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
];
const BENEFICIARY: &[u8] = b"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
const AMOUNT: u64 = 150_000;

/// A legacy-serialized transaction: one input, the given outputs.
fn transaction(outputs: &[(u64, Vec<u8>)]) -> Vec<u8> {
    let mut tx = 2u32.to_le_bytes().to_vec();
    tx.push(1);
    tx.extend_from_slice(&[0x11; 32]);
    tx.extend_from_slice(&0u32.to_le_bytes());
    tx.push(0);
    tx.extend_from_slice(&u32::MAX.to_le_bytes());
    tx.push(outputs.len() as u8);
    for (value, script) in outputs {
        tx.extend_from_slice(&value.to_le_bytes());
        tx.push(script.len() as u8);
        tx.extend_from_slice(script);
    }
    tx.extend_from_slice(&0u32.to_le_bytes());
    tx
}

fn deposit_tx() -> Vec<u8> {
    transaction(&[
        (AMOUNT, VAULT.to_vec()),
        (0, deposit_commitment_script(BENEFICIARY)),
    ])
}

struct Block {
    header: BlockHeader,
    coinbase: Hash,
}

/// A regtest block holding a stand-in coinbase at 0 and `tx` at 1.
fn block_with(tx: &[u8]) -> Block {
    let coinbase = sha256d(b"coinbase");
    let root = merkle_root_from_path(&coinbase, 0, &[sha256d(tx)]).unwrap();
    let mut header = BlockHeader {
        version: 4,
        prev_blockhash: [9; 32],
        merkle_root: root,
        time: 1_700_000_000,
        bits: 0x207fffff,
        nonce: 0,
    };
    while check_proof_of_work(&header, &Params::regtest()).is_err() {
        header.nonce += 1;
    }
    Block { header, coinbase }
}

fn envelope(payload: Vec<u8>, proof_id: [u8; 32]) -> ProofEnvelope {
    ProofEnvelope {
        proof_id,
        strategy: VerificationStrategy::BitcoinSpvProof,
        source_chain: ChainKind::Bitcoin,
        destination_chain: ChainKind::X3,
        payload,
        expected_asset_id: [0; 32],
        expected_amount: u128::from(AMOUNT),
        expected_sender: Vec::new(),
        expected_recipient: VAULT.to_vec(),
        expected_beneficiary: BENEFICIARY.to_vec(),
    }
}

fn verifier() -> BitcoinSpvVerifier {
    BitcoinSpvVerifier::anchored_by::<TestAnchor>(6).with_params(Params::regtest())
}

/// The honest proof of `deposit_tx` at vout 0, with its block anchored six deep.
fn honest() -> (ProofEnvelope, Block) {
    let tx = deposit_tx();
    let block = block_with(&tx);
    anchor(block.header.block_hash(), 100, 105);
    let payload = encode_btc_proof_v1(&block.header, 1, &[block.coinbase], 0, &tx);
    (
        envelope(payload, bitcoin_deposit_id(&sha256d(&tx), 0)),
        block,
    )
}

#[test]
fn an_anchored_deposit_that_pays_the_vault_and_names_its_beneficiary_is_accepted() {
    let (proof, _) = honest();
    let outcome = verifier().verify(&proof).expect("verifies");
    assert!(outcome.accepted);
    assert_eq!(outcome.verified_at_height, Some(100));
}

#[test]
fn a_block_the_chain_does_not_hold_is_refused_whatever_the_proof_says() {
    let (proof, _) = honest();
    let unanchored =
        BitcoinSpvVerifier::anchored_by::<NoBtcHeaderAnchor>(6).with_params(Params::regtest());
    assert_eq!(
        unanchored.verify(&proof),
        Err(VerificationError::UnanchoredBlock)
    );
}

#[test]
fn a_shallow_block_is_refused() {
    let (proof, block) = honest();
    anchor(block.header.block_hash(), 100, 104);
    assert_eq!(
        verifier().verify(&proof),
        Err(VerificationError::InsufficientConfirmations)
    );
    // A route may demand more than the vault's default, never less.
    anchor(block.header.block_hash(), 100, 105);
    let strict =
        BitcoinSpvVerifier::from_vault_defaults::<TestAnchor>(50).with_params(Params::regtest());
    assert_eq!(strict.min_confirmations, 50);
    assert_eq!(
        strict.verify(&proof),
        Err(VerificationError::InsufficientConfirmations)
    );
    assert!(BitcoinSpvVerifier::from_vault_defaults::<TestAnchor>(0).min_confirmations >= 1);
}

#[test]
fn the_deposit_must_be_exactly_the_one_the_envelope_describes() {
    let (proof, _) = honest();
    let v = verifier();

    let mut more = proof.clone();
    more.expected_amount += 1;
    assert_eq!(v.verify(&more), Err(VerificationError::DepositMismatch));

    let mut elsewhere = proof.clone();
    elsewhere.expected_recipient[2] ^= 1;
    assert_eq!(
        v.verify(&elsewhere),
        Err(VerificationError::DepositMismatch)
    );

    // Someone who saw the deposit cannot claim it for themselves.
    let mut thief = proof.clone();
    thief.expected_beneficiary = b"someone else".to_vec();
    assert_eq!(v.verify(&thief), Err(VerificationError::DepositMismatch));

    // One id per outpoint: a second, chosen id for the same deposit is refused.
    let mut replay = proof.clone();
    replay.proof_id = [7; 32];
    assert_eq!(v.verify(&replay), Err(VerificationError::DepositMismatch));
}

#[test]
fn the_commitment_output_cannot_be_claimed_as_the_payment() {
    let tx = deposit_tx();
    let block = block_with(&tx);
    anchor(block.header.block_hash(), 100, 105);
    let payload = encode_btc_proof_v1(&block.header, 1, &[block.coinbase], 1, &tx);
    let mut proof = envelope(payload, bitcoin_deposit_id(&sha256d(&tx), 1));
    proof.expected_amount = 0;
    assert_eq!(
        verifier().verify(&proof),
        Err(VerificationError::DepositMismatch)
    );
}

#[test]
fn a_transaction_without_a_beneficiary_commitment_is_refused() {
    let tx = transaction(&[(AMOUNT, VAULT.to_vec())]);
    let block = block_with(&tx);
    anchor(block.header.block_hash(), 100, 105);
    let payload = encode_btc_proof_v1(&block.header, 1, &[block.coinbase], 0, &tx);
    let proof = envelope(payload, bitcoin_deposit_id(&sha256d(&tx), 0));
    assert_eq!(
        verifier().verify(&proof),
        Err(VerificationError::DepositMismatch)
    );
}

#[test]
fn inclusion_is_bitcoins_rule_with_position() {
    let (honest_proof, block) = honest();
    let tx = deposit_tx();
    // The transaction at position 0 instead of 1.
    let payload = encode_btc_proof_v1(&block.header, 0, &[block.coinbase], 0, &tx);
    assert_eq!(
        verifier().verify(&envelope(payload, honest_proof.proof_id)),
        Err(VerificationError::MalformedProof)
    );
}

#[test]
fn the_previous_formats_forgery_is_refused() {
    // The payload the old verifier accepted: tip 200, one all-zero header with an easy target
    // mined in 2561 tries, transaction "index" 100, an all-zero txid.
    let mut old = Vec::new();
    old.extend_from_slice(&200u64.to_le_bytes());
    old.extend_from_slice(&1u32.to_le_bytes());
    let mut header = [0u8; 80];
    header[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
    header[76..80].copy_from_slice(&2561u32.to_le_bytes());
    old.extend_from_slice(&header);
    old.extend_from_slice(&100u32.to_le_bytes());
    old.extend_from_slice(&0u32.to_le_bytes());
    old.extend_from_slice(&[0u8; 32]);
    let mainnet = BitcoinSpvVerifier::anchored_by::<TestAnchor>(6);
    assert_eq!(
        mainnet.verify(&envelope(old, [0; 32])),
        Err(VerificationError::UnsupportedProofFormat)
    );

    // The same header in the current format: its target is easier than mainnet allows.
    let forged = BlockHeader::decode(&header).unwrap();
    anchor(forged.block_hash(), 100, 200);
    let payload = encode_btc_proof_v1(&forged, 0, &[], 0, &deposit_tx());
    assert_eq!(
        mainnet.verify(&envelope(payload, [0; 32])),
        Err(VerificationError::MalformedProof)
    );
}

#[test]
fn malformed_payloads_and_witness_serializations_are_refused() {
    let (proof, block) = honest();
    let mut trailing = proof.clone();
    trailing.payload.push(0);
    assert_eq!(
        verifier().verify(&trailing),
        Err(VerificationError::MalformedProof)
    );

    let mut short = proof.clone();
    short.payload.truncate(50);
    assert_eq!(
        verifier().verify(&short),
        Err(VerificationError::MalformedProof)
    );

    // Marker and flag after the version: the wtxid form, which no Merkle proof binds.
    let mut witness = deposit_tx();
    witness.splice(4..4, [0x00, 0x01]);
    let payload = encode_btc_proof_v1(&block.header, 1, &[block.coinbase], 0, &witness);
    assert_eq!(
        verifier().verify(&envelope(payload, proof.proof_id)),
        Err(VerificationError::MalformedProof)
    );

    let mut other_chain = proof;
    other_chain.source_chain = ChainKind::Solana;
    assert_eq!(
        verifier().verify(&other_chain),
        Err(VerificationError::UnsupportedChain)
    );
}
