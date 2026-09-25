//! Robustness of the cross-chain evidence decoder.
//!
//! `evm_receipt` is a trust boundary: `DecodedProof::decode` parses bytes a relayer
//! supplies, and `validate` decides whether an EVM leg happened. Those bytes are chosen by
//! whoever submits the proof, so the decoder's behaviour on *damaged* input is part of the
//! security surface rather than a hygiene concern.
//!
//! The proof this file damages is built by the module's own producer — the same
//! `receipts_trie_root` / `receipts_trie_proof` / `encode_proof_payload` the node uses —
//! and the first test asserts it decodes *and* validates, so every sweep below provably
//! starts from a real proof rather than from something that fails early.
//!
//! # The wire format has two layers, and they are easy to confuse
//!
//! `encode_proof_payload` writes a 16-byte preamble (the prover's claimed head height and
//! its own minimum confirmations) followed by four length-prefixed sections.
//! `DecodedProof::decode` does **not** read the preamble: `ProductionEvmReceiptVerifier::verify`
//! consumes it first (`let body = &proof.payload[16..]`) and passes the rest. So the two
//! functions are not a matched pair — `decode(&encode_proof_payload(..))` returns
//! `TooShort`, because the preamble's first four bytes are read as a section length. This
//! file writes the split down as a test.

use x3_verification_router::evm_receipt::{
    deposit_locked_selector, encode_proof_payload, receipts_trie_proof, receipts_trie_root,
    DecodedProof,
};

const AMOUNT: u128 = 42;
const RECIPIENT: [u8; 20] = [0x11; 20];
const MIN_CONFIRMATIONS: u64 = 3;
const HEAD_NUMBER: u64 = 10;
const CHAIN_ID: u64 = 31337;

/// Bytes of preamble `encode_proof_payload` writes before the first section.
const PREAMBLE: usize = 16;

fn rlp_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut stream = rlp::RlpStream::new();
    stream.append(&bytes.to_vec());
    stream.out().to_vec()
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut stream = rlp::RlpStream::new_list(items.len());
    for item in items {
        stream.append_raw(item, 1);
    }
    stream.out().to_vec()
}

/// The receipt the proof is about: one `DepositLocked`-shaped log carrying the amount.
fn receipt_rlp() -> Vec<u8> {
    let mut data = [0u8; 32];
    data[16..].copy_from_slice(&AMOUNT.to_be_bytes());
    let log = rlp_list(&[
        rlp_bytes(&RECIPIENT),
        rlp_list(&[rlp_bytes(&deposit_locked_selector())]),
        rlp_bytes(&data),
    ]);
    // status = 1, cumulative gas = 0, logs = [log]
    rlp_list(&[rlp_bytes(&[1u8]), rlp_bytes(&[]), rlp_list(&[log])])
}

/// A 15-element EIP-1186 header whose `receiptsRoot` is the root of that receipt.
fn header_rlp(receipts_root: &[u8; 32]) -> Vec<u8> {
    rlp_list(&[
        rlp_bytes(&[0u8; 32]),    // 0 parentHash
        rlp_bytes(&[]),           // 1 sha3Uncles
        rlp_bytes(&RECIPIENT),    // 2 beneficiary
        rlp_bytes(&[1u8; 32]),    // 3 stateRoot
        rlp_bytes(&[2u8; 32]),    // 4 transactionsRoot
        rlp_bytes(receipts_root), // 5 receiptsRoot
        rlp_bytes(&[]),           // 6 logsBloom
        rlp_bytes(&[]),           // 7 difficulty
        rlp_bytes(&[2u8]),        // 8 number = 2
        rlp_bytes(&[]),           // 9 gasLimit
        rlp_bytes(&[]),           // 10 gasUsed
        rlp_bytes(&[5u8]),        // 11 timestamp
        rlp_bytes(&[]),           // 12 extraData
        rlp_bytes(&[0u8; 32]),    // 13 mixHash
        rlp_bytes(&[0u8; 8]),     // 14 nonce
    ])
}

struct Fixture {
    /// The whole wire payload, preamble included, as a relayer would submit it.
    payload: Vec<u8>,
    receipts_root: [u8; 32],
}

impl Fixture {
    /// The part `DecodedProof::decode` takes: everything after the preamble.
    fn body(&self) -> &[u8] {
        &self.payload[PREAMBLE..]
    }
}

