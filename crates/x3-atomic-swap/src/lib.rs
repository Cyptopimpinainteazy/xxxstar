//! # X3 Cross-VM Atomic Swap Proof Mode
//!
//! Production-grade atomic swap primitive for X3 using HTLC-style hashlocks,
//! chain-specific timelocks, relayer proof capture, and a mandatory scoreboard.
//!
//! ## Components
//!
//! - [`intent`] - `AtomicIntent` schema with full hashlock + timelock model
//! - [`ledger`] - Durable proof ledger recording every swap step with tx hashes
//! - [`evm_htlc`] - EVM HTLC adapter (contract simulation + trait)
#![allow(clippy::too_many_arguments)]
#![allow(clippy::inconsistent_digit_grouping)]
//! - [`svm_htlc`] - SVM HTLC adapter (lock account + program simulation + trait)
//! - [`relayer`] - Relayer watcher: watches locks, verifies hashlocks + finality,
//!   captures preimages, submits claims, writes proof records
//! - [`timeout`] - Timeout/refund engine: validates ordering, enforces refund path
//! - [`scoreboard`] - Mandatory swap scoreboard: 100-point proof completeness check
//! - [`registry`] - Solver and relayer registries with reputation and selection
//! - [`chain_health`] - Chain health monitoring with pause/resume, thresholds
//! - [`dashboard`] - Atomic Command Center: aggregated swap snapshot + alerts
//! - [`error`] - All error types with descriptive messages
//!
//! ## No Mocks, No Fakes
//!
//! All HTLC adapters include full verification logic:
//!
//! - EVM: hashlock verification (SHA-256), timeout enforcement, event emission
//! - SVM: PDA lock accounts, claimant/refund authority enforcement, event emission
//! - Relayer: refuses to mark success without finality or tx hashes
//! - Scoreboard: cannot reach 100 with missing proof or tx hash
//! - Timeout: expired swaps become REFUNDABLE/REFUNDED, never FAILED_SILENTLY

extern crate alloc;

pub mod adapter;
pub mod adapter_ledger;
pub mod bitcoin_htlc;
pub mod cairo_vm_htlc;
pub mod chain_health;
pub mod cosmwasm_htlc;
pub mod dashboard;
pub mod dispute;
pub mod error;
pub mod event_watcher;
pub mod evm_htlc;
pub mod finality;
pub mod fuel_htlc;
pub mod insurance;
pub mod intent;
pub mod ledger;
pub mod move_vm_htlc;
pub mod near_htlc;
pub mod plutus_htlc;
pub mod polkadot_ink_htlc;
pub mod registry;
pub mod relayer;
pub mod rpc_client;
pub mod rpc_quorum;
pub mod scoreboard;
pub mod slashing;
pub mod solana_watcher;
pub mod soroban_htlc;
pub mod substrate_htlc;
pub mod svm_htlc;
pub mod timeout;
pub mod ton_htlc;
pub mod x3vm_htlc;
pub mod zkvm_htlc;

#[cfg(feature = "std")]
pub mod ethereum_tx;

#[cfg(feature = "std")]
pub use ethereum_tx::Transaction;

