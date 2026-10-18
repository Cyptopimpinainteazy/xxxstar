//! On-chain execution of compiled X3 Lang trading artifacts (X3BC format 1.1).
//!
//! `submit_trade` runs one trading module on the production engine
//! (`x3_x3_integration::trading::execute_trade`) against state read from this chain:
//!
//! * the trader's `CanonicalLedger` balances of every asset the module names;
//! * the venues and flash pools the module names, from the runtime's [`KernelTradingHost`]
//!   (`pallet_x3_dex` pools and `pallet_x3_flashloan` asset pools, whose reserves are themselves
//!   `CanonicalLedger` balances of their custody accounts);
//! * oracle prices for every swap pair, converted to base units with the decimals in this chain's
//!   `AssetRegistry` (never the decimals the program declares, which must agree or the trade is
//!   refused);
//! * the cost schedule and fee collector governance set with `set_trading_config`. Until it is
//!   set, every trade is refused (`TradingNotConfigured`): the chain does not guess its own fees.
//!
//! A committed trade's ledger intents are applied as `CanonicalLedger` transfers, the venues'
//! reserves are written through the host, and the result is checked against the engine's final
//! state (trader balances and flash-pool liquidity). Any disagreement fails the extrinsic, and the
//! extrinsic's storage layer discards every write it made. A trade the engine rolls back moves
//! nothing; its receipt is still recorded, so the refusal is on chain and replayable.

use crate::pallet::{
    AssetRegistry, CanonicalLedger, Config, Error, Event, NextIssuedAssetId, Pallet,
    ProtocolPaused, TradeNonces, TradeReceipts, TradingConfiguration,
};
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{CheckedAdd, CheckedSub, Get, Zero};
use x3bc_core::format::BytecodeModule;
use x3bc_core::receipt::{self, Account, Outcome};
use x3bc_core::state::{
    ChainAnchor, CostSchedule, FlashPoolState, OracleObservation, TradingPreState, VenueState,
};
use x3bc_core::trading::{
    CapabilityKind, TradeOp, TradingSection, CAPABILITY_VERSION, FLASH_PROVIDER, ORACLE,
    X3_CHAIN_ID,
};

/// Ledger asset ids from here up are issued by [`Pallet::issue_ledger_asset`] (factory tokens);
/// ids below it are registered by governance.
pub const ISSUED_ASSET_BASE: u32 = 1 << 31;

/// Largest trading module `submit_trade` accepts.
pub const MAX_TRADE_MODULE_BYTES: u32 = 64 * 1024;
/// Largest encoded receipt the chain stores.
pub const MAX_TRADE_RECEIPT_BYTES: u32 = 64 * 1024;
/// Decimals of the oracle's USD prices (`pallet_x3_oracle::Price`).
pub const ORACLE_PRICE_DECIMALS: u32 = 6;

/// Most ledger intents one trade can produce: the engine emits one per borrow and per repay, two
/// per swap, and one per cost kind when costs are charged (seven kinds, charged once).
pub const MAX_INTENTS: u64 = 2 * x3bc_core::trading::MAX_OPS as u64 + 7;

// Database accesses per item, counted from the code paths `submit_trade` runs. The up-front charge
// is these at the section limits; the refund is these at the trade's own sizes, so the two can
// only differ by how big the trade actually was.
//
// Fixed: `ProtocolPaused` (twice), the transfer halt, `TradingConfiguration`, the receipt's
// existence check, `TradeNonces`, and the system reads for the anchor; writes: the receipt record,
// the nonce, the event.
const FIXED_READS: u64 = 8;
const FIXED_WRITES: u64 = 3;
/// Registry, the trader's balance, and the post-settlement balance check.
const READS_PER_ASSET: u64 = 3;
/// A venue capability: the dex pool and its custody marker.
const READS_PER_CAPABILITY: u64 = 3;
/// A borrow (flash pool and its liquidity) or a swap pair (two oracle prices).
const READS_PER_OP: u64 = 3;
/// Each side's account (a venue or flash pool lookup, two reads each) and the transfer itself
/// (pause flag, registry, both balances); two balance writes.
const READS_PER_INTENT: u64 = 8;
const WRITES_PER_INTENT: u64 = 2;
/// The pool, its custody marker and both custody balances; the pool write.
const READS_PER_VENUE_UPDATE: u64 = 7;
const WRITES_PER_VENUE_UPDATE: u64 = 1;
/// The flash pool, its liquidity and the final balance check.
const READS_PER_FLASH_CHECK: u64 = 4;

