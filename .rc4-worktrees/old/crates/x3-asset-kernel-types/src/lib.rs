// SPDX-License-Identifier: Apache-2.0
//
// X3 Universal Asset Kernel — shared types.
//
// Design rules enforced here (from TOKEN_SYSTEM_IMPLEMENTATION_ROADMAP.md):
//
//   1. One canonical AssetId per asset.          → `AssetId` + `derive_asset_id`
//   2. One supply ledger per asset.              → `SupplyLedger`
//   3. King invariant: represented ≤ canonical.  → `SupplyLedger::check_invariant`
//   4. Domain-separated message IDs.             → `derive_message_id`
//   5. Every transfer has a state machine.       → `TransferStatus::can_transition_to`
//   6. Every transfer has an expiry.             → `X3TransferMessage::expires_at`
//   7. Every recipient is typed by domain.       → `AccountBytes`
//   8. Every message is versioned.               → `MESSAGE_FORMAT_VERSION`
//   9. Integer-only decimal math.                → `convert_amount`
//
// This crate is `no_std` and carries no runtime state. It is a pure types library
// consumed by `pallet-x3-asset-registry`, `pallet-x3-supply-ledger`, and
// `pallet-x3-cross-vm-router`.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Shared types for the X3 Universal Asset Kernel.
//!
//! See [`derive_asset_id`] for the canonical AssetId derivation rule and
//! [`SupplyLedger::check_invariant`] for the king invariant.

use frame_support::{traits::ConstU32, BoundedVec};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// ── Versioning ──────────────────────────────────────────────────────────────

/// Wire-format version for [`X3TransferMessage`].
///
/// Bump this ONLY when the on-the-wire shape changes in a non-backwards-compatible
/// way. Consumers must reject messages with a version they do not understand.
pub const MESSAGE_FORMAT_VERSION: u16 = 1;

/// Domain-separation prefix for AssetId derivation.
pub const ASSET_ID_DOMAIN: &[u8] = b"X3_ASSET_ID_V1";

/// Domain-separation prefix for cross-VM / cross-domain transfer message IDs.
pub const TRANSFER_MESSAGE_DOMAIN: &[u8] = b"X3_CROSS_DOMAIN_TRANSFER_V1";

// ── Canonical AssetId ───────────────────────────────────────────────────────

/// Canonical, deterministic identifier for an asset.
///
/// Every representation (native balance, EVM ERC20 mirror, SVM token account,
/// external wrapped token) of a given asset maps to the same `AssetId`.
///
/// AssetIds are derived via [`derive_asset_id`] and are never assigned arbitrarily.
pub type AssetId = H256;

/// Balance type used by the kernel. Chosen wide enough to represent any reasonable
/// token supply × highest canonical decimals without truncation.
pub type Balance = u128;

/// 128-bit nonce space per (domain, sender) pair. Adequate for any realistic
/// throughput horizon.
pub type Nonce = u128;

/// Derive the canonical [`AssetId`] for an asset.
///
/// `blake2_256(ASSET_ID_DOMAIN || origin_domain || origin_chain_id || origin_address || symbol || decimals)`
///
/// The fact that derivation is domain-separated means an attacker cannot collide
/// two assets by picking clever symbol/decimal combinations — the origin domain
/// and chain id are always mixed in.
///
/// # Example
///
/// ```
/// use x3_asset_kernel_types::{derive_asset_id, DomainId};
///
/// // USDC on two different chains — must produce distinct AssetIds.
/// let id_usdc_eth = derive_asset_id(DomainId::Ethereum, 1, &[0xA0; 20], b"USDC", 6);
/// let id_usdc_base = derive_asset_id(DomainId::Base, 8453, &[0x83; 20], b"USDC", 6);
/// assert_ne!(id_usdc_eth, id_usdc_base, "USDC on Ethereum and Base are distinct assets");
/// ```
pub fn derive_asset_id(
    origin_domain: DomainId,
    origin_chain_id: u64,
    origin_asset_address: &[u8],
    symbol: &[u8],
    decimals: u8,
) -> AssetId {
    let mut preimage = Vec::with_capacity(
        ASSET_ID_DOMAIN.len() + 1 + 8 + origin_asset_address.len() + 2 + symbol.len() + 1,
    );
    preimage.extend_from_slice(ASSET_ID_DOMAIN);
    preimage.extend_from_slice(&(origin_domain as u8).to_be_bytes());
    preimage.extend_from_slice(&origin_chain_id.to_be_bytes());
    preimage.extend_from_slice(origin_asset_address);
    // Length-prefix the symbol to prevent boundary ambiguity.
    preimage.extend_from_slice(&(symbol.len() as u16).to_be_bytes());
    preimage.extend_from_slice(symbol);
    preimage.extend_from_slice(&decimals.to_be_bytes());
    H256::from(sp_io::hashing::blake2_256(&preimage))
}

