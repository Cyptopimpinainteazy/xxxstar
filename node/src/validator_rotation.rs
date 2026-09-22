//! Validator key rotation operator support.
//!
//! The on-chain `pallet_x3_custody` registry (`ValidatorKeyRegistry` and
//! `KeyRotationSchedule`) is the single source of truth for who is registered
//! and when their next rotation is due. This module turns that registry into a
//! concrete operator action: read the due block, derive a fresh session key
//! pair, build a signed `session.set_keys` extrinsic, and hand the encoded
//! transaction back to the caller for submission.
//!
//! The node never fabricates key material or due dates: every value it reports
//! is either derived from a caller-supplied seed or read from on-chain state.

use codec::{Decode, Encode};
use sp_core::{Pair as PairTrait, H256};
use sp_runtime::generic::Era;
use sp_runtime::traits::{IdentifyAccount, Verify};
use x3_chain_runtime::{
    AccountId, Address, Runtime, RuntimeCall, SessionKeys, Signature, SignedExtra, SignedPayload,
    UncheckedExtrinsic, VERSION,
};

/// A loaded sr25519 operator keypair (the account that signs `set_keys`).
pub struct OperatorKey {
    pair: sp_core::sr25519::Pair,
}

impl OperatorKey {
    /// Load the operator signing key from an sr25519 URI (e.g. `//Alice`).
    pub fn from_uri(uri: &str) -> Result<Self, String> {
        let pair = sp_core::sr25519::Pair::from_string(uri, None)
            .map_err(|e| format!("failed to load operator key {uri}: {e:?}"))?;
        Ok(Self { pair })
    }

    /// The on-chain account this key signs as.
    pub fn account(&self) -> AccountId {
        <Signature as Verify>::Signer::from(self.pair.public()).into_account()
    }

    /// Build a signed `session.set_keys` extrinsic announcing `keys`.
    pub fn set_keys(
        &self,
        keys: SessionKeys,
        genesis_hash: H256,
        tx_nonce: u32,
    ) -> Result<UncheckedExtrinsic, String> {
        let call = RuntimeCall::Session(pallet_session::Call::<Runtime>::set_keys {
            keys,
            proof: Vec::new(),
        });
        self.signed_extrinsic(call, genesis_hash, tx_nonce)
    }

    fn signed_extrinsic(
        &self,
        call: RuntimeCall,
        genesis_hash: H256,
        nonce: u32,
    ) -> Result<UncheckedExtrinsic, String> {
        let account = self.account();
        // The X3 security gates are part of `SignedExtra` and must be present on
        // every transaction. This mirrors `atomic_gateway::AtomicGatewayKey`.
        let agent_law_check = pallet_x3_agent_law::AgentLawCheck::<Runtime>::decode(&mut &[][..])
            .map_err(|e| format!("failed to decode agent-law extension: {e}"))?;

        let extra: SignedExtra = (
            frame_system::CheckNonZeroSender::<Runtime>::new(),
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(Era::Immortal),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
            pallet_x3_invariants::InvariantCheck::<Runtime>::new(),
            agent_law_check,
        );
        let payload = SignedPayload::from_raw(
            call.clone(),
            extra.clone(),
            (
                (),
                VERSION.spec_version,
                VERSION.transaction_version,
                genesis_hash,
                genesis_hash,
                (),
                (),
                (),
                (),
                (),
            ),
        );
        let signature = payload.using_encoded(|payload| Signature::from(self.pair.sign(payload)));
        Ok(UncheckedExtrinsic::new_signed(
            call,
            Address::Id(account),
            signature,
            extra,
        ))
    }
}

/// Derive the Aura (sr25519) authority id for a new session key.
pub fn derive_aura(seed: &str) -> Result<sp_consensus_aura::sr25519::AuthorityId, String> {
    let pair = sp_core::sr25519::Pair::from_string(seed, None)
        .map_err(|e| format!("failed to load aura seed {seed}: {e:?}"))?;
    Ok(pair.public().into())
}

/// Derive the GRANDPA (ed25519) authority id for a new session key.
pub fn derive_grandpa(seed: &str) -> Result<sp_consensus_grandpa::AuthorityId, String> {
    let pair = sp_core::ed25519::Pair::from_string(seed, None)
        .map_err(|e| format!("failed to load grandpa seed {seed}: {e:?}"))?;
    Ok(pair.public().into())
}