/// What one `submit_trade` did, filled in as it runs; `weight` prices it. Every count is the
/// size of a structure the call actually walked, so a refused module pays only for what was
/// read before the refusal.
#[derive(Default, Clone, Copy, Debug)]
pub struct TradeMeter {
    pub assets: u64,
    pub capabilities: u64,
    pub ops: u64,
    pub intents: u64,
    pub venue_updates: u64,
    pub flash_checks: u64,
    /// Gas the engine used; `None` when it never ran.
    pub gas: Option<u64>,
}

impl TradeMeter {
    /// The largest trade a section within its limits can make, with the engine at its gas limit.
    pub fn upper_bound(gas_limit: u64) -> Self {
        use x3bc_core::trading::{MAX_ASSETS, MAX_CAPABILITIES, MAX_OPS};
        Self {
            // The section's assets plus the fee asset.
            assets: MAX_ASSETS as u64 + 1,
            capabilities: MAX_CAPABILITIES as u64,
            ops: MAX_OPS as u64,
            intents: MAX_INTENTS,
            // Every venue the trade touched is a capability, and every flash pool a borrow.
            venue_updates: MAX_CAPABILITIES as u64,
            flash_checks: MAX_OPS as u64,
            gas: Some(gas_limit),
        }
    }

    pub fn weight<T: Config>(&self) -> Weight {
        let reads = FIXED_READS
            .saturating_add(self.assets.saturating_mul(READS_PER_ASSET))
            .saturating_add(self.capabilities.saturating_mul(READS_PER_CAPABILITY))
            .saturating_add(self.ops.saturating_mul(READS_PER_OP))
            .saturating_add(self.intents.saturating_mul(READS_PER_INTENT))
            .saturating_add(self.venue_updates.saturating_mul(READS_PER_VENUE_UPDATE))
            .saturating_add(self.flash_checks.saturating_mul(READS_PER_FLASH_CHECK));
        let writes = FIXED_WRITES
            .saturating_add(self.intents.saturating_mul(WRITES_PER_INTENT))
            .saturating_add(self.venue_updates.saturating_mul(WRITES_PER_VENUE_UPDATE));
        let execution = match self.gas {
            Some(gas) => <<T as Config>::WeightInfo as crate::WeightInfo>::x3_execute(
                gas.min(u64::from(u32::MAX)) as u32,
            ),
            None => Weight::zero(),
        };
        execution.saturating_add(T::DbWeight::get().reads_writes(reads, writes))
    }
}

/// What the runtime provides to the trading engine. `()` provides nothing, so every trade that
/// names a venue, a flash pool or an oracle price is refused by the engine.
pub trait KernelTradingHost<AccountId> {
    /// The venue `name` (`x3dex_pool_<id>`) and the account that custodies its reserves.
    fn venue(name: &str) -> Option<(VenueState, AccountId)>;
    /// Record `name`'s reserves after its custody account's balances moved. Must refuse reserves
    /// the custody account does not hold.
    fn set_venue_reserves(name: &str, reserve_a: u128, reserve_b: u128) -> DispatchResult;
    /// `provider`'s flash pool for `asset` and the account that holds its liquidity.
    fn flash_pool(provider: &str, asset: u32) -> Option<(FlashPoolState, AccountId)>;
    /// `asset`'s USD price per whole unit, with [`ORACLE_PRICE_DECIMALS`] decimals, and the block
    /// it was last updated in.
    fn usd_price(asset: u32) -> Option<(u128, u64)>;
    /// Whether the chain's economic halt freezes asset movement. A trade moves assets, so it is
    /// refused while this is true.
    fn transfers_halted() -> bool;

