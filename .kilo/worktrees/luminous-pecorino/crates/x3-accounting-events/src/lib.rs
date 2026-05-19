//! # X3 Accounting Events — Phase 3 Revenue Spine
//!
//! This crate defines the **single canonical accounting event type** for the X3 DeFi
//! revenue spine. Every revenue-generating module in the X3 multichain system — swaps,
//! lending, flashloans, auctions, DNS, dApp hub, RPC monetization, AI services, and
//! more — **must** emit one [`AccountingEvent`] for each material financial action
//! via the [`AccountingSpine`] trait instead of emitting bespoke pallet events.
//!
//! ## Why a Shared Spine?
//!
//! X3 operates a fee-as-infrastructure model that requires authoritative implementation
//! rules: where fees are taken, what counts as principal versus yield, and how splits are
//! routed. Without a shared event type every pallet invents its own schema, making
//! treasury reconciliation, auditing, and cross-chain accounting impossible to automate.
//!
//! The target is **one accounting spine** that covers all revenue categories today and
//! remains forward-compatible via [`RevenueModule::Other`].
//!
//! ## Usage Pattern
//!
//! ```rust,ignore
//! // Inside a pallet dispatchable:
//! let event = AccountingEvent::fee_collected(
//!     RevenueModule::Swap,
//!     0,                    // source_chain (native X3)
//!     asset_id,
//!     principal,
//!     fee_amount,
//!     FeeDestination::ProtocolTreasury,
//!     receipt_id,
//!     <frame_system::Pallet<T>>::block_number(),
//! );
//! S::emit(event);           // S: AccountingSpine bound on Config
//! ```
//!
//! ## `NoOpSpine` — Wiring in Progress
//!
//! Pallets that have not yet been connected to a live spine implementation use
//! [`NoOpSpine`]. It satisfies the [`AccountingSpine`] trait bound at zero cost:
//! the compiler elides the call entirely. Replace it with the real spine
//! implementation (`x3-accounting-spine` crate, Phase 3 milestone) once that crate
//! is ready.
//!
//! ## `no_std` Compatibility
//!
//! This crate is `no_std` when the `std` feature is disabled. It is safe to use
//! inside WASM runtimes. The only heap allocation is the `Vec` inside [`FeeSplits`],
//! which is why [`AccountingEvent`] intentionally does **not** implement
//! `MaxEncodedLen` — it is an off-chain / event-stream type, not a storage map value.
//! Types intended for Substrate storage ([`RevenueModule`], [`AccountingEventKind`],
//! [`FeeDestination`]) do implement `MaxEncodedLen`.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
// Allow the pedantic `module_name_repetitions` lint; the type names are intentionally
// explicit so that downstream consumers can glob-import this crate unambiguously.
#![allow(clippy::module_name_repetitions)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[cfg(test)]
mod tests;

// ─── Revenue module identifier ───────────────────────────────────────────────

/// Identifies which X3 revenue module generated an accounting event.
///
/// Add new variants here as X3 expands. For modules not yet enumerated, use
/// [`RevenueModule::Other`] with a `blake2_256` hash of the module name so that
/// downstream indexers can map it without a runtime upgrade.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum RevenueModule {
    /// AMM / CLMM swap fees.
    Swap,
    /// Lending protocol origination and interest fees.
    Lending,
    /// Flashloan premium.
    Flashloan,
    /// Dutch / English / sealed-bid auction settlement fees.
    Auction,
    /// DNS name registration and renewal fees.
    DnsRegistration,
    /// dApp hub listing and interaction fees.
    DappHub,
    /// RPC monetization — pay-per-call or subscription fees.
    RpcMonetization,
    /// AI inference and orchestration service fees.
    AiService,
    /// Bot rental / automation subscription fees.
    BotRental,
    /// Launchpad token sale platform fees.
    LaunchpadToken,
    /// Launchpad NFT mint platform fees.
    LaunchpadNft,
    /// Validator pre-sale allocation fees.
    ValidatorPresale,
    /// Blockspace auction (priority fee market) proceeds.
    BlockspaceAuction,
    /// Cross-chain bridge usage fees.
    Bridge,
    /// Forward-compatible catch-all: `blake2_256` of the module name (UTF-8).
    Other([u8; 32]),
}

