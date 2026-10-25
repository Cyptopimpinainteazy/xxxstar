//! External-root registration proofs.
//!
//! The router's `register_external_root` stores a root that other code later
//! trusts. These adapters bind that root to an already-attested external block
//! and to the exact chain claimed by the caller.

extern crate alloc;

use alloc::vec::Vec;

/// Wire-format version for external-root binding proofs.
pub const EXTERNAL_ROOT_PROOF_V1: u8 = 1;

/// Chain ids used by the external-root proof envelope.
pub const EXTERNAL_ROOT_ETHEREUM: u32 = 1;
pub const EXTERNAL_ROOT_SOLANA: u32 = 2;
pub const EXTERNAL_ROOT_BITCOIN: u32 = 3;

/// The external header field whose value this registration treats as its root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    BlockHash = 0,
    StateRoot = 1,
    ReceiptsRoot = 2,
    ValidatorSetHash = 3,
}

impl RootKind {
    fn decode(value: u8) -> Result<Self, ExternalRootError> {
        match value {
            0 => Ok(Self::BlockHash),
            1 => Ok(Self::StateRoot),
            2 => Ok(Self::ReceiptsRoot),
            3 => Ok(Self::ValidatorSetHash),
            _ => Err(ExternalRootError::WrongRootKind),
        }
    }
}

/// Why an external-root proof was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalRootError {
    UnsupportedProofFormat,
    MalformedProof,
    UnsupportedChain,
    WrongChain,
    WrongRootKind,
    BlockMismatch,
    RootMismatch,
    UnanchoredBlock,
}

/// Decoded v1 external-root binding proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalRootProof {
    pub chain_id: u32,
    pub root_kind: RootKind,
    pub block_hash: [u8; 32],
}

/// The EVM header fields the runtime already holds for the claimed block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvmRootState {
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub receipts_root: [u8; 32],
}

/// The Solana slot fields the runtime already holds for the claimed block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvmRootState {
    pub slot: u64,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub validator_set_hash: [u8; 32],
}

/// The anchored Bitcoin header state for the claimed block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BtcRootState {
    pub height: u64,
    pub block_hash: [u8; 32],
    pub anchored: bool,
}