    /// Build the market the `submit_trade` benchmark trades in, through the runtime's own pallets:
    /// assets [`BENCHMARK_USDX`] and [`BENCHMARK_WETX`] registered (6 decimals); venue
    /// `x3dex_pool_1` (USDX/WETX, 1 000 000 / 500 000 whole units) and `x3dex_pool_2` (WETX/USDX,
    /// 400 000 / 830 000), both 30 bps; a `x3_flashloan` pool of 5 000 000 USDX; prices of $1 and
    /// $2 updated in the current block; and `trader` holding 50 USDX. The benchmark sets the cost
    /// schedule itself. A host that cannot build it returns an error and the benchmark stops.
    #[cfg(feature = "runtime-benchmarks")]
    fn benchmark_market(_trader: &AccountId) -> Result<(), &'static str> {
        Err("this trading host cannot build the benchmark market")
    }
}

/// The benchmark market's quote asset (see [`KernelTradingHost::benchmark_market`]).
#[cfg(feature = "runtime-benchmarks")]
pub const BENCHMARK_USDX: u32 = 1;
/// The benchmark market's base asset.
#[cfg(feature = "runtime-benchmarks")]
pub const BENCHMARK_WETX: u32 = 2;
/// The block the benchmark trade was signed against; it executes [`BENCHMARK_EXECUTED_AT`].
#[cfg(feature = "runtime-benchmarks")]
pub const BENCHMARK_SUBMITTED_AT: u32 = 1_000;
#[cfg(feature = "runtime-benchmarks")]
pub const BENCHMARK_EXECUTED_AT: u32 = 1_002;

/// The benchmark trade: `x3-lang/examples/x3chain_flash_arb.x3` with its private-submission line
/// set to `false`, compiled by `compile_trading_x3bc` (testnet mode). A committed build product,
/// because the benchmark also runs inside the wasm runtime where the compiler does not;
/// `tests_trading::the_benchmark_trade_is_the_compiled_flagship` fails if it goes stale.
pub const BENCHMARK_TRADE_MODULE: &[u8] = include_bytes!("benchmark_trade.x3bc");

impl<AccountId> KernelTradingHost<AccountId> for () {
    fn venue(_name: &str) -> Option<(VenueState, AccountId)> {
        None
    }
    fn set_venue_reserves(_name: &str, _reserve_a: u128, _reserve_b: u128) -> DispatchResult {
        Err(DispatchError::Other("no trading host"))
    }
    fn flash_pool(_provider: &str, _asset: u32) -> Option<(FlashPoolState, AccountId)> {
        None
    }
    fn usd_price(_asset: u32) -> Option<(u128, u64)> {
        None
    }
    fn transfers_halted() -> bool {
        false
    }
}

/// The chain's trade cost schedule and fee collector, set by governance (or at genesis on a
/// development or test network).
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct TradingConfig<AccountId> {
    /// `CanonicalLedger` asset every chain-mandated cost is paid in.
    pub fee_asset: u32,
    /// Per unit of engine gas.
    pub gas_price: u128,
    pub settlement_fee: u128,
    pub relay_fee: u128,
    pub protocol_fee: u128,
    /// Per oracle observation read.
    pub oracle_fee: u128,
    pub solver_fee: u128,
    pub cross_domain_fee: u128,
    /// Receives every chain-mandated cost.
    pub fee_collector: AccountId,
}

/// A trade's on-chain record, keyed by its receipt digest.
///
/// Only the digest is kept in state: the full receipt (up to [`MAX_TRADE_RECEIPT_BYTES`]) is in
/// the trade's event, where an indexer or a replayer reads it and checks it against this digest.
/// Keeping every receipt in state would let anyone grow the chain's state by tens of KiB per
/// trade, rolled-back ones included, for a transaction fee.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(BlockNumber))]
pub struct TradeRecord<AccountId, BlockNumber> {
    pub trader: AccountId,
    pub block: BlockNumber,
    pub committed: bool,
}