// ── Domains ─────────────────────────────────────────────────────────────────

/// Settlement domain. A domain is either an X3-internal VM or an external chain.
///
/// MVP scope: only `X3Native`, `X3Evm`, `X3Svm` are enabled in routes.
/// External domains are reserved for the cross-chain gateway in a later phase.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    RuntimeDebug,
    Hash,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum DomainId {
    /// X3 native runtime (pallet balances / asset ledger).
    X3Native = 0,
    /// X3 EVM execution environment.
    X3Evm = 1,
    /// X3 SVM execution environment.
    X3Svm = 2,
    /// Ethereum mainnet (external).
    Ethereum = 10,
    /// Base L2 (external).
    Base = 11,
    /// Arbitrum One (external).
    Arbitrum = 12,
    /// BNB Smart Chain (external).
    Bsc = 13,
    /// Solana mainnet (external).
    Solana = 20,
    /// Bitcoin (external; custody-vault path).
    Bitcoin = 30,
}

impl DomainId {
    /// Returns true for X3-internal VMs. Used to gate atomic cross-VM transfers.
    pub const fn is_x3_internal(&self) -> bool {
        matches!(self, Self::X3Native | Self::X3Evm | Self::X3Svm)
    }
}

// ── Typed recipient addresses ───────────────────────────────────────────────

/// Bounded upper size for a recipient address payload.
///
/// Longest we realistically need is a 32-byte SVM pubkey or a Bitcoin script up
/// to ~80 bytes. 128 gives generous headroom without unbounded allocation.
pub const MAX_ACCOUNT_BYTES: u32 = 128;

/// Typed recipient address. Domain and address type are always carried together
/// so a `0x123...` hex blob is never ambiguous across VMs.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    RuntimeDebug,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AccountBytes {
    /// X3 native runtime account (32-byte sp_core::AccountId32-style).
    X3Native([u8; 32]),
    /// 20-byte Ethereum-style EVM address.
    Evm([u8; 20]),
    /// 32-byte Solana-style pubkey.
    Svm([u8; 32]),
    /// Bitcoin output (scriptPubKey bytes, variable length, bounded).
    ///
    /// Bounded by [`MAX_ACCOUNT_BYTES`] on decode. Anything larger is invalid.
    Bitcoin(BoundedVec<u8, ConstU32<MAX_ACCOUNT_BYTES>>),
}

impl AccountBytes {
    /// Return the domain this address natively belongs to.
    pub const fn natural_domain(&self) -> DomainId {
        match self {
            Self::X3Native(_) => DomainId::X3Native,
            Self::Evm(_) => DomainId::X3Evm,
            Self::Svm(_) => DomainId::X3Svm,
            Self::Bitcoin(_) => DomainId::Bitcoin,
        }
    }

    /// Return the address bytes (copied) without the type tag. For hashing only.
    pub fn raw(&self) -> Vec<u8> {
        match self {
            Self::X3Native(b) => b.to_vec(),
            Self::Evm(b) => b.to_vec(),
            Self::Svm(b) => b.to_vec(),
            Self::Bitcoin(b) => b.to_vec(),
        }
    }