/// Encode the compact external-root binding proof.
pub fn encode_root_proof(chain_id: u32, kind: RootKind, block_hash: [u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(35);
    out.push(EXTERNAL_ROOT_PROOF_V1);
    out.push(chain_id as u8);
    out.push(kind as u8);
    out.extend_from_slice(&block_hash);
    out
}

/// Decode and validate the proof envelope against the caller's chain id.
pub fn decode_root_proof(
    expected_chain: u32,
    payload: &[u8],
) -> Result<ExternalRootProof, ExternalRootError> {
    if !matches!(
        expected_chain,
        EXTERNAL_ROOT_ETHEREUM | EXTERNAL_ROOT_SOLANA | EXTERNAL_ROOT_BITCOIN
    ) {
        return Err(ExternalRootError::UnsupportedChain);
    }
    let Some((&version, rest)) = payload.split_first() else {
        return Err(ExternalRootError::MalformedProof);
    };
    if version != EXTERNAL_ROOT_PROOF_V1 {
        return Err(ExternalRootError::UnsupportedProofFormat);
    }
    if rest.len() != 34 {
        return Err(ExternalRootError::MalformedProof);
    }
    let chain_id = u32::from(rest[0]);
    if chain_id != expected_chain {
        return Err(ExternalRootError::WrongChain);
    }
    let root_kind = RootKind::decode(rest[1])?;
    let mut block_hash = [0u8; 32];
    block_hash.copy_from_slice(&rest[2..]);
    Ok(ExternalRootProof {
        chain_id,
        root_kind,
        block_hash,
    })
}

/// Verify an EVM state or receipts root against the stored header.
pub fn verify_evm_root(
    expected_block: u64,
    root_hash: [u8; 32],
    payload: &[u8],
    state: EvmRootState,
) -> Result<(), ExternalRootError> {
    let proof = decode_root_proof(EXTERNAL_ROOT_ETHEREUM, payload)?;
    if expected_block != state.block_number || proof.block_hash != state.block_hash {
        return Err(ExternalRootError::BlockMismatch);
    }
    let stored_root = match proof.root_kind {
        RootKind::StateRoot => state.state_root,
        RootKind::ReceiptsRoot => state.receipts_root,
        _ => return Err(ExternalRootError::WrongRootKind),
    };
    if stored_root != root_hash {
        return Err(ExternalRootError::RootMismatch);
    }
    Ok(())
}

/// Verify a Solana state or validator-set root against the stored slot.
pub fn verify_svm_root(
    expected_slot: u64,
    root_hash: [u8; 32],
    payload: &[u8],
    state: SvmRootState,
) -> Result<(), ExternalRootError> {
    let proof = decode_root_proof(EXTERNAL_ROOT_SOLANA, payload)?;
    if expected_slot != state.slot || proof.block_hash != state.block_hash {
        return Err(ExternalRootError::BlockMismatch);
    }
    let stored_root = match proof.root_kind {
        RootKind::StateRoot => state.state_root,
        RootKind::ValidatorSetHash => state.validator_set_hash,
        _ => return Err(ExternalRootError::WrongRootKind),
    };
    if stored_root != root_hash {
        return Err(ExternalRootError::RootMismatch);
    }
    Ok(())
}

/// Verify that a Bitcoin registration names an anchored block on the stored best chain.
pub fn verify_btc_root(
    expected_height: u64,
    root_hash: [u8; 32],
    payload: &[u8],
    state: BtcRootState,
) -> Result<(), ExternalRootError> {
    let proof = decode_root_proof(EXTERNAL_ROOT_BITCOIN, payload)?;
    if proof.root_kind != RootKind::BlockHash {
        return Err(ExternalRootError::WrongRootKind);
    }
    if expected_height != state.height {
        return Err(ExternalRootError::BlockMismatch);
    }
    if root_hash != state.block_hash {
        return Err(ExternalRootError::RootMismatch);
    }
    if proof.block_hash != state.block_hash {
        return Err(ExternalRootError::BlockMismatch);
    }
    if !state.anchored {
        return Err(ExternalRootError::UnanchoredBlock);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ETH: u32 = 1;
    const SOL: u32 = 2;
    const BTC: u32 = 3;

    fn root(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn proof(chain: u32, kind: RootKind, block_hash: [u8; 32]) -> Vec<u8> {
        encode_root_proof(chain, kind, block_hash)
    }

    #[test]
    fn evm_state_root_binds_chain_block_and_root() {
        let state = EvmRootState {
            block_number: 42,
            block_hash: root(0x11),
            state_root: root(0x22),
            receipts_root: root(0x33),
        };
        assert_eq!(
            verify_evm_root(
                42,
                root(0x22),
                &proof(ETH, RootKind::StateRoot, root(0x11)),
                state
            ),
            Ok(())
        );
    }

    #[test]
    fn evm_receipts_root_is_bound_to_the_same_block() {
        let state = EvmRootState {
            block_number: 42,
            block_hash: root(0x11),
            state_root: root(0x22),
            receipts_root: root(0x33),
        };
        assert_eq!(
            verify_evm_root(
                42,
                root(0x33),
                &proof(ETH, RootKind::ReceiptsRoot, root(0x11)),
                state,
            ),
            Ok(())
        );
        assert_eq!(
            verify_evm_root(
                42,
                root(0x33),
                &proof(ETH, RootKind::ReceiptsRoot, root(0x12)),
                state,
            ),
            Err(ExternalRootError::BlockMismatch)
        );
    }

    #[test]
    fn evm_rejects_wrong_chain_root_kind_and_root_hash() {
        let state = EvmRootState {
            block_number: 42,
            block_hash: root(0x11),
            state_root: root(0x22),
            receipts_root: root(0x33),
        };
        assert_eq!(
            verify_evm_root(
                42,
                root(0x22),
                &proof(SOL, RootKind::StateRoot, root(0x11)),
                state
            ),
            Err(ExternalRootError::WrongChain)
        );
        assert_eq!(
            verify_evm_root(
                42,
                root(0x22),
                &proof(ETH, RootKind::ValidatorSetHash, root(0x11)),
                state,
            ),
            Err(ExternalRootError::WrongRootKind)
        );
        assert_eq!(
            verify_evm_root(
                42,
                root(0x99),
                &proof(ETH, RootKind::StateRoot, root(0x11)),
                state
            ),
            Err(ExternalRootError::RootMismatch)
        );
    }

    #[test]
    fn solana_state_root_binds_slot_block_and_root() {
        let state = SvmRootState {
            slot: 7,
            block_hash: root(0x44),
            state_root: root(0x55),
            validator_set_hash: root(0x66),
        };
        assert_eq!(
            verify_svm_root(
                7,
                root(0x55),
                &proof(SOL, RootKind::StateRoot, root(0x44)),
                state
            ),
            Ok(())
        );
        assert_eq!(
            verify_svm_root(
                7,
                root(0x66),
                &proof(SOL, RootKind::ValidatorSetHash, root(0x44)),
                state,
            ),
            Ok(())
        );
        assert_eq!(
            verify_svm_root(
                8,
                root(0x55),
                &proof(SOL, RootKind::StateRoot, root(0x44)),
                state
            ),
            Err(ExternalRootError::BlockMismatch)
        );
    }

    #[test]
    fn solana_rejects_eth_kind_and_wrong_root() {
        let state = SvmRootState {
            slot: 7,
            block_hash: root(0x44),
            state_root: root(0x55),
            validator_set_hash: root(0x66),
        };
        assert_eq!(
            verify_svm_root(
                7,
                root(0x55),
                &proof(SOL, RootKind::ReceiptsRoot, root(0x44)),
                state,
            ),
            Err(ExternalRootError::WrongRootKind)
        );
        assert_eq!(
            verify_svm_root(
                7,
                root(0x99),
                &proof(SOL, RootKind::StateRoot, root(0x44)),
                state
            ),
            Err(ExternalRootError::RootMismatch)
        );
    }

    #[test]
    fn bitcoin_root_must_be_the_admitted_best_chain_block() {
        let state = BtcRootState {
            height: 120_000,
            block_hash: root(0x77),
            anchored: true,
        };
        assert_eq!(
            verify_btc_root(
                120_000,
                root(0x77),
                &proof(BTC, RootKind::BlockHash, root(0x77)),
                state,
            ),
            Ok(())
        );
        assert_eq!(
            verify_btc_root(
                120_000,
                root(0x78),
                &proof(BTC, RootKind::BlockHash, root(0x78)),
                state,
            ),
            Err(ExternalRootError::RootMismatch)
        );
        assert_eq!(
            verify_btc_root(
                120_000,
                root(0x77),
                &proof(BTC, RootKind::BlockHash, root(0x77)),
                BtcRootState {
                    anchored: false,
                    ..state
                },
            ),
            Err(ExternalRootError::UnanchoredBlock)
        );
    }

    #[test]
    fn malformed_trailing_and_unknown_versions_fail_closed() {
        let state = EvmRootState {
            block_number: 42,
            block_hash: root(0x11),
            state_root: root(0x22),
            receipts_root: root(0x33),
        };
        let mut trailing = proof(ETH, RootKind::StateRoot, root(0x11));
        trailing.push(0);
        assert_eq!(
            verify_evm_root(42, root(0x22), &trailing, state),
            Err(ExternalRootError::MalformedProof)
        );
        assert_eq!(
            verify_evm_root(42, root(0x22), &[9, ETH as u8, 1], state),
            Err(ExternalRootError::UnsupportedProofFormat)
        );
        assert_eq!(
            verify_evm_root(42, root(0x22), &[], state),
            Err(ExternalRootError::MalformedProof)
        );
    }

    #[test]
    fn unsupported_chain_is_rejected_before_binding() {
        assert_eq!(
            decode_root_proof(99, &proof(99, RootKind::StateRoot, root(1))),
            Err(ExternalRootError::UnsupportedChain)
        );
    }
}
