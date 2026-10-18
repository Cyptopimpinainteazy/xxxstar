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
    AssetRegistry, CanonicalLedger, Config, Error, Event, Pallet, ProtocolPaused, TradeNonces,
    TradeReceipts, TradingConfiguration,
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

/// Largest trading module `submit_trade` accepts.
pub const MAX_TRADE_MODULE_BYTES: u32 = 64 * 1024;
/// Largest encoded receipt the chain stores.
pub const MAX_TRADE_RECEIPT_BYTES: u32 = 64 * 1024;
/// Decimals of the oracle's USD prices (`pallet_x3_oracle::Price`).
pub const ORACLE_PRICE_DECIMALS: u32 = 6;

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
}

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
}

/// The chain's trade cost schedule and fee collector, set by governance.
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(BlockNumber))]
pub struct TradeRecord<AccountId, BlockNumber> {
    pub trader: AccountId,
    pub block: BlockNumber,
    pub committed: bool,
    /// The canonical receipt encoding (`x3bc_core::receipt::encode`).
    pub receipt: BoundedVec<u8, ConstU32<MAX_TRADE_RECEIPT_BYTES>>,
}

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

    /// Run a trading module and settle it. The dispatchable's body.
    pub(crate) fn do_submit_trade(
        who: T::AccountId,
        bytecode: &[u8],
        submitted_at_block: u64,
    ) -> DispatchResult {
        ensure!(!ProtocolPaused::<T>::get(), Error::<T>::ProtocolIsPaused);
        ensure!(
            bytecode.len() <= MAX_TRADE_MODULE_BYTES as usize,
            Error::<T>::TradeModuleTooLarge
        );
        let config = TradingConfiguration::<T>::get().ok_or(Error::<T>::TradingNotConfigured)?;
        let module = BytecodeModule::from_bytes(bytecode).map_err(|_| Error::<T>::TradeRefused)?;
        let section = module.trading.as_ref().ok_or(Error::<T>::TradeRefused)?;
        ensure!(section.chain_id == X3_CHAIN_ID, Error::<T>::TradeRefused);

        let block_number = Self::current_block_u64();
        let pre =
            Self::trading_pre_state(&who, section, &config, block_number, submitted_at_block)?;
        let anchor = Self::trading_anchor(block_number);
        let gas_limit = T::DefaultX3GasLimit::get();

        let run = x3_x3_integration::trading::execute_trade(bytecode, &pre, &anchor, gas_limit)
            .map_err(|_| Error::<T>::TradeRefused)?;
        // A trading module is not a deployed contract: it has no slots to write. A write here
        // would be dropped, so it is refused instead.
        ensure!(run.storage_writes.is_empty(), Error::<T>::TradeRefused);

        let encoded = receipt::encode(&run.receipt);
        let digest = H256(receipt::digest(&run.receipt));
        let bounded: BoundedVec<u8, ConstU32<MAX_TRADE_RECEIPT_BYTES>> = encoded
            .try_into()
            .map_err(|_| Error::<T>::TradeReceiptTooLarge)?;
        ensure!(
            !TradeReceipts::<T>::contains_key(digest),
            Error::<T>::DuplicateTradeReceipt
        );

        let committed = match (&run.receipt.outcome, run.committed, &run.delta) {
            (Outcome::Committed, true, Some(delta)) => {
                Self::settle_trade(&who, &config, delta)?;
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
                receipt: bounded,
            },
        );
        if committed {
            Self::deposit_event(Event::TradeCommitted {
                trader: who,
                receipt: digest,
                gas_used: run.gas_used,
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
        delta: &x3_vm::trading::TradingDelta,
    ) -> DispatchResult {
        for intent in &delta.intents {
            let from = Self::trading_account(who, config, &intent.from, intent.asset)?;
            let to = Self::trading_account(who, config, &intent.to, intent.asset)?;
            Self::ledger_transfer(intent.asset, &from, &to, intent.amount)?;
        }
        for (venue, (reserve_a, reserve_b)) in &delta.venue_reserves {
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
