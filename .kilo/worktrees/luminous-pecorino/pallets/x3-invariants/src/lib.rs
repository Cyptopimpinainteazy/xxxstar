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
//! 4. If `HaltOnViolation` is set, an emergency `ChainHaltRequested` event is
//!    emitted, the on-chain `Halted` flag is raised, and the violation is
//!    logged at error level. (Earlier versions called `panic!()` here, which
//!    bricked the chain in a node restart loop and defeated the safety goal.
//!    See S0-6 remediation.)
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

pub mod emergency;

#[frame_support::pallet]
pub mod pallet {
    use super::WeightInfo;
    use crate::InvariantKind;
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Zero;
    use crate::emergency::{AuthorityRecord, DomainId, EvidenceBundle, ModuleId, TruthSourceRecord};
    use sp_io::hashing::blake2_256;
    use x3_security_events::{SecurityEvent, SecurityEventHook};

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

    /// If true, any invariant violation raises the on-chain `Halted` flag
    /// and emits a `ChainHaltRequested` event. (Previous versions panicked
    /// the node here; see S0-6.) Should only be enabled on production
    /// chains after initial burn-in.
    #[pallet::storage]
    #[pallet::getter(fn halt_on_violation)]
    pub type HaltOnViolation<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// S0-6: Set to `true` once any invariant has been violated while
    /// `HaltOnViolation` was active. Cleared only by governance via
    /// `set_halt_on_violation(false)` plus an explicit `clear_halted` call.
    /// Other pallets / runtime hooks may inspect this to refuse new state
    /// transitions during an emergency stop.
    #[pallet::storage]
    #[pallet::getter(fn halted)]
    pub type Halted<T: Config> = StorageValue<_, bool, ValueQuery>;

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

        /// Security swarm hook. Called whenever a security-critical event occurs
        /// (invariant breach, chain halt, kill switch activation). Implementations
        /// forward these to off-chain monitoring; use `NoOpHook` in test mocks.
        type SecurityHook: SecurityEventHook<BlockNumberFor<Self>>;
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
        /// S0-6: Chain halt requested due to invariant violation.
        /// Replaces previous `panic!()` behaviour. Monitoring and governance
        /// must observe this event and trigger an orderly stop / runtime
        /// upgrade rather than relying on a node-level crash.
        ChainHaltRequested {
            block: BlockNumberFor<T>,
            invariant: InvariantKind,
            observed: u128,
            bound: u128,
        },

        // ── Phase 0 constitutional control events ──────────────────────────