    /// Check that this address is valid as a destination on the given domain.
    ///
    /// This is the front door of "wrong recipient type fails" — callers must
    /// invoke this before debiting anything.
    pub fn is_compatible_with(&self, dest: DomainId) -> bool {
        matches!(
            (self, dest),
            (Self::X3Native(_), DomainId::X3Native)
                | (
                    Self::Evm(_),
                    DomainId::X3Evm
                        | DomainId::Ethereum
                        | DomainId::Base
                        | DomainId::Arbitrum
                        | DomainId::Bsc,
                )
                | (Self::Svm(_), DomainId::X3Svm | DomainId::Solana)
                | (Self::Bitcoin(_), DomainId::Bitcoin)
        )
    }
}

// ── Asset metadata & policy ─────────────────────────────────────────────────

/// Classification of how supply is controlled for a given asset.
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
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SupplyPolicy {
    /// Asset is native to X3. Supply is minted/burned natively; representations
    /// in other VMs are mirrors constrained by the canonical ledger.
    NativeMintBurn,
    /// Asset originates externally. Moving IN = lock-and-mint; moving OUT = burn-and-release.
    LockMint,
    /// Asset originates externally but moves OUT via burn-and-release (inverse of LockMint).
    BurnRelease,
    /// Synthetic asset backed by liquidity reserves, not 1:1 collateral.
    ///
    /// Requires a solvency oracle. Not usable for MVP.
    LiquidityBacked,
}

/// Lifecycle status of an asset.
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
    Default,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AssetStatus {
    /// Asset is registered but not yet active; no routes will accept it.
    #[default]
    Registered,
    /// Asset is open for transfers on its enabled routes.
    Active,
    /// Asset is paused globally. No route will accept it until unpaused.
    ///
    /// Pauses are instant; unpauses require governance timelock.
    Paused,
    /// Asset has been retired and cannot be reactivated.
    Retired,
}

// ── Routes ──────────────────────────────────────────────────────────────────

/// Route identifier: the tuple `(asset, source, destination)` uniquely names a route.
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
pub struct RouteKey {
    /// Asset being transferred.
    pub asset_id: AssetId,
    /// Source settlement domain.
    pub source: DomainId,
    /// Destination settlement domain.
    pub destination: DomainId,
}

/// Per-route volume limits. Every route must declare these before being enabled.
///
/// "Bridges do not die from being too cautious. They die from 'send it.'"
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
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RouteLimits {
    /// Minimum transfer amount (in canonical units). Zero means no minimum.
    pub min_amount: Balance,
    /// Maximum single-transfer amount. Must be > 0 for an enabled route.
    pub max_amount: Balance,
    /// Rolling 24h volume cap across all senders.
    pub daily_limit: Balance,
    /// Rolling 24h volume cap per-wallet.
    pub per_wallet_daily_limit: Balance,
    /// Maximum number of simultaneously-pending (non-finalized, non-expired) transfers.
    pub pending_limit: u32,
}

impl RouteLimits {
    /// A permissive limits set — **development only**.
    pub const DEV_PERMISSIVE: Self = Self {
        min_amount: 0,
        max_amount: Balance::MAX,
        daily_limit: Balance::MAX,
        per_wallet_daily_limit: Balance::MAX,
        pending_limit: u32::MAX,
    };

    /// A conservative limits set for initial mainnet rollout (must be overridden per-asset).
    pub const MAINNET_CONSERVATIVE_INITIAL: Self = Self {
        min_amount: 0,
        max_amount: 10_000,
        daily_limit: 100_000,
        per_wallet_daily_limit: 25_000,
        pending_limit: 500,
    };
}

/// Full route configuration.
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
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RouteConfig {
    /// Is this route currently accepting transfers?
    pub enabled: bool,
    /// Volume limits.
    pub limits: RouteLimits,
    /// Protocol fee in basis points (1 bps = 0.01%).
    pub fee_bps: u16,
    /// Number of blocks a transfer on this route remains valid before it may be refunded.
    pub expiry_blocks: u32,
    /// Proof tier required to credit the destination.
    ///
    /// For cross-VM (internal) routes this is `TrustedInternal` — no off-chain proof
    /// is required, the kernel itself is the proof. For external chains this must
    /// be a tiered verifier.
    pub proof_tier: ProofTier,
}

