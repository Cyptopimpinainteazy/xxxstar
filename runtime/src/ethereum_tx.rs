//! Signed Ethereum transactions as self-contained extrinsics.
//!
//! A `pallet_ethereum::Call::transact` extrinsic carries no Substrate signature: the runtime
//! recovers the sender from the transaction's own ECDSA signature (`check_self_contained`) and
//! dispatches with that address as the origin. Nothing the submitter claims about the sender is
//! trusted.
//!
//! Self-contained extrinsics bypass `SignedExtra`, so the X3 gates that live there —
//! `InvariantCheck` (the constitutional halt) and `AgentLawCheck` (blacklist and policies) — are
//! run here against the sender's mapped account, in pool validation and again before dispatch.

use super::*;
use codec::Decode;
use pallet_evm::AddressMapping;
use sp_core::H160;
use sp_runtime::traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, TransactionExtension};
use sp_runtime::transaction_validity::{
    InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
};

/// Ethereum types the node needs to read `pallet-ethereum`'s per-block records, re-exported so the
/// node uses exactly the versions the runtime encodes with.
pub use ethereum;
pub use pallet_ethereum::{Receipt, Transaction, TransactionStatus};

/// Storage keys of `pallet-ethereum`'s per-block records, in the post-state of each block:
/// `(CurrentBlock, CurrentReceipts, CurrentTransactionStatuses)`. Each is overwritten by the
/// next block, so a node that wants them later has to copy them out as blocks are imported.
pub fn current_block_storage_keys() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        pallet_ethereum::CurrentBlock::<Runtime>::hashed_key().to_vec(),
        pallet_ethereum::CurrentReceipts::<Runtime>::hashed_key().to_vec(),
        pallet_ethereum::CurrentTransactionStatuses::<Runtime>::hashed_key().to_vec(),
    )
}

/// Decode a signed, EIP-2718 enveloped Ethereum transaction (the bytes a wallet sends to
/// `eth_sendRawTransaction`) and wrap it as the extrinsic the transaction pool accepts, with the
/// transaction's Ethereum hash.
///
/// Decoding checks only the encoding. The signature is checked by the runtime when the pool
/// validates the extrinsic, so a forged transaction is refused there, not here.
pub fn signed_ethereum_extrinsic(
    raw: &[u8],
) -> Result<(UncheckedExtrinsic, sp_core::H256), &'static str> {
    let transaction = <pallet_ethereum::Transaction as ethereum::EnvelopedDecodable>::decode(raw)
        .map_err(|_| "not an RLP / EIP-2718 encoded Ethereum transaction")?;
    let hash = transaction.hash();
    Ok((
        UncheckedExtrinsic::new_bare(
            pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
        ),
        hash,
    ))
}

/// The account the recovered sender of a `transact` call maps to, and the transaction nonce.
pub fn sender_and_nonce(call: &RuntimeCall) -> Option<(AccountId, u64)> {
    use fp_self_contained::SelfContainedCall;

    let RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction }) = call else {
        return None;
    };
    let signer = call.check_self_contained()?.ok()?;
    let nonce = match transaction {
        pallet_ethereum::Transaction::Legacy(t) => t.nonce,
        pallet_ethereum::Transaction::EIP2930(t) => t.nonce,
        pallet_ethereum::Transaction::EIP1559(t) => t.nonce,
        pallet_ethereum::Transaction::EIP7702(t) => t.nonce,
    };
    Some((
        <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(signer),
        nonce.low_u64(),
    ))
}