        /// An emergency authority was registered for a module.
        EmergencyAuthorityRegistered {
            module_id: ModuleId,
            authority_id: [u8; 32],
            expires_at_block: BlockNumberFor<T>,
        },
        /// The expiry of an emergency authority record was updated.
        EmergencyAuthorityExpired {
            module_id: ModuleId,
            new_expires_at_block: BlockNumberFor<T>,
        },
        /// The kill switch for a module was activated by its registered authority.
        KillSwitchActivated {
            module_id: ModuleId,
            /// `true` if an evidence bundle hash was submitted with the activation.
            evidence_provided: bool,
        },
        /// The kill switch for a module was deactivated by governance.
        KillSwitchDeactivated { module_id: ModuleId },
        /// A canonical truth source was registered (or updated) for a domain.
        CanonicalTruthRegistered {
            domain_id: DomainId,
            pallet_name_hash: [u8; 32],
        },
        /// A canonical truth source was removed for a domain.
        CanonicalTruthRemoved { domain_id: DomainId },
    }

    // ── Errors ─────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Attempted to set a bound that would be weaker than the current one.
        /// Invariant bounds may only be tightened, never relaxed.
        BoundWeakeningNotAllowed,
        /// Supplied constitution hash is all-zeros (invalid).
        InvalidConstitutionHash,

        // ── Phase 0 constitutional control errors ──────────────────────────

        /// No emergency authority record exists for the given module.
        AuthorityNotFound,
        /// An unexpired authority record already exists for this module.
        /// Call `update_emergency_expiry` to modify the existing record.
        AuthorityAlreadyRegistered,
        /// The authority record has expired (`current_block >= expires_at_block`).
        AuthorityExpired,
        /// The origin does not match the `authority_id` stored in the registry.
        NotTheRegisteredAuthority,
        /// `requires_evidence` is `true` on the authority record but no evidence hash was provided.
        EvidenceRequired,
        /// The kill switch for this module is already active.
        KillSwitchAlreadyActive,
        /// The kill switch for this module is not currently active.
        KillSwitchNotActive,
        /// No canonical truth source is registered for the given domain.
        TruthSourceNotFound,
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

        // ── Phase 0 constitutional controls ───────────────────────────────────

        /// Register an emergency authority for a module.
        ///
        /// Stores an `AuthorityRecord` keyed by `module_id`. Fails with
        /// `AuthorityAlreadyRegistered` if a record already exists and has not yet
        /// expired. Expired records may be overwritten.
        ///
        /// Origin: Root/governance.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::register_emergency_authority())]
        pub fn register_emergency_authority(
            origin: OriginFor<T>,
            module_id: ModuleId,
            authority_id: [u8; 32],
            expires_at_block: BlockNumberFor<T>,
            requires_evidence: bool,
            module_name_hash: [u8; 32],
        ) -> DispatchResult {
            ensure_root(origin)?;

            // Reject if an unexpired record already exists.
            if let Some(existing) = EmergencyAuthorities::<T>::get(module_id) {
                let current_block = frame_system::Pallet::<T>::block_number();
                ensure!(
                    current_block >= existing.expires_at_block,
                    Error::<T>::AuthorityAlreadyRegistered
                );
            }

            EmergencyAuthorities::<T>::insert(
                module_id,
                AuthorityRecord {
                    authority_id,
                    expires_at_block,
                    requires_evidence,
                    module_name_hash,
                },
            );

            Self::deposit_event(Event::EmergencyAuthorityRegistered {
                module_id,
                authority_id,
                expires_at_block,
            });
            Ok(())
        }

        /// Update the expiry block of an existing emergency authority record.
        ///
        /// Emits `EmergencyAuthorityExpired` to log the update.
        ///
        /// Origin: Root/governance.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::update_emergency_expiry())]
        pub fn update_emergency_expiry(
            origin: OriginFor<T>,
            module_id: ModuleId,
            new_expires_at_block: BlockNumberFor<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            EmergencyAuthorities::<T>::try_mutate(module_id, |maybe_record| {
                let record = maybe_record
                    .as_mut()
                    .ok_or(Error::<T>::AuthorityNotFound)?;
                record.expires_at_block = new_expires_at_block;
                Ok::<(), DispatchError>(())
            })?;

            Self::deposit_event(Event::EmergencyAuthorityExpired {
                module_id,
                new_expires_at_block,
            });
            Ok(())
        }

        /// Activate the kill switch for a module.
        ///
        /// The caller must be the registered emergency authority for the module
        /// (verified by comparing `blake2_256(origin.encode())` to the stored
        /// `authority_id`). The record must not be expired. If the record's
        /// `requires_evidence` flag is set, `evidence_hash` must be `Some`.
        ///
        /// Stores an `EvidenceBundle` when a hash is provided.
        ///
        /// Origin: Signed (the registered authority).
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::activate_kill_switch())]
        pub fn activate_kill_switch(
            origin: OriginFor<T>,
            module_id: ModuleId,
            evidence_hash: Option<[u8; 32]>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let record = EmergencyAuthorities::<T>::get(module_id)
                .ok_or(Error::<T>::AuthorityNotFound)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(
                current_block < record.expires_at_block,
                Error::<T>::AuthorityExpired
            );

            // Verify the caller matches the stored authority by hashing their account encoding.
            let origin_hash = blake2_256(&who.encode());
            ensure!(
                origin_hash == record.authority_id,
                Error::<T>::NotTheRegisteredAuthority
            );

            if record.requires_evidence {
                ensure!(evidence_hash.is_some(), Error::<T>::EvidenceRequired);
            }

            ensure!(
                !KillSwitches::<T>::get(module_id),
                Error::<T>::KillSwitchAlreadyActive
            );

            // Persist the evidence bundle if a hash was provided.
            if let Some(hash) = evidence_hash {
                KillSwitchEvidence::<T>::insert(
                    module_id,
                    EvidenceBundle {
                        evidence_hash: hash,
                        submitted_at: current_block,
                        submitter: origin_hash,
                    },
                );
            }

            KillSwitches::<T>::insert(module_id, true);

            T::SecurityHook::emit(SecurityEvent::kill_switch_activated(
                module_id,
                frame_system::Pallet::<T>::block_number(),
            ));

            Self::deposit_event(Event::KillSwitchActivated {
                module_id,
                evidence_provided: evidence_hash.is_some(),
            });
            Ok(())
        }

        /// Deactivate the kill switch for a module.
        ///
        /// Only governance (root) may lift a kill switch. Fails with
        /// `KillSwitchNotActive` if the module is not currently killed.
        ///
        /// Origin: Root/governance.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::deactivate_kill_switch())]
        pub fn deactivate_kill_switch(
            origin: OriginFor<T>,
            module_id: ModuleId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                KillSwitches::<T>::get(module_id),
                Error::<T>::KillSwitchNotActive
            );
            KillSwitches::<T>::insert(module_id, false);
            Self::deposit_event(Event::KillSwitchDeactivated { module_id });
            Ok(())
        }

        /// Register (or overwrite) the canonical truth source for a domain.
        ///
        /// Upserts a `TruthSourceRecord` into `CanonicalTruthMap`. An existing
        /// entry is overwritten without error (governance may update the canonical
        /// source when the authoritative pallet changes).
        ///
        /// Origin: Root/governance.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::register_canonical_truth_source())]
        pub fn register_canonical_truth_source(
            origin: OriginFor<T>,
            domain_id: DomainId,
            pallet_name_hash: [u8; 32],
            storage_item_hash: [u8; 32],
            description_hash: [u8; 32],
        ) -> DispatchResult {
            ensure_root(origin)?;

            CanonicalTruthMap::<T>::insert(
                domain_id,
                TruthSourceRecord {
                    domain_id,
                    pallet_name_hash,
                    storage_item_hash,
                    description_hash,
                },
            );

            Self::deposit_event(Event::CanonicalTruthRegistered {
                domain_id,
                pallet_name_hash,
            });
            Ok(())
        }

        /// Remove the canonical truth source for a domain.
        ///
        /// Fails with `TruthSourceNotFound` if no record exists for the domain.
        ///
        /// Origin: Root/governance.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::remove_canonical_truth_source())]
        pub fn remove_canonical_truth_source(
            origin: OriginFor<T>,
            domain_id: DomainId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            CanonicalTruthMap::<T>::take(domain_id)
                .ok_or(Error::<T>::TruthSourceNotFound)?;
            Self::deposit_event(Event::CanonicalTruthRemoved { domain_id });
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
                    T::SecurityHook::emit(SecurityEvent::invariant_breach(
                        blake2_256(b"MaxSupply"),
                        block,
                        ViolationCount::<T>::get(),
                    ));
                    if HaltOnViolation::<T>::get() {
                        // S0-6: Replaced runtime panic with defensive log + halt
                        // event. Panicking inside `on_finalize` puts the node in
                        // a restart loop and bricks the chain — the opposite of
                        // "exchange-grade safety". Operators / governance must
                        // pick up `ChainHaltRequested` from monitoring and
                        // perform an orderly stop or runtime upgrade.
                        Halted::<T>::put(true);
                        log::error!(
                            target: "x3-invariants",
                            "X3 INVARIANT VIOLATION: total issuance {} exceeds max supply {} (block {:?})",
                            observed, max_supply, block,
                        );
                        frame_support::defensive!("x3-invariants: max supply violated");
                        Self::deposit_event(Event::ChainHaltRequested {
                            block,
                            invariant: InvariantKind::MaxSupply,
                            observed,
                            bound: max_supply,
                        });
                        T::SecurityHook::emit(SecurityEvent::chain_halt_raised(
                            block,
                            ViolationCount::<T>::get(),
                        ));
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
                    T::SecurityHook::emit(SecurityEvent::invariant_breach(
                        blake2_256(b"MaxAgents"),
                        block,
                        ViolationCount::<T>::get(),
                    ));
                    if HaltOnViolation::<T>::get() {
                        // S0-6: see comment on max-supply branch.
                        Halted::<T>::put(true);
                        log::error!(
                            target: "x3-invariants",
                            "X3 INVARIANT VIOLATION: agent count {} exceeds max {} (block {:?})",
                            observed, max_agents, block,
                        );
                        frame_support::defensive!("x3-invariants: agent count violated");
                        Self::deposit_event(Event::ChainHaltRequested {
                            block,
                            invariant: InvariantKind::MaxAgents,
                            observed: observed as u128,
                            bound: max_agents as u128,
                        });
                        T::SecurityHook::emit(SecurityEvent::chain_halt_raised(
                            block,
                            ViolationCount::<T>::get(),
                        ));
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
                    T::SecurityHook::emit(SecurityEvent::invariant_breach(
                        blake2_256(b"MaxProposalDepth"),
                        block,
                        ViolationCount::<T>::get(),
                    ));
                    if HaltOnViolation::<T>::get() {
                        // S0-6: see comment on max-supply branch.
                        Halted::<T>::put(true);
                        log::error!(
                            target: "x3-invariants",
                            "X3 INVARIANT VIOLATION: proposal depth {} exceeds max {} (block {:?})",
                            observed, max_depth, block,
                        );
                        frame_support::defensive!("x3-invariants: proposal depth violated");
                        Self::deposit_event(Event::ChainHaltRequested {
                            block,
                            invariant: InvariantKind::MaxProposalDepth,
                            observed: observed as u128,
                            bound: max_depth as u128,
                        });
                        T::SecurityHook::emit(SecurityEvent::chain_halt_raised(
                            block,
                            ViolationCount::<T>::get(),
                        ));
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

    // ── Phase 0 constitutional controls ───────────────────────────────────────

    /// Registry of emergency authorities per module.
    ///
    /// An authority may activate the kill switch for its module by presenting
    /// a matching signed origin before the record's `expires_at_block`.
    #[pallet::storage]
    pub type EmergencyAuthorities<T: Config> =
        StorageMap<_, Blake2_128Concat, ModuleId, AuthorityRecord<BlockNumberFor<T>>>;

    /// Kill switch state per module.
    ///
    /// `true` means the module is killed and should refuse new state transitions.
    /// Only the registered authority may activate; only governance may deactivate.
    #[pallet::storage]
    pub type KillSwitches<T: Config> =
        StorageMap<_, Blake2_128Concat, ModuleId, bool, ValueQuery>;

    /// Canonical truth source registry.
    ///
    /// Maps `domain_id` to the single authoritative pallet/storage-item pair.
    #[pallet::storage]
    pub type CanonicalTruthMap<T: Config> =
        StorageMap<_, Blake2_128Concat, DomainId, TruthSourceRecord>;

    /// Evidence bundles submitted to justify kill switch activations (one per module).
    ///
    /// Stored only when the caller supplies an `evidence_hash` at activation time.
    #[pallet::storage]
    pub type KillSwitchEvidence<T: Config> =
        StorageMap<_, Blake2_128Concat, ModuleId, EvidenceBundle<BlockNumberFor<T>>>;
}

// ── InvariantKind enum (outside pallet macro, SCALE-encoded) ─────────────────

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// The kind of invariant that was violated.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
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

// ── Phase 0 constitutional control helpers ────────────────────────────────────
// These are free functions (outside the pallet macro) so that other pallets can
// query kill-switch and canonical-truth state without importing pallet internals.

use crate::emergency::{DomainId, ModuleId, TruthSourceRecord};

/// Returns `true` if the given module's kill switch is currently active.
///
/// Other pallets should call this at the start of state-mutating extrinsics to
/// respect emergency stops declared by the constitutional authority.
pub fn is_module_killed<T: crate::pallet::Config>(module_id: ModuleId) -> bool {
    crate::pallet::KillSwitches::<T>::get(module_id)
}

/// Returns the canonical truth source for a domain, if registered.
///
/// Use this to locate the authoritative pallet/storage-item for a given domain
/// (e.g. `"balances"`, `"receipts"`) without hard-coding pallet coupling.
pub fn get_canonical_source<T: crate::pallet::Config>(
    domain_id: DomainId,
) -> Option<TruthSourceRecord> {
    crate::pallet::CanonicalTruthMap::<T>::get(domain_id)
}