/// A receipt as a trade event carries it: the canonical encoding (`x3bc_core::receipt::encode`).
pub type ReceiptBytes = BoundedVec<u8, ConstU32<MAX_TRADE_RECEIPT_BYTES>>;

/// The 32-byte key the engine knows the trader by: the account's encoding when it is 32 bytes
/// (the runtime's `AccountId32`), its blake2-256 otherwise.
pub fn trader_key<AccountId: Encode>(who: &AccountId) -> [u8; 32] {
    let encoded = who.encode();
    if encoded.len() == 32 {
        let mut key = [0u8; 32];
        key.copy_from_slice(&encoded);
        key
    } else {
        blake2_256(&encoded)
    }
}

fn pow10(exponent: u32) -> Option<u128> {
    10u128.checked_pow(exponent)
}

impl<T: Config> Pallet<T> {
    /// `who`'s `CanonicalLedger` balance of `asset`, as `u128`.
    pub fn ledger_balance_of(asset: u32, who: &T::AccountId) -> u128 {
        match T::AssetId::try_from(asset) {
            Ok(id) => CanonicalLedger::<T>::get(who, id).into(),
            Err(_) => 0,
        }
    }

    /// Move `amount` of `asset` between two `CanonicalLedger` accounts, with checked arithmetic.
    /// Refused while the protocol is paused, for an unregistered asset, on an insufficient
    /// balance and on an overflow; nothing is written unless both sides succeed.
    pub fn ledger_transfer(
        asset: u32,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: u128,
    ) -> DispatchResult {
        ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolIsPaused);
        let id = T::AssetId::try_from(asset).map_err(|_| Error::<T>::UnknownAsset)?;
        ensure!(
            AssetRegistry::<T>::contains_key(id),
            Error::<T>::UnknownAsset
        );
        let amount = T::Balance::try_from(amount).map_err(|_| Error::<T>::LedgerOverflow)?;
        if amount.is_zero() || from == to {
            return Ok(());
        }
        let from_balance = CanonicalLedger::<T>::get(from, id)
            .checked_sub(&amount)
            .ok_or(Error::<T>::InsufficientLedgerBalance)?;
        let to_balance = CanonicalLedger::<T>::get(to, id)
            .checked_add(&amount)
            .ok_or(Error::<T>::LedgerOverflow)?;
        CanonicalLedger::<T>::insert(from, id, from_balance);
        CanonicalLedger::<T>::insert(to, id, to_balance);
        Ok(())
    }

    /// Register a new ledger asset for an issuer (the token factory), at the next free id from
    /// [`ISSUED_ASSET_BASE`] up, so issued tokens never take an id governance registers below it.
    pub fn issue_ledger_asset(symbol: &[u8], decimals: u8) -> Result<u32, DispatchError> {
        ensure!(!symbol.is_empty(), Error::<T>::EmptySymbol);
        ensure!(decimals <= 30, Error::<T>::InvalidDecimals);
        let bounded: BoundedVec<u8, T::MaxAssetSymbolLength> = symbol
            .to_vec()
            .try_into()
            .map_err(|_| Error::<T>::SymbolTooLong)?;
        let mut id = NextIssuedAssetId::<T>::get().max(ISSUED_ASSET_BASE);
        // An id governance registered in the issued range is skipped; a run of them is refused
        // rather than searched (the loop is bounded).
        for attempt in 0..=16u8 {
            ensure!(attempt < 16, Error::<T>::AssetAlreadyRegistered);
            let key = T::AssetId::try_from(id).map_err(|_| Error::<T>::UnknownAsset)?;
            if !AssetRegistry::<T>::contains_key(key) {
                AssetRegistry::<T>::insert(
                    key,
                    crate::pallet::AssetMetadata {
                        symbol: bounded,
                        decimals,
                    },
                );
                break;
            }
            id = id.checked_add(1).ok_or(Error::<T>::LedgerOverflow)?;
        }
        NextIssuedAssetId::<T>::put(id.checked_add(1).ok_or(Error::<T>::LedgerOverflow)?);
        Ok(id)
    }

    /// Credit `amount` of a registered ledger asset to `who` (an issuer's mint).
    pub fn mint_to_ledger(asset: u32, who: &T::AccountId, amount: u128) -> DispatchResult {
        let id = T::AssetId::try_from(asset).map_err(|_| Error::<T>::UnknownAsset)?;
        ensure!(
            AssetRegistry::<T>::contains_key(id),
            Error::<T>::UnknownAsset
        );
        let amount = T::Balance::try_from(amount).map_err(|_| Error::<T>::LedgerOverflow)?;
        CanonicalLedger::<T>::try_mutate(who, id, |balance| -> DispatchResult {
            *balance = balance
                .checked_add(&amount)
                .ok_or(Error::<T>::LedgerOverflow)?;
            Ok(())
        })
    }

    /// Debit `amount` of a registered ledger asset from `who` (an issuer's burn).
    pub fn burn_from_ledger(asset: u32, who: &T::AccountId, amount: u128) -> DispatchResult {
        let id = T::AssetId::try_from(asset).map_err(|_| Error::<T>::UnknownAsset)?;
        ensure!(
            AssetRegistry::<T>::contains_key(id),
            Error::<T>::UnknownAsset
        );
        let amount = T::Balance::try_from(amount).map_err(|_| Error::<T>::LedgerOverflow)?;
        CanonicalLedger::<T>::try_mutate(who, id, |balance| -> DispatchResult {
            *balance = balance
                .checked_sub(&amount)
                .ok_or(Error::<T>::InsufficientLedgerBalance)?;
            Ok(())
        })
    }

    /// Run a trading module and settle it. The dispatchable's body.
    pub(crate) fn do_submit_trade(
        meter: &mut TradeMeter,
        who: T::AccountId,
        bytecode: &[u8],
        submitted_at_block: u64,
    ) -> DispatchResult {
        ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolIsPaused);
        ensure!(
            bytecode.len() <= MAX_TRADE_MODULE_BYTES as usize,
            Error::<T>::TradeModuleTooLarge
        );
        // A trade moves assets; the chain's economic halt freezes asset movement, as it does for
        // a direct dex swap.
        ensure!(
            !T::TradingHost::transfers_halted(),
            Error::<T>::TradingHalted
        );
        let config = TradingConfiguration::<T>::get().ok_or(Error::<T>::TradingNotConfigured)?;
        // A transfer to oneself moves nothing, so a trader who is also the fee collector would
        // settle against a debit that never happened. Refused up front rather than at the check.
        ensure!(config.fee_collector != who, Error::<T>::TradeRefused);
        let module = BytecodeModule::from_bytes(bytecode).map_err(|_| Error::<T>::TradeRefused)?;
        let section = module.trading.as_ref().ok_or(Error::<T>::TradeRefused)?;
        ensure!(section.chain_id == X3_CHAIN_ID, Error::<T>::TradeRefused);
        // The section's own rules, including its size bounds (`MAX_OPS`, `MAX_CAPABILITIES`,
        // `MAX_ASSETS`), before any chain state is read on its behalf. The engine verifies the
        // whole module again before it runs.
        x3bc_core::trading::verify_section(section).map_err(|_| Error::<T>::TradeRefused)?;
        meter.assets = section.assets.len() as u64 + 1;
        meter.capabilities = section.capabilities.len() as u64;
        meter.ops = section.ops.len() as u64;

        let block_number = Self::current_block_u64();
        let pre =
            Self::trading_pre_state(&who, section, &config, block_number, submitted_at_block)?;
        let anchor = Self::trading_anchor(block_number);
        let gas_limit = T::DefaultX3GasLimit::get();

        // `execute_trade` refuses before the interpreter starts (decode, verifier, private
        // channel). Such a refusal pays the engine's base cost plus one gas unit's cost per module
        // byte: decoding and verifying are linear in the module's size, and the benchmarked
        // per-gas cost is the interpreter's, a conservative price for a byte of verification. A
        // trade that ran pays for the gas it used.
        meter.gas = Some((bytecode.len() as u64).min(gas_limit));
        let run = x3_x3_integration::trading::execute_trade(bytecode, &pre, &anchor, gas_limit)
            .map_err(|_| Error::<T>::TradeRefused)?;
        meter.gas = Some(run.gas_used.max(bytecode.len() as u64).min(gas_limit));
        // A trading module is not a deployed contract: it has no slots to write. A write here
        // would be dropped, so it is refused instead.
        ensure!(run.storage_writes.is_empty(), Error::<T>::TradeRefused);

        let encoded = receipt::encode(&run.receipt);
        let digest = H256(receipt::digest(&run.receipt));
        let bounded: ReceiptBytes = encoded
            .try_into()
            .map_err(|_| Error::<T>::TradeReceiptTooLarge)?;
        ensure!(
            !TradeReceipts::<T>::contains_key(digest),
            Error::<T>::DuplicateTradeReceipt
        );

        let committed = match (&run.receipt.outcome, run.committed, &run.delta) {
            (Outcome::Committed, true, Some(delta)) => {
                meter.intents = delta.intents.len() as u64;
                meter.venue_updates = delta.venue_reserves.len() as u64;
                meter.flash_checks = delta.flash_liquidity.len() as u64;
                Self::settle_trade(&who, &config, &pre, delta)?;
                true
            }
            (Outcome::RolledBack { .. }, false, None) => false,
            // The engine's verdict, its receipt and its delta must agree; anything else is a
            // fault, and nothing is applied.
            _ => return Err(Error::<T>::TradeSettlementInconsistent.into()),
        };

        TradeReceipts::<T>::insert(
            digest,
            crate::trading::TradeRecord {
                trader: who.clone(),
                block: frame_system::Pallet::<T>::block_number(),
                committed,
            },
        );
        if committed {
            Self::deposit_event(Event::TradeCommitted {
                trader: who,
                receipt: digest,
                gas_used: run.gas_used,
                receipt_bytes: bounded,
            });
        } else {
            let (trap, op_index) = match run.receipt.outcome {
                Outcome::RolledBack { trap, op_index } => (trap as u8, op_index),
                Outcome::Committed => (0, 0),
            };
            Self::deposit_event(Event::TradeRolledBack {
                trader: who,
                receipt: digest,
                trap,
                op_index,
                receipt_bytes: bounded,
            });
        }
        Ok(())
    }

    fn current_block_u64() -> u64 {
        use sp_runtime::SaturatedConversion;
        frame_system::Pallet::<T>::block_number().saturated_into::<u64>()
    }

    fn trading_anchor(block_number: u64) -> ChainAnchor {
        let version = <T as frame_system::Config>::Version::get();
        let mut parent_hash = [0u8; 32];
        let parent = frame_system::Pallet::<T>::parent_hash();
        let bytes = parent.as_ref();
        let take = bytes.len().min(32);
        parent_hash[..take].copy_from_slice(&bytes[..take]);
        ChainAnchor {
            spec_version: version.spec_version,
            transaction_version: version.transaction_version,
            block_number,
            parent_hash,
            extrinsic_index: frame_system::Pallet::<T>::extrinsic_index().unwrap_or(0),
            finality: None,
        }
    }

    /// The registry decimals of `asset`, refused when the program declares different ones.
    fn registry_decimals(asset: u32, declared: u8) -> Result<u8, DispatchError> {
        let id = T::AssetId::try_from(asset).map_err(|_| Error::<T>::UnknownAsset)?;
        let metadata = AssetRegistry::<T>::get(id).ok_or(Error::<T>::UnknownAsset)?;
        ensure!(
            metadata.decimals == declared,
            Error::<T>::TradeAssetMismatch
        );
        Ok(metadata.decimals)
    }

    fn trading_pre_state(
        who: &T::AccountId,
        section: &TradingSection,
        config: &crate::trading::TradingConfig<T::AccountId>,
        block_number: u64,
        submitted_at_block: u64,
    ) -> Result<TradingPreState, DispatchError> {
        let mut trader_balances = alloc::collections::BTreeMap::new();
        let mut decimals = alloc::collections::BTreeMap::new();
        for asset in &section.assets {
            decimals.insert(asset.id, Self::registry_decimals(asset.id, asset.decimals)?);
            trader_balances.insert(asset.id, Self::ledger_balance_of(asset.id, who));
        }
        trader_balances
            .entry(config.fee_asset)
            .or_insert_with(|| Self::ledger_balance_of(config.fee_asset, who));

        let mut venues = Vec::new();
        for capability in &section.capabilities {
            if capability.kind == CapabilityKind::Venue {
                if let Some((venue, _)) = T::TradingHost::venue(&capability.name) {
                    venues.push(venue);
                }
            }
        }

        let mut flash_pools = Vec::new();
        let mut pairs = BTreeSet::new();
        for op in &section.ops {
            match op {
                TradeOp::Borrow {
                    provider, asset, ..
                } => {
                    let provider = section
                        .capabilities
                        .get(usize::from(*provider))
                        .ok_or(Error::<T>::TradeRefused)?;
                    let asset = section
                        .assets
                        .get(usize::from(*asset))
                        .ok_or(Error::<T>::TradeRefused)?;
                    if let Some((pool, _)) = T::TradingHost::flash_pool(&provider.name, asset.id) {
                        if !flash_pools.contains(&pool) {
                            flash_pools.push(pool);
                        }
                    }
                }
                TradeOp::Swap { from, to, .. } => {
                    let from = section.assets.get(usize::from(*from));
                    let to = section.assets.get(usize::from(*to));
                    if let (Some(from), Some(to)) = (from, to) {
                        let pair = if from.id <= to.id {
                            (from.id, to.id)
                        } else {
                            (to.id, from.id)
                        };
                        pairs.insert(pair);
                    }
                }
                _ => {}
            }
        }

        // One reading per swap pair: units of `quote` per unit of `base`, in base units.
        let mut oracle = Vec::new();
        for (base, quote) in pairs {
            let (Some((base_price, base_block)), Some((quote_price, quote_block))) = (
                T::TradingHost::usd_price(base),
                T::TradingHost::usd_price(quote),
            ) else {
                continue;
            };
            if base_price == 0 || quote_price == 0 {
                continue;
            }
            let (Some(base_decimals), Some(quote_decimals)) =
                (decimals.get(&base), decimals.get(&quote))
            else {
                continue;
            };
            let price_num = base_price
                .checked_mul(pow10(u32::from(*quote_decimals)).ok_or(Error::<T>::LedgerOverflow)?)
                .ok_or(Error::<T>::LedgerOverflow)?;
            let price_den = quote_price
                .checked_mul(pow10(u32::from(*base_decimals)).ok_or(Error::<T>::LedgerOverflow)?)
                .ok_or(Error::<T>::LedgerOverflow)?;
            oracle.push(OracleObservation {
                base,
                quote,
                price_num,
                price_den,
                // The older of the two prices is the age of the pair.
                observed_at_block: base_block.min(quote_block),
                source: String::from(ORACLE),
            });
        }

        Ok(TradingPreState {
            chain_id: X3_CHAIN_ID,
            block_number,
            submitted_at_block,
            trader: trader_key(who),
            trader_balances,
            flash_pools,
            venues,
            oracle,
            costs: CostSchedule {
                fee_asset: config.fee_asset,
                gas_price: config.gas_price,
                settlement_fee: config.settlement_fee,
                relay_fee: config.relay_fee,
                protocol_fee: config.protocol_fee,
                oracle_fee: config.oracle_fee,
                solver_fee: config.solver_fee,
                cross_domain_fee: config.cross_domain_fee,
            },
            // Extrinsics arrive through the public pool; no private channel exists on chain.
            private_channel: false,
        })
    }

    /// The chain account behind one of the engine's ledger accounts.
    fn trading_account(
        who: &T::AccountId,
        config: &crate::trading::TradingConfig<T::AccountId>,
        account: &Account,
        asset: u32,
    ) -> Result<T::AccountId, DispatchError> {
        Ok(match account {
            Account::Trader => who.clone(),
            Account::FeeCollector => config.fee_collector.clone(),
            Account::Venue(name) => {
                T::TradingHost::venue(name)
                    .ok_or(Error::<T>::TradeSettlementInconsistent)?
                    .1
            }
            Account::FlashPool(provider) => {
                T::TradingHost::flash_pool(provider, asset)
                    .ok_or(Error::<T>::TradeSettlementInconsistent)?
                    .1
            }
        })
    }

    /// Apply a committed trade: every intent as a ledger transfer, then the venues' reserves,
    /// then check the chain now holds exactly what the engine ended with.
    fn settle_trade(
        who: &T::AccountId,
        config: &crate::trading::TradingConfig<T::AccountId>,
        pre: &TradingPreState,
        delta: &x3_vm::trading::TradingDelta,
    ) -> DispatchResult {
        for intent in &delta.intents {
            let from = Self::trading_account(who, config, &intent.from, intent.asset)?;
            let to = Self::trading_account(who, config, &intent.to, intent.asset)?;
            Self::ledger_transfer(intent.asset, &from, &to, intent.amount)?;
        }
        for (venue, (reserve_a, reserve_b)) in &delta.venue_reserves {
            // New reserves are the old reserves plus exactly what the intents moved into and out
            // of this venue, per asset. Custody covering the new figure is not enough: a pool
            // holding more than its reserves would otherwise let a trade record reserves the moved
            // funds do not account for.
            let before = pre
                .venues
                .iter()
                .find(|v| &v.name == venue)
                .ok_or(Error::<T>::TradeSettlementInconsistent)?;
            let flowed = |asset: u32, reserve: u128| -> Option<u128> {
                let mut value = reserve;
                for intent in delta.intents.iter().filter(|i| i.asset == asset) {
                    if matches!(&intent.to, Account::Venue(name) if name == venue) {
                        value = value.checked_add(intent.amount)?;
                    }
                    if matches!(&intent.from, Account::Venue(name) if name == venue) {
                        value = value.checked_sub(intent.amount)?;
                    }
                }
                Some(value)
            };
            ensure!(
                flowed(before.asset_a, before.reserve_a) == Some(*reserve_a)
                    && flowed(before.asset_b, before.reserve_b) == Some(*reserve_b),
                Error::<T>::TradeSettlementInconsistent
            );
            T::TradingHost::set_venue_reserves(venue, *reserve_a, *reserve_b)?;
        }
        for (asset, expected) in &delta.trader_balances {
            ensure!(
                Self::ledger_balance_of(*asset, who) == *expected,
                Error::<T>::TradeSettlementInconsistent
            );
        }
        for ((provider, asset), expected) in &delta.flash_liquidity {
            ensure!(
                provider == FLASH_PROVIDER,
                Error::<T>::TradeSettlementInconsistent
            );
            let (pool, account) = T::TradingHost::flash_pool(provider, *asset)
                .ok_or(Error::<T>::TradeSettlementInconsistent)?;
            ensure!(
                pool.version == CAPABILITY_VERSION
                    && Self::ledger_balance_of(*asset, &account) == *expected,
                Error::<T>::TradeSettlementInconsistent
            );
        }
        TradeNonces::<T>::mutate(who, |nonce| {
            *nonce = nonce.saturating_add(u64::from(delta.trader_nonce_increment))
        });
        Ok(())
    }
}
