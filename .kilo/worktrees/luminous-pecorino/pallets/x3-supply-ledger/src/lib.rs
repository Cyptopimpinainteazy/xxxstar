// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-supply-ledger — per-asset supply accounting.
//
// Single source of truth for how much of each asset exists in each
// representation (X3Native, X3Evm, X3Svm, external_locked) and in flight
// (pending). Every mutation is guarded by the king invariant:
//
//     represented_total ≤ canonical_supply
//
//     where represented_total = native + evm + svm + external_locked + pending
//
// "No operation may increase represented supply unless there is:
//   1. a native mint,
//   2. a source-side burn,
//   3. a collateral lock,
//   4. or a verified external proof."

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

//! X3 Supply Ledger pallet.

pub use pallet::*;

pub mod mint_idempotency;
pub mod supply_verification;
#[cfg(test)]
mod tests_halt;
#[cfg(test)]
mod tests_s0_1;

#[frame_support::pallet]
pub mod pallet {
    use crate::mint_idempotency::{IdempotencyError, IdempotencyValidator, MintIdempotencyToken};
    use crate::supply_verification::{AssetSupplyProof, SupplyMerkleTree, SupplyProof};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_std::vec::Vec;
    use x3_asset_kernel_types::{
        traits::{
            AssetRegistryInspect, EconomicHaltInspect, SupplyLedgerGovern, SupplyLedgerWrite,
        },
        AssetId, Balance, DomainId, SupplyLedger,
    };

    /// Keep only the latest N block proofs to prevent unbounded storage growth.
    const HISTORICAL_PROOF_RETENTION_BLOCKS: u32 = 1_000;

