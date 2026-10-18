//! The production trading path: an X3BC trading artifact executed by `x3-vm` against the chain's
//! seeded state, with a receipt for every outcome — and, off chain, attestor signing and
//! independent replay.
//!
//! [`execute_trade`] builds without `std`: it is the function the kernel pallet calls inside a
//! block. It decodes with the canonical decoder, verifies with the chain's trading options, runs
//! the entry function, and returns a [`TradeRun`]: the receipt, and — only if the execution
//! returned with the trade committed — the [`TradingDelta`] the chain applies. Anything else is a
//! rollback: no delta, no storage writes, a receipt saying which trap stopped it and where.
//!
//! [`sign_receipt`] and [`replay_receipt`] (std) are the attestor's half: a registered key signs
//! the receipt's digest, and replay re-executes the artifact from the receipt's own pre-state and
//! anchor and requires the identical receipt. It never takes a total from the receipt on trust.

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec::Vec};

use sp_core::H256;

use crate::error::{X3IntegrationError, X3Result};
use crate::types::X3StorageWrite;
use x3_vm::trading::{ChainAnchor, Outcome, TradingDelta, TradingPreState, TradingReceipt, TrapId};
use x3_vm::{BytecodeModule, VMConfig, VMError, VMErrorKind, Verifier, VerifyOptions, VM};

/// Call depth and stack bound for a trade (the chain's figures for any X3 execution).
const TRADE_MAX_CALL_DEPTH: usize = 64;
const TRADE_MAX_STACK_SIZE: usize = 1024;

/// What one trade execution produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeRun {
    /// The execution returned with `trade_commit` done.
    pub committed: bool,
    pub receipt: TradingReceipt,
    /// What the chain applies; `None` for every rollback.
    pub delta: Option<TradingDelta>,
    pub gas_used: u64,
    /// Contract-slot writes, released only with a committed trade.
    pub storage_writes: Vec<X3StorageWrite>,
}

/// The receipt-visible trap for an engine error.
pub fn trap_of(error: &VMError) -> TrapId {
    match &error.kind {
        VMErrorKind::Trade(trap) => *trap,
        VMErrorKind::GasLimitExceeded => TrapId::GasExhausted,
        VMErrorKind::AtomicAborted => TrapId::AtomicAborted,
        VMErrorKind::IntegerOverflow => TrapId::ArithmeticOverflow,
        _ => TrapId::EngineFault,
    }
}