pub use adapter::{
    AdapterReadinessScore, ChainHealth, ChainId, ClaimProof, FeeEstimate, FinalityProof, LockProof,
    RefundProof, VmType, X3VmAdapter,
};
pub use adapter_ledger::{
    claim_proof_to_entry, lock_proof_to_entry, refund_proof_to_entry, AdapterLedgerBridge,
};
pub use bitcoin_htlc::{
    BitcoinNetwork, BitcoinScript, BtcHtlcAdapter, BtcTransactionBuilder, StatefulBtcAdapter,
};
pub use cairo_vm_htlc::{CairoHtlcContract, CairoVmAdapter, StatefulCairoVmAdapter};
pub use chain_health::{
    ChainHealthOracle, ChainHealthStatus, HealthCheck, HealthThresholds, InMemoryChainHealth,
    PausableChainHealth, SwapSafetyCheck,
};
pub use cosmwasm_htlc::{
    CosmWasmAdapter, ExecuteMsg, IbcHtlcRoute, IbcRouteMetadata, LockStatusResponse, QueryMsg,
    StatefulCosmWasmAdapter,
};
pub use dashboard::{
    chaos_scoreboard_default, AtomicCommandCenter, ChaosTestResult, ChaosTestScoreboard,
    SwapDashboardSnapshot, SwapDetail, TxLink,
};
pub use error::SwapError;
pub use event_watcher::{
    EventLog, EventWatcher, HtlcEvent, WatcherConfig, CLAIMED_EVENT_TOPIC_HASH,
    LOCKED_EVENT_TOPIC_HASH, REFUNDED_EVENT_TOPIC_HASH,
};
pub use evm_htlc::{
    EvmAdapter, EvmClaimedEvent, EvmHtlcAdapter, EvmHtlcContract, EvmLockedEvent, EvmRefundedEvent,
};
pub use finality::{FinalityCheckData, FinalityConfig, FinalityOracle, InMemoryFinalityOracle};
pub use fuel_htlc::{FuelHtlcAdapter, FuelNetwork, FuelPredicate, StatefulFuelAdapter};
pub use intent::{
    AtomicIntent, AtomicIntentBuilder, AtomicSwapStatus, ChainKind, FinalityLevel,
    FinalityRequirement, RefundPath, RouteMode,
};
pub use ledger::{ProofEntry, ProofKind, ProofLedger, ProofRecord, RpcQuorumProof, TxStatus};
pub use move_vm_htlc::{
    LockResource, MoveHtlcModule, MoveNetwork, MoveVmAdapter, StatefulMoveVmAdapter,
};
pub use near_htlc::{
    NearHtlcAdapter, NearHtlcContract, NearLockState, NearNetwork, StatefulNearAdapter,
};
pub use plutus_htlc::{
    PlutusDatum, PlutusHtlcAdapter, PlutusNetwork, PlutusRedeemer, PlutusScript,
    StatefulPlutusAdapter,
};
pub use polkadot_ink_htlc::{InkHtlcAdapter, InkHtlcContract, InkNetwork, StatefulInkAdapter};
pub use registry::{RelayerModel, RelayerRegistry, SolverModel, SolverRegistry};
pub use relayer::{scan_for_alerts, Relayer, RelayerObservation, RelayerState, WatcherAlert};
pub use rpc_client::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, RpcClient, RpcClientConfig};
pub use rpc_quorum::{
    ConsensusResult, ConsolidatedQuorum, RpcProvider, RpcQuorumOracle, RpcVote, SimpleRpcQuorum,
};
pub use scoreboard::{AdapterScoreEntry, AdapterScoreboard, ScoredCategory, SwapScoreboard};
pub use slashing::{
    SlashCaseStatus, SlashReason, SlashRecord, SlashSummary, SlashableActor, SlashingEngine,
};
pub use soroban_htlc::{
    SorobanContract, SorobanHtlcAdapter, SorobanLockData, SorobanNetwork, StatefulSorobanAdapter,
};
pub use substrate_htlc::{StatefulSubstrateAdapter, SubstrateHtlcAdapter};
pub use svm_htlc::{
    SolPubkey, SvmAdapter, SvmClaimedEvent, SvmHtlcAccount, SvmHtlcAdapter, SvmHtlcProgram,
    SvmLockedEvent, SvmRefundedEvent,
};
pub use timeout::{TimeoutCheckResult, TimeoutEngine};
pub use ton_htlc::{StatefulTonAdapter, TonContract, TonHtlcAdapter, TonLockData, TonNetwork};
pub use x3vm_htlc::{StatefulX3VmAdapter, X3VmAdapterImpl};
pub use zkvm_htlc::{ZkProofRecord, ZkProofType, ZkVmAdapter, ZkVmTarget};