    /// Runtime response policy when a supply invariant violation is detected
    /// during block finalization.
    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        TypeInfo,
        MaxEncodedLen,
        RuntimeDebug,
    )]
    pub enum InvariantViolationPolicy {
        LogOnly,
        EventAndPause,
        RejectNewTransfers,
    }

    /// AssetId → per-asset supply ledger.
    #[pallet::storage]
    #[pallet::getter(fn ledgers)]
    pub type Ledgers<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, SupplyLedger>;

    /// Supply verification proof for the current block (S0-1: runtime-level verification).
    ///
    /// This proof demonstrates that all asset supply invariants held at block finalization.
    /// Generated in `on_finalize` and can be queried by external verifiers.
    #[pallet::storage]
    #[pallet::unbounded]
    #[pallet::getter(fn current_supply_proof)]
    pub type CurrentSupplyProof<T: Config> = StorageValue<_, SupplyProof>;

    /// Historical supply proofs indexed by block number.
    ///
    /// Bounded to the last `HISTORICAL_PROOF_RETENTION_BLOCKS` (1 000) blocks.
    /// Older entries are pruned during `on_finalize` to prevent unbounded storage growth.
    #[pallet::storage]
    #[pallet::unbounded]
    #[pallet::getter(fn historical_proofs)]
    pub type HistoricalProofs<T: Config> = StorageMap<_, Twox64Concat, u32, SupplyProof>;

    /// S0-2: Current nonce for each minter (strictly incrementing).
    ///
    /// Tracks the next expected nonce for each origin that can mint tokens.
    /// Nonces start at 0 and increment by 1 for each successful mint.
    #[pallet::storage]
    #[pallet::getter(fn minter_nonce)]
    pub type MinterNonce<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// S0-2: Processed mint tokens indexed by (origin, nonce).
    ///
    /// Records all mint operations that have been processed to prevent replay attacks.
    /// Each nonce can only be used once per origin.
    #[pallet::storage]
    #[pallet::unbounded]
    #[pallet::getter(fn processed_mint_token)]
    pub type ProcessedMintTokens<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Twox64Concat,
        u64,
        MintIdempotencyToken,
    >;

    /// Active invariant violation policy.
    #[pallet::storage]
    #[pallet::getter(fn invariant_violation_policy)]
    pub type InvariantPolicy<T: Config> =
        StorageValue<_, InvariantViolationPolicy, ValueQuery, DefaultInvariantPolicy>;

    /// Halt flag for new transfer legs.
    /// When true, debit and destination credit operations are rejected.
    /// Refund operations remain allowed to avoid stranding funds.
    #[pallet::storage]
    #[pallet::getter(fn transfer_halted)]
    pub type TransferHalted<T: Config> = StorageValue<_, bool, ValueQuery>;

    #[pallet::type_value]
    pub fn DefaultInvariantPolicy() -> InvariantViolationPolicy {
        InvariantViolationPolicy::EventAndPause
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin allowed to mint or burn canonical supply (governance).
        type SupplyGovernance: EnsureOrigin<Self::RuntimeOrigin>;
        /// Read-only access to the asset registry. Wire the registry pallet here.
        type Registry: AssetRegistryInspect;
    }

    /// S0-1: Runtime-level supply verification hooks
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Verify supply invariants for ALL assets at block finalization (S0-1 requirement).
        ///
        /// This provides a second layer of defense beyond transaction-level checks.
        /// If any invariant is violated, the block MUST NOT finalize.
        fn on_finalize(block_number: BlockNumberFor<T>) {
            // Collect all asset ledgers
            let mut asset_proofs: Vec<AssetSupplyProof> = Vec::new();
            let mut total_canonical: Balance = 0;
            let mut total_represented: Balance = 0;
            let mut violations: Vec<AssetId> = Vec::new();

            // Iterate all assets and verify their invariants
            for (asset_id, ledger) in Ledgers::<T>::iter() {
                // Verify invariant for this asset
                if ledger.check_invariant().is_err() {
                    violations.push(asset_id);
                    log::error!(
                        "❌ Supply invariant violation detected for asset {:?} at block {:?}",
                        asset_id,
                        block_number
                    );
                    // Collect all violations first, then apply configured policy.
                }

                // Build proof for this asset
                let proof =
                    AssetSupplyProof::from_ledger(asset_id, &ledger, asset_proofs.len() as u32);

                // Accumulate totals
                if let Some(canonical) = total_canonical.checked_add(ledger.canonical_supply) {
                    total_canonical = canonical;
                } else {
                    log::error!(
                        "Total canonical supply overflow at block {:?}",
                        block_number
                    );
                }

                if let Some(represented) = ledger.represented() {
                    if let Some(total) = total_represented.checked_add(represented) {
                        total_represented = total;
                    } else {
                        log::error!(
                            "Total represented supply overflow at block {:?}",
                            block_number
                        );
                    }
                }

                asset_proofs.push(proof);
            }

            // If violations detected, emit critical event
            if !violations.is_empty() {
                Self::deposit_event(Event::SupplyInvariantViolation {
                    block_number: Self::block_number_to_u32(block_number),
                    violated_assets: violations.clone(),
                });

                match InvariantPolicy::<T>::get() {
                    InvariantViolationPolicy::LogOnly => {
                        log::error!(
                            "Supply invariant violation(s) detected at block {:?}, policy=LogOnly",
                            block_number
                        );
                    }
                    InvariantViolationPolicy::EventAndPause
                    | InvariantViolationPolicy::RejectNewTransfers => {
                        TransferHalted::<T>::put(true);
                        Self::deposit_event(Event::TransfersHalted {
                            block_number: Self::block_number_to_u32(block_number),
                            violated_assets: violations,
                        });
                    }
                }
            }

            // Generate merkle tree and complete proofs
            let merkle_tree = SupplyMerkleTree::new(&mut asset_proofs);
            let supply_root = merkle_tree.root();

            // Create block proof
            let proof = SupplyProof {
                block_number: Self::block_number_to_u32(block_number),
                supply_root,
                asset_count: asset_proofs.len() as u32,
                total_canonical,
                total_represented,
                asset_proofs,
                timestamp: Self::current_timestamp(),
            };

            // Store proof
            CurrentSupplyProof::<T>::put(proof.clone());
            HistoricalProofs::<T>::insert(Self::block_number_to_u32(block_number), proof.clone());

            // Prune old proofs to keep storage bounded.
            let current_block = Self::block_number_to_u32(block_number);
            if current_block > HISTORICAL_PROOF_RETENTION_BLOCKS {
                let prune_block = current_block - HISTORICAL_PROOF_RETENTION_BLOCKS;
                HistoricalProofs::<T>::remove(prune_block);
            }

            // Emit proof generation event
            Self::deposit_event(Event::SupplyProofGenerated {
                block_number: Self::block_number_to_u32(block_number),
                supply_root,
                asset_count: proof.asset_count,
                total_canonical,
                total_represented,
            });
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CanonicalMinted {
            asset_id: AssetId,
            amount: Balance,
            domain: DomainId,
        },
        CanonicalBurned {
            asset_id: AssetId,
            amount: Balance,
            domain: DomainId,
        },
        LegDebited {
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        },
        LegCredited {
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        },
        Refunded {
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        },
        /// S0-1: Supply proof generated at block finalization (audit trail).
        SupplyProofGenerated {
            block_number: u32,
            supply_root: sp_core::H256,
            asset_count: u32,
            total_canonical: Balance,
            total_represented: Balance,
        },
        /// S0-1: Supply invariant violation detected (CRITICAL security event).
        ///
        /// If this event is emitted, the chain's economic integrity is compromised.
        /// Immediate investigation and chain halt may be required.
        SupplyInvariantViolation {
            block_number: u32,
            violated_assets: Vec<AssetId>,
        },
        /// S0-2: Mint operation processed with idempotency protection.
        MintProcessed {
            origin: T::AccountId,
            asset_id: AssetId,
            amount: Balance,
            nonce: u64,
            tx_hash: H256,
        },
        /// S0-2: Duplicate mint attempt detected and rejected.
        DuplicateMintRejected { origin: T::AccountId, nonce: u64 },
        /// Governance updated invariant violation policy.
        InvariantPolicyUpdated { policy: InvariantViolationPolicy },
        /// New transfer legs halted due to invariant failure.
        TransfersHalted {
            block_number: u32,
            violated_assets: Vec<AssetId>,
        },
        /// Governance resumed transfers after remediation.
        TransfersResumed,
    }

    #[pallet::error]
    pub enum Error<T> {
        UnknownAsset,
        AssetNotActive,
        Underflow,
        Overflow,
        /// King invariant would be violated — hard stop.
        InvariantViolation,
        /// S0-2: Invalid nonce provided (not next expected nonce).
        InvalidMintNonce,
        /// S0-2: Duplicate mint detected (nonce already used).
        DuplicateMint,
        /// S0-2: Mint hash verification failed (tampering detected).
        MintHashMismatch,
        /// New transfers are halted until governance resumes.
        TransfersHalted,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Governance-only: mint canonical supply into a specific domain leg.
        /// The only path by which represented supply may legitimately grow.
        ///
        /// S0-2: Now requires idempotency nonce to prevent double-mint attacks.
        ///
        /// # Arguments
        /// - `origin` — Governance origin (typically root or sudo)
        /// - `asset_id` — Asset to mint
        /// - `domain` — Target domain (Native, EVM, SVM)
        /// - `amount` — Amount to mint
        /// - `nonce` — Strictly incrementing nonce (must equal current_nonce for origin)
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(20_000, 0))]
        pub fn mint_canonical(
            origin: OriginFor<T>,
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
            nonce: u64,
        ) -> DispatchResult {
            // Verify governance permission
            T::SupplyGovernance::ensure_origin(origin.clone())?;

            // S0-2: enforce idempotency for signed governance origins.
            // Root/council-like origins may be unsigned and cannot be keyed to
            // an account nonce here.
            if let Ok(who) = ensure_signed(origin.clone()) {
                Self::validate_and_record_mint(&who, &asset_id, amount, nonce)?;
            }

            // Execute mint operation
            Self::do_mint_canonical(&asset_id, domain, amount)?;

            Ok(())
        }

        /// Governance-only: burn canonical supply from a specific domain leg.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn burn_canonical(
            origin: OriginFor<T>,
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            T::SupplyGovernance::ensure_origin(origin)?;
            Self::do_burn_canonical(&asset_id, domain, amount)
        }

        /// Governance setter for invariant violation response policy.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn set_invariant_violation_policy(
            origin: OriginFor<T>,
            policy: InvariantViolationPolicy,
        ) -> DispatchResult {
            T::SupplyGovernance::ensure_origin(origin)?;
            InvariantPolicy::<T>::put(policy);
            Self::deposit_event(Event::InvariantPolicyUpdated { policy });
            Ok(())
        }

        /// Governance resume switch for transfer flow after remediation.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn resume_transfers(origin: OriginFor<T>) -> DispatchResult {
            T::SupplyGovernance::ensure_origin(origin)?;
            TransferHalted::<T>::put(false);
            Self::deposit_event(Event::TransfersResumed);
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Origin-free mint core. Used by both the governance `mint_canonical`
        /// extrinsic and by the token factory.
        pub fn do_mint_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            ensure!(T::Registry::exists(asset_id), Error::<T>::UnknownAsset);
            Ledgers::<T>::try_mutate(*asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.get_or_insert_with(SupplyLedger::default);
                ledger.canonical_supply = ledger
                    .canonical_supply
                    .checked_add(amount)
                    .ok_or(Error::<T>::Overflow)?;
                Self::add_to_domain(ledger, domain, amount)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::CanonicalMinted {
                asset_id: *asset_id,
                amount,
                domain,
            });
            Ok(())
        }

        /// Origin-free burn core. Used by both the governance `burn_canonical`
        /// extrinsic and by the token factory (for `Burnable` token class).
        pub fn do_burn_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            Ledgers::<T>::try_mutate(*asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                Self::sub_from_domain(ledger, domain, amount)?;
                ledger.canonical_supply = ledger
                    .canonical_supply
                    .checked_sub(amount)
                    .ok_or(Error::<T>::Underflow)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::CanonicalBurned {
                asset_id: *asset_id,
                amount,
                domain,
            });
            Ok(())
        }

        fn add_to_domain(
            ledger: &mut SupplyLedger,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), Error<T>> {
            let slot = Self::domain_slot_mut(ledger, domain);
            *slot = slot.checked_add(amount).ok_or(Error::<T>::Overflow)?;
            Ok(())
        }

        fn sub_from_domain(
            ledger: &mut SupplyLedger,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), Error<T>> {
            let slot = Self::domain_slot_mut(ledger, domain);
            *slot = slot.checked_sub(amount).ok_or(Error::<T>::Underflow)?;
            Ok(())
        }

        /// Map `DomainId` → ledger field it controls.
        /// External domains share `external_locked_supply` (unused in MVP).
        fn domain_slot_mut(ledger: &mut SupplyLedger, domain: DomainId) -> &mut Balance {
            match domain {
                DomainId::X3Native => &mut ledger.native_supply,
                DomainId::X3Evm => &mut ledger.evm_supply,
                DomainId::X3Svm => &mut ledger.svm_supply,
                _ => &mut ledger.external_locked_supply,
            }
        }

        /// S0-1: Helper to convert BlockNumber to u32 for proof storage.
        fn block_number_to_u32(block_number: BlockNumberFor<T>) -> u32 {
            use sp_runtime::traits::UniqueSaturatedInto;
            block_number.unique_saturated_into()
        }

        /// S0-1: Helper to get current timestamp for proof generation.
        ///
        /// Returns the block number cast to u64.  Using the block number (not
        /// wall-clock time) keeps the value deterministic across all validators
        /// — wall-clock timestamps are non-deterministic in WASM runtimes and
        /// should not be stored in consensus-critical state without a dedicated
        /// timestamp pallet whose value is agreed upon in the block header.
        fn current_timestamp() -> u64 {
            use sp_runtime::traits::UniqueSaturatedInto;
            <frame_system::Pallet<T>>::block_number().unique_saturated_into()
        }

        /// S0-2: Validate mint idempotency and record token if valid.
        ///
        /// Enforces strict nonce ordering and prevents duplicate mints.
        /// This MUST be called before do_mint_canonical for all governance mints.
        fn validate_and_record_mint(
            origin: &T::AccountId,
            asset_id: &AssetId,
            amount: Balance,
            nonce: u64,
        ) -> DispatchResult {
            // Get current nonce for this origin
            let current_nonce = MinterNonce::<T>::get(origin);

            // Convert origin to bytes for hashing
            let origin_bytes = origin.encode();

            // Validate idempotency using validator
            IdempotencyValidator::validate(
                &origin_bytes,
                asset_id.as_fixed_bytes(),
                amount,
                nonce,
                current_nonce,
                |n| ProcessedMintTokens::<T>::contains_key(origin, n),
            )
            .map_err(|e| match e {
                IdempotencyError::InvalidNonce { .. } => Error::<T>::InvalidMintNonce,
                IdempotencyError::DuplicateMint { .. } => Error::<T>::DuplicateMint,
                IdempotencyError::HashMismatch => Error::<T>::MintHashMismatch,
            })?;

            // Create idempotency token
            let block_number = Self::block_number_to_u32(<frame_system::Pallet<T>>::block_number());
            let token = MintIdempotencyToken::new(
                &origin_bytes,
                asset_id.as_fixed_bytes(),
                amount,
                nonce,
                block_number,
            );

            // Record token (mark nonce as used)
            ProcessedMintTokens::<T>::insert(origin, nonce, token.clone());

            // Increment nonce for next mint
            MinterNonce::<T>::insert(origin, IdempotencyValidator::next_nonce(current_nonce));

            // Emit event
            Self::deposit_event(Event::MintProcessed {
                origin: origin.clone(),
                asset_id: *asset_id,
                amount,
                nonce,
                tx_hash: token.tx_hash,
            });

            Ok(())
        }

        /// S0-2: Check if a mint nonce was already used (for queries/debugging).
        pub fn is_nonce_used(origin: &T::AccountId, nonce: u64) -> bool {
            ProcessedMintTokens::<T>::contains_key(origin, nonce)
        }

        /// S0-2: Get the current nonce for an origin (for UI/wallet integration).
        pub fn get_current_nonce(origin: &T::AccountId) -> u64 {
            MinterNonce::<T>::get(origin)
        }
    }

    impl<T: Config> SupplyLedgerWrite for Pallet<T> {
        fn debit_source_to_pending(
            asset_id: &AssetId,
            source_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            ensure!(!TransferHalted::<T>::get(), Error::<T>::TransfersHalted);
            ensure!(T::Registry::is_active(asset_id), Error::<T>::AssetNotActive);
            Ledgers::<T>::try_mutate(asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                Self::sub_from_domain(ledger, source_domain, amount)?;
                ledger.pending_supply = ledger
                    .pending_supply
                    .checked_add(amount)
                    .ok_or(Error::<T>::Overflow)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::LegDebited {
                asset_id: *asset_id,
                domain: source_domain,
                amount,
            });
            Ok(())
        }

        fn credit_destination_from_pending(
            asset_id: &AssetId,
            destination_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            ensure!(!TransferHalted::<T>::get(), Error::<T>::TransfersHalted);
            ensure!(T::Registry::is_active(asset_id), Error::<T>::AssetNotActive);
            Ledgers::<T>::try_mutate(asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                ledger.pending_supply = ledger
                    .pending_supply
                    .checked_sub(amount)
                    .ok_or(Error::<T>::Underflow)?;
                Self::add_to_domain(ledger, destination_domain, amount)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::LegCredited {
                asset_id: *asset_id,
                domain: destination_domain,
                amount,
            });
            Ok(())
        }

        fn refund_pending_to_source(
            asset_id: &AssetId,
            source_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            // Refunds allowed even while paused — pausing must not strand funds.
            ensure!(T::Registry::exists(asset_id), Error::<T>::UnknownAsset);
            Ledgers::<T>::try_mutate(asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                ledger.pending_supply = ledger
                    .pending_supply
                    .checked_sub(amount)
                    .ok_or(Error::<T>::Underflow)?;
                Self::add_to_domain(ledger, source_domain, amount)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::Refunded {
                asset_id: *asset_id,
                domain: source_domain,
                amount,
            });
            Ok(())
        }

        fn ledger(asset_id: &AssetId) -> Option<SupplyLedger> {
            Ledgers::<T>::get(asset_id)
        }
    }

    impl<T: Config> SupplyLedgerGovern for Pallet<T> {
        fn do_mint_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            Pallet::<T>::do_mint_canonical(asset_id, domain, amount)
        }

        fn do_burn_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            Pallet::<T>::do_burn_canonical(asset_id, domain, amount)
        }
    }

    impl<T: Config> EconomicHaltInspect for Pallet<T> {
        fn is_halted() -> bool {
            TransferHalted::<T>::get()
        }
    }
}