// ─── Accounting event kind ───────────────────────────────────────────────────

/// Classifies what financial action occurred within a revenue module.
///
/// The kind is used by the accounting spine to route events to the correct
/// reconciliation bucket (e.g. revenue recognition vs. settlement vs. treasury).
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum AccountingEventKind {
    // --- Revenue recognition ------------------------------------------------
    /// A fee was taken from a user action and is ready for distribution.
    FeeCollected,
    /// Yield accrued on a lending position or liquidity provision.
    YieldAccrued,
    /// Principal funds moved between protocol accounts (no fee taken).
    PrincipalMoved,
    /// A fee was split and each portion routed to its destination.
    SplitRouted,

    // --- Settlement ---------------------------------------------------------
    /// A pending route was fully settled on-chain.
    RouteSettled,
    /// A pending route failed and no fees were taken.
    RouteFailed,
    /// A previously recorded event was reversed (refund / revert).
    RollbackExecuted,

    // --- Cross-chain --------------------------------------------------------
    /// A fee that originated on a remote chain was settled on this chain.
    CrossChainFeeSettled,
    /// Maintenance / relay cost accrued for a cross-chain operation.
    MaintenanceAccrued,

    // --- Treasury -----------------------------------------------------------
    /// Funds flowed into the protocol treasury.
    TreasuryInflow,
    /// Funds flowed out of the protocol treasury (grant, burn, etc.).
    TreasuryOutflow,
    /// The insurance fund was drawn to cover a deficit.
    InsuranceDraw,

    // --- Reconciliation -----------------------------------------------------
    /// A reconciliation window was opened (e.g. end-of-epoch audit).
    ReconciliationOpened,
    /// The reconciliation window was closed with a matching balance.
    ReconciliationClosed,
    /// The reconciliation window was closed with a detected mismatch.
    ReconciliationMismatch,
}

// ─── Fee destination ─────────────────────────────────────────────────────────

/// Specifies where a collected fee ultimately goes.
///
/// When a fee is split among multiple recipients, set this to
/// [`FeeDestination::Split`] and populate [`FeeSplits`] with the breakdown.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum FeeDestination {
    /// Fee goes to the on-chain protocol treasury pallet.
    ProtocolTreasury,
    /// Fee is permanently burned (deflation mechanism).
    Burned,
    /// Fee is distributed pro-rata to active liquidity providers.
    LiquidityProviders,
    /// Fee is credited to a specific referrer (`blake2_256` of their account).
    Referrer([u8; 32]),
    /// Fee is added to the validator rewards pool for the current era.
    ValidatorRewards,
    /// Fee is deposited into the protocol insurance fund.
    InsuranceFund,
    /// Fee is split: see [`FeeSplits`] for the individual portions.
    Split,
}

// ─── Fee splits ──────────────────────────────────────────────────────────────

/// A single entry in a fee-split breakdown.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct FeeSplitEntry<Balance> {
    /// Where this portion of the fee goes.
    pub destination: FeeDestination,
    /// Absolute amount of the fee sent to this destination.
    pub amount: Balance,
    /// Share of the total fee expressed in basis points (0–10 000).
    ///
    /// The spine may verify that all entries in a [`FeeSplits`] sum to 10 000
    /// bps for fully-split events, but this is not enforced at the type level.
    pub basis_points: u32,
}