impl RouteConfig {
    /// Build a cross-VM internal route configuration.
    pub const fn internal(limits: RouteLimits, expiry_blocks: u32) -> Self {
        Self {
            enabled: true,
            limits,
            fee_bps: 0,
            expiry_blocks,
            proof_tier: ProofTier::TrustedInternal,
        }
    }
}

// ── Proofs ──────────────────────────────────────────────────────────────────

/// Proof tier required to credit a destination for a given route.
///
/// `TrustedInternal` is the only tier valid for X3-internal cross-VM routes
/// because both legs execute inside the same finalized X3 block. External
/// routes must use a real tier (1 = validator attestation quorum, 2 =
/// light-client, 3 = zk). `MockForTesting` **must** panic under the
/// `production-verifier` feature at the call site.
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
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ProofTier {
    /// Internal cross-VM: the X3 kernel itself is the proof.
    TrustedInternal,
    /// External route proven by a validator attestation quorum.
    ValidatorQuorum,
    /// External route proven by a light-client inclusion proof.
    LightClient,
    /// External route proven by a zk/validity proof.
    Zk,
    /// Test-only verifier — **never enable on mainnet**.
    MockForTesting,
}

// ── Transfer message & state machine ────────────────────────────────────────

/// Immutable cross-domain transfer instruction.
///
/// The `message_id` is derived from the other fields plus the creation block,
/// so no caller can forge or reuse one. Replay protection is enforced at both
/// the message-id level (hash) and the per-(domain, sender, nonce) level.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    RuntimeDebug,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct X3TransferMessage<BlockNumber: MaxEncodedLen> {
    /// Wire-format version. Must equal [`MESSAGE_FORMAT_VERSION`].
    pub version: u16,
    /// Asset being transferred.
    pub asset_id: AssetId,
    /// Source settlement domain.
    pub source_domain: DomainId,
    /// Destination settlement domain.
    pub destination_domain: DomainId,
    /// Sender (typed by domain).
    pub sender: AccountBytes,
    /// Recipient (typed by domain).
    pub recipient: AccountBytes,
    /// Amount in canonical units (integer).
    pub amount: Balance,
    /// Strictly-monotonic per-sender nonce.
    pub nonce: Nonce,
    /// Block at which this message was created.
    pub created_at: BlockNumber,
    /// Block at which this message expires and may be refunded if not finalized.
    pub expires_at: BlockNumber,
}

/// State of a pending transfer. Only transitions listed in [`TransferStatus::can_transition_to`]
/// are allowed. All others are rejected by the router.
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
    Default,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TransferStatus {
    /// Message has been created but no side effects yet.
    #[default]
    Created,
    /// Source domain has been debited (funds removed from sender).
    SourceDebited,
    /// Destination domain has been credited (funds appear at recipient).
    DestinationCredited,
    /// Both sides have settled; the message is archived.
    Finalized,
    /// Expiry block passed before the message finalized.
    Expired,
    /// Funds have been returned to the sender after expiry.
    Refunded,
    /// Terminal failure state (e.g., proof verification failed permanently).
    Failed,
}

impl TransferStatus {
    /// Whitelist of allowed transitions. Any transition not listed here MUST be
    /// rejected by the router. This is the authoritative state graph.
    pub const fn can_transition_to(self, next: Self) -> bool {
        use TransferStatus::*;
        matches!(
            (self, next),
            (Created, SourceDebited)
                | (Created, Failed)
                | (SourceDebited, DestinationCredited)
                | (SourceDebited, Expired)
                | (SourceDebited, Failed)
                | (DestinationCredited, Finalized)
                | (Expired, Refunded)
                | (Expired, Failed)
        )
    }

    /// Terminal states can never transition further.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Finalized | Self::Refunded | Self::Failed)
    }
}

// ── Message ID derivation ───────────────────────────────────────────────────

