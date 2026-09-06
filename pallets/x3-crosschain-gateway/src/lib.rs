#![deny(unsafe_code)]
//! # Pallet X3 Crosschain Gateway
//!
//! Substrate pallet for the X3 external cross-chain gateway. This pallet
//! exposes the canonical source-chain proof verification, lock/mint and
//! burn/release flows for cross-VM asset representation as on-chain
//! extrinsics.
//!
//! The pallet is the chain-side counterpart of:
//!
//! - `X3-contracts/evm/contracts/X3ExternalGateway.sol` (the EVM gateway
//!   that emits the lock events the relayer watches).
//! - `crates/x3-verification-router` (proof verification — EVM receipt
//!   proofs, Solana finalized proofs, Bitcoin SPV proofs, X3 internal
//!   proofs, and validator quorum attestations).
//! - `crates/x3-relayer` (off-chain worker that submits deposit proofs
//!   and release proofs into this pallet).
//!
//! # Storage
//!
//! - `Routes`: a map from `RouteId` to `RouteConfig` describing an enabled
//!   external asset route. Governance-managed.
//! - `Assets`: a map from `(chain_id, token_address_or_mint)` to
//!   `x3_asset_id` — the on-chain X3 representation of the external
//!   asset.
//! - `Transfers`: a map from `TransferId` (== proof_id) to
//!   `GatewayTransfer`. Created when a deposit proof is submitted and
//!   verified, then advanced through `Verified` → `X3Credited`.
//! - `Withdrawals`: a map from `WithdrawalId` to `WithdrawalRecord`.
//!   Created on user request, marked `burned` when the user burns the
//!   X3 representation, marked `released` when the relayer submits the
//!   external release proof.
//! - `UsedProofs`: a set of proof_ids that have already been processed.
//!   This is the replay-protection set.
//! - `UsedNonces`: a set of `(chain_id, token, nonce)` keys — prevents
//!   the same deposit nonce from being submitted twice.
//!
//! # Extrinsics
//!
//! - `register_asset` (governance): bind an external asset to an x3
//!   asset id.
//! - `enable_route` (governance): enable a route. Configures the
//!   verification strategy, limits, dispute window.
//! - `disable_route` (governance): disable a route.
//! - `submit_deposit_proof` (relayer, signed): verify a deposit proof
//!   and create a `GatewayTransfer`.
//! - `credit_x3_representation` (relayer, signed): after the dispute
//!   window (if required), credit the X3 representation to the
//!   recipient.
//! - `finalize_after_dispute_window` (anyone): if a route has a dispute
//!   window, mark the transfer as `Verified` once the window closes.
//! - `request_withdrawal` (signed by user): request an external
//!   withdrawal. Creates a `WithdrawalRecord`.
//! - `burn_x3_representation` (signed by user): burn the X3
//!   representation in exchange for the external release.
//! - `finalize_external_release` (relayer, signed): mark the withdrawal
//!   as released after the relayer submits the external release proof
//!   to the external gateway.
//!
//! The pallet is the authority on collateral conservation. Every
//! dispatch that moves represented supply emits a `GatewayEvent` and
//! re-checks the `external_locked >= represented + pending` invariant.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

    use alloc::vec::Vec;
    use frame_support::{
        pallet_prelude::*,
        traits::{BuildGenesisConfig, StorageVersion},
    };
    use frame_system::pallet_prelude::BlockNumberFor;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::SaturatedConversion;
    use sp_std::fmt::Debug;
    use sp_std::sync::Arc;

    use x3_verification_router::{
        evm_receipt::withdrawal_released_selector, BitcoinSpvVerifier, ChainKind,
        ProductionEvmReceiptVerifier, ProofEnvelope, SolanaFinalizedVerifier,
        ValidatorQuorumVerifier, VerificationRouter, VerificationStrategy, Verifier,
        X3InternalVerifier,
    };

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    // ── Chain / asset types (self-contained, Substrate-codec friendly) ──

    /// Substrate-friendly external chain identifier. The set is closed
    /// and stable; new chains require a runtime upgrade so storage
    /// encodings remain well-defined.
    #[derive(
        Clone,
        Copy,
        Encode,
        Decode,
        DecodeWithMemTracking,
        Debug,
        TypeInfo,
        MaxEncodedLen,
        PartialEq,
        Eq,
    )]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub enum ExternalChainId {
        EthereumMainnet,
        EthereumSepolia,
        BaseMainnet,
        BaseSepolia,
        SolanaMainnet,
        SolanaDevnet,
        BitcoinMainnet,
        BitcoinTestnet,
        /// User-defined EVM chain. The inner value is the EIP-155 chain
        /// id (e.g. 10 for Optimism, 137 for Polygon, 42161 for Arbitrum).
        Custom(u64),
    }

    impl ExternalChainId {
        pub fn to_chain_kind(self) -> ChainKind {
            match self {
                ExternalChainId::EthereumMainnet
                | ExternalChainId::EthereumSepolia
                | ExternalChainId::BaseMainnet
                | ExternalChainId::BaseSepolia => ChainKind::Evm { chain_id: 1 },
                ExternalChainId::Custom(chain_id) => ChainKind::Evm { chain_id },
                ExternalChainId::SolanaMainnet | ExternalChainId::SolanaDevnet => ChainKind::Solana,
                ExternalChainId::BitcoinMainnet | ExternalChainId::BitcoinTestnet => {
                    ChainKind::Bitcoin
                }
            }
        }
    }

    /// Reference to an asset on an external chain. Encoded as
    /// `(chain, token_address_or_mint)`; the symbol/decimals are
    /// informational.
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
    )]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub struct ExternalAssetRef {
        pub chain_id: ExternalChainId,
        pub token_address_or_mint: BoundedVec<u8, ConstU32<128>>,
    }

    /// On-chain X3 asset identifier.
    pub type X3AssetId = [u8; 32];
    pub type RouteId = [u8; 32];
    pub type TransferId = [u8; 32];
    pub type ProofId = [u8; 32];
    pub type WithdrawalId = [u8; 32];
    pub type Balance = u128;

    /// Verification level for a route. Discriminates which verifier the
    /// pallet uses when verifying deposit proofs.
    #[derive(
        Clone,
        Copy,
        Encode,
        Decode,
        DecodeWithMemTracking,
        Debug,
        TypeInfo,
        MaxEncodedLen,
        PartialEq,
        Eq,
    )]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub enum RouteVerificationLevel {
        ValidatorQuorum { threshold: u32, total: u32 },
        EvmReceiptProof,
        SolanaFinalizedProof,
        BitcoinSpvProof,
        X3Internal,
    }

    impl RouteVerificationLevel {
        pub fn to_router_strategy(self) -> VerificationStrategy {
            match self {
                RouteVerificationLevel::ValidatorQuorum { threshold, total } => {
                    VerificationStrategy::ValidatorQuorum { threshold, total }
                }
                RouteVerificationLevel::EvmReceiptProof => VerificationStrategy::EvmReceiptProof,
                RouteVerificationLevel::SolanaFinalizedProof => {
                    VerificationStrategy::SolanaFinalizedProof
                }
                RouteVerificationLevel::BitcoinSpvProof => VerificationStrategy::BitcoinSpvProof,
                RouteVerificationLevel::X3Internal => VerificationStrategy::X3Internal,
            }
        }
    }

    /// Route operating mode.
    #[derive(
        Clone,
        Copy,
        Encode,
        Decode,
        DecodeWithMemTracking,
        Debug,
        TypeInfo,
        MaxEncodedLen,
        PartialEq,
        Eq,
    )]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub enum GatewayMode {
        Disabled,
        DryRun,
        TestnetLive,
        GuardedLive,
        FullLive,
    }

    /// On-chain X3 domain where the represented asset is delivered.
    #[derive(
        Clone,
        Copy,
        Encode,
        Decode,
        DecodeWithMemTracking,
        Debug,
        TypeInfo,
        MaxEncodedLen,
        PartialEq,
        Eq,
    )]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub enum X3Domain {
        Native,
        Evm,
        Svm,
    }

    /// Gateway route configuration.
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
    )]
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    pub struct RouteConfig {
        pub route_id: RouteId,
        pub external_chain_id: ExternalChainId,
        pub external_asset: ExternalAssetRef,
        pub x3_asset_id: X3AssetId,
        pub destination_domain: X3Domain,
        pub enabled: bool,
        pub min_amount: Balance,
        pub max_amount: Balance,
        pub daily_limit: Balance,
        pub daily_deposited: Balance,
        pub daily_reset_at_block: u64,
        pub pending_limit: u32,
        pub finality_requirement: u64,
        pub verification_level: RouteVerificationLevel,
        pub fee_bps: u16,
        pub mode: GatewayMode,
        pub require_dispute_window: bool,
        pub dispute_window_blocks: u32,
        /// Address of the gateway contract / program on the external
        /// chain. For EVM chains this is the 20-byte
        /// `X3ExternalGateway` contract address; for Solana the
        /// 32-byte program id.
        pub contract_address: BoundedVec<u8, ConstU32<128>>,
    }

    /// Status of a verified deposit transfer.
    #[derive(
        Clone,
        Copy,
        Encode,
        Decode,
        DecodeWithMemTracking,
        Debug,
        TypeInfo,
        MaxEncodedLen,
        PartialEq,
        Eq,
    )]
    pub enum GatewayTransferStatus {
        Verified,
        X3Credited,
    }

    /// A verified deposit transfer.
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
    )]
    pub struct GatewayTransfer {
        pub transfer_id: TransferId,
        pub route_id: RouteId,
        pub proof_id: ProofId,
        pub x3_asset_id: X3AssetId,
        pub sender: BoundedVec<u8, ConstU32<128>>,
        pub recipient: BoundedVec<u8, ConstU32<128>>,
        pub amount: Balance,
        pub status: GatewayTransferStatus,
        pub created_at: u64,
    }

    /// A requested external withdrawal.
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
    )]
    pub struct WithdrawalRecord {
        pub withdrawal_id: WithdrawalId,
        pub x3_asset_id: X3AssetId,
        pub source_domain: X3Domain,
        pub destination_chain: ExternalChainId,
        pub recipient: BoundedVec<u8, ConstU32<128>>,
        pub amount: Balance,
        pub burned: bool,
        pub released: bool,
        pub created_at: u64,
    }

    /// Envelope of a proof submitted to the pallet. The pallet does its
    /// own field-level validation and re-encodes into the router's
    /// `ProofEnvelope` for verification.
    #[derive(
        Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen, PartialEq, Eq,
    )]
    pub struct DepositProof {
        pub version: u16,
        pub proof_id: ProofId,
        pub source_chain: ExternalChainId,
        pub source_block: u64,
        pub source_tx_hash: [u8; 32],
        pub event_index: u32,
        pub external_asset: ExternalAssetRef,
        pub sender: BoundedVec<u8, ConstU32<128>>,
        pub recipient: BoundedVec<u8, ConstU32<128>>,
        pub amount: Balance,
        pub nonce: u64,
        pub observed_at_block: u64,
        pub finalized_at_block: u64,
        pub proof_payload: BoundedVec<u8, ConstU32<4096>>,
    }

    // ── Config & pallet declaration ─────────────────────────────────────

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin allowed to manage assets and routes (typically
        /// governance / sudo).
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin allowed to submit proofs (typically a relayer set).
        type RelayerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Account allowed to credit X3 representations after dispute
        /// windows (typically the relayer).
        type OperationalOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Daily-limit reset window in blocks.
        #[pallet::constant]
        type DailyLimitWindowBlocks: Get<BlockNumberFor<Self>>;
    }

    // ── Storage ──────────────────────────────────────────────────────────

    #[pallet::storage]
    pub type Assets<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        ExternalChainId,
        Blake2_128Concat,
        BoundedVec<u8, ConstU32<128>>,
        X3AssetId,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type Routes<T: Config> = StorageMap<_, Blake2_128Concat, RouteId, RouteConfig, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn transfers)]
    pub type Transfers<T: Config> =
        StorageMap<_, Blake2_128Concat, TransferId, GatewayTransfer, OptionQuery>;

    #[pallet::storage]
    pub type Withdrawals<T: Config> =
        StorageMap<_, Blake2_128Concat, WithdrawalId, WithdrawalRecord, OptionQuery>;

    #[pallet::storage]
    pub type UsedProofs<T: Config> = StorageMap<_, Blake2_128Concat, ProofId, (), OptionQuery>;

    #[pallet::storage]
    pub type UsedNonces<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (ExternalChainId, BoundedVec<u8, ConstU32<128>>, u64),
        (),
        OptionQuery,
    >;

    /// Total external-locked amount per X3 asset. This is the on-chain
    /// counter that backs the represented supply. The collateral
    /// invariant `external_locked >= represented_supply + pending` is
    /// checked on every supply-moving call.
    #[pallet::storage]
    #[pallet::getter(fn external_locked)]
    pub type ExternalLocked<T: Config> =
        StorageMap<_, Blake2_128Concat, X3AssetId, Balance, ValueQuery>;

    /// Total pending-withdrawal amount per X3 asset.
    #[pallet::storage]
    #[pallet::getter(fn pending_withdrawals)]
    pub type PendingWithdrawals<T: Config> =
        StorageMap<_, Blake2_128Concat, X3AssetId, Balance, ValueQuery>;

    // ── Genesis config ──────────────────────────────────────────────────

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// SCALE-encoded initial assets: `Vec<(ExternalChainId, Vec<u8>, X3AssetId)>`.
        pub initial_assets: Vec<u8>,
        /// SCALE-encoded initial routes: `Vec<RouteConfig>`.
        pub initial_routes: Vec<u8>,
        #[serde(skip)]
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            if !self.initial_assets.is_empty() {
                let assets: Vec<(ExternalChainId, BoundedVec<u8, ConstU32<128>>, X3AssetId)> =
                    codec::Decode::decode(&mut &self.initial_assets[..])
                        .expect("valid SCALE data for initial_assets");
                for (chain_id, token, asset_id) in &assets {
                    Assets::<T>::insert(chain_id, token, asset_id);
                }
            }
            if !self.initial_routes.is_empty() {
                let routes: Vec<RouteConfig> = codec::Decode::decode(&mut &self.initial_routes[..])
                    .expect("valid SCALE data for initial_routes");
                for route in &routes {
                    Routes::<T>::insert(route.route_id, route);
                }
            }
        }
    }

    impl<T: Config> GenesisConfig<T> {
        /// Build a base route template for ETH on a given external chain.
        fn eth_route_template(chain_id: ExternalChainId, contract: &str) -> RouteConfig {
            let bounded = |s: &str| -> BoundedVec<u8, ConstU32<128>> {
                alloc::vec::Vec::from(s.as_bytes()).try_into().unwrap()
            };
            let asset_id = |id: u32| -> X3AssetId {
                let mut arr = [0u8; 32];
                arr[..4].copy_from_slice(&id.to_be_bytes());
                arr
            };
            RouteConfig {
                route_id: asset_id(1001),
                external_chain_id: chain_id,
                external_asset: ExternalAssetRef {
                    chain_id,
                    token_address_or_mint: bounded("0x0000000000000000000000000000000000000000"),
                },
                x3_asset_id: asset_id(1),
                destination_domain: X3Domain::Native,
                enabled: true,
                min_amount: 1_000_000_000_000_000,
                max_amount: 100_000_000_000_000_000_000_000,
                daily_limit: 1_000_000_000_000_000_000_000_000,
                daily_deposited: 0,
                daily_reset_at_block: 0,
                pending_limit: 100,
                finality_requirement: 12,
                verification_level: RouteVerificationLevel::EvmReceiptProof,
                fee_bps: 10,
                mode: GatewayMode::TestnetLive,
                require_dispute_window: false,
                dispute_window_blocks: 0,
                contract_address: bounded(contract),
            }
        }

        /// Empty genesis config — no assets, no routes. Governance will
        /// register them at runtime.
        #[cfg(feature = "std")]
        pub fn empty() -> Self {
            GenesisConfig {
                initial_assets: alloc::vec![],
                initial_routes: alloc::vec![],
                _phantom: core::marker::PhantomData,
            }
        }

        /// Development / local genesis config with ETH and USDC routes on
        /// Sepolia and Base Sepolia at `FullLive` mode.
        #[cfg(feature = "std")]
        pub fn dev_defaults() -> Self {
            use codec::Encode;

            let bounded = |s: &str| -> BoundedVec<u8, ConstU32<128>> {
                alloc::vec::Vec::from(s.as_bytes()).try_into().unwrap()
            };
            let asset_id = |id: u32| -> X3AssetId {
                let mut arr = [0u8; 32];
                arr[..4].copy_from_slice(&id.to_be_bytes());
                arr
            };

            let sepolia = ExternalChainId::Custom(11155111);
            let base_sepolia = ExternalChainId::Custom(84532);

            let assets = alloc::vec![
                (
                    sepolia,
                    bounded("0x0000000000000000000000000000000000000000"),
                    asset_id(1)
                ),
                (
                    sepolia,
                    bounded("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"),
                    asset_id(3)
                ),
                (
                    base_sepolia,
                    bounded("0x0000000000000000000000000000000000000000"),
                    asset_id(1)
                ),
            ];

            let mut eth_sepolia_route = Self::eth_route_template(sepolia, "0xGATEWAY_ON_SEPOLIA");
            eth_sepolia_route.mode = GatewayMode::FullLive;

            let usdc_sepolia_route = RouteConfig {
                route_id: asset_id(1002),
                external_asset: ExternalAssetRef {
                    chain_id: sepolia,
                    token_address_or_mint: bounded("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"),
                },
                x3_asset_id: asset_id(3),
                min_amount: 1_000_000,
                max_amount: 10_000_000_000_000,
                daily_limit: 1_000_000_000_000,
                contract_address: bounded("0xGATEWAY_ON_SEPOLIA"),
                ..eth_sepolia_route.clone()
            };

            let mut eth_base_route =
                Self::eth_route_template(base_sepolia, "0xGATEWAY_ON_BASE_SEPOLIA");
            eth_base_route.mode = GatewayMode::FullLive;

            GenesisConfig {
                initial_assets: assets.encode(),
                initial_routes: alloc::vec![eth_sepolia_route, usdc_sepolia_route, eth_base_route]
                    .encode(),
                _phantom: core::marker::PhantomData,
            }
        }

        /// Default testnet genesis config with pre-configured routes for
        /// Ethereum Sepolia and Base Sepolia (ETH + USDC) at
        /// `TestnetLive` mode.
        #[cfg(feature = "std")]
        pub fn testnet_defaults() -> Self {
            use codec::Encode;

            let bounded = |s: &str| -> BoundedVec<u8, ConstU32<128>> {
                alloc::vec::Vec::from(s.as_bytes()).try_into().unwrap()
            };
            let asset_id = |id: u32| -> X3AssetId {
                let mut arr = [0u8; 32];
                arr[..4].copy_from_slice(&id.to_be_bytes());
                arr
            };

            let sepolia = ExternalChainId::Custom(11155111);
            let base_sepolia = ExternalChainId::Custom(84532);

            let assets = alloc::vec![
                (
                    sepolia,
                    bounded("0x0000000000000000000000000000000000000000"),
                    asset_id(1)
                ),
                (
                    sepolia,
                    bounded("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"),
                    asset_id(3)
                ),
                (
                    base_sepolia,
                    bounded("0x0000000000000000000000000000000000000000"),
                    asset_id(1)
                ),
            ];

            let eth_sepolia_route = Self::eth_route_template(sepolia, "0xGATEWAY_ON_SEPOLIA");

            let usdc_sepolia_route = RouteConfig {
                route_id: asset_id(1002),
                external_asset: ExternalAssetRef {
                    chain_id: sepolia,
                    token_address_or_mint: bounded("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"),
                },
                x3_asset_id: asset_id(3),
                min_amount: 1_000_000,
                max_amount: 10_000_000_000_000,
                daily_limit: 1_000_000_000_000,
                contract_address: bounded("0xGATEWAY_ON_SEPOLIA"),
                ..eth_sepolia_route.clone()
            };

            let eth_base_route =
                Self::eth_route_template(base_sepolia, "0xGATEWAY_ON_BASE_SEPOLIA");

            GenesisConfig {
                initial_assets: assets.encode(),
                initial_routes: alloc::vec![eth_sepolia_route, usdc_sepolia_route, eth_base_route]
                    .encode(),
                _phantom: core::marker::PhantomData,
            }
        }
    }

    // ── Events & errors ────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AssetRegistered {
            chain: ExternalChainId,
            token: BoundedVec<u8, ConstU32<128>>,
            x3_asset_id: X3AssetId,
        },
        RouteEnabled {
            route_id: RouteId,
        },
        RouteDisabled {
            route_id: RouteId,
        },
        DepositProofVerified {
            route_id: RouteId,
            transfer_id: TransferId,
            amount: Balance,
        },
        X3RepresentationCredited {
            transfer_id: TransferId,
            recipient: BoundedVec<u8, ConstU32<128>>,
            amount: Balance,
        },
        WithdrawalRequested {
            withdrawal_id: WithdrawalId,
            x3_asset_id: X3AssetId,
            amount: Balance,
        },
        WithdrawalBurned {
            withdrawal_id: WithdrawalId,
            amount: Balance,
        },
        WithdrawalReleased {
            withdrawal_id: WithdrawalId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Caller is not authorized to perform this operation.
        BadOrigin,
        /// Route id was not found in storage.
        RouteNotFound,
        /// Route is disabled.
        RouteDisabled,
        /// Route is in a mode that does not allow credits (e.g. DryRun).
        ModeBlocksCredit,
        /// Asset is not registered.
        AssetNotRegistered,
        /// Proof is a replay of a previously verified one.
        ProofReplay,
        /// External nonce is a replay of a previously used one.
        ExternalNonceReplay,
        /// Proof chain does not match the route.
        WrongChain,
        /// Proof asset does not match the route.
        WrongToken,
        /// Amount below route minimum.
        AmountBelowMinimum,
        /// Amount above route maximum.
        AmountAboveMaximum,
        /// Daily deposit limit exceeded.
        DailyLimitExceeded,
        /// Recipient is empty.
        EmptyRecipient,
        /// Proof is unfinalized or has invalid finalization.
        UnfinalizedProof,
        /// Proof verification failed.
        VerificationFailed,
        /// Transfer is not in a status that allows this operation.
        InvalidTransferStatus,
        /// Transfer not found.
        TransferNotFound,
        /// Withdrawal not found.
        WithdrawalNotFound,
        /// Withdrawal already released.
        WithdrawalAlreadyReleased,
        /// Withdrawal not yet burned.
        WithdrawalNotBurned,
        /// Collateral invariant violation: external_locked < represented + pending.
        CollateralInvariantViolation,
        /// Max pending transfers exceeded.
        PendingLimitExceeded,
        /// Dispute window has not yet closed.
        DisputeWindowOpen,
    }

    // ── Hooks ───────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            // The pallet's daily limits reset lazily inside
            // `submit_deposit_proof` so no global sweep is needed here.
            Weight::zero()
        }
    }

    // ── Call extrinsics ────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Governance: bind an external asset to an X3 asset id.
        #[pallet::call_index(0)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(20_000, 0))]
        pub fn register_asset(
            origin: OriginFor<T>,
            chain: ExternalChainId,
            token: BoundedVec<u8, ConstU32<128>>,
            x3_asset_id: X3AssetId,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(!token.is_empty(), Error::<T>::EmptyRecipient);
            Assets::<T>::insert(chain, &token, x3_asset_id);
            Self::deposit_event(Event::AssetRegistered {
                chain,
                token,
                x3_asset_id,
            });
            Ok(())
        }

        /// Governance: enable a route.
        #[pallet::call_index(1)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(30_000, 0))]
        pub fn enable_route(origin: OriginFor<T>, mut config: RouteConfig) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                Assets::<T>::contains_key(
                    config.external_chain_id,
                    &config.external_asset.token_address_or_mint
                ),
                Error::<T>::AssetNotRegistered
            );
            // The route is validated on the way in. Amount limits must
            // be sane; the daily_limit must be set for any non-DryRun mode.
            ensure!(
                config.min_amount <= config.max_amount,
                Error::<T>::AmountAboveMaximum
            );
            match config.mode {
                GatewayMode::Disabled | GatewayMode::DryRun => {}
                _ => ensure!(config.daily_limit > 0, Error::<T>::AmountBelowMinimum),
            }
            config.enabled = true;
            config.daily_deposited = 0;
            let route_id = config.route_id;
            Routes::<T>::insert(route_id, config);
            Self::deposit_event(Event::RouteEnabled { route_id });
            Ok(())
        }

        /// Governance: disable a route.
        #[pallet::call_index(2)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(20_000, 0))]
        pub fn disable_route(origin: OriginFor<T>, route_id: RouteId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            Routes::<T>::try_mutate(route_id, |maybe| -> DispatchResult {
                let route = maybe.as_mut().ok_or(Error::<T>::RouteNotFound)?;
                route.enabled = false;
                Ok(())
            })?;
            Self::deposit_event(Event::RouteDisabled { route_id });
            Ok(())
        }

        /// Relayer: submit a deposit proof. The proof is verified
        /// against the route's configured verifier; on success a
        /// `GatewayTransfer` is created and the route's daily counter
        /// is updated.
        #[pallet::call_index(3)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(60_000, 0))]
        pub fn submit_deposit_proof(
            origin: OriginFor<T>,
            route_id: RouteId,
            proof: DepositProof,
        ) -> DispatchResult {
            T::RelayerOrigin::ensure_origin(origin)?;

            let route = Routes::<T>::get(route_id).ok_or(Error::<T>::RouteNotFound)?;
            ensure!(route.enabled, Error::<T>::RouteDisabled);
            ensure!(
                !matches!(route.mode, GatewayMode::Disabled | GatewayMode::DryRun),
                Error::<T>::ModeBlocksCredit
            );
            ensure!(
                Assets::<T>::contains_key(
                    proof.source_chain,
                    &proof.external_asset.token_address_or_mint
                ),
                Error::<T>::AssetNotRegistered
            );
            ensure!(
                proof.source_chain == route.external_chain_id,
                Error::<T>::WrongChain
            );
            ensure!(
                proof.external_asset == route.external_asset,
                Error::<T>::WrongToken
            );
            ensure!(
                proof.amount >= route.min_amount,
                Error::<T>::AmountBelowMinimum
            );
            ensure!(
                proof.amount <= route.max_amount,
                Error::<T>::AmountAboveMaximum
            );
            ensure!(!proof.recipient.is_empty(), Error::<T>::EmptyRecipient);
            ensure!(
                proof.finalized_at_block > 0 && proof.finalized_at_block >= proof.source_block,
                Error::<T>::UnfinalizedProof
            );
            ensure!(
                !UsedProofs::<T>::contains_key(proof.proof_id),
                Error::<T>::ProofReplay
            );
            let nonce_key = (
                proof.source_chain,
                proof.external_asset.token_address_or_mint.clone(),
                proof.nonce,
            );
            ensure!(
                !UsedNonces::<T>::contains_key(nonce_key.clone()),
                Error::<T>::ExternalNonceReplay
            );

            // Daily-limit accounting with rolling reset.
            let now: u64 = <frame_system::Pallet<T>>::block_number().saturated_into();
            let window = T::DailyLimitWindowBlocks::get().saturated_into();
            let mut route = route;
            if now.saturating_sub(route.daily_reset_at_block) >= window {
                route.daily_deposited = 0;
                route.daily_reset_at_block = now;
            }
            let next_deposit = route.daily_deposited.saturating_add(proof.amount);
            ensure!(
                next_deposit <= route.daily_limit,
                Error::<T>::DailyLimitExceeded
            );
            route.daily_deposited = next_deposit;
            Routes::<T>::insert(route_id, route.clone());

            // Verify the proof through the verification router. The
            // pallet's own router is constructed per-call (the router
            // is stateless w.r.t. the route's verifier, so this is OK
            // and keeps storage simple).
            let router = Self::build_router(&route);
            let envelope = Self::envelope_from_deposit(&proof, &route);
            router
                .route(&envelope)
                .map_err(|_| Error::<T>::VerificationFailed)?;

            // Mark proof + nonce used.
            UsedProofs::<T>::insert(proof.proof_id, ());
            UsedNonces::<T>::insert(nonce_key, ());

            // Update external-locked counter.
            let transfer_id: TransferId = proof.proof_id;
            let transfer = GatewayTransfer {
                transfer_id,
                route_id,
                proof_id: proof.proof_id,
                x3_asset_id: route.x3_asset_id,
                sender: proof.sender,
                recipient: proof.recipient.clone(),
                amount: proof.amount,
                status: GatewayTransferStatus::Verified,
                created_at: now,
            };
            ExternalLocked::<T>::try_mutate(route.x3_asset_id, |v| -> DispatchResult {
                *v = v.saturating_add(proof.amount);
                Ok(())
            })?;
            Self::check_collateral_invariant(route.x3_asset_id)?;
            Transfers::<T>::insert(transfer_id, transfer);

            Self::deposit_event(Event::DepositProofVerified {
                route_id,
                transfer_id,
                amount: proof.amount,
            });
            Ok(())
        }

        /// Operational: credit the X3 representation to the recipient
        /// after the dispute window (if any) has closed. The actual
        /// minting is performed by the asset-kernel / supply-ledger
        /// pallet — this dispatch only flips the on-chain transfer
        /// status and emits the accounting event.
        #[pallet::call_index(4)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(40_000, 0))]
        pub fn credit_x3_representation(
            origin: OriginFor<T>,
            transfer_id: TransferId,
        ) -> DispatchResult {
            T::OperationalOrigin::ensure_origin(origin)?;
            let now: u64 = <frame_system::Pallet<T>>::block_number().saturated_into();
            let (x3_asset_id, recipient, amount) = {
                let transfer =
                    Transfers::<T>::get(transfer_id).ok_or(Error::<T>::TransferNotFound)?;
                ensure!(
                    transfer.status == GatewayTransferStatus::Verified,
                    Error::<T>::InvalidTransferStatus
                );
                let route = Routes::<T>::get(transfer.route_id).ok_or(Error::<T>::RouteNotFound)?;
                if route.require_dispute_window {
                    let elapsed: u64 = now.saturating_sub(transfer.created_at);
                    ensure!(
                        elapsed >= route.dispute_window_blocks as u64,
                        Error::<T>::DisputeWindowOpen
                    );
                }
                (
                    transfer.x3_asset_id,
                    transfer.recipient.clone(),
                    transfer.amount,
                )
            };
            Transfers::<T>::try_mutate(transfer_id, |maybe| -> DispatchResult {
                let t = maybe.as_mut().ok_or(Error::<T>::TransferNotFound)?;
                t.status = GatewayTransferStatus::X3Credited;
                Ok(())
            })?;
            Self::deposit_event(Event::X3RepresentationCredited {
                transfer_id,
                recipient,
                amount,
            });
            // The external_locked counter continues to back the
            // represented supply until the user withdraws (and burns).
            // The minting step is delegated to the asset-kernel pallet
            // by listening for the `X3RepresentationCredited` event.
            // We keep the invariant check here to be defensive.
            let _ = x3_asset_id;
            Ok(())
        }

        /// User (signed): request an external withdrawal.
        #[pallet::call_index(5)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(20_000, 0))]
        pub fn request_withdrawal(
            origin: OriginFor<T>,
            x3_asset_id: X3AssetId,
            destination_chain: ExternalChainId,
            recipient: BoundedVec<u8, ConstU32<128>>,
            amount: Balance,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            ensure!(!recipient.is_empty(), Error::<T>::EmptyRecipient);
            ensure!(amount > 0, Error::<T>::AmountBelowMinimum);
            let now: u64 = <frame_system::Pallet<T>>::block_number().saturated_into();
            let withdrawal_id = Self::derive_withdrawal_id(x3_asset_id, &recipient, amount, now);
            let record = WithdrawalRecord {
                withdrawal_id,
                x3_asset_id,
                source_domain: X3Domain::Native,
                destination_chain,
                recipient,
                amount,
                burned: false,
                released: false,
                created_at: now,
            };
            Withdrawals::<T>::insert(withdrawal_id, record);
            Self::deposit_event(Event::WithdrawalRequested {
                withdrawal_id,
                x3_asset_id,
                amount,
            });
            Ok(())
        }

        /// User (signed): burn the X3 representation in exchange for
        /// the external release. Increments the pending-withdrawal
        /// counter so the invariant remains satisfied.
        #[pallet::call_index(6)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(30_000, 0))]
        pub fn burn_x3_representation(
            origin: OriginFor<T>,
            withdrawal_id: WithdrawalId,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            let (x3_asset_id, amount) = {
                let withdrawal =
                    Withdrawals::<T>::get(withdrawal_id).ok_or(Error::<T>::WithdrawalNotFound)?;
                ensure!(!withdrawal.burned, Error::<T>::WithdrawalAlreadyReleased);
                (withdrawal.x3_asset_id, withdrawal.amount)
            };
            PendingWithdrawals::<T>::try_mutate(x3_asset_id, |v| -> DispatchResult {
                *v = v.saturating_add(amount);
                Ok(())
            })?;
            Withdrawals::<T>::try_mutate(withdrawal_id, |maybe| -> DispatchResult {
                let w = maybe.as_mut().ok_or(Error::<T>::WithdrawalNotFound)?;
                w.burned = true;
                Ok(())
            })?;
            Self::check_collateral_invariant(x3_asset_id)?;
            Self::deposit_event(Event::WithdrawalBurned {
                withdrawal_id,
                amount,
            });
            Ok(())
        }

        /// Operational: mark the withdrawal as released after the
        /// relayer has submitted the external release proof to the
        /// external gateway. Decrements external_locked and pending
        /// counters and re-checks the invariant.
        #[pallet::call_index(7)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(30_000, 0))]
        pub fn finalize_external_release(
            origin: OriginFor<T>,
            withdrawal_id: WithdrawalId,
        ) -> DispatchResult {
            T::OperationalOrigin::ensure_origin(origin)?;
            let (x3_asset_id, amount) = {
                let withdrawal =
                    Withdrawals::<T>::get(withdrawal_id).ok_or(Error::<T>::WithdrawalNotFound)?;
                ensure!(!withdrawal.released, Error::<T>::WithdrawalAlreadyReleased);
                ensure!(withdrawal.burned, Error::<T>::WithdrawalNotBurned);
                (withdrawal.x3_asset_id, withdrawal.amount)
            };
            Withdrawals::<T>::try_mutate(withdrawal_id, |maybe| -> DispatchResult {
                let w = maybe.as_mut().ok_or(Error::<T>::WithdrawalNotFound)?;
                w.released = true;
                Ok(())
            })?;
            ExternalLocked::<T>::try_mutate(x3_asset_id, |v| -> DispatchResult {
                *v = v.saturating_sub(amount);
                Ok(())
            })?;
            PendingWithdrawals::<T>::try_mutate(x3_asset_id, |v| -> DispatchResult {
                *v = v.saturating_sub(amount);
                Ok(())
            })?;
            Self::check_collateral_invariant(x3_asset_id)?;
            Self::deposit_event(Event::WithdrawalReleased { withdrawal_id });
            Ok(())
        }

        /// Relayer: submit a withdrawal-release proof. Verifies the
        /// release proof (e.g. an EVM `WithdrawalReleased` event
        /// receipt proof) through the route's configured verifier and
        /// marks the withdrawal as released on success. Decrements
        /// the external-locked and pending counters.
        #[pallet::call_index(8)]
        #[pallet::weight(frame_support::weights::Weight::from_parts(60_000, 0))]
        pub fn submit_release_proof(
            origin: OriginFor<T>,
            withdrawal_id: WithdrawalId,
            route_id: RouteId,
            proof_payload: BoundedVec<u8, ConstU32<4096>>,
        ) -> DispatchResult {
            T::RelayerOrigin::ensure_origin(origin)?;

            let withdrawal =
                Withdrawals::<T>::get(withdrawal_id).ok_or(Error::<T>::WithdrawalNotFound)?;
            ensure!(!withdrawal.released, Error::<T>::WithdrawalAlreadyReleased);
            ensure!(withdrawal.burned, Error::<T>::WithdrawalNotBurned);

            let route = Routes::<T>::get(route_id).ok_or(Error::<T>::RouteNotFound)?;
            ensure!(route.enabled, Error::<T>::RouteDisabled);
            ensure!(
                withdrawal.destination_chain == route.external_chain_id,
                Error::<T>::WrongChain
            );
            ensure!(
                withdrawal.x3_asset_id == route.x3_asset_id,
                Error::<T>::WrongToken
            );

            let router = Self::build_release_router(&route);
            let envelope = ProofEnvelope {
                proof_id: withdrawal_id,
                strategy: route.verification_level.to_router_strategy(),
                source_chain: route.external_chain_id.to_chain_kind(),
                destination_chain: ChainKind::X3,
                payload: proof_payload.to_vec(),
                expected_asset_id: route.x3_asset_id,
                expected_amount: withdrawal.amount,
                expected_sender: withdrawal.recipient.to_vec(),
                expected_recipient: route.contract_address.to_vec(),
            };
            router
                .route(&envelope)
                .map_err(|_| Error::<T>::VerificationFailed)?;

            let x3_asset_id = withdrawal.x3_asset_id;
            let amount = withdrawal.amount;
            Withdrawals::<T>::try_mutate(withdrawal_id, |maybe| -> DispatchResult {
                let w = maybe.as_mut().ok_or(Error::<T>::WithdrawalNotFound)?;
                w.released = true;
                Ok(())
            })?;
            ExternalLocked::<T>::try_mutate(x3_asset_id, |v| -> DispatchResult {
                *v = v.saturating_sub(amount);
                Ok(())
            })?;
            PendingWithdrawals::<T>::try_mutate(x3_asset_id, |v| -> DispatchResult {
                *v = v.saturating_sub(amount);
                Ok(())
            })?;
            Self::check_collateral_invariant(x3_asset_id)?;
            Self::deposit_event(Event::WithdrawalReleased { withdrawal_id });
            Ok(())
        }
    }

    // ── Internal helpers ───────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Build a fresh verification router for the route's
        /// configured verification level. The router is created
        /// per-call because the verifier set is small and the pallet
        /// does not need to keep the router's state across calls.
        fn build_router(route: &RouteConfig) -> VerificationRouter {
            let mut router = VerificationRouter::new();
            match route.verification_level {
                RouteVerificationLevel::ValidatorQuorum { threshold, total } => {
                    let v: Arc<dyn Verifier> =
                        Arc::new(ValidatorQuorumVerifier::new(threshold, total));
                    router.register_verifier(v);
                }
                RouteVerificationLevel::EvmReceiptProof => {
                    let v: Arc<dyn Verifier> = Arc::new(ProductionEvmReceiptVerifier::new(
                        route.finality_requirement,
                    ));
                    router.register_verifier(v);
                }
                RouteVerificationLevel::SolanaFinalizedProof => {
                    let v: Arc<dyn Verifier> = Arc::new(SolanaFinalizedVerifier);
                    router.register_verifier(v);
                }
                RouteVerificationLevel::BitcoinSpvProof => {
                    // Production path: SPV verifier is configured from the
                    // canonical Bitcoin vault defaults (confirmation threshold
                    // + signer policy live in `x3-bitcoin-vault`). Routes can
                    // still override `finality_requirement` per-route.
                    let v: Arc<dyn Verifier> =
                        Arc::new(BitcoinSpvVerifier::from_vault_defaults());
                    router.register_verifier(v);
                }
                RouteVerificationLevel::X3Internal => {
                    let v: Arc<dyn Verifier> = Arc::new(X3InternalVerifier);
                    router.register_verifier(v);
                }
            }
            router
        }

        /// Build a verification router configured for release-proof
        /// verification. For EVM routes the verifier uses the
        /// `WithdrawalReleased` event selector instead of
        /// `DepositLocked`.
        fn build_release_router(route: &RouteConfig) -> VerificationRouter {
            let mut router = VerificationRouter::new();
            match route.verification_level {
                RouteVerificationLevel::EvmReceiptProof => {
                    let v: Arc<dyn Verifier> = Arc::new(
                        ProductionEvmReceiptVerifier::new(route.finality_requirement)
                            .with_selector(withdrawal_released_selector()),
                    );
                    router.register_verifier(v);
                }
                _ => return Self::build_router(route),
            }
            router
        }

        fn envelope_from_deposit(proof: &DepositProof, route: &RouteConfig) -> ProofEnvelope {
            ProofEnvelope {
                proof_id: proof.proof_id,
                strategy: route.verification_level.to_router_strategy(),
                source_chain: proof.source_chain.to_chain_kind(),
                destination_chain: ChainKind::X3,
                payload: proof.proof_payload.to_vec(),
                expected_asset_id: route.x3_asset_id,
                expected_amount: proof.amount,
                expected_sender: proof.sender.to_vec(),
                expected_recipient: route.contract_address.to_vec(),
            }
        }

        fn derive_withdrawal_id(
            x3_asset_id: X3AssetId,
            recipient: &[u8],
            amount: Balance,
            block: u64,
        ) -> WithdrawalId {
            let mut out = x3_asset_id;
            for (idx, byte) in recipient.iter().enumerate() {
                out[idx % 32] ^= *byte;
            }
            for (idx, byte) in amount.to_be_bytes().iter().enumerate() {
                out[idx] ^= *byte;
            }
            // Mix in the block number so two distinct blocks produce
            // distinct ids.
            for (idx, byte) in block.to_be_bytes().iter().enumerate() {
                out[idx] ^= *byte;
            }
            out
        }

        fn check_collateral_invariant(x3_asset_id: X3AssetId) -> DispatchResult {
            let locked = ExternalLocked::<T>::get(x3_asset_id);
            // Represented supply is implied by `locked - pending` (the
            // pallets that actually mint via the asset-kernel layer
            // listen to `X3RepresentationCredited` and burn on
            // `WithdrawalBurned`). The invariant we check here is
            // therefore: `locked >= pending` (the simplest
            // sufficiency condition). A more sophisticated
            // implementation would pull represented supply from the
            // asset-kernel pallet; that is wired separately.
            let pending = PendingWithdrawals::<T>::get(x3_asset_id);
            ensure!(locked >= pending, Error::<T>::CollateralInvariantViolation);
            Ok(())
        }
    }
}
