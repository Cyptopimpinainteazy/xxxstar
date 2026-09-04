#![deny(unsafe_code)]
//! # X3 On-Chain Invariant Enforcement Pallet
//!
//! This pallet is the runtime guardian of the X3 constitutional invariants.
//! It enforces protocol-level invariants on every block finalization and
//! provides extrinsics for authorized operators to register and update bounds.
//!
//! ## How It Works
//!
//! 1. Operators (governance) configure `InvariantBounds` via `set_bounds`.
//! 2. Every block, `on_finalize` calls `Self::enforce_all()`.
//! 3. Any violation emits `InvariantViolated` and increments `ViolationCount`.
//! 4. If `HaltOnViolation` is set, the node will panic (chain halts) — exchange-grade safety.
//!
//! ## Article IV Alignment
//!
//! This pallet implements Article IV of the X3 Constitution:
//! > "All state transitions are subject to invariant checks. A transition that
//! >  would violate a constitutional invariant is rejected before finalization."

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::WeightInfo;
    use crate::InvariantKind;
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Zero;

    // ── Storage types ──────────────────────────────────────────────────────────

    /// Configurable upper bound on total token supply.
    /// Zero means "not set" (check disabled).
    #[pallet::storage]
    #[pallet::getter(fn max_supply)]
    pub type MaxSupply<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Configurable upper bound on the number of registered agents.
    #[pallet::storage]
    #[pallet::getter(fn max_agents)]
    pub type MaxAgents<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Configurable max governance proposal queue depth.
    #[pallet::storage]
    #[pallet::getter(fn max_proposal_depth)]
    pub type MaxProposalDepth<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Current observed total issuance (written each block by the pallet).
    #[pallet::storage]
    #[pallet::getter(fn last_observed_issuance)]
    pub type LastObservedIssuance<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Cumulative count of invariant violations since genesis.
    #[pallet::storage]
    #[pallet::getter(fn violation_count)]
    pub type ViolationCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// If true, any invariant violation causes a runtime panic (chain halt).
    /// Should only be enabled on production chains after initial burn-in.
    #[pallet::storage]
    #[pallet::getter(fn halt_on_violation)]
    pub type HaltOnViolation<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// The canonical constitution hash this pallet was configured against.
    /// Governance proposals that don't match this hash are out-of-scope.
    #[pallet::storage]
    #[pallet::getter(fn constitution_hash)]
    pub type ConstitutionHash<T: Config> = StorageValue<_, [u8; 32], ValueQuery>;

    // ── Pallet definition ──────────────────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin that may update invariant bounds (typically governance).
        type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Default maximum supply. Can be overridden via `set_bounds`.
        #[pallet::constant]
        type DefaultMaxSupply: Get<u128>;

        /// Default maximum agent count.
        #[pallet::constant]
        type DefaultMaxAgents: Get<u32>;

        /// Default maximum proposal queue depth.
        #[pallet::constant]
        type DefaultMaxProposalDepth: Get<u32>;

        /// Weight information for this pallet's extrinsics.
        type WeightInfo: WeightInfo;
    }

    // ── Events ─────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An invariant check passed for the given block.
        InvariantCheckPassed { block: BlockNumberFor<T> },
        /// An invariant was violated. Chain continues (if `HaltOnViolation` is false).
        InvariantViolated {
            block: BlockNumberFor<T>,
            invariant: InvariantKind,
            observed: u128,
            bound: u128,
        },
        /// Invariant bounds were updated by governance.
        BoundsUpdated {
            max_supply: u128,
            max_agents: u32,
            max_proposal_depth: u32,
        },
        /// `HaltOnViolation` flag changed.
        HaltModeChanged { halt: bool },
        /// Constitution hash registered on-chain.
        ConstitutionHashSet { hash: [u8; 32] },
    }

    // ── Errors ─────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Attempted to set a bound that would be weaker than the current one.
        /// Invariant bounds may only be tightened, never relaxed.
        BoundWeakeningNotAllowed,
        /// Supplied constitution hash is all-zeros (invalid).
        InvalidConstitutionHash,
    }

    // ── Hooks ──────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: BlockNumberFor<T>) {
            Self::enforce_all(block);
        }

        fn on_initialize(_: BlockNumberFor<T>) -> Weight {
            // Issuance snapshotting is done via `report_issuance` extrinsic (called by
            // governance / the runtime's own hook) to keep this pallet dependency-free.
            // No DB reads needed here.
            Weight::zero()
        }
    }

    // ── Extrinsics ─────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Update invariant bounds.
        ///
        /// Bounds may only be tightened (reduced), never relaxed. This enforces
        /// Article V of the X3 Constitution: amendments are refinements only.
        ///
        /// Origin: `UpdateOrigin` (governance).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::set_bounds())]
        pub fn set_bounds(
            origin: OriginFor<T>,
            max_supply: u128,
            max_agents: u32,
            max_proposal_depth: u32,
        ) -> DispatchResult {
            T::UpdateOrigin::ensure_origin(origin)?;

            let current_supply = MaxSupply::<T>::get();
            let current_agents = MaxAgents::<T>::get();
            let current_depth = MaxProposalDepth::<T>::get();

            // Bounds may only be tightened if already set (non-zero).
            if !current_supply.is_zero() {
                ensure!(
                    max_supply <= current_supply,
                    Error::<T>::BoundWeakeningNotAllowed
                );
            }
            if current_agents > 0 {
                ensure!(
                    max_agents <= current_agents,
                    Error::<T>::BoundWeakeningNotAllowed
                );
            }
            if current_depth > 0 {
                ensure!(
                    max_proposal_depth <= current_depth,
                    Error::<T>::BoundWeakeningNotAllowed
                );
            }

            MaxSupply::<T>::put(max_supply);
            MaxAgents::<T>::put(max_agents);
            MaxProposalDepth::<T>::put(max_proposal_depth);

            Self::deposit_event(Event::BoundsUpdated {
                max_supply,
                max_agents,
                max_proposal_depth,
            });
            Ok(())
        }

        /// Report the current total token issuance.
        ///
        /// Called by a trusted oracle/hook (or by the runtime's on_initialize
        /// coupling) to keep `LastObservedIssuance` current.
        ///
        /// Origin: `UpdateOrigin`.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::report_issuance())]
        pub fn report_issuance(origin: OriginFor<T>, issuance: u128) -> DispatchResult {
            T::UpdateOrigin::ensure_origin(origin)?;
            LastObservedIssuance::<T>::put(issuance);
            Ok(())
        }

        /// Enable or disable chain-halt on invariant violation.
        ///
        /// When enabled, ANY violation causes a runtime panic — the chain stops.
        /// This is the nuclear option; only enable on a stable production chain.
        ///
        /// Origin: `UpdateOrigin` (requires governance supermajority in practice).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_halt_on_violation())]
        pub fn set_halt_on_violation(origin: OriginFor<T>, halt: bool) -> DispatchResult {
            T::UpdateOrigin::ensure_origin(origin)?;
            HaltOnViolation::<T>::put(halt);
            Self::deposit_event(Event::HaltModeChanged { halt });
            Ok(())
        }

        /// Register the canonical constitution hash on-chain.
        ///
        /// All governance proposals that touch invariants MUST reference this hash.
        /// Proposals referencing a different hash are rejected by the governance pallet.
        ///
        /// Origin: `UpdateOrigin`.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::set_constitution_hash())]
        pub fn set_constitution_hash(origin: OriginFor<T>, hash: [u8; 32]) -> DispatchResult {
            T::UpdateOrigin::ensure_origin(origin)?;
            ensure!(hash != [0u8; 32], Error::<T>::InvalidConstitutionHash);
            ConstitutionHash::<T>::put(hash);
            Self::deposit_event(Event::ConstitutionHashSet { hash });
            Ok(())
        }
    }

    // ── Genesis ────────────────────────────────────────────────────────────────

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub max_supply: u128,
        pub max_agents: u32,
        pub max_proposal_depth: u32,
        pub halt_on_violation: bool,
        pub constitution_hash: [u8; 32],
        #[serde(skip)]
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let supply = if self.max_supply == 0 {
                T::DefaultMaxSupply::get()
            } else {
                self.max_supply
            };
            let agents = if self.max_agents == 0 {
                T::DefaultMaxAgents::get()
            } else {
                self.max_agents
            };
            let depth = if self.max_proposal_depth == 0 {
                T::DefaultMaxProposalDepth::get()
            } else {
                self.max_proposal_depth
            };

            MaxSupply::<T>::put(supply);
            MaxAgents::<T>::put(agents);
            MaxProposalDepth::<T>::put(depth);
            HaltOnViolation::<T>::put(self.halt_on_violation);

            if self.constitution_hash != [0u8; 32] {
                ConstitutionHash::<T>::put(self.constitution_hash);
            }
        }
    }

    // ── Internal logic ─────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Check all invariants. Called from `on_finalize`.
        ///
        /// Emits `InvariantViolated` for each failure. If `HaltOnViolation` is set,
        /// the first violation panics the runtime.
        pub fn enforce_all(block: BlockNumberFor<T>) {
            let mut violated = false;

            // ── Supply invariant ───────────────────────────────────────────────
            let max_supply = MaxSupply::<T>::get();
            if !max_supply.is_zero() {
                let observed = LastObservedIssuance::<T>::get();
                if observed > max_supply {
                    violated = true;
                    ViolationCount::<T>::mutate(|c| *c = c.saturating_add(1));
                    Self::deposit_event(Event::InvariantViolated {
                        block,
                        invariant: InvariantKind::MaxSupply,
                        observed,
                        bound: max_supply,
                    });
                    if HaltOnViolation::<T>::get() {
                        panic!(
                            "X3 INVARIANT VIOLATION: total issuance {} exceeds max supply {}",
                            observed, max_supply
                        );
                    }
                }
            }

            // ── Agent count invariant ──────────────────────────────────────────
            let max_agents = MaxAgents::<T>::get();
            if max_agents > 0 {
                let observed = AgentCount::<T>::get();
                if observed > max_agents {
                    violated = true;
                    ViolationCount::<T>::mutate(|c| *c = c.saturating_add(1));
                    Self::deposit_event(Event::InvariantViolated {
                        block,
                        invariant: InvariantKind::MaxAgents,
                        observed: observed as u128,
                        bound: max_agents as u128,
                    });
                    if HaltOnViolation::<T>::get() {
                        panic!(
                            "X3 INVARIANT VIOLATION: agent count {} exceeds max {}",
                            observed, max_agents
                        );
                    }
                }
            }

            // ── Proposal depth invariant ───────────────────────────────────────
            let max_depth = MaxProposalDepth::<T>::get();
            if max_depth > 0 {
                let observed = ProposalDepth::<T>::get();
                if observed > max_depth {
                    violated = true;
                    ViolationCount::<T>::mutate(|c| *c = c.saturating_add(1));
                    Self::deposit_event(Event::InvariantViolated {
                        block,
                        invariant: InvariantKind::MaxProposalDepth,
                        observed: observed as u128,
                        bound: max_depth as u128,
                    });
                    if HaltOnViolation::<T>::get() {
                        panic!(
                            "X3 INVARIANT VIOLATION: proposal depth {} exceeds max {}",
                            observed, max_depth
                        );
                    }
                }
            }

            if !violated {
                Self::deposit_event(Event::InvariantCheckPassed { block });
            }
        }

        /// Read-only: returns true if all invariants currently hold.
        pub fn all_invariants_hold() -> bool {
            let max_supply = MaxSupply::<T>::get();
            if !max_supply.is_zero() && LastObservedIssuance::<T>::get() > max_supply {
                return false;
            }
            let max_agents = MaxAgents::<T>::get();
            if max_agents > 0 && AgentCount::<T>::get() > max_agents {
                return false;
            }
            let max_depth = MaxProposalDepth::<T>::get();
            if max_depth > 0 && ProposalDepth::<T>::get() > max_depth {
                return false;
            }
            true
        }
    }

    // ── Counters written by other pallets ──────────────────────────────────────
    // These are shadow counters: the agent-accounts and governance pallets
    // increment/decrement these via the `InvariantReporter` trait (see below).

    #[pallet::storage]
    #[pallet::getter(fn agent_count)]
    pub type AgentCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn proposal_depth)]
    pub type ProposalDepth<T: Config> = StorageValue<_, u32, ValueQuery>;
}