fn fixture() -> Fixture {
    let receipt = receipt_rlp();
    let receipts = vec![receipt.clone()];
    let root = receipts_trie_root(&receipts).expect("the producer must build a root");
    let proof = receipts_trie_proof(&receipts, 0).expect("the producer must build a proof");
    let index = vec![0u8];
    let payload = encode_proof_payload(
        HEAD_NUMBER,
        MIN_CONFIRMATIONS,
        &header_rlp(&root),
        &receipt,
        &index,
        &proof,
    );
    Fixture {
        payload,
        receipts_root: root,
    }
}

fn decode(
    body: &[u8],
) -> Result<DecodedProof, x3_verification_router::evm_receipt::EvmReceiptError> {
    DecodedProof::decode(
        body,
        AMOUNT,
        RECIPIENT,
        deposit_locked_selector(),
        MIN_CONFIRMATIONS,
        HEAD_NUMBER,
        CHAIN_ID,
    )
}

#[test]
fn the_fixture_is_a_proof_that_decodes_and_validates() {
    // Without this, every sweep below could be passing because the starting bytes were
    // already refused by the first length check.
    let f = fixture();
    let decoded = decode(f.body()).expect("the fixture body must decode");
    decoded
        .validate()
        .expect("the fixture must pass the inclusion walk");
}

#[test]
fn the_preamble_belongs_to_the_verifier_not_to_the_decoder() {
    // Pinning the two-layer format, because getting it wrong silently turns a valid proof
    // into `TooShort` at the first section length — which is how this test file was
    // written the first time.
    let f = fixture();
    assert!(f.payload.len() > PREAMBLE);
    assert!(
        decode(&f.payload).is_err(),
        "decode must not be handed the preamble; the verifier strips it"
    );
    let head = u64::from_le_bytes(f.payload[0..8].try_into().expect("8 bytes"));
    assert_eq!(head, HEAD_NUMBER, "the preamble carries the prover's head");
    let min = u64::from_le_bytes(f.payload[8..16].try_into().expect("8 bytes"));
    assert_eq!(min, MIN_CONFIRMATIONS, "and its own minimum confirmations");
    assert!(decode(f.body()).is_ok(), "the body is what decode takes");
}

#[test]
fn every_truncation_of_the_body_is_refused() {
    let f = fixture();
    for len in 0..f.body().len() {
        assert!(
            decode(&f.body()[..len]).is_err(),
            "a {len}-byte prefix of a valid body decoded"
        );
    }
}

#[test]
fn no_single_byte_mutation_panics_the_decoder_or_the_walk() {
    let f = fixture();
    for offset in 0..f.body().len() {
        for value in [0x00u8, 0x01, 0x7F, 0x80, 0xFF] {
            let mut damaged = f.body().to_vec();
            if damaged[offset] == value {
                continue;
            }
            damaged[offset] = value;
            // The decoder must return, not abort. If it accepts the bytes, the walk has to
            // return too: a panic either way is what this test exists to catch.
            if let Ok(decoded) = decode(&damaged) {
                let _ = decoded.validate();
            }
        }
    }
}

#[test]
fn the_walk_is_bound_to_the_headers_receipts_root() {
    // `validate` checks inclusion against `header.receipts_root`, which the prover supplies.
    // Flipping any byte of it must therefore break the walk; if a mutated root still
    // validated, the header would not be what the proof is checked against.
    let f = fixture();
    let body = f.body().to_vec();
    let positions: Vec<usize> = body
        .windows(32)
        .enumerate()
        .filter(|(_, w)| *w == f.receipts_root)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        positions.len(),
        1,
        "the receipts root should appear exactly once in the body"
    );
    let start = positions[0];

    for offset in start..start + 32 {
        let mut damaged = body.clone();
        damaged[offset] ^= 0x01;
        if let Ok(decoded) = decode(&damaged) {
            assert!(
                decoded.validate().is_err(),
                "a proof whose receipts root was flipped at byte {offset} still validated"
            );
        }
    }
}

#[test]
fn trailing_bytes_do_not_change_what_the_proof_says() {
    // `decode` reads four self-delimiting sections and ignores whatever follows, which is a
    // deliberate property of the format. Pinned here so it is a decision rather than a
    // surprise: trailing bytes must not become a fifth section or alter the verdict.
    let f = fixture();
    let mut padded = f.body().to_vec();
    padded.extend_from_slice(&[0xAB; 64]);

    let base = decode(f.body()).expect("fixture body decodes");
    let with_tail = decode(&padded).expect("trailing bytes must not break decoding");
    assert_eq!(base.header.receipts_root, with_tail.header.receipts_root);
    assert_eq!(base.receipt_rlp, with_tail.receipt_rlp);
    assert_eq!(base.proof, with_tail.proof);
    with_tail
        .validate()
        .expect("trailing bytes must not change the verdict");
}
