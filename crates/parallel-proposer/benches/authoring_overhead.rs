//! Faithful micro-benchmark of the per-tx authoring overhead in
//! `apply_extrinsics_parallel` (crates/parallel-proposer/src/substrate.rs).
//!
//! For every queued extrinsic, each proposal slot pays:
//!   (a) `data.clone()`                  - copy encoded tx bytes
//!   (b) `encoded_size()`                - SCALE length computation
//!   (c) `BlakeTwo256::hash_of(&data)`   - order hash
//!   (d) `extract_tx_metadata(data, tx_hash)` - full `UncheckedExtrinsic::decode`
//!       of already-encoded bytes, then `function.encode()` re-encode, purely to
//!       read sender / nonce / a 4-byte call selector.
//!
//! This harness builds REAL runtime extrinsics of realistic sizes and times the
//! real `extract_tx_metadata` imported from the crate plus the hashing work,
//! against an irreducible hash-only minimum. It links the real runtime and calls
//! the real decode path, so the numbers reflect production behaviour here.
//!
//! Run: cargo bench -p parallel-proposer --bench authoring_overhead

use codec::{Decode, Encode};
use parallel_proposer::extract_tx_metadata;
use sp_core::{sr25519, Pair};
use sp_runtime::{
    traits::{BlakeTwo256, Hash, Verify},
};
use std::time::Instant;
use x3_chain_runtime::{
    AccountId, Address, RuntimeCall, Signature, SignedExtra, SignedPayload, UncheckedExtrinsic,
};

fn account_from_public(public: sr25519::Public) -> AccountId {
    use sp_runtime::traits::IdentifyAccount;
    <Signature as Verify>::Signer::from(public).into_account()
}

/// Build a real, correctly-signed `remark` extrinsic for the X3 runtime.
/// Mirrors the signed-extrinsic construction in `node/src/rpc.rs` verbatim so
/// the same runtime types, SignedExtra tuple and additional-signed payload are
/// used.
fn make_remark(pair: sr25519::Pair, raw: Vec<u8>, nonce: u32, genesis: [u8; 32]) -> Vec<u8> {
    use frame_system::Call as SysCall;
    use pallet_x3_agent_law::AgentLawCheck;
    use pallet_x3_invariants::InvariantCheck;
    use sp_runtime::generic::Era;

    let account = account_from_public(pair.public());
    let call: RuntimeCall =
        RuntimeCall::System(SysCall::<x3_chain_runtime::Runtime>::remark { remark: raw });

    let extra: SignedExtra = (
        frame_system::CheckNonZeroSender::<x3_chain_runtime::Runtime>::new(),
        frame_system::CheckSpecVersion::<x3_chain_runtime::Runtime>::new(),
        frame_system::CheckTxVersion::<x3_chain_runtime::Runtime>::new(),
        frame_system::CheckGenesis::<x3_chain_runtime::Runtime>::new(),
        frame_system::CheckEra::<x3_chain_runtime::Runtime>::from(Era::Immortal),
        frame_system::CheckNonce::<x3_chain_runtime::Runtime>::from(nonce),
        frame_system::CheckWeight::<x3_chain_runtime::Runtime>::new(),
        pallet_transaction_payment::ChargeTransactionPayment::<x3_chain_runtime::Runtime>::from(
            0,
        ),
        InvariantCheck::<x3_chain_runtime::Runtime>::new(),
        AgentLawCheck::<x3_chain_runtime::Runtime>::decode(&mut &[][..])
            .expect("AgentLawCheck decodes from empty bytes"),
    );

    let genesis_hash = sp_core::H256::from(genesis);
    let payload = SignedPayload::from_raw(
        call.clone(),
        extra.clone(),
        (
            (),
            x3_chain_runtime::VERSION.spec_version,
            x3_chain_runtime::VERSION.transaction_version,
            genesis_hash,
            genesis_hash,
            (),
            (),
            (),
            (),
            (),
        ),
    );
    let signature = payload.using_encoded(|payload| Signature::from(pair.sign(payload)));
    let extrinsic =
        UncheckedExtrinsic::new_signed(call, Address::Id(account), signature, extra);
    extrinsic.encode()
}