/// Derive the deterministic message id for a transfer.
///
/// `blake2_256(TRANSFER_MESSAGE_DOMAIN || version || asset || src || dst || sender || recipient || amount || nonce || expires_at)`
///
/// All fields are SCALE-encoded into the preimage. The domain separator prevents
/// collision with any other hash used in the protocol. **Callers never supply
/// the message id themselves** — the router derives it.
pub fn derive_message_id<BlockNumber>(msg: &X3TransferMessage<BlockNumber>) -> H256
where
    BlockNumber: MaxEncodedLen + Encode,
{
    let mut preimage = Vec::with_capacity(256);
    preimage.extend_from_slice(TRANSFER_MESSAGE_DOMAIN);
    preimage.extend_from_slice(&msg.version.to_be_bytes());
    preimage.extend_from_slice(msg.asset_id.as_bytes());
    preimage.extend_from_slice(&(msg.source_domain as u8).to_be_bytes());
    preimage.extend_from_slice(&(msg.destination_domain as u8).to_be_bytes());
    preimage.extend_from_slice(&msg.sender.encode());
    preimage.extend_from_slice(&msg.recipient.encode());
    preimage.extend_from_slice(&msg.amount.to_be_bytes());
    preimage.extend_from_slice(&msg.nonce.to_be_bytes());
    preimage.extend_from_slice(&msg.expires_at.encode());
    H256::from(sp_io::hashing::blake2_256(&preimage))
}

// ── Supply ledger ───────────────────────────────────────────────────────────

/// Per-asset supply accounting. **This is the single source of truth for
/// "how much of this asset exists in each representation".**
///
/// The king invariant is:
///
/// ```text
/// native_supply + evm_supply + svm_supply + pending_supply
///     ≤ canonical_supply_or_locked_collateral
/// ```
///
/// Enforced by [`SupplyLedger::check_invariant`].
#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug, Default,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SupplyLedger {
    /// Total supply represented in the X3 native runtime ledger.
    pub native_supply: Balance,
    /// Total supply represented on the X3 EVM side.
    pub evm_supply: Balance,
    /// Total supply represented on the X3 SVM side.
    pub svm_supply: Balance,
    /// Total external locked / off-chain custody supply (external wrapped only).
    pub external_locked_supply: Balance,
    /// Supply that has been debited from one domain but not yet credited to another.
    /// Ephemeral; counts toward the invariant ceiling.
    pub pending_supply: Balance,
    /// The canonical cap: either the native total issuance ceiling (for NativeMintBurn)
    /// or the total external collateral locked (for LockMint / BurnRelease).
    pub canonical_supply: Balance,
}

/// Reason an invariant check failed.
#[derive(Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum InvariantError {
    /// Integer overflow while summing representations. Treated as a violation.
    ArithmeticOverflow,
    /// Sum of representations strictly exceeded canonical supply.
    SupplyCeilingExceeded,
}

impl SupplyLedger {
    /// Total represented supply (excluding canonical cap). Saturates on overflow.
    pub fn represented(&self) -> Option<Balance> {
        self.native_supply
            .checked_add(self.evm_supply)?
            .checked_add(self.svm_supply)?
            .checked_add(self.external_locked_supply)?
            .checked_add(self.pending_supply)
    }

    /// The king invariant.
    ///
    /// Must hold after every mutation of this ledger — mint, burn, lock,
    /// release, transfer step. Violation is unrecoverable and the caller must
    /// halt the state transition.
    pub fn check_invariant(&self) -> Result<(), InvariantError> {
        let represented = self
            .represented()
            .ok_or(InvariantError::ArithmeticOverflow)?;
        if represented > self.canonical_supply {
            return Err(InvariantError::SupplyCeilingExceeded);
        }
        Ok(())
    }
}

// ── Decimal math ────────────────────────────────────────────────────────────

