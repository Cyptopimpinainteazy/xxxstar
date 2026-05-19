use crate::{
    law_engine::{PolicyContext, PolicyEngine},
    types::PolicyResult,
    ActivePolicies, Blacklist, Config, ExtrinsicCountThisEpoch, LastEpoch, Pallet, TasksThisBlock,
};
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{pallet_prelude::TransactionSource, traits::Get, weights::Weight};
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;
use sp_runtime::{
    impl_tx_ext_default,
    traits::{
        AsSystemOriginSigner, DispatchInfoOf, Dispatchable, TransactionExtension, ValidateResult,
    },
    transaction_validity::{InvalidTransaction, TransactionValidityError, ValidTransaction},
};
use sp_std::{fmt, marker::PhantomData, prelude::*};

/// SignedExtension for Agent Law enforcement
///
/// **SECURITY-CRITICAL**: This runs in the pre-dispatch phase BEFORE any state mutations.
/// Order in SignedExtra tuple is strict:
/// ```
/// pub type SignedExtra = (
///     frame_system::CheckNonZeroSender<Runtime>,
///     frame_system::CheckSpecVersion<Runtime>,
///     frame_system::CheckTxVersion<Runtime>,
///     frame_system::CheckGenesis<Runtime>,
///     frame_system::CheckEra<Runtime>,
///     frame_system::CheckNonce<Runtime>,
///     frame_system::CheckWeight<Runtime>,
///     pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
///
///     x3_invariants::InvariantCheck,        // 1. CRITICAL INVARIANTS FIRST
///     x3_agent_law::AgentLawCheck,          // 2. POLICY ENFORCEMENT
///     x3_swarm::CapabilityEnvelopeCheck,    // 3. LONG-RANGE VALIDATION
///     x3_kernel::AtomicSettlementCheck,     // 4. CROSS-VM ATOMICITY
///     x3_flash_finality::FlashFinalityExt,  // 5. FLASH FINALITY
/// );
/// ```
///
/// ⚠️ Reordering breaks the security model. Invariants MUST fail before policies are evaluated.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct AgentLawCheck<T: Config + Send + Sync + 'static>(PhantomData<T>);

impl<T: Config + Send + Sync + 'static> fmt::Debug for AgentLawCheck<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AgentLawCheck")
    }
}

impl<T: Config + Send + Sync + 'static> TransactionExtension<T::RuntimeCall> for AgentLawCheck<T>
where
    T::RuntimeCall: Dispatchable,
    <T::RuntimeCall as Dispatchable>::RuntimeOrigin: AsSystemOriginSigner<T::AccountId> + Clone,
{
    const IDENTIFIER: &'static str = "AgentLawCheck";

    type Implicit = ();
    type Val = ();
    type Pre = ();

    fn weight(&self, _call: &T::RuntimeCall) -> Weight {
        Weight::zero()
    }

    fn validate(
        &self,
        origin: <T::RuntimeCall as Dispatchable>::RuntimeOrigin,
        _call: &T::RuntimeCall,
        _info: &DispatchInfoOf<T::RuntimeCall>,
        _len: usize,
        _self_implicit: Self::Implicit,
        _inherited_implication: &impl Encode,
        _source: TransactionSource,
    ) -> ValidateResult<Self::Val, T::RuntimeCall> {
        let Some(who) = origin.as_system_origin_signer() else {
            // Unsigned transaction — pass through
            return Ok((ValidTransaction::default(), (), origin));
        };

        // 1. Check if agent is blacklisted
        let current_block = frame_system::Pallet::<T>::block_number();
        if let Some(expiry) = Blacklist::<T>::get(who) {
            if current_block < expiry {
                return Err(InvalidTransaction::Custom(100).into()); // Agent blacklisted
            }
        }

        // 2. Get active policies for this agent
        let policies = ActivePolicies::<T>::get(who);
        if policies.is_empty() {
            return Ok((ValidTransaction::default(), (), origin));
        }

        // 3. Build policy context
        let reputation_score = 100u64;
        let tasks_this_block = TasksThisBlock::<T>::get((current_block, who.clone()));
        let extrinsics_this_epoch = Self::get_extrinsic_count_this_epoch(who, current_block);
        let related_agents: Vec<T::AccountId> = Vec::new();

        let context = PolicyContext {
            reputation_score,
            tasks_this_block,
            extrinsics_this_epoch,
            requested_capability: Self::extract_requested_capability(_call),
            related_agents,
            current_block,
            last_activity_block: current_block,
        };

        // 4. Evaluate policies
        let policy_result = PolicyEngine::evaluate_policies::<T>(who, &policies, &context);

        match policy_result {
            PolicyResult::Pass => {
                ExtrinsicCountThisEpoch::<T>::mutate(who, |count| *count = count.saturating_add(1));
                Ok((ValidTransaction::default(), (), origin))
            }
            PolicyResult::Fail(violation_type) => {
                Pallet::<T>::deposit_event(crate::pallet::Event::PolicyViolation {
                    agent: who.clone(),
                    violation_type,
                    enforcement: crate::types::EnforcementAction::Slash(100),
                });
                Err(InvalidTransaction::Custom(101).into()) // Policy violation
            }
        }
    }

    impl_tx_ext_default!(T::RuntimeCall; prepare);
}

impl<T: Config + Send + Sync + 'static> AgentLawCheck<T> {
    /// Get extrinsic count for current epoch, resetting if epoch changed
    fn get_extrinsic_count_this_epoch(
        agent: &T::AccountId,
        current_block: BlockNumberFor<T>,
    ) -> u32 {
        let last_epoch = LastEpoch::<T>::get(agent);
        let epoch_length = T::RateLimitEpochLength::get();
        let current_epoch = current_block / epoch_length;
        let last_epoch_num = last_epoch / epoch_length;

        if current_epoch > last_epoch_num {
            // Epoch changed, reset counter
            ExtrinsicCountThisEpoch::<T>::insert(agent, 0);
            LastEpoch::<T>::insert(agent, current_block);
            0
        } else {
            // Same epoch, return current count
            ExtrinsicCountThisEpoch::<T>::get(agent)
        }
    }

    fn extract_requested_capability(
        call: &<T as frame_system::Config>::RuntimeCall,
    ) -> Option<Vec<u8>> {
        // TODO: map runtime calls to agent capability labels.
        // This currently returns `None` for generic calls and should be extended
        // as the capability model is integrated with X3 call routing.
        let _ = call;
        None
    }
}

#[cfg(test)]
mod tests {
    // Full tests require mock Config trait implementation
    // See pallet/tests.rs for integration tests
}
