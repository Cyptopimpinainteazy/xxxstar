//! Every external chain's proof verifier must refuse every proof type.
//!
//! The public-testnet rule for this area is explicit: an EVM/SVM *external*
//! path is either proven against a real testnet or it is disabled. Nothing in
//! this workspace has verified a settlement proof against a public network, so
//! every path in `x3-external-chains` must refuse — for every chain in the
//! registry, not just the single chain `settlement.rs`'s inline unit test names.
//!
//! `SettlementVerifier`'s `verify_*` bodies used to be data-shape checks
//! reported as verification (`Ok(!receipt_proof.is_empty())` and friends), which
//! a caller cannot tell apart from a real verifier. They now return
//! `VerificationUnavailable`. This sweep is the load-bearing half of that
//! guarantee: re-adding a permissive body for *any* chain fails here, not only
//! for the one chain an inline test happens to cover.
//!
//! This is not a duplicate of
//! `settlement.rs::every_proof_type_is_refused_while_unimplemented`, which
//! sweeps the five proof types against Polygon alone. This one sweeps all six
//! `ChainType` variants against all five proof types, so a chain that grows a
//! real verifier cannot be left half-wired while the Polygon test stays green.
//!
//! When a real per-chain verifier does land, this test must be narrowed to the
//! chains that still have none. That edit *is* the evidence the path changed —
//! the whole point is that turning an external path on has to be a deliberate,
//! reviewed diff rather than a default.

use sp_core::{H160, H256, U256};
use x3_external_chains::adapter::{CrossChainTransfer, TransferStatus};
use x3_external_chains::{
    ChainType, ExternalChainError, ProofType, SettlementConfig, SettlementProof, SettlementVerifier,
};

/// Every variant of [`ChainType`].
///
/// The crate's own `SettlementConfig::for_chain` matches `ChainType`
/// exhaustively, so adding a variant is already a compile error there; this
/// array has to be extended in the same change, which is how a new chain gets
/// pulled into the refusal sweep.
const ALL_CHAINS: [ChainType; 6] = [
    ChainType::Base,
    ChainType::Arbitrum,
    ChainType::Polygon,
    ChainType::Avalanche,
    ChainType::Bnb,
    // The local chain. It has no external settlement verifier either, and a
    // permissive body here would be just as indistinguishable from a real one.
    ChainType::AtlasSphere,
];

/// Every variant of [`ProofType`]; `SettlementVerifier::verify_proof` matches
/// these exhaustively, so a new one cannot be added without being covered.
const ALL_PROOF_TYPES: [ProofType; 5] = [
    ProofType::MerkleTrie,
    ProofType::LightClient,
    ProofType::ZkProof,
    ProofType::Signature,
    ProofType::Optimistic,
];

fn transfer() -> CrossChainTransfer {
    CrossChainTransfer {
        id: H256::from([0xAA; 32]),
        source_chain: 1,
        dest_chain: 2,
        source_token: H160::zero(),
        dest_token: H160::zero(),
        sender: H160::from([0x01; 20]),
        recipient: H160::from([0x02; 20]),
        amount: U256::from(1_000u64),
        fee: U256::zero(),
        status: TransferStatus::Pending,
        source_tx: None,
        dest_tx: None,
    }
}

/// A proof with deliberately non-empty blobs: these are exactly the shapes the
/// old data-shape checks accepted as "verified".
fn proof_with(proof_type: ProofType) -> SettlementProof {
    SettlementProof {
        proof_type,
        source_block: 42,
        source_block_hash: H256::from([0xBB; 32]),
        source_tx_hash: H256::from([0xCC; 32]),
        merkle_proof: vec![H256::from([0xDD; 32])],
        receipt_proof: vec![0x01, 0x02, 0x03],
        witness: vec![0xAB; 65 * 4],
    }
}

/// No external chain may verify a proof of any type while it has no verifier.
#[test]
fn every_chain_refuses_every_proof_type_it_has_no_verifier_for() {
    for chain in ALL_CHAINS {
        for proof_type in ALL_PROOF_TYPES {
            let verifier = SettlementVerifier::with_config(SettlementConfig {
                proof_type,
                ..SettlementConfig::for_chain(chain)
            });
            let result = verifier.verify_proof(&transfer(), &proof_with(proof_type));
            assert!(
                matches!(result, Err(ExternalChainError::VerificationUnavailable)),
                "{chain:?} answered {result:?} for {proof_type:?}: an external proof this crate \
                 cannot verify must be refused as `VerificationUnavailable`, never accepted"
            );
        }
    }
}

/// The fund-safety property, stated on its own so a regression cannot hide
/// behind an error variant: no chain/proof-type pair may answer `true`.
#[test]
fn no_external_chain_accepts_a_proof_it_cannot_verify() {
    for chain in ALL_CHAINS {
        for proof_type in ALL_PROOF_TYPES {
            let verifier = SettlementVerifier::with_config(SettlementConfig {
                proof_type,
                ..SettlementConfig::for_chain(chain)
            });
            let result = verifier.verify_proof(&transfer(), &proof_with(proof_type));
            assert!(
                !matches!(result, Ok(true)),
                "{chain:?} accepted {proof_type:?} it cannot verify: {result:?}"
            );
        }
    }
}

/// Control, so the sweeps above are not vacuously satisfied.
///
/// A proof of the *wrong* type is answered `Ok(false)` ("invalid"), not an
/// error. That is what proves the `VerificationUnavailable` above comes from a
/// verifier body rather than from the type check swallowing every proof.
#[test]
fn a_wrong_type_proof_is_invalid_not_unavailable() {
    let verifier = SettlementVerifier::new(ChainType::Polygon); // default: MerkleTrie
    let result = verifier.verify_proof(&transfer(), &proof_with(ProofType::ZkProof));
    assert!(
        matches!(result, Ok(false)),
        "a mismatched proof type must read as `Ok(false)`, got {result:?}"
    );
}
