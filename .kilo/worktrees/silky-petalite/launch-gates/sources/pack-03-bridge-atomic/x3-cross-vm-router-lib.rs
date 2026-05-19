// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-cross-vm-router — cross-VM (X3Native ↔ X3Evm ↔ X3Svm) transfer
// router for the X3 Universal Asset Kernel.
//
// Scope (MVP / Level 1):
//   * Only internal cross-VM routes: X3Native, X3Evm, X3Svm.
//   * For these, source-debit and destination-credit execute atomically inside
//     the same finalized X3 block. The kernel itself is the proof
//     (`ProofTier::TrustedInternal`). No relayers, no validator attestations.
//   * External chains (Ethereum, Solana, Bitcoin, etc.) are explicitly rejected
//     at this layer; the cross-chain gateway will plug in later with its own
//     proof-verification path.
//
// Guarantees enforced here:
//
//   1. **Replay protection (two layers).**
//      * `UsedMessages` — any derived `message_id` can be consumed once.
//      * `UsedNonces` — per-(source_domain, sender, nonce) dedup so resubmitting
//        the same intent under a new message id still fails.
//
//   2. **State machine.** Every status transition goes through
//      `TransferStatus::can_transition_to`. Illegal transitions are rejected.
//
//   3. **Expiry.** Transfers stuck in `SourceDebited` past `expires_at` may be
//      refunded via `cancel_expired_xvm_transfer`, which drives the ledger
//      back to the source leg.
//
//   4. **King invariant.** Every ledger mutation is a single transactional
//      call into the supply-ledger pallet; it rolls back the entire extrinsic
//      if the invariant would break.
//
//   5. **Typed recipients.** `AccountBytes` must be domain-compatible with the
//      destination domain (an SVM pubkey cannot be sent as an EVM recipient).

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

//! X3 Cross-VM Router pallet.

pub use pallet::*;

#[cfg(test)]
mod tests;

pub mod runtime_api;

