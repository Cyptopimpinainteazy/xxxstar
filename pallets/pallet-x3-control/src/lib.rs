#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_std::prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	/// Events emitted by this pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Control action executed
		ControlActionExecuted { domain: ControlDomain, action: ControlAction, authority: ControlAuthority },
		/// Control state changed
		ControlStateChanged { domain: ControlDomain, from: ControlState, to: ControlState },
	}

	/// Errors emitted by this pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient authority for this action
		InsufficientAuthority,
		/// Domain not found
		DomainNotFound,
		/// Invalid state transition
		InvalidStateTransition,
		/// Action rate limited
		ActionRateLimited,
	}

	/// Control domains
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen)]
	pub enum ControlDomain {
		Chain,
		VM(VmId),
		Swarm,
		Agent(AgentId),
		Strategy(StrategyId),
		HTLC,
		ProofRelay,
		Indexer,
	}

	/// VM identifiers
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen)]
	pub enum VmId {
		EVM,
		SVM,
		X3VM,
		Bitcoin,
	}

	/// Agent and strategy IDs
	pub type AgentId = [u8; 32];
	pub type StrategyId = [u8; 32];

	/// Control authorities
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen)]
	pub enum ControlAuthority {
		Root,
		Governance,
		EmergencyCommittee,
		Operator,
	}

	/// Control actions
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen)]
	pub enum ControlAction {
		Freeze,
		Thaw,
		Pause,
		Resume,
		Quarantine,
		Release,
		ParameterOverride,
	}

	/// Control states
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen)]
	pub enum ControlState {
		Active,
		Paused,
		Frozen,
		Quarantined,
	}

	/// Control record
	#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen)]
	pub struct ControlRecord<T: Config> {
		pub domain: ControlDomain,
		pub state: ControlState,
		pub authority: ControlAuthority,
		pub last_action: T::BlockNumber,
		pub cooldown_until: T::BlockNumber,
	}

	/// Storage for control records
	#[pallet::storage]
	pub type ControlRecords<T: Config> = StorageMap<_, Blake2_128Concat, ControlDomain, ControlRecord<T>, OptionQuery>;

	/// Global control state
	#[pallet::storage]
	pub type GlobalState<T: Config> = StorageValue<_, ControlState, ValueQuery, DefaultGlobalState>;

	#[pallet::type_value]
	pub fn DefaultGlobalState() -> ControlState {
		ControlState::Active
	}

	/// Emergency committee members
	#[pallet::storage]
	pub type EmergencyCommittee<T: Config> = StorageValue<_, BoundedVec<T::AccountId, ConstU32<10>>, ValueQuery>;

	/// Governance origin for control actions
	pub type EnsureGovernance<T> = frame_system::EnsureRoot<T::AccountId>;

	pub type EnsureEmergencyCommittee<T> = frame_system::EnsureSignedBy<EmergencyCommittee<T>, T::AccountId>;

	/// Pallet hooks
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	/// Pallet callable functions
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Execute a control action on a domain
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::execute_control_action())]
		pub fn execute_control_action(
			origin: OriginFor<T>,
			domain: ControlDomain,
			action: ControlAction,
			authority: ControlAuthority,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Check authority
			Self::ensure_authority(&caller, &authority)?;

			// Check cooldown
			let record = ControlRecords::<T>::get(&domain).unwrap_or_default();
			let current_block = <frame_system::Pallet<T>>::block_number();
			ensure!(current_block >= record.cooldown_until, Error::<T>::ActionRateLimited);

			// Validate transition
			let new_state = Self::compute_new_state(record.state, action.clone())?;
			ensure!(new_state != record.state, Error::<T>::InvalidStateTransition);

			// Update record
			let cooldown_period = Self::get_cooldown_period(&authority);
			let new_record = ControlRecord {
				domain: domain.clone(),
				state: new_state.clone(),
				authority: authority.clone(),
				last_action: current_block,
				cooldown_until: current_block + cooldown_period,
			};

			ControlRecords::<T>::insert(&domain, new_record);

			// Emit events
			Self::deposit_event(Event::ControlActionExecuted { domain: domain.clone(), action, authority });
			Self::deposit_event(Event::ControlStateChanged {
				domain,
				from: record.state,
				to: new_state,
			});

			Ok(())
		}

		/// Set emergency committee
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_emergency_committee())]
		pub fn set_emergency_committee(
			origin: OriginFor<T>,
			members: BoundedVec<T::AccountId, ConstU32<10>>,
		) -> DispatchResult {
			EnsureGovernance::<T>::ensure_origin(origin)?;
			EmergencyCommittee::<T>::put(members);
			Ok(())
		}
	}

	/// Helper functions
	impl<T: Config> Pallet<T> {
		fn ensure_authority(caller: &T::AccountId, authority: &ControlAuthority) -> DispatchResult {
			match authority {
				ControlAuthority::Root => EnsureGovernance::<T>::ensure_origin(OriginFor::<T>::root())?,
				ControlAuthority::Governance => EnsureGovernance::<T>::ensure_origin(OriginFor::<T>::root())?,
				ControlAuthority::EmergencyCommittee => {
					let committee = EmergencyCommittee::<T>::get();
					ensure!(committee.contains(caller), Error::<T>::InsufficientAuthority);
				}
				ControlAuthority::Operator => {
					// Allow any signed origin for operator actions (rate limited)
				}
			}
			Ok(())
		}

		fn compute_new_state(current: ControlState, action: ControlAction) -> Result<ControlState, Error<T>> {
			match (current, action) {
				(ControlState::Active, ControlAction::Pause) => Ok(ControlState::Paused),
				(ControlState::Active, ControlAction::Freeze) => Ok(ControlState::Frozen),
				(ControlState::Paused, ControlAction::Resume) => Ok(ControlState::Active),
				(ControlState::Paused, ControlAction::Freeze) => Ok(ControlState::Frozen),
				(ControlState::Frozen, ControlAction::Thaw) => Ok(ControlState::Active),
				(ControlState::Quarantined, ControlAction::Release) => Ok(ControlState::Active),
				(ControlState::Active, ControlAction::Quarantine) => Ok(ControlState::Quarantined),
				_ => Err(Error::<T>::InvalidStateTransition),
			}
		}

		fn get_cooldown_period(authority: &ControlAuthority) -> T::BlockNumber {
			match authority {
				ControlAuthority::Root => 0u32.into(),
				ControlAuthority::EmergencyCommittee => 1u32.into(),
				ControlAuthority::Governance => 100u32.into(), // Time-locked
				ControlAuthority::Operator => 10u32.into(),
			}
		}
	}

	/// Default implementation for ControlRecord
	impl<T: Config> Default for ControlRecord<T> {
		fn default() -> Self {
			Self {
				domain: ControlDomain::Chain,
				state: ControlState::Active,
				authority: ControlAuthority::Root,
				last_action: 0u32.into(),
				cooldown_until: 0u32.into(),
			}
		}
	}
}

pub mod weights {
	use frame_support::weights::Weight;

	/// Weight functions for pallet_x3_control
	pub trait WeightInfo {
		fn execute_control_action() -> Weight;
		fn set_emergency_committee() -> Weight;
	}

	/// Default weight implementation
	pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);

	impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
		fn execute_control_action() -> Weight {
			Weight::from_parts(10_000_000, 0)
		}

		fn set_emergency_committee() -> Weight {
			Weight::from_parts(5_000_000, 0)
		}
	}
}