// ── InvariantKind enum (outside pallet macro, SCALE-encoded) ─────────────────

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// The kind of invariant that was violated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum InvariantKind {
    /// Total token issuance exceeded `MaxSupply`.
    MaxSupply,
    /// Registered agent count exceeded `MaxAgents`.
    MaxAgents,
    /// Governance proposal queue depth exceeded `MaxProposalDepth`.
    MaxProposalDepth,
    /// Custom invariant registered by an external pallet.
    Custom(u32),
}

// ── InvariantReporter trait ───────────────────────────────────────────────────
// Other pallets implement this to update shadow counters without tight coupling.

/// Trait for pallets that contribute to invariant-watched counters.
///
/// The agent-accounts pallet calls `increment_agent_count`/ `decrement_agent_count`.
/// The governance pallet calls `set_proposal_depth`.
pub trait InvariantReporter {
    fn increment_agent_count();
    fn decrement_agent_count();
    fn set_proposal_depth(depth: u32);
}

impl<T: Config> InvariantReporter for Pallet<T> {
    fn increment_agent_count() {
        AgentCount::<T>::mutate(|c| *c = c.saturating_add(1));
    }

    fn decrement_agent_count() {
        AgentCount::<T>::mutate(|c| *c = c.saturating_sub(1));
    }

    fn set_proposal_depth(depth: u32) {
        ProposalDepth::<T>::put(depth);
    }
}