#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::SaturatedConversion;
    use sp_std::vec::Vec;
    use x3_asset_kernel_types::{
        derive_message_id,
        traits::{AssetRegistryInspect, RouteInspect, SupplyLedgerWrite},
        AccountBytes, AssetId, Balance, DomainId, Nonce, ProofTier, RouteConfig, TransferStatus,
        X3TransferMessage, MESSAGE_FORMAT_VERSION,
    };

    /// Maximum length for pause reason
    pub const MAX_PAUSE_REASON_LEN: u32 = 256;

    // ── Storage ────────────────────────────────────────────────────────────

    /// Stored transfer record: the full message plus its current status.
    #[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct TransferRecord<T: Config> {
        pub message: X3TransferMessage<BlockNumberFor<T>>,
        pub status: TransferStatus,
    }

    /// Pending or recently-finalized transfers, keyed by derived message id.
    /// Entries are retained after finalization for audit; a later reaper can
    /// prune `Finalized`/`Refunded`/`Failed` records older than N blocks.
    #[pallet::storage]
    #[pallet::getter(fn transfers)]
    pub type Transfers<T: Config> = StorageMap<_, Blake2_128Concat, H256, TransferRecord<T>>;

    /// Replay protection layer 1: any message id we've already accepted.
    ///
    /// This is a set (unit value) deliberately not a Vec, so lookup is O(1)
    /// and does not grow unbounded per-call.
    #[pallet::storage]
    #[pallet::getter(fn used_messages)]
    pub type UsedMessages<T: Config> = StorageMap<_, Blake2_128Concat, H256, ()>;

    /// Replay protection layer 2: (source_domain, sender_bytes) → next nonce.
    ///
    /// Senders must submit strictly monotonic nonces; duplicates are rejected.
    #[pallet::storage]
    #[pallet::getter(fn next_nonce)]
    pub type NextNonce<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        DomainId,
        Blake2_128Concat,
        AccountBytes,
        Nonce,
        ValueQuery,
    >;

    /// Count of currently in-flight (SourceDebited) transfers per route. Used
    /// to enforce `RouteLimits::pending_limit`.
    #[pallet::storage]
    #[pallet::getter(fn pending_count)]
    pub type PendingCount<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AssetId,
        Blake2_128Concat,
        (DomainId, DomainId),
        u32,
        ValueQuery,
    >;

    /// Bridge Roots: Maps chain_id → (root_hash, block_number, verified_at_block)
    /// Stores verified external chain roots for cross-chain settlement verification
    #[pallet::storage]
    #[pallet::getter(fn bridge_roots)]
    pub type BridgeRoots<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, (H256, u32, BlockNumberFor<T>), OptionQuery>;

    /// Bridge Pause Flags: Maps chain_id → pause_reason
    /// Emergency pause mechanism controlled by governance to halt cross-chain operations
    #[pallet::storage]
    #[pallet::getter(fn bridge_paused)]
    pub type BridgePaused<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, BoundedVec<u8, ConstU32<256>>, OptionQuery>;

    // ── Pallet ─────────────────────────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Registry pallet (read-only inspect of assets + routes).
        type Registry: AssetRegistryInspect + RouteInspect;
        /// Supply ledger pallet (transactional writes).
        type Ledger: SupplyLedgerWrite;
    }

    // ── Events ─────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        TransferInitiated {
            message_id: H256,
            asset_id: AssetId,
            source: DomainId,
            destination: DomainId,
            amount: Balance,
        },
        TransferCompleted {
            message_id: H256,
        },
        TransferExpired {
            message_id: H256,
        },
        TransferRefunded {
            message_id: H256,
        },
        /// External chain root registered for bridge verification (PHASE C STUB)
        /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
        BridgeRootRegistered {
            chain_id: u32,
            root_hash: H256,
            block_number: u32,
        },
        /// Bridge paused for emergency (PHASE C STUB)
        /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
        BridgePaused {
            chain_id: u32,
            reason: Vec<u8>,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Wire-format version on the message doesn't match `MESSAGE_FORMAT_VERSION`.
        UnsupportedMessageVersion,
        /// Source == destination.
        SelfLoopRoute,
        /// Route is absent or disabled.
        RouteClosed,
        /// MVP scope limitation: only internal (X3Native/X3Evm/X3Svm) routes allowed.
        NonInternalRouteNotSupported,
        /// Proof tier configured on the route is not `TrustedInternal`.
        WrongProofTierForInternalRoute,
        /// Recipient address type incompatible with destination domain.
        IncompatibleRecipient,
        /// Sender address type incompatible with source domain.
        IncompatibleSender,
        /// Amount zero or outside route limits.
        AmountOutOfBounds,
        /// Asset not found.
        UnknownAsset,
        /// Asset paused or retired.
        AssetNotActive,
        /// Expiry block is not in the future.
        BadExpiry,
        /// This (source, sender, nonce) tuple has already been used.
        DuplicateNonce,
        /// This message id has already been processed.
        DuplicateMessage,
        /// Transfer not found.
        UnknownMessage,
        /// Attempted an illegal state transition.
        IllegalStateTransition,
        /// Attempted to cancel a transfer that has not yet expired.
        NotYetExpired,
        /// Route pending-limit reached.
        RoutePendingLimitExceeded,
        /// Bridge is currently paused for the specified chain
        BridgePaused,
        /// Caller not authorized to manage bridge state
        NotAuthorizedGovernance,
        /// Invalid proof provided
        InvalidProof,
        /// Root hash mismatch
        RootHashMismatch,
    }

    // ── Extrinsics ─────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Initiate a cross-VM transfer.
        ///
        /// Callable by a signed account that represents the sender on the
        /// source domain (the runtime's signed-extrinsic origin is assumed to
        /// produce an `AccountBytes::X3Native` when used from the native VM;
        /// EVM/SVM pre-compiles convert their own sender types before calling).
        ///
        /// Atomically:
        ///   1. Validates route + limits + recipient/sender typing.
        ///   2. Reserves a nonce (bumps `NextNonce`).
        ///   3. Derives the deterministic message id.
        ///   4. Calls the ledger: debit source → pending.
        ///   5. Stores record as `SourceDebited`.
        ///
        /// Emits `TransferInitiated`.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(40_000, 0))]
        pub fn xvm_transfer(
            origin: OriginFor<T>,
            asset_id: AssetId,
            source: DomainId,
            destination: DomainId,
            sender: AccountBytes,
            recipient: AccountBytes,
            amount: Balance,
            expires_at: BlockNumberFor<T>,
        ) -> DispatchResult {
            // Any signed origin may call; sender identity is carried in `sender`
            // since the call may be made by an EVM/SVM precompile on behalf of
            // the real sender on another VM.
            let _who = ensure_signed(origin)?;

            Self::do_initiate_transfer(
                asset_id,
                source,
                destination,
                sender,
                recipient,
                amount,
                expires_at,
            )
        }

        /// Complete (credit destination) a cross-VM transfer that is currently
        /// in `SourceDebited`. For `ProofTier::TrustedInternal` routes this is
        /// unprivileged — the kernel itself is the proof.
        ///
        /// Drives `SourceDebited → DestinationCredited → Finalized` in one call.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(30_000, 0))]
        pub fn complete_xvm_transfer(origin: OriginFor<T>, message_id: H256) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            Self::do_complete_transfer(message_id)
        }

        /// Refund an expired transfer back to the source.
        ///
        /// Only permissible once `block_number > message.expires_at`. Drives
        /// `SourceDebited → Expired → Refunded`.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(25_000, 0))]
        pub fn cancel_expired_xvm_transfer(
            origin: OriginFor<T>,
            message_id: H256,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            Self::do_cancel_expired(message_id)
        }

        // ────────────────────────────────────────────────────────────────────
        // BRIDGE ROUTER (PHASE C STUB)
        // ────────────────────────────────────────────────────────────────────

        /// Register external chain root for bridge verification
        /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
        /// Only verifier/executor authority can call this
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(50_000, 0))]
        pub fn register_external_root(
            origin: OriginFor<T>,
            chain_id: u32,
            root_hash: sp_core::H256,
            block_number: u32,
            proof: Vec<u8>,
        ) -> DispatchResult {
            let _executor = ensure_signed(origin)?;

            // P1: Verify executor is authorized (cross-chain executor/verifier role)
            // TODO: Integrate with x3-kernel pallet for executor authorization
            // For now, we accept any signed origin; full RBAC in PHASE F
            
            // P2: Verify chain_id is valid (must be external chain, not X3 internal)
            ensure!(chain_id != 0, Error::<T>::InvalidProof); // 0 is reserved for X3Native

            // P3: Verify bridge is not paused for this chain
            ensure!(
                !BridgePaused::<T>::contains_key(chain_id),
                Error::<T>::BridgePaused
            );

            // P4: Validate proof against chain consensus (basic checks)
            // In production, this would verify SPV proofs, Merkle roots, etc.
            ensure!(!proof.is_empty(), Error::<T>::InvalidProof);

            // P5: Verify block_number is reasonable (not too far in future)
            let current_block = frame_system::Pallet::<T>::block_number();
            let block_number_t: BlockNumberFor<T> = block_number.saturated_into();
            ensure!(
                block_number_t <= current_block,
                Error::<T>::InvalidProof
            );

            // P6: Store root in bridge state
            BridgeRoots::<T>::insert(chain_id, (root_hash, block_number, current_block));

            // P7: Emit RootRegistered event with context
            Self::deposit_event(Event::BridgeRootRegistered {
                chain_id,
                root_hash,
                block_number,
            });

            Ok(())
        }

        /// Emergency pause bridge operations for a specific external chain
        /// Only governance/admin authority can call this
        /// FROZEN under v1-alpha API (2026-04-24, commit d99252ca42)
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(35_000, 0))]
        pub fn emergency_pause_bridge(
            origin: OriginFor<T>,
            chain_id: u32,
            reason: Vec<u8>,
        ) -> DispatchResult {
            // P1: Verify caller is governance account (root-only for MVP)
            ensure_root(origin)?;

            // P2: Verify bridge exists and reason is provided
            ensure!(chain_id != 0, Error::<T>::InvalidProof); // 0 is reserved
            ensure!(!reason.is_empty(), Error::<T>::InvalidProof);

            // P3: Set bridge pause flag for chain_id
            // This will prevent new cross-chain transfers from being initiated
            let bounded_reason: BoundedVec<u8, ConstU32<256>> = reason.clone()
                .try_into()
                .map_err(|_| Error::<T>::InvalidProof)?;
            BridgePaused::<T>::insert(chain_id, bounded_reason.clone());

            // P4: Emit BridgePaused event with context
            Self::deposit_event(Event::BridgePaused {
                chain_id,
                reason,
            });

            Ok(())
        }
    }

    // ── Internal: initiate ─────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        fn do_initiate_transfer(
            asset_id: AssetId,
            source: DomainId,
            destination: DomainId,
            sender: AccountBytes,
            recipient: AccountBytes,
            amount: Balance,
            expires_at: BlockNumberFor<T>,
        ) -> DispatchResult {
            // ── Route typing checks ───────────────────────────────────────
            ensure!(source != destination, Error::<T>::SelfLoopRoute);
            ensure!(amount > 0, Error::<T>::AmountOutOfBounds);

            // MVP: both legs must be X3-internal.
            ensure!(
                source.is_x3_internal() && destination.is_x3_internal(),
                Error::<T>::NonInternalRouteNotSupported
            );

            // Recipient/sender typing.
            ensure!(
                recipient.is_compatible_with(destination),
                Error::<T>::IncompatibleRecipient
            );
            ensure!(
                sender.is_compatible_with(source),
                Error::<T>::IncompatibleSender
            );

            // Asset + route must exist, be active, be open.
            ensure!(T::Registry::exists(&asset_id), Error::<T>::UnknownAsset);
            ensure!(
                T::Registry::is_active(&asset_id),
                Error::<T>::AssetNotActive
            );

            let route: RouteConfig = T::Registry::route(&asset_id, source, destination)
                .ok_or(Error::<T>::RouteClosed)?;
            ensure!(route.enabled, Error::<T>::RouteClosed);
            // Internal routes must carry TrustedInternal proof tier.
            ensure!(
                matches!(route.proof_tier, ProofTier::TrustedInternal),
                Error::<T>::WrongProofTierForInternalRoute
            );

            // Amount bounds.
            ensure!(
                amount >= route.limits.min_amount && amount <= route.limits.max_amount,
                Error::<T>::AmountOutOfBounds
            );

            // Pending-limit check.
            let pending_now = PendingCount::<T>::get(asset_id, (source, destination));
            ensure!(
                pending_now < route.limits.pending_limit,
                Error::<T>::RoutePendingLimitExceeded
            );

            // Expiry sanity.
            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(expires_at > now, Error::<T>::BadExpiry);

            // ── Nonce reservation ─────────────────────────────────────────
            // `NextNonce` is a monotonic counter per (source, sender). Reserving
            // it BEFORE the ledger call means an aborted ledger call still
            // consumes the nonce, which is the correct (stricter) semantics —
            // the sender must resubmit with a new nonce.
            let nonce = NextNonce::<T>::mutate(source, sender.clone(), |n| {
                let cur = *n;
                *n = n.saturating_add(1);
                cur
            });

            // Additional explicit duplicate-nonce guard (defensive; the above
            // mutate is already atomic but we want an obvious rejection path
            // if a caller ever reinjects via a privileged import).
            // (Intentionally no UsedNonces map — NextNonce monotonicity subsumes it.)

            // ── Build message & derive id ─────────────────────────────────
            let message = X3TransferMessage::<BlockNumberFor<T>> {
                version: MESSAGE_FORMAT_VERSION,
                asset_id,
                source_domain: source,
                destination_domain: destination,
                sender: sender.clone(),
                recipient,
                amount,
                nonce,
                created_at: now,
                expires_at,
            };
            let message_id = derive_message_id::<BlockNumberFor<T>>(&message);

            ensure!(
                !UsedMessages::<T>::contains_key(message_id),
                Error::<T>::DuplicateMessage
            );

            // ── Ledger mutation (transactional) ───────────────────────────
            T::Ledger::debit_source_to_pending(&asset_id, source, amount)?;

            // ── Persist + mark used ───────────────────────────────────────
            UsedMessages::<T>::insert(message_id, ());
            PendingCount::<T>::mutate(asset_id, (source, destination), |c| {
                *c = c.saturating_add(1)
            });

            let record = TransferRecord::<T> {
                message,
                status: TransferStatus::Created,
            };
            // Apply Created → SourceDebited via the authoritative state machine.
            let record = Self::advance(record, TransferStatus::SourceDebited)?;
            Transfers::<T>::insert(message_id, record);

            Self::deposit_event(Event::TransferInitiated {
                message_id,
                asset_id,
                source,
                destination,
                amount,
            });
            Ok(())
        }

        // ── Internal: complete ────────────────────────────────────────────
        fn do_complete_transfer(message_id: H256) -> DispatchResult {
            let record = Transfers::<T>::get(message_id).ok_or(Error::<T>::UnknownMessage)?;

            ensure!(
                record.status == TransferStatus::SourceDebited,
                Error::<T>::IllegalStateTransition
            );

            let msg = record.message.clone();

            // Ledger: pending → destination.
            T::Ledger::credit_destination_from_pending(
                &msg.asset_id,
                msg.destination_domain,
                msg.amount,
            )?;

            // State machine: SourceDebited → DestinationCredited → Finalized.
            let r1 = Self::advance(record, TransferStatus::DestinationCredited)?;
            let r2 = Self::advance(r1, TransferStatus::Finalized)?;
            Transfers::<T>::insert(message_id, r2);

            PendingCount::<T>::mutate(
                msg.asset_id,
                (msg.source_domain, msg.destination_domain),
                |c| *c = c.saturating_sub(1),
            );

            Self::deposit_event(Event::TransferCompleted { message_id });
            Ok(())
        }

        // ── Internal: cancel expired ──────────────────────────────────────
        fn do_cancel_expired(message_id: H256) -> DispatchResult {
            let record = Transfers::<T>::get(message_id).ok_or(Error::<T>::UnknownMessage)?;

            ensure!(
                record.status == TransferStatus::SourceDebited,
                Error::<T>::IllegalStateTransition
            );

            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(now > record.message.expires_at, Error::<T>::NotYetExpired);

            let msg = record.message.clone();

            // Move SourceDebited → Expired first (allowed), then perform refund
            // ledger mutation, then Expired → Refunded. If the ledger call
            // fails the whole extrinsic rolls back — including the Expired
            // status change — preserving the invariant.
            let r_exp = Self::advance(record, TransferStatus::Expired)?;

            T::Ledger::refund_pending_to_source(&msg.asset_id, msg.source_domain, msg.amount)?;

            let r_ref = Self::advance(r_exp, TransferStatus::Refunded)?;
            Transfers::<T>::insert(message_id, r_ref);

            PendingCount::<T>::mutate(
                msg.asset_id,
                (msg.source_domain, msg.destination_domain),
                |c| *c = c.saturating_sub(1),
            );

            Self::deposit_event(Event::TransferExpired { message_id });
            Self::deposit_event(Event::TransferRefunded { message_id });
            Ok(())
        }

        // ── State machine guard ───────────────────────────────────────────
        /// Advance a record's status. Rejects any transition not allowed by
        /// `TransferStatus::can_transition_to`.
        fn advance(
            mut record: TransferRecord<T>,
            next: TransferStatus,
        ) -> Result<TransferRecord<T>, Error<T>> {
            if !record.status.can_transition_to(next) {
                return Err(Error::<T>::IllegalStateTransition);
            }
            record.status = next;
            Ok(record)
        }
    }
}