/// Convert an amount between two decimal scales, integer-only, rejecting any
/// conversion that would lose precision.
///
/// # Example
///
/// ```
/// use x3_asset_kernel_types::convert_amount;
/// // 1 USDC (6dp) → 12dp canonical: 1_000_000 → 1_000_000_000_000
/// assert_eq!(convert_amount(1_000_000, 6, 12), Some(1_000_000_000_000));
/// // 1.5 USDC at 6dp → 4dp (lossy) → rejected
/// assert_eq!(convert_amount(1_500_000, 6, 4), Some(15_000));
/// // Odd amount that doesn't divide evenly → rejected.
/// assert_eq!(convert_amount(1_234_567, 6, 4), None);
/// ```
pub fn convert_amount(amount: Balance, from_decimals: u8, to_decimals: u8) -> Option<Balance> {
    if from_decimals == to_decimals {
        return Some(amount);
    }
    if to_decimals > from_decimals {
        let factor = 10u128.checked_pow((to_decimals - from_decimals) as u32)?;
        amount.checked_mul(factor)
    } else {
        let divisor = 10u128.checked_pow((from_decimals - to_decimals) as u32)?;
        if !amount.is_multiple_of(divisor) {
            // Lossy conversion — reject by default per integer-only rule.
            return None;
        }
        Some(amount / divisor)
    }
}

// ── Shared pallet trait surface ─────────────────────────────────────────────

/// Trait interfaces exposed by kernel pallets to each other. Placing them in
/// the shared types crate lets us wire the pallets via `Config` without any
/// pallet-to-pallet Cargo dependency (clean layering).
pub mod traits {
    use super::*;
    use sp_runtime::DispatchError;

    /// Read-only view of asset metadata. Implemented by `pallet-x3-asset-registry`.
    pub trait AssetRegistryInspect {
        /// Return `true` when the asset id is known to the registry.
        fn exists(asset_id: &AssetId) -> bool;

        /// Return the current lifecycle status for an asset, if registered.
        fn status(asset_id: &AssetId) -> Option<AssetStatus>;

        /// Return `true` when the asset exists and is currently active.
        fn is_active(asset_id: &AssetId) -> bool {
            matches!(Self::status(asset_id), Some(AssetStatus::Active))
        }

        /// Return the supply-control policy for the asset, if registered.
        fn supply_policy(asset_id: &AssetId) -> Option<SupplyPolicy>;

        /// Return the canonical decimal precision used for the asset.
        fn canonical_decimals(asset_id: &AssetId) -> Option<u8>;
    }

    /// Read-only view of the route table. Implemented by `pallet-x3-asset-registry`.
    pub trait RouteInspect {
        /// Return the route configuration for an asset/domain pair, if configured.
        fn route(
            asset_id: &AssetId,
            source: DomainId,
            destination: DomainId,
        ) -> Option<RouteConfig>;

        /// Return `true` when the configured route exists and is enabled.
        fn is_route_open(asset_id: &AssetId, source: DomainId, destination: DomainId) -> bool {
            Self::route(asset_id, source, destination)
                .map(|r| r.enabled)
                .unwrap_or(false)
        }
    }