/// Execute a trading artifact against the chain's seeded state.
///
/// Refused before anything runs (`Err`): bytes the canonical decoder does not accept
/// (`InvalidBytecode`), a module the chain's trading verifier refuses (`VerificationFailed`), a
/// module that is not a trading module, and a private-submission artifact without a private
/// channel. Everything after that is a verdict (`Ok`) with a receipt.
pub fn execute_trade(
    bytecode: &[u8],
    pre: &TradingPreState,
    anchor: &ChainAnchor,
    gas_limit: u64,
) -> X3Result<TradeRun> {
    let module = BytecodeModule::from_bytes(bytecode)
        .map_err(|error| X3IntegrationError::InvalidBytecode(format!("{error}")))?;
    if module.trading.is_none() {
        return Err(X3IntegrationError::InvalidBytecode(
            "not a trading module (no trading section)".into(),
        ));
    }
    // The artifact's compiled demand against the channel this submission actually used.
    if x3bc_core::bytecode::requires_private_submission(bytecode) && !pre.private_channel {
        return Err(X3IntegrationError::PrivateSubmissionUnavailable);
    }
    Verifier::verify_module(&module, &VerifyOptions::on_chain_trading())
        .map_err(|error| X3IntegrationError::VerificationFailed(format!("{error}")))?;

    let bytecode_hash = x3bc_core::hash::sha256(bytecode);
    let bytecode_version = module.version.to_packed();
    let mut vm = VM::with_config_and_seeds(
        module,
        VMConfig {
            gas_limit,
            max_call_depth: TRADE_MAX_CALL_DEPTH,
            max_stack_size: TRADE_MAX_STACK_SIZE,
            trace: false,
        },
        &[],
    )
    .map_err(|error| X3IntegrationError::ExecutionFailed(format!("{error}")))?;
    vm.set_context(x3_vm::BlockContext {
        sender: Some(pre.trader),
        block_height: Some(pre.block_number),
        timestamp_ms: None,
        value: Some(0),
        chain_id: Some(pre.chain_id),
    });
    vm.set_trading(pre.clone())
        .map_err(|error| X3IntegrationError::ExecutionFailed(format!("{error}")))?;

    let result = vm.call_function(0, &[]);
    let gas_used = vm.gas_used();
    let session = vm
        .trading()
        .ok_or_else(|| X3IntegrationError::ExecutionFailed("trading session lost".into()))?;

    let outcome = match &result {
        Ok(_) if session.committed() => Outcome::Committed,
        // The function returned without committing the trade: nothing it did stands.
        Ok(_) => Outcome::RolledBack {
            trap: TrapId::SequenceViolation,
            op_index: session.current_op(),
        },
        Err(error) => Outcome::RolledBack {
            trap: trap_of(error),
            op_index: session.current_op(),
        },
    };
    let committed = matches!(outcome, Outcome::Committed);
    let receipt = session.receipt(
        outcome,
        bytecode_hash,
        bytecode_version,
        anchor.clone(),
        gas_limit,
        gas_used,
    );
    let delta = if committed { session.delta() } else { None };
    let storage_writes = if committed {
        vm.drain_storage_journal()
            .into_iter()
            .map(|write| X3StorageWrite {
                key: H256::from(write.key),
                old_value: write.old_value,
                new_value: write.new_value,
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(TradeRun {
        committed,
        receipt,
        delta,
        gas_used,
        storage_writes,
    })
}

#[cfg(feature = "std")]
pub use attestation::*;

#[cfg(feature = "std")]
mod attestation {
    use super::*;
    use ed25519_dalek::{Signature, Signer, SigningKey, Verifier as _, VerifyingKey};
    use std::collections::BTreeMap;

    /// An attestor's signing key and the identity it signs under.
    pub struct AttestorKey {
        id: String,
        key: SigningKey,
    }

    impl AttestorKey {
        /// A key from a 32-byte seed (the seed is the Ed25519 secret key).
        pub fn from_seed(id: &str, seed: [u8; 32]) -> Self {
            Self {
                id: id.to_string(),
                key: SigningKey::from_bytes(&seed),
            }
        }

        pub fn id(&self) -> &str {
            &self.id
        }

        pub fn public_key(&self) -> [u8; 32] {
            self.key.verifying_key().to_bytes()
        }
    }

    /// The signature an attestor binds to a receipt.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Attestation {
        pub attestor_id: String,
        /// SHA-256 of the public key: the key id a registry is keyed on.
        pub key_id: [u8; 32],
        pub public_key: [u8; 32],
        /// Ed25519 over `x3bc_core::receipt::digest(receipt)`.
        pub signature: [u8; 64],
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SignedTradingReceipt {
        pub receipt: TradingReceipt,
        pub attestation: Attestation,
    }

    /// Sign a receipt's canonical digest.
    pub fn sign_receipt(receipt: &TradingReceipt, key: &AttestorKey) -> SignedTradingReceipt {
        let digest = x3bc_core::receipt::digest(receipt);
        let signature = key.key.sign(&digest).to_bytes();
        let public_key = key.public_key();
        SignedTradingReceipt {
            receipt: receipt.clone(),
            attestation: Attestation {
                attestor_id: key.id.clone(),
                key_id: x3bc_core::hash::sha256(&public_key),
                public_key,
                signature,
            },
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum KeyStatus {
        Active,
        Revoked,
    }

    /// Attestor keys the verifier trusts, by attestor id.
    #[derive(Clone, Debug, Default)]
    pub struct AttestorRegistry {
        keys: BTreeMap<String, ([u8; 32], KeyStatus)>,
    }

    impl AttestorRegistry {
        pub fn new() -> Self {
            Self::default()
        }

        /// Trust `key`'s public key under its id.
        pub fn trust(&mut self, key: &AttestorKey) {
            self.keys
                .insert(key.id.clone(), (key.public_key(), KeyStatus::Active));
        }

        /// Stop trusting an attestor id. Receipts it signed no longer verify.
        pub fn revoke(&mut self, id: &str) {
            if let Some(entry) = self.keys.get_mut(id) {
                entry.1 = KeyStatus::Revoked;
            }
        }

        fn active_key(&self, id: &str) -> Option<[u8; 32]> {
            match self.keys.get(id) {
                Some((key, KeyStatus::Active)) => Some(*key),
                _ => None,
            }
        }
    }

    /// Why a signed receipt was refused.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ReplayError {
        /// No active registered key for the attestor id, or the receipt names a different key.
        UnknownAttestor,
        /// The signature does not cover these receipt bytes.
        BadSignature,
        /// The receipt is about a different artifact.
        ArtifactMismatch,
        /// The artifact no longer executes (decoder, verifier).
        NotReplayable(String),
        /// Re-execution produced a different receipt; `field` is the first that differs.
        Diverged { field: &'static str },
        /// The receipt's own figures do not add up (checked independently of re-execution).
        Inconsistent(&'static str),
    }

    /// What a successful replay re-derived.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ReplayReport {
        pub recomputed: TradingReceipt,
    }

    /// The receipt's economic identities, recomputed from its own movements rather than read from
    /// its totals: the profit is the post-minus-pre balance of the profit asset, the post balances
    /// follow from the pre balances and the trader's intents, and every value movement nets to
    /// zero across all accounts.
    fn audit_economics(receipt: &TradingReceipt) -> Result<(), ReplayError> {
        use x3_vm::trading::Account;
        let mut balances: BTreeMap<u32, i128> = receipt
            .pre_balances
            .iter()
            .map(|(asset, amount)| (*asset, *amount as i128))
            .collect();
        for intent in &receipt.intents {
            let amount =
                i128::try_from(intent.amount).map_err(|_| ReplayError::Inconsistent("intent"))?;
            if intent.from == Account::Trader {
                *balances.entry(intent.asset).or_insert(0) -= amount;
            }
            if intent.to == Account::Trader {
                *balances.entry(intent.asset).or_insert(0) += amount;
            }
            if intent.from == intent.to {
                return Err(ReplayError::Inconsistent(
                    "intent moves value to its own account",
                ));
            }
        }
        for (asset, amount) in &receipt.post_balances {
            if balances.get(asset).copied().unwrap_or(0) != *amount as i128 {
                return Err(ReplayError::Inconsistent("post balances"));
            }
        }
        let pre = |asset: u32| {
            receipt
                .pre_balances
                .iter()
                .find(|(a, _)| *a == asset)
                .map(|(_, v)| *v as i128)
                .unwrap_or(0)
        };
        let post = |asset: u32| {
            receipt
                .post_balances
                .iter()
                .find(|(a, _)| *a == asset)
                .map(|(_, v)| *v as i128)
                .unwrap_or(0)
        };
        if post(receipt.profit.asset) - pre(receipt.profit.asset) != receipt.profit.net {
            return Err(ReplayError::Inconsistent("profit"));
        }
        // Every non-embedded cost was paid to the fee collector, exactly once.
        let charged: u128 = receipt
            .costs
            .iter()
            .filter(|cost| !cost.embedded)
            .map(|cost| cost.amount)
            .sum();
        let collected: u128 = receipt
            .intents
            .iter()
            .filter(|intent| intent.to == Account::FeeCollector)
            .map(|intent| intent.amount)
            .sum();
        if charged != collected {
            return Err(ReplayError::Inconsistent("costs"));
        }
        Ok(())
    }

    /// Verify a signed receipt against the artifact it claims to describe.
    ///
    /// The attestor must be registered and active and the signature valid over the receipt's
    /// canonical digest; the artifact's hash must match; the receipt's economics must be internally
    /// consistent; and re-executing the artifact from the receipt's pre-state and anchor must
    /// produce the identical receipt.
    pub fn replay_receipt(
        bytecode: &[u8],
        signed: &SignedTradingReceipt,
        registry: &AttestorRegistry,
    ) -> Result<ReplayReport, ReplayError> {
        let attestation = &signed.attestation;
        let trusted = registry
            .active_key(&attestation.attestor_id)
            .ok_or(ReplayError::UnknownAttestor)?;
        if trusted != attestation.public_key
            || x3bc_core::hash::sha256(&trusted) != attestation.key_id
        {
            return Err(ReplayError::UnknownAttestor);
        }
        let key = VerifyingKey::from_bytes(&trusted).map_err(|_| ReplayError::UnknownAttestor)?;
        let digest = x3bc_core::receipt::digest(&signed.receipt);
        key.verify(&digest, &Signature::from_bytes(&attestation.signature))
            .map_err(|_| ReplayError::BadSignature)?;

        let claimed = &signed.receipt;
        if x3bc_core::hash::sha256(bytecode) != claimed.bytecode_hash {
            return Err(ReplayError::ArtifactMismatch);
        }
        let run = execute_trade(
            bytecode,
            &claimed.pre_state,
            &claimed.anchor,
            claimed.gas_limit,
        )
        .map_err(|error| ReplayError::NotReplayable(format!("{error:?}")))?;
        if let Some(field) = x3bc_core::receipt::first_divergence(&run.receipt, claimed) {
            return Err(ReplayError::Diverged { field });
        }
        // A second, independent check: the identities must hold of the receipt itself, so an
        // engine that produced an inconsistent receipt cannot vouch for it by reproducing it.
        audit_economics(claimed)?;
        Ok(ReplayReport {
            recomputed: run.receipt,
        })
    }
}