/// Up to N fee-split entries for a single accounting event.
///
/// Using `Vec` keeps the type `no_std`-friendly via `sp_std` and avoids a fixed
/// `BoundedVec` capacity that would require a runtime `Get` type parameter. As
/// noted in the crate docs, [`AccountingEvent`] is an off-chain / event-stream
/// type — it is never written to Substrate storage maps directly.
///
/// Callers are expected to keep the number of split entries small (≤ 4 is the
/// conventional maximum defined in the X3 fee policy).
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct FeeSplits<Balance> {
    pub entries: sp_std::vec::Vec<FeeSplitEntry<Balance>>,
}

impl<Balance: Default> Default for FeeSplits<Balance> {
    fn default() -> Self {
        Self {
            entries: sp_std::vec::Vec::new(),
        }
    }
}

// ─── Canonical accounting event ──────────────────────────────────────────────

/// A single canonical accounting event emitted by any X3 revenue module.
///
/// Every revenue-generating module **must** emit one of these for each material
/// financial action. The accounting spine can then reconstruct treasury reports,
/// reconcile by module / chain / asset / receipt, and verify revenue totals
/// without any per-module custom logic.
///
/// # Field Semantics
///
/// - `principal` is the **gross** amount before fee deduction.
/// - `fee` is the amount taken from `principal`; it may be 0 for non-fee events
///   such as `PrincipalMoved`.
/// - `receipt_id` links this event to the on-chain receipt or route ID so the
///   spine can correlate events within a single user operation.
/// - `source_chain` and `dest_chain` use CAIP-2 numeric chain IDs; 0 means the
///   native X3 chain. For local events, both fields are equal.
///
/// # Why `AccountingEvent` Does Not Implement `MaxEncodedLen`
///
/// [`FeeSplits`] contains a `Vec` whose length is unbounded at compile time.
/// `MaxEncodedLen` requires a statically-known upper bound; absent a storage
/// `Get<u32>` bound we cannot provide one. Since this type is emitted as an
/// off-chain event (not stored in a FRAME `StorageMap`), the omission is correct.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountingEvent<BlockNumber, Balance> {
    /// Which revenue module generated this event.
    pub module: RevenueModule,
    /// What financial action occurred.
    pub kind: AccountingEventKind,
    /// Source chain (CAIP-2 numeric). 0 = native X3 chain.
    pub source_chain: u32,
    /// Destination chain for cross-chain events. Equal to `source_chain` for
    /// local events.
    pub dest_chain: u32,
    /// Asset identifier — matches the `AssetId` used by `x3-inventory`.
    pub asset_id: u32,
    /// Gross principal amount involved, before fee deduction.
    pub principal: Balance,
    /// Fee amount collected. May be 0 for non-fee event kinds.
    pub fee: Balance,
    /// Where the collected fee is routed. Use [`FeeDestination::Split`] when
    /// `splits` is non-empty.
    pub fee_destination: FeeDestination,
    /// Detailed breakdown of a split fee (empty for non-split events).
    pub splits: FeeSplits<Balance>,
    /// Opaque receipt or route ID linking this event to an on-chain receipt.
    /// Typically `blake2_256(route_id.encode())`.
    pub receipt_id: [u8; 32],
    /// Block number at which this event was emitted.
    pub block: BlockNumber,
    /// `true` when this event is part of a cross-chain atomic sequence.
    pub is_cross_chain: bool,
}

// ─── Builder helpers ─────────────────────────────────────────────────────────