/// Build a [`SessionKeys`] from freshly derived Aura/GRANDPA authority ids.
pub fn session_keys(aura_seed: &str, grandpa_seed: &str) -> Result<SessionKeys, String> {
    Ok(SessionKeys {
        aura: derive_aura(aura_seed)?,
        grandpa: derive_grandpa(grandpa_seed)?,
    })
}

/// Encode the `state_getStorage` key for `X3Custody.ValidatorKeyRegistry(account)`.
pub fn validator_key_registry_storage_key(account: &AccountId) -> Vec<u8> {
    storage_map_key(b"X3Custody", b"ValidatorKeyRegistry", &account.encode())
}

/// Encode the `state_getStorage` key for `X3Custody.KeyRotationSchedule(account)`.
pub fn key_rotation_schedule_storage_key(account: &AccountId) -> Vec<u8> {
    storage_map_key(b"X3Custody", b"KeyRotationSchedule", &account.encode())
}

fn storage_map_key(pallet: &[u8], storage: &[u8], key: &[u8]) -> Vec<u8> {
    let mut out = sp_core::hashing::twox_128(pallet).to_vec();
    out.extend_from_slice(&sp_core::hashing::twox_128(storage));
    // `Blake2_128Concat` = blake2_128(key) concatenated with the raw key.
    out.extend_from_slice(&sp_core::hashing::blake2_128(key));
    out.extend_from_slice(key);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_account_is_stable_across_loads() {
        let a = OperatorKey::from_uri("//Alice").expect("loads");
        let b = OperatorKey::from_uri("//Alice").expect("loads");
        assert_eq!(a.account(), b.account());
    }

    #[test]
    fn set_keys_call_is_deterministic() {
        let keys = session_keys("//aura-1", "//grandpa-1").expect("derives");
        let a = OperatorKey::from_uri("//Alice")
            .expect("loads")
            .set_keys(keys.clone(), H256::zero(), 0)
            .expect("builds");
        let b = OperatorKey::from_uri("//Alice")
            .expect("loads")
            .set_keys(keys, H256::zero(), 0)
            .expect("builds");
        // The call and signed payload are deterministic. The sr25519 signature
        // is intentionally randomized per signature scheme, so only the call
        // bytes are compared here.
        assert_eq!(
            a.function.encode(),
            b.function.encode(),
            "set_keys call must be deterministic"
        );
        assert!(!a.encode().is_empty());
    }

    #[test]
    fn set_keys_extrinsic_is_signed_and_names_the_session_call() {
        let keys = session_keys("//aura-1", "//grandpa-1").expect("derives");
        let xt = OperatorKey::from_uri("//Alice")
            .expect("loads")
            .set_keys(keys, H256::zero(), 0)
            .expect("builds");
        assert!(
            matches!(xt.preamble, sp_runtime::generic::Preamble::Signed(..)),
            "a set_keys transaction must be signed"
        );
        assert!(matches!(
            xt.function,
            RuntimeCall::Session(pallet_session::Call::set_keys { .. })
        ));
    }

    #[test]
    fn storage_keys_use_the_custody_prefix_and_hashers() {
        let account = OperatorKey::from_uri("//Alice").expect("loads").account();
        let key = validator_key_registry_storage_key(&account);
        assert_eq!(
            &key[..16],
            &sp_core::hashing::twox_128(b"X3Custody"),
            "pallet prefix must be twox_128(\"X3Custody\")"
        );
        assert_eq!(
            &key[16..32],
            &sp_core::hashing::twox_128(b"ValidatorKeyRegistry"),
            "storage prefix must be twox_128(\"ValidatorKeyRegistry\")"
        );
        // blake2_128_concat appends the raw key after its 16-byte hash.
        assert_eq!(&key[32..48].len(), &16);
        assert_eq!(
            &key[48..],
            &account.encode()[..],
            "raw account must be appended"
        );

        let schedule = key_rotation_schedule_storage_key(&account);
        assert_eq!(
            &schedule[16..32],
            &sp_core::hashing::twox_128(b"KeyRotationSchedule")
        );
    }
}