    /// Transactional supply ledger mutations. Implemented by `pallet-x3-supply-ledger`.
    ///
    /// Every call must atomically re-check the king invariant and return an
    /// error without mutating state if it would be violated.
    pub trait SupplyLedgerWrite {
        /// source_supply -= amount; pending_supply += amount. Invariant preserved.
        fn debit_source_to_pending(
            asset_id: &AssetId,
            source_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError>;

        /// pending_supply -= amount; dest_supply += amount. Invariant preserved.
        fn credit_destination_from_pending(
            asset_id: &AssetId,
            destination_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError>;

        /// pending_supply -= amount; source_supply += amount. Invariant preserved.
        fn refund_pending_to_source(
            asset_id: &AssetId,
            source_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError>;

        /// Read-only ledger accessor (for explorers, invariant audits, tests).
        fn ledger(asset_id: &AssetId) -> Option<SupplyLedger>;
    }

    /// Read-only economic halt gate used by execution-facing pallets.
    ///
    /// When this returns `true`, new economic entrypoints (new transfers,
    /// swaps, launches, bundle submissions) must reject immediately, while
    /// recovery/refund flows remain available.
    pub trait EconomicHaltInspect {
        /// Returns `true` when new economic operations must be blocked.
        fn is_halted() -> bool;
    }

    /// Default open gate for pallets/tests that do not wire a halt provider.
    pub struct NoEconomicHalt;

    impl EconomicHaltInspect for NoEconomicHalt {
        fn is_halted() -> bool {
            false
        }
    }

    /// Privileged, origin-free registry mutations used by the token factory.
    ///
    /// These bypass the signed-origin checks performed by the registry's
    /// extrinsics. The caller (the factory pallet) is responsible for
    /// authorizing the action at its own layer (e.g. charging a launch fee,
    /// gating by `EnsureSigned`, etc.). Implementations MUST preserve every
    /// consistency check that the extrinsic form performs — duplicate asset
    /// detection, `TotalAssets` bound, self-loop / limits validation, etc.
    pub trait AssetRegistryMutate {
        /// Register an asset without an origin check. Returns the derived `AssetId`.
        fn do_register_asset(
            symbol: sp_std::vec::Vec<u8>,
            name: sp_std::vec::Vec<u8>,
            canonical_decimals: u8,
            origin_domain: DomainId,
            origin_chain_id: u64,
            origin_address: sp_std::vec::Vec<u8>,
            supply_policy: SupplyPolicy,
        ) -> Result<AssetId, DispatchError>;

        /// Move asset from `Registered` → `Active`.
        fn do_activate_asset(asset_id: &AssetId) -> Result<(), DispatchError>;

        /// Create/overwrite a route without an origin check.
        fn do_configure_route(
            asset_id: &AssetId,
            source: DomainId,
            destination: DomainId,
            config: RouteConfig,
        ) -> Result<(), DispatchError>;
    }

    /// Privileged supply mutations used by the token factory for initial
    /// minting and (where the token class permits) post-launch mint/burn.
    ///
    /// All calls MUST re-check the king invariant and roll back without any
    /// mutation if it would be violated.
    pub trait SupplyLedgerGovern {
        /// Mint `amount` of canonical supply for `asset_id` into `domain`.
        /// Grows both `canonical_supply` and the target leg.
        fn do_mint_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError>;

        /// Burn `amount` of supply for `asset_id` from `domain`.
        /// Shrinks both the leg and `canonical_supply` atomically.
        fn do_burn_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError>;
    }
}

// ── Token factory surface ──────────────────────────────────────────────────

/// Classification of a factory-launched token.
///
/// This is an **application-level** extension layered on top of the kernel's
/// [`SupplyPolicy`]. All classes map to the underlying `NativeMintBurn`
/// policy — the supply ledger's invariant is the only authority on how much
/// of each representation may exist. `TokenClass` merely dictates what the
/// factory will let the creator's `mint_authority` do after launch.
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
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenClass {
    /// Initial supply minted once at launch. No further mint is ever permitted.
    FixedSupply,
    /// Initial supply minted at launch; further mints allowed up to `max_supply`.
    CappedMintable,
    /// Initial supply minted at launch; the mint authority may burn supply
    /// permanently. Burns shrink `canonical_supply` and the source leg together.
    Burnable,
    /// Mint authority is governance, not the creator. For chain-native assets
    /// that must be provisioned by a runtime upgrade or council motion.
    GovernanceMintable,
    /// Reserved for assets bridged in from an external chain. Deferred past
    /// the factory MVP — the factory will reject this class until the
    /// cross-chain gateway is wired.
    WrappedExternal,
}

impl TokenClass {
    /// May the factory allow `mint_more` calls on a token of this class?
    pub const fn allows_post_launch_mint(&self) -> bool {
        matches!(self, Self::CappedMintable | Self::GovernanceMintable)
    }

    /// May the factory allow `burn` calls on a token of this class?
    pub const fn allows_burn(&self) -> bool {
        matches!(self, Self::Burnable)
    }

    /// Is this class currently supported for launch?
    pub const fn is_supported_at_launch(&self) -> bool {
        !matches!(self, Self::WrappedExternal)
    }
}

/// Receipt emitted by the factory when a token is successfully launched.
///
/// Carries the derived `AssetId`, the creator, and the initial supply. Indexers
/// can treat this as the "token exists" event.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    RuntimeDebug,
)]
pub struct TokenLaunchReceipt<AccountId, BlockNumber>
where
    AccountId: MaxEncodedLen,
    BlockNumber: MaxEncodedLen,
{
    /// Canonical asset id derived from the factory-launch parameters.
    pub asset_id: AssetId,
    /// Who launched it.
    pub creator: AccountId,
    /// Class of token launched.
    pub class: TokenClass,
    /// Decimals used for the canonical unit.
    pub canonical_decimals: u8,
    /// Total units minted at launch.
    pub initial_supply: Balance,
    /// Hard ceiling on supply (ignored for `FixedSupply` / `Burnable`).
    pub max_supply: Balance,
    /// Block at which the token was launched.
    pub launched_at: BlockNumber,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_origin_chains_produce_different_asset_ids() {
        let eth = derive_asset_id(DomainId::Ethereum, 1, b"\x00", b"USDC", 6);
        let base = derive_asset_id(DomainId::Base, 8453, b"\x00", b"USDC", 6);
        assert_ne!(eth, base);
    }

    #[test]
    fn transfer_status_state_machine_rejects_illegal_transitions() {
        use TransferStatus::*;
        // Happy path
        assert!(Created.can_transition_to(SourceDebited));
        assert!(SourceDebited.can_transition_to(DestinationCredited));
        assert!(DestinationCredited.can_transition_to(Finalized));
        // Expiry path
        assert!(SourceDebited.can_transition_to(Expired));
        assert!(Expired.can_transition_to(Refunded));
        // Illegal jumps
        assert!(!Created.can_transition_to(Finalized));
        assert!(!Created.can_transition_to(DestinationCredited));
        assert!(!Finalized.can_transition_to(Refunded));
        assert!(!Refunded.can_transition_to(DestinationCredited));
        assert!(!Failed.can_transition_to(Finalized));
        // Terminals cannot move
        assert!(Finalized.is_terminal());
        assert!(Refunded.is_terminal());
        assert!(Failed.is_terminal());
    }

    #[test]
    fn supply_invariant_holds_and_breaks_as_expected() {
        let mut l = SupplyLedger {
            native_supply: 100,
            evm_supply: 50,
            svm_supply: 50,
            external_locked_supply: 0,
            pending_supply: 0,
            canonical_supply: 200,
        };
        assert_eq!(l.check_invariant(), Ok(()));
        l.evm_supply = 51; // 101 + 50 + 50 = 201 > 200
        assert_eq!(
            l.check_invariant(),
            Err(InvariantError::SupplyCeilingExceeded)
        );
    }

    #[test]
    fn convert_amount_rejects_lossy_conversion() {
        assert_eq!(convert_amount(1_234_567, 6, 4), None);
        assert_eq!(convert_amount(1_000_000, 6, 4), Some(10_000));
        assert_eq!(convert_amount(1, 0, 18), Some(1_000_000_000_000_000_000));
    }

    #[test]
    fn account_bytes_domain_compatibility() {
        let evm = AccountBytes::Evm([0u8; 20]);
        assert!(evm.is_compatible_with(DomainId::X3Evm));
        assert!(evm.is_compatible_with(DomainId::Ethereum));
        assert!(!evm.is_compatible_with(DomainId::X3Svm));
        let svm = AccountBytes::Svm([0u8; 32]);
        assert!(svm.is_compatible_with(DomainId::X3Svm));
        assert!(!svm.is_compatible_with(DomainId::X3Evm));
    }

    #[test]
    fn message_id_is_deterministic_and_changes_on_any_field() {
        let base = X3TransferMessage::<u32> {
            version: MESSAGE_FORMAT_VERSION,
            asset_id: H256::repeat_byte(0x11),
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender: AccountBytes::X3Native([1u8; 32]),
            recipient: AccountBytes::Evm([2u8; 20]),
            amount: 1_000,
            nonce: 1,
            created_at: 100,
            expires_at: 200,
        };
        let id1 = derive_message_id(&base);
        let id2 = derive_message_id(&base);
        assert_eq!(id1, id2, "derivation must be deterministic");

        let mut different = base.clone();
        different.amount = 1_001;
        assert_ne!(derive_message_id(&different), id1);

        let mut different = base.clone();
        different.nonce = 2;
        assert_ne!(derive_message_id(&different), id1);
    }
}