fn main() {
    let rounds: u64 = 300;
    let sizes: [usize; 4] = [128, 512, 1_024, 4_096];

    let seed_pair = sr25519::Pair::from_string("//X3Bench/0", None).expect("valid seed");
    let genesis = [0x11u8; 32];

    println!("parallel-proposer single-tx authoring cost (real runtime extrinsics)");
    println!(
        "{:<10} {:<18} {:<18} {:<14} {:<10}",
        "tx_bytes", "hash_ns/tx", "metadata_ns/tx", "overhead_ns", "overhead_x"
    );

    for &sz in &sizes {
        let raw = vec![0xABu8; sz];
        let encoded = make_remark(seed_pair.clone(), raw, 0, genesis);

        // (A) irreducible: hash only
        let t0 = Instant::now();
        let mut sink = 0u64;
        for _ in 0..rounds {
            let h = BlakeTwo256::hash(&encoded);
            sink ^= h.as_ref()[0] as u64;
        }
        let hash_ns = t0.elapsed().as_nanos() as f64 / (rounds as f64);

        // (B) full current path: order-hash + real metadata decode
        let t1 = Instant::now();
        let mut sink2 = 0u64;
        for _ in 0..rounds {
            let h = BlakeTwo256::hash(&encoded);
            let meta = extract_tx_metadata(&encoded, *h.as_fixed_bytes());
            sink2 ^= hash_idx_compat(&meta.sender, meta.nonce);
        }
        let meta_ns = t1.elapsed().as_nanos() as f64 / (rounds as f64);

        let overhead = meta_ns - hash_ns;
        let x = if hash_ns > 0.0 { meta_ns / hash_ns } else { 0.0 };
        println!(
            "{:<10} {:<18.1} {:<18.1} {:<14.1} {:<10.2}x",
            sz, hash_ns, meta_ns, overhead, x
        );
        black_box(sink ^ sink2);
    }

    // Pool-walk cost model, exactly as apply_extrinsics_parallel spends it per tx.
    let pools: [usize; 4] = [1_000, 10_000, 50_000, 200_000];
    let len = 512;
    let raw = vec![0xCDu8; len];
    println!();
    println!(
        "pipeline pool-walk per-tx cost over {} rounds at {len}B txs",
        rounds
    );
    println!(
        "{:<10} {:<18} {:<20} {:<14}",
        "pool_size", "hash_only_ns/tx", "full_walk_ns/tx", "full/hash_x"
    );
    for &pool in &pools {
        let txs: Vec<Vec<u8>> = (0..pool)
            .map(|n| make_remark(seed_pair.clone(), raw.clone(), (n % 1_000) as u32, genesis))
            .collect();

        let mut sink = 0u64;
        let ta = Instant::now();
        for _ in 0..50 {
            for tx in &txs {
                let h = BlakeTwo256::hash(tx);
                sink ^= h.as_ref()[0] as u64;
            }
        }
        let hash_only = ta.elapsed().as_nanos() as f64 / (50.0 * pool as f64);

        let mut sink2 = 0u64;
        let tb = Instant::now();
        for _ in 0..50 {
            for tx in &txs {
                // exact operations in apply_extrinsics_parallel pre-push
                let data = tx.clone();
                let _size = data.encoded_size();
                let h = BlakeTwo256::hash(&data);
                let meta = extract_tx_metadata(&data, *h.as_fixed_bytes());
                sink2 ^= hash_idx_compat(&meta.sender, meta.nonce);
            }
        }
        let full = tb.elapsed().as_nanos() as f64 / (50.0 * pool as f64);
        let x = if hash_only > 0.0 { full / hash_only } else { 0.0 };
        println!(
            "{:<10} {:<18.1} {:<20.1} {:<14.2}x",
            pool, hash_only, full, x
        );
        black_box(sink ^ sink2);
    }
}

fn hash_idx_compat(sender: &[u8; 32], nonce: u64) -> u64 {
    let mut h = 0x9E37u64;
    for (i, b) in sender.iter().take(8).enumerate() {
        h ^= (*b as u64) << (8 * (i % 8));
    }
    h ^ nonce
}

#[inline(never)]
fn black_box<T>(_: T) {}