impl<BlockNumber: Default + Copy, Balance: Default + Copy>
    AccountingEvent<BlockNumber, Balance>
{
    /// Construct a [`AccountingEventKind::FeeCollected`] event for a local
    /// (single-chain) fee collection.
    ///
    /// `dest_chain` is set equal to `source_chain` and `is_cross_chain` is
    /// `false`. `splits` is left empty; populate it after construction if the
    /// fee is split and `fee_destination` is [`FeeDestination::Split`].
    #[must_use]
    pub fn fee_collected(
        module: RevenueModule,
        source_chain: u32,
        asset_id: u32,
        principal: Balance,
        fee: Balance,
        fee_destination: FeeDestination,
        receipt_id: [u8; 32],
        block: BlockNumber,
    ) -> Self {
        Self {
            module,
            kind: AccountingEventKind::FeeCollected,
            source_chain,
            dest_chain: source_chain,
            asset_id,
            principal,
            fee,
            fee_destination,
            splits: FeeSplits::default(),
            receipt_id,
            block,
            is_cross_chain: false,
        }
    }

    /// Construct a [`AccountingEventKind::CrossChainFeeSettled`] event.
    ///
    /// `is_cross_chain` is set to `true` and `fee_destination` defaults to
    /// [`FeeDestination::Split`] because cross-chain fees are always split
    /// between the source-chain protocol treasury, the bridge relay, and the
    /// destination-chain pool. Populate `splits` after construction.
    ///
    /// # Panics (debug only)
    ///
    /// In debug builds this will `debug_assert!` that `source_chain !=
    /// dest_chain`. Cross-chain events with identical chains should use
    /// [`Self::fee_collected`] instead.
    #[must_use]
    pub fn cross_chain_fee(
        module: RevenueModule,
        source_chain: u32,
        dest_chain: u32,
        asset_id: u32,
        principal: Balance,
        fee: Balance,
        receipt_id: [u8; 32],
        block: BlockNumber,
    ) -> Self {
        debug_assert!(
            source_chain != dest_chain,
            "cross_chain_fee: source_chain and dest_chain must differ; \
             use fee_collected for single-chain events"
        );
        Self {
            module,
            kind: AccountingEventKind::CrossChainFeeSettled,
            source_chain,
            dest_chain,
            asset_id,
            principal,
            fee,
            fee_destination: FeeDestination::Split,
            splits: FeeSplits::default(),
            receipt_id,
            block,
            is_cross_chain: true,
        }
    }
}

// ─── Accounting spine trait ───────────────────────────────────────────────────

/// Implemented by any type that can receive canonical accounting events.
///
/// Pallets call [`AccountingSpine::emit`] instead of emitting ad-hoc raw events,
/// ensuring the revenue spine receives every material financial action in a
/// uniform, machine-readable form.
///
/// # Implementing the Spine
///
/// A full spine implementation typically:
/// 1. Appends the event to an off-chain-worker queue (e.g. `sp_io::offchain`).
/// 2. Emits a lightweight on-chain event for indexers (`frame_system::Pallet::deposit_event`).
/// 3. Optionally writes a summary hash to on-chain storage for reconciliation.
///
/// Phase 3 of the X3 go-mode sequence will ship `x3-accounting-spine` which
/// provides the production implementation. Until then, wire pallets with
/// [`NoOpSpine`].
pub trait AccountingSpine<BlockNumber, Balance> {
    /// Emit a single canonical accounting event to the revenue spine.
    ///
    /// Implementations must be infallible: they may log a warning on internal
    /// queue overflow but must never return an error or panic, since this is
    /// called inside dispatchables where a panic would roll back the entire block.
    fn emit(event: AccountingEvent<BlockNumber, Balance>);
}

// ─── No-op spine ─────────────────────────────────────────────────────────────

/// A zero-cost no-op implementation of [`AccountingSpine`].
///
/// Use this as the `Spine` associated type on pallet `Config` while the
/// production spine crate (`x3-accounting-spine`) is still under development.
/// The compiler will eliminate every call to `NoOpSpine::emit` entirely.
///
/// # Example — pallet Config
///
/// ```rust,ignore
/// impl pallet_x3_swap::Config for Runtime {
///     type AccountingSpine = x3_accounting_events::NoOpSpine;
///     // ... other items
/// }
/// ```
pub struct NoOpSpine;

impl<B, C> AccountingSpine<B, C> for NoOpSpine {
    #[inline(always)]
    fn emit(_event: AccountingEvent<B, C>) {
        // Intentional no-op. The compiler optimises this away completely.
    }
}
