//! An EIP-155 transaction must carry `r` and `s` as *minimal* RLP integers.
//!
//! This is not cosmetic. An ECDSA signature is 32-byte fixed width, and roughly
//! one signature in 128 has a leading zero byte in `r` or `s`. Writing those 32
//! bytes into the transaction verbatim produces an RLP integer with a leading
//! zero — a non-canonical encoding that anvil/reth reject at the RPC boundary:
//!
//! ```text
//! eth_sendRawTransaction -> -32602 Failed to decode transaction
//! ```
//!
//! which is how a test that signs and broadcasts intermittently fails: the
//! transaction is well-formed RLP, it is just not a valid transaction. The scan
//! below walks fixed (key, nonce) pairs — deterministic, no RNG — until it finds
//! such a signature, asserts it found one (so the test cannot pass vacuously),
//! and then asserts every signed transaction in the scan is minimally encoded.

#![cfg(feature = "std")]

use x3_atomic_swap::ethereum_tx::Transaction;

/// Anvil's first development key. Its signatures are as good as any other's for
/// this purpose: the leading-byte distribution does not depend on the key.
const DEV_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

/// How many nonces to walk before giving up on finding the dangerous signature.
/// One signature in ~128 has a zero leading byte, so this is ~1 - 2^-32.
const SCAN: u64 = 4096;

fn signature_fields(raw: &[u8]) -> ([u8; 32], [u8; 32]) {
    let rlp = rlp::Rlp::new(raw);
    assert_eq!(
        rlp.item_count()
            .expect("the signed transaction is an RLP list"),
        9,
        "a legacy EIP-155 transaction is nine fields"
    );
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    // `data()` returns the field exactly as encoded. A minimal 32-byte integer
    // round-trips through it unchanged; a non-minimal one arrives with the zero
    // still in front.
    let r_bytes = rlp.at(7).expect("r field").data().expect("r data");
    let s_bytes = rlp.at(8).expect("s field").data().expect("s data");
    if r_bytes.len() == 32 {
        r.copy_from_slice(r_bytes);
    }
    if s_bytes.len() == 32 {
        s.copy_from_slice(s_bytes);
    }
    (r, s)
}

#[test]
fn signed_transactions_encode_r_and_s_as_minimal_integers() {
    let mut dangerous: Option<(u64, String)> = None;
    let mut checked = 0u64;

    for nonce in 0..SCAN {
        let tx = Transaction {
            nonce,
            gas_price: 1_000_000_000,
            gas_limit: 100_000,
            to: Some("0x70997970c51812dc3a010c7d01b50e0d17dc79c8".to_string()),
            value: 0,
            data: "0xdeadbeef".to_string(),
            chain_id: 31337,
        };
        let signed = tx.sign(DEV_KEY).expect("signing works");
        let raw = hex::decode(signed.trim_start_matches("0x")).expect("hex");
        let (r, s) = signature_fields(&raw);

        // What the fixed-width signature looked like before encoding. When this
        // is zero the fixed-width write is a non-minimal integer.
        let preimage_has_leading_zero = r[0] == 0 || s[0] == 0;
        if preimage_has_leading_zero && dangerous.is_none() {
            dangerous = Some((nonce, signed.clone()));
        }

        if r[0] == 0 && r != [0u8; 32] {
            panic!(
                "nonce {nonce}: r is encoded with a leading zero byte, which no EVM node \
                 accepts: {signed}"
            );
        }
        if s[0] == 0 && s != [0u8; 32] {
            panic!(
                "nonce {nonce}: s is encoded with a leading zero byte, which no EVM node \
                 accepts: {signed}"
            );
        }
        checked += 1;
    }

    assert_eq!(checked, SCAN, "the whole scan ran");
    let (nonce, signed) = dangerous.expect(
        "the scan must actually contain a signature with a leading zero byte, otherwise \
         this test proves nothing about the encoding",
    );
    // Printed for `--nocapture`: this is the transaction that used to be refused
    // with `-32602 Failed to decode transaction`, and it is the one to hand to a
    // node when checking the fix by hand.
    println!("the scan contained a leading-zero signature at nonce {nonce}: {signed}");
}