/// Run the X3 `SignedExtra` gates for an Ethereum transaction from `signer`.
fn x3_gates(
    call: &RuntimeCall,
    signer: &H160,
    dispatch_info: &DispatchInfoOf<RuntimeCall>,
    len: usize,
) -> Result<(), TransactionValidityError> {
    let who = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(*signer);
    let implication = sp_runtime::traits::TxBaseImplication(call);
    let (_, _, origin) = pallet_x3_invariants::InvariantCheck::<Runtime>::new().validate(
        RuntimeOrigin::signed(who),
        call,
        dispatch_info,
        len,
        (),
        &implication,
        TransactionSource::External,
    )?;
    let agent_law = pallet_x3_agent_law::AgentLawCheck::<Runtime>::decode(&mut &[][..])
        .map_err(|_| InvalidTransaction::Call)?;
    agent_law.validate(
        origin,
        call,
        dispatch_info,
        len,
        (),
        &implication,
        TransactionSource::External,
    )?;
    Ok(())
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
    type SignedInfo = H160;

    fn is_self_contained(&self) -> bool {
        match self {
            RuntimeCall::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            RuntimeCall::Ethereum(call) => {
                if let Err(e) = x3_gates(self, info, dispatch_info, len) {
                    return Some(Err(e));
                }
                call.validate_self_contained(info, dispatch_info, len)
            }
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => {
                if let Err(e) = x3_gates(self, info, dispatch_info, len) {
                    return Some(Err(e));
                }
                call.pre_dispatch_self_contained(info, dispatch_info, len)
            }
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => {
                Some(call.dispatch(RuntimeOrigin::from(
                    pallet_ethereum::RawOrigin::EthereumTransaction(info),
                )))
            }
            _ => None,
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use ethereum::legacy::TransactionSignature;
    use ethereum::{EnvelopedEncodable, LegacyTransaction};
    use fp_self_contained::SelfContainedCall;
    use frame_support::dispatch::GetDispatchInfo;
    use sp_core::{ecdsa, Pair, U256};

    fn chain_id() -> u64 {
        <Runtime as pallet_evm::Config>::ChainId::get()
    }

    /// The Ethereum address of an ECDSA key: the last 20 bytes of keccak(uncompressed pubkey).
    fn address_of(pair: &ecdsa::Pair) -> H160 {
        let probe = [7u8; 32];
        let sig = pair.sign_prehashed(&probe);
        let public = sp_io::crypto::secp256k1_ecdsa_recover(&sig.0, &probe)
            .ok()
            .expect("a fresh signature recovers");
        H160::from_slice(&sp_io::hashing::keccak_256(&public)[12..])
    }

    /// An EIP-155 legacy transaction signed by `pair`.
    fn signed_legacy(pair: &ecdsa::Pair, nonce: u64, value: u64) -> pallet_ethereum::Transaction {
        let message = pallet_ethereum::LegacyTransactionMessage {
            nonce: U256::from(nonce),
            gas_price: U256::from(1_000_000_000u64),
            gas_limit: U256::from(21_000u64),
            action: pallet_ethereum::TransactionAction::Call(H160::repeat_byte(0x22)),
            value: U256::from(value),
            input: Vec::new(),
            chain_id: Some(chain_id()),
        };
        let sig = pair.sign_prehashed(&message.hash().0);
        let v = u64::from(sig.0[64]) + chain_id() * 2 + 35;
        let signature = TransactionSignature::new(
            v,
            sp_core::H256::from_slice(&sig.0[0..32]),
            sp_core::H256::from_slice(&sig.0[32..64]),
        )
        .expect("a fresh signature is canonical");
        pallet_ethereum::Transaction::Legacy(LegacyTransaction {
            nonce: message.nonce,
            gas_price: message.gas_price,
            gas_limit: message.gas_limit,
            action: message.action,
            value: message.value,
            input: message.input,
            signature,
        })
    }

    fn transact(transaction: pallet_ethereum::Transaction) -> RuntimeCall {
        pallet_ethereum::Call::<Runtime>::transact { transaction }.into()
    }

    fn signer_of(call: &RuntimeCall) -> Option<H160> {
        call.check_self_contained()?.ok()
    }

    #[test]
    fn sender_is_recovered_from_the_signature() {
        sp_io::TestExternalities::default().execute_with(|| {
            let pair = ecdsa::Pair::from_seed(&[0x11; 32]);
            let call = transact(signed_legacy(&pair, 0, 5));
            assert!(call.is_self_contained());
            assert_eq!(signer_of(&call), Some(address_of(&pair)));
        });
    }

    #[test]
    fn a_tampered_transaction_does_not_recover_the_original_sender() {
        sp_io::TestExternalities::default().execute_with(|| {
            let pair = ecdsa::Pair::from_seed(&[0x11; 32]);
            let pallet_ethereum::Transaction::Legacy(mut tx) = signed_legacy(&pair, 0, 5) else {
                unreachable!("built as legacy")
            };
            tx.value = U256::from(5_000_000u64);
            let recovered = signer_of(&transact(pallet_ethereum::Transaction::Legacy(tx)));
            assert_ne!(recovered, Some(address_of(&pair)));
        });
    }

    #[test]
    fn raw_bytes_decode_to_the_same_signed_call() {
        sp_io::TestExternalities::default().execute_with(|| {
            let pair = ecdsa::Pair::from_seed(&[0x11; 32]);
            let tx = signed_legacy(&pair, 3, 9);
            let raw = EnvelopedEncodable::encode(&tx).to_vec();
            let (xt, hash) = signed_ethereum_extrinsic(&raw).expect("valid envelope");
            assert_eq!(hash, sp_core::H256(sp_io::hashing::keccak_256(&raw)));
            assert_eq!(xt.0.function, transact(tx));
            assert_eq!(signer_of(&xt.0.function), Some(address_of(&pair)));

            assert!(signed_ethereum_extrinsic(&[0xde, 0xad]).is_err());
            assert!(signed_ethereum_extrinsic(&[]).is_err());
        });
    }

    #[test]
    fn a_funded_signed_transaction_passes_the_gates_and_validates() {
        sp_io::TestExternalities::default().execute_with(|| {
            System::set_block_number(1);
            let pair = ecdsa::Pair::from_seed(&[0x11; 32]);
            let call = transact(signed_legacy(&pair, 0, 5));
            let signer = signer_of(&call).unwrap();
            let who = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(signer);
            // The EVM reads base units as wei: 21_000 gas at NATIVE_GAS_PRICE is 21_000 X3.
            let _ = Balances::deposit_creating(&who, 100_000 * X3);

            let info = call.get_dispatch_info();
            let validity = call
                .validate_self_contained(&signer, &info, 200)
                .expect("an Ethereum call is self-contained");
            assert!(validity.is_ok(), "{validity:?}");
        });
    }

    #[test]
    fn halted_chain_refuses_ethereum_transactions() {
        sp_io::TestExternalities::default().execute_with(|| {
            let pair = ecdsa::Pair::from_seed(&[0x11; 32]);
            let call = transact(signed_legacy(&pair, 0, 5));
            let signer = signer_of(&call).unwrap();
            let info = call.get_dispatch_info();

            pallet_x3_invariants::pallet::Halted::<Runtime>::put(true);
            let halted = TransactionValidityError::Invalid(InvalidTransaction::Custom(
                pallet_x3_invariants::INVARIANT_HALT_CODE,
            ));
            assert_eq!(
                call.validate_self_contained(&signer, &info, 200),
                Some(Err(halted))
            );
            assert_eq!(
                call.pre_dispatch_self_contained(&signer, &info, 200),
                Some(Err(halted))
            );
        });
    }

    #[test]
    fn blacklisted_sender_is_refused() {
        sp_io::TestExternalities::default().execute_with(|| {
            System::set_block_number(1);
            let pair = ecdsa::Pair::from_seed(&[0x11; 32]);
            let call = transact(signed_legacy(&pair, 0, 5));
            let signer = signer_of(&call).unwrap();
            let who = <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(signer);
            pallet_x3_agent_law::Blacklist::<Runtime>::insert(&who, 1_000u32);

            let info = call.get_dispatch_info();
            assert_eq!(
                call.validate_self_contained(&signer, &info, 200),
                Some(Err(TransactionValidityError::Invalid(
                    InvalidTransaction::Custom(100)
                )))
            );
        });
    }

    #[test]
    fn sender_and_nonce_report_the_recovered_account() {
        sp_io::TestExternalities::default().execute_with(|| {
            let pair = ecdsa::Pair::from_seed(&[0x11; 32]);
            let expected =
                <Runtime as pallet_evm::Config>::AddressMapping::into_account_id(address_of(&pair));
            let call = transact(signed_legacy(&pair, 7, 1));
            assert_eq!(sender_and_nonce(&call), Some((expected, 7)));
            assert_eq!(
                crate::ethereum_sender_and_nonce(&call),
                sender_and_nonce(&call)
            );

            let remark = RuntimeCall::System(frame_system::Call::remark { remark: vec![1] });
            assert_eq!(sender_and_nonce(&remark), None);
        });
    }

    #[test]
    fn current_block_storage_keys_address_pallet_ethereum_records() {
        sp_io::TestExternalities::default().execute_with(|| {
            let pair = ecdsa::Pair::from_seed(&[0x11; 32]);
            let statuses = vec![pallet_ethereum::TransactionStatus {
                transaction_hash: signed_legacy(&pair, 0, 1).hash(),
                transaction_index: 0,
                from: address_of(&pair),
                to: None,
                contract_address: None,
                logs: vec![],
                logs_bloom: Default::default(),
            }];
            pallet_ethereum::CurrentTransactionStatuses::<Runtime>::put(statuses.clone());
            let (_, _, statuses_key) = current_block_storage_keys();
            let raw = sp_io::storage::get(&statuses_key).expect("written under this key");
            assert_eq!(
                Vec::<pallet_ethereum::TransactionStatus>::decode(&mut &raw[..]).unwrap(),
                statuses
            );
        });
    }

    #[test]
    fn non_ethereum_calls_are_not_self_contained() {
        let call = RuntimeCall::System(frame_system::Call::remark { remark: vec![1] });
        assert!(!call.is_self_contained());
        assert!(call.check_self_contained().is_none());
    }
}
