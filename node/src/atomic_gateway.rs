//! Atomic gateway signing support.
//!
//! The node's atomic service holds the X3-lang gateway key and builds signed
//! extrinsics for `submit_atomic_bundle` / `assign_bundle_executor`. The
//! gateway account is configured at the runtime boundary; this module only
//! constructs the signed transaction payloads and never fabricates proof data.

use codec::{Decode, Encode};
use frame_support::BoundedVec;
use sp_core::{Pair as PairTrait, H256};
use sp_runtime::generic::Era;
use sp_runtime::traits::{IdentifyAccount, Verify};
use x3_chain_runtime::{
    AccountId, Address, Runtime, RuntimeCall, Signature, SignedExtra, SignedPayload,
    UncheckedExtrinsic, VERSION,
};

/// A loaded sr25519 gateway keypair.
pub struct AtomicGatewayKey {
    pair: sp_core::sr25519::Pair,
}

impl AtomicGatewayKey {
    /// Load the gateway key from an sr25519 URI (e.g. `//x3-atomic-gateway`).
    pub fn from_uri(uri: &str) -> Result<Self, String> {
        let pair = sp_core::sr25519::Pair::from_string(uri, None)
            .map_err(|e| format!("failed to load atomic gateway key {uri}: {e:?}"))?;
        Ok(Self { pair })
    }

    /// The gateway `AccountId` this key signs as.
    pub fn account(&self) -> AccountId {
        account_from_public(self.pair.public())
    }

    /// Build a signed `submit_atomic_bundle` extrinsic.
    pub fn submit_atomic_bundle(
        &self,
        legs: Vec<pallet_x3_atomic_kernel::proof::BundleLeg>,
        deadline_blocks: u32,
        chain_id: u32,
        nonce: u64,
        genesis_hash: H256,
        tx_nonce: u32,
    ) -> Result<UncheckedExtrinsic, String> {
        type MaxLegs = <Runtime as pallet_x3_atomic_kernel::Config>::MaxLegsPerBundle;
        let legs: BoundedVec<_, MaxLegs> = legs
            .try_into()
            .map_err(|_| "atomic bundle exceeds maximum leg count".to_string())?;
        let call = RuntimeCall::X3AtomicKernel(
            pallet_x3_atomic_kernel::Call::<Runtime>::submit_atomic_bundle {
                legs,
                deadline_blocks,
                chain_id,
                nonce,
            },
        );
        self.signed_extrinsic(call, genesis_hash, tx_nonce)
    }

    /// Build a signed `assign_bundle_executor` extrinsic.
    pub fn assign_bundle_executor(
        &self,
        bundle_id: H256,
        genesis_hash: H256,
        tx_nonce: u32,
    ) -> Result<UncheckedExtrinsic, String> {
        let call = RuntimeCall::X3AtomicKernel(
            pallet_x3_atomic_kernel::Call::<Runtime>::assign_bundle_executor { bundle_id },
        );
        self.signed_extrinsic(call, genesis_hash, tx_nonce)
    }

    /// Build a signed `finalize_atomic_bundle` extrinsic for the finalized
    /// block that owns the bundle.
    pub fn finalize_atomic_bundle(
        &self,
        bundle_id: H256,
        receipt_root: H256,
        finality_cert: H256,
        finalized_block: u32,
        genesis_hash: H256,
        tx_nonce: u32,
    ) -> Result<UncheckedExtrinsic, String> {
        let call = RuntimeCall::X3AtomicKernel(
            pallet_x3_atomic_kernel::Call::<Runtime>::finalize_atomic_bundle {
                bundle_id,
                receipt_root,
                finality_cert,
                finalized_block,
            },
        );
        self.signed_extrinsic(call, genesis_hash, tx_nonce)
    }

    fn signed_extrinsic(
        &self,
        call: RuntimeCall,
        genesis_hash: H256,
        nonce: u32,
    ) -> Result<UncheckedExtrinsic, String> {
        let account = self.account();
        let agent_law_check = pallet_x3_agent_law::AgentLawCheck::<Runtime>::decode(
            &mut &[][..],
        )
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
        let signature = payload.using_encoded(|payload| {
            Signature::from(self.pair.sign(payload))
        });
        Ok(UncheckedExtrinsic::new_signed(
            call,
            Address::Id(account),
            signature,
            extra,
        ))
    }
}

fn account_from_public(public: sp_core::sr25519::Public) -> AccountId {
    <Signature as Verify>::Signer::from(public).into_account()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pallet_x3_atomic_kernel::proof::{BundleLeg, DeclaredAccess, VmType};

    fn gateway() -> AtomicGatewayKey {
        AtomicGatewayKey::from_uri("//x3-atomic-gateway").expect("well-known dev key loads")
    }

    #[test]
    fn gateway_account_matches_runtime_constant() {
        let expected = AccountId::new([
            0x4c, 0x81, 0xd4, 0x16, 0xba, 0xa8, 0xc0, 0xe2,
            0xb2, 0xe9, 0x99, 0x77, 0xe4, 0x52, 0x32, 0x87,
            0xe1, 0x1c, 0xd6, 0xf6, 0x2c, 0xd3, 0x32, 0x8e,
            0x4f, 0xb8, 0xd8, 0x23, 0xdd, 0xe6, 0x29, 0x35,
        ]);
        assert_eq!(gateway().account(), expected);
    }

    fn leg() -> BundleLeg {
        BundleLeg {
            vm_type: VmType::Svm,
            token_in: H256([0xAA; 32]),
            token_out: H256([0xBB; 32]),
            amount_in: 1_000_000,
            min_amount_out: 999_000,
            deadline: 1_800,
            access: DeclaredAccess {
                reads: BoundedVec::try_from(vec![H256([1; 32])]).unwrap(),
                writes: BoundedVec::try_from(vec![H256([2; 32])]).unwrap(),
            },
        }
    }

    #[test]
    fn submit_atomic_bundle_builds_signable_extrinsic() {
        let xt = gateway()
            .submit_atomic_bundle(vec![leg()], 100, 1, 1, H256::zero(), 0)
            .expect("submit extrinsic builds");
        assert!(!xt.encode().is_empty());
    }

    #[test]
    fn assign_executor_builds_signable_extrinsic() {
        let xt = gateway()
            .assign_bundle_executor(H256([0xAB; 32]), H256::zero(), 1)
            .expect("assign extrinsic builds");
        assert!(!xt.encode().is_empty());
    }
}
