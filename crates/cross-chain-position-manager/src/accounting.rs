//! Cross-chain accounting engine for position tracking.
//!
//! This module provides:
//! - USD normalization across chains
//! - Multi-chain balance tracking
//! - Position snapshot system
//! - Fast state diffing
//! - Phase 4.5 inventory reservations and pending obligation tracking

use crate::router::InventoryBand;
use crate::{PositionId, PositionManagerConfig, PositionManagerError, Result, H160, H256, U256};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sp_std::collections::btree_map::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObligationStatus {
    Reserved,
    PendingOutbound,
    PendingInbound,
    Settled,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InventoryKey {
    pub chain_id: u64,
    pub asset: H160,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryBalance {
    pub key: InventoryKey,
    pub available: U256,
    pub reserved: U256,
    pub pending_out: U256,
    pub pending_in: U256,
    pub band: Option<InventoryBand>,
}

impl InventoryBalance {
    fn new(chain_id: u64, asset: H160) -> Self {
        Self {
            key: InventoryKey { chain_id, asset },
            available: U256::zero(),
            reserved: U256::zero(),
            pending_out: U256::zero(),
            pending_in: U256::zero(),
            band: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteObligation {
    pub route_id: H256,
    pub lane_id: H256,
    pub source: InventoryKey,
    pub destination: InventoryKey,
    pub source_amount: U256,
    pub destination_amount: U256,
    pub status: ObligationStatus,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySnapshot {
    pub snapshot_id: H256,
    pub timestamp: u64,
    pub balances: Vec<InventoryBalance>,
    pub obligations: Vec<RouteObligation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryReservationRequest {
    pub route_id: H256,
    pub lane_id: H256,
    pub source_chain: u64,
    pub source_asset: H160,
    pub source_amount: U256,
    pub destination_chain: u64,
    pub destination_asset: H160,
    pub destination_amount: U256,
}

#[derive(Debug, Clone, Default)]
pub struct InventoryManager {
    balances: BTreeMap<InventoryKey, InventoryBalance>,
    obligations: BTreeMap<H256, RouteObligation>,
}

type AssetBreakdownEntry = (String, U256, Vec<(u64, U256)>);

impl InventoryManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_available_balance(&mut self, chain_id: u64, asset: H160, available: U256) {
        self.ensure_balance_mut(chain_id, asset).available = available;
    }

    pub fn set_inventory_band(&mut self, chain_id: u64, asset: H160, band: InventoryBand) {
        self.ensure_balance_mut(chain_id, asset).band = Some(band);
    }

    pub fn balance(&self, chain_id: u64, asset: H160) -> Option<&InventoryBalance> {
        self.balances.get(&InventoryKey { chain_id, asset })
    }

    pub fn free_balance(&self, chain_id: u64, asset: H160) -> U256 {
        self.balance(chain_id, asset)
            .map(|balance| balance.available)
            .unwrap_or_else(U256::zero)
    }

    pub fn obligation(&self, route_id: &H256) -> Option<&RouteObligation> {
        self.obligations.get(route_id)
    }

    pub fn reserve(&mut self, request: InventoryReservationRequest) -> Result<RouteObligation> {
        if self.obligations.contains_key(&request.route_id) {
            return Err(PositionManagerError::InvalidInput(format!(
                "duplicate reservation for route {}",
                hex::encode(request.route_id)
            )));
        }

        let source = self.ensure_balance_mut(request.source_chain, request.source_asset);
        if source.available < request.source_amount {
            return Err(PositionManagerError::InsufficientInventory(format!(
                "insufficient free inventory for {}:{} requested={} available={}",
                request.source_chain,
                hex::encode(request.source_asset),
                request.source_amount,
                source.available
            )));
        }

        source.available = source.available.saturating_sub(request.source_amount);
        source.reserved = source.reserved.saturating_add(request.source_amount);

        let now = current_time_ms();
        let obligation = RouteObligation {
            route_id: request.route_id,
            lane_id: request.lane_id,
            source: InventoryKey {
                chain_id: request.source_chain,
                asset: request.source_asset,
            },
            destination: InventoryKey {
                chain_id: request.destination_chain,
                asset: request.destination_asset,
            },
            source_amount: request.source_amount,
            destination_amount: request.destination_amount,
            status: ObligationStatus::Reserved,
            created_at_ms: now,
            updated_at_ms: now,
        };

        self.obligations
            .insert(request.route_id, obligation.clone());
        Ok(obligation)
    }

    pub fn release_reservation(&mut self, route_id: &H256) -> Result<()> {
        let (source_key, source_amount) = {
            let obligation = self
                .obligations
                .get_mut(route_id)
                .ok_or_else(|| PositionManagerError::ObligationNotFound(hex::encode(route_id)))?;

            if obligation.status != ObligationStatus::Reserved {
                return Err(PositionManagerError::InvalidObligationState(format!(
                    "cannot release route {} from state {:?}",
                    hex::encode(route_id),
                    obligation.status
                )));
            }

            obligation.status = ObligationStatus::Released;
            obligation.updated_at_ms = current_time_ms();
            (obligation.source, obligation.source_amount)
        };

        let source = self.ensure_balance_mut(source_key.chain_id, source_key.asset);
        source.reserved = source.reserved.saturating_sub(source_amount);
        source.available = source.available.saturating_add(source_amount);
        Ok(())
    }

    pub fn mark_pending_out(&mut self, route_id: &H256) -> Result<()> {
        let (source_key, source_amount) = {
            let obligation = self
                .obligations
                .get_mut(route_id)
                .ok_or_else(|| PositionManagerError::ObligationNotFound(hex::encode(route_id)))?;

            if obligation.status != ObligationStatus::Reserved {
                return Err(PositionManagerError::InvalidObligationState(format!(
                    "cannot mark pending outbound for route {} from state {:?}",
                    hex::encode(route_id),
                    obligation.status
                )));
            }

            obligation.status = ObligationStatus::PendingOutbound;
            obligation.updated_at_ms = current_time_ms();
            (obligation.source, obligation.source_amount)
        };

        let source = self.ensure_balance_mut(source_key.chain_id, source_key.asset);
        source.reserved = source.reserved.saturating_sub(source_amount);
        source.pending_out = source.pending_out.saturating_add(source_amount);
        Ok(())
    }

    pub fn mark_pending_in(&mut self, route_id: &H256) -> Result<()> {
        let (source_key, destination_key, source_amount, destination_amount) = {
            let obligation = self
                .obligations
                .get_mut(route_id)
                .ok_or_else(|| PositionManagerError::ObligationNotFound(hex::encode(route_id)))?;

            if obligation.status != ObligationStatus::PendingOutbound {
                return Err(PositionManagerError::InvalidObligationState(format!(
                    "cannot mark pending inbound for route {} from state {:?}",
                    hex::encode(route_id),
                    obligation.status
                )));
            }

            obligation.status = ObligationStatus::PendingInbound;
            obligation.updated_at_ms = current_time_ms();
            (
                obligation.source,
                obligation.destination,
                obligation.source_amount,
                obligation.destination_amount,
            )
        };

        {
            let source = self.ensure_balance_mut(source_key.chain_id, source_key.asset);
            source.pending_out = source.pending_out.saturating_sub(source_amount);
        }

        {
            let destination =
                self.ensure_balance_mut(destination_key.chain_id, destination_key.asset);
            destination.pending_in = destination.pending_in.saturating_add(destination_amount);
        }

        Ok(())
    }

    pub fn settle_inbound(&mut self, route_id: &H256) -> Result<()> {
        let (destination_key, destination_amount) = {
            let obligation = self
                .obligations
                .get_mut(route_id)
                .ok_or_else(|| PositionManagerError::ObligationNotFound(hex::encode(route_id)))?;

            if obligation.status != ObligationStatus::PendingInbound {
                return Err(PositionManagerError::InvalidObligationState(format!(
                    "cannot settle route {} from state {:?}",
                    hex::encode(route_id),
                    obligation.status
                )));
            }

            obligation.status = ObligationStatus::Settled;
            obligation.updated_at_ms = current_time_ms();
            (obligation.destination, obligation.destination_amount)
        };

        let destination = self.ensure_balance_mut(destination_key.chain_id, destination_key.asset);
        destination.pending_in = destination.pending_in.saturating_sub(destination_amount);
        destination.available = destination.available.saturating_add(destination_amount);
        Ok(())
    }

    pub fn snapshot(&self) -> InventorySnapshot {
        let timestamp = current_time_ms();
        let balances: Vec<InventoryBalance> = self.balances.values().cloned().collect();
        let obligations: Vec<RouteObligation> = self.obligations.values().cloned().collect();
        let snapshot_id = inventory_snapshot_id(timestamp, &balances, &obligations);

        InventorySnapshot {
            snapshot_id,
            timestamp,
            balances,
            obligations,
        }
    }

    fn ensure_balance_mut(&mut self, chain_id: u64, asset: H160) -> &mut InventoryBalance {
        self.balances
            .entry(InventoryKey { chain_id, asset })
            .or_insert_with(|| InventoryBalance::new(chain_id, asset))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub snapshot_id: H256,
    pub timestamp: u64,
    pub positions: Vec<PositionBalance>,
    pub total_value_usd: U256,
    pub chain_breakdown: Vec<ChainBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionBalance {
    pub position_id: PositionId,
    pub asset: H160,
    pub amount: U256,
    pub value_usd: U256,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBalance {
    pub chain_id: u64,
    pub total_value_usd: U256,
    pub asset_count: usize,
    pub positions_count: usize,
}

#[derive(Debug, Clone)]
pub struct UsdNormalizer {
    price_feeds: BTreeMap<H160, U256>,
    stablecoins: Vec<H160>,
    last_update: u64,
}

impl UsdNormalizer {
    pub fn new() -> Result<Self> {
        Ok(Self {
            price_feeds: BTreeMap::new(),
            stablecoins: Vec::new(),
            last_update: 0,
        })
    }

    pub fn normalize_to_usd(&self, asset: &H160, amount: U256) -> Result<U256> {
        if self.stablecoins.contains(asset) {
            return Ok(amount);
        }

        let price = self
            .price_feeds
            .get(asset)
            .ok_or_else(|| PositionManagerError::PriceFeedNotFound(format!("{:?}", asset)))?;

        amount
            .checked_mul(*price)
            .ok_or(PositionManagerError::ArithmeticOverflow)?
            .checked_div(U256::from(10).pow(U256::from(18)))
            .ok_or(PositionManagerError::ArithmeticOverflow)
    }

    pub fn update_price(&mut self, asset: H160, price: U256) {
        self.price_feeds.insert(asset, price);
        self.last_update = current_time_ms();
    }

    pub fn register_stablecoin(&mut self, asset: H160) {
        if !self.stablecoins.contains(&asset) {
            self.stablecoins.push(asset);
        }
    }

    pub fn last_update(&self) -> u64 {
        self.last_update
    }
}

#[derive(Debug, Clone)]
pub struct AccountingEngine {
    normalizer: UsdNormalizer,
    snapshots: Vec<PositionSnapshot>,
    current_balances: BTreeMap<PositionId, PositionBalance>,
    inventory_manager: InventoryManager,
    config: PositionManagerConfig,
}

impl AccountingEngine {
    pub fn new() -> Result<Self> {
        Self::with_config(PositionManagerConfig::default())
    }

    pub fn with_config(config: PositionManagerConfig) -> Result<Self> {
        Ok(Self {
            normalizer: UsdNormalizer::new()?,
            snapshots: Vec::new(),
            current_balances: BTreeMap::new(),
            inventory_manager: InventoryManager::new(),
            config,
        })
    }

    pub fn inventory(&self) -> &InventoryManager {
        &self.inventory_manager
    }

    pub fn inventory_mut(&mut self) -> &mut InventoryManager {
        &mut self.inventory_manager
    }

    pub fn register_stablecoin(&mut self, asset: H160) {
        self.normalizer.register_stablecoin(asset);
    }

    pub fn update_price(&mut self, asset: H160, price: U256) {
        self.normalizer.update_price(asset, price);
    }

    pub fn update_balance(
        &mut self,
        position_id: PositionId,
        asset: H160,
        amount: U256,
        chain_id: u64,
    ) -> Result<()> {
        let value_usd = self.normalizer.normalize_to_usd(&asset, amount)?;
        self.current_balances.insert(
            position_id.clone(),
            PositionBalance {
                position_id,
                asset,
                amount,
                value_usd,
                chain_id,
            },
        );
        Ok(())
    }

    pub fn get_balance(&self, position_id: &PositionId) -> Option<&PositionBalance> {
        self.current_balances.get(position_id)
    }

    pub fn take_snapshot(&mut self) -> Result<PositionSnapshot> {
        let timestamp = current_time_ms();
        let mut total_value_usd = U256::zero();
        let mut chain_totals: BTreeMap<u64, (U256, usize, usize)> = BTreeMap::new();
        let mut positions = Vec::new();

        for balance in self.current_balances.values() {
            positions.push(balance.clone());
            total_value_usd = total_value_usd.saturating_add(balance.value_usd);

            let entry = chain_totals
                .entry(balance.chain_id)
                .or_insert((U256::zero(), 0, 0));
            entry.0 = entry.0.saturating_add(balance.value_usd);
            entry.1 += 1;
            entry.2 += 1;
        }

        let chain_breakdown = chain_totals
            .into_iter()
            .map(
                |(chain_id, (total_value_usd, asset_count, positions_count))| ChainBalance {
                    chain_id,
                    total_value_usd,
                    asset_count,
                    positions_count,
                },
            )
            .collect();

        let snapshot = PositionSnapshot {
            snapshot_id: self.generate_snapshot_id(timestamp, &positions),
            timestamp,
            positions,
            total_value_usd,
            chain_breakdown,
        };

        self.snapshots.push(snapshot.clone());
        if self.snapshots.len() > 100 {
            self.snapshots.remove(0);
        }

        Ok(snapshot)
    }

    pub fn latest_snapshot(&self) -> Option<&PositionSnapshot> {
        self.snapshots.last()
    }

    pub fn snapshot_history(&self, limit: usize) -> Vec<&PositionSnapshot> {
        let start = self.snapshots.len().saturating_sub(limit);
        self.snapshots[start..].iter().collect()
    }

    pub fn calculate_diff(
        &self,
        snapshot1: &PositionSnapshot,
        snapshot2: &PositionSnapshot,
    ) -> SnapshotDiff {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        for next in &snapshot2.positions {
            match snapshot1
                .positions
                .iter()
                .find(|existing| existing.position_id == next.position_id)
            {
                Some(previous)
                    if previous.amount != next.amount || previous.value_usd != next.value_usd =>
                {
                    changed.push(PositionDiff {
                        position_id: next.position_id.clone(),
                        old_amount: previous.amount,
                        new_amount: next.amount,
                        old_value_usd: previous.value_usd,
                        new_value_usd: next.value_usd,
                    });
                }
                None => added.push(next.clone()),
                _ => {}
            }
        }

        for previous in &snapshot1.positions {
            if !snapshot2
                .positions
                .iter()
                .any(|next| next.position_id == previous.position_id)
            {
                removed.push(previous.clone());
            }
        }

        let total_value_change = snapshot2
            .total_value_usd
            .checked_sub(snapshot1.total_value_usd)
            .unwrap_or(U256::zero());

        SnapshotDiff {
            from_timestamp: snapshot1.timestamp,
            to_timestamp: snapshot2.timestamp,
            added,
            removed,
            changed,
            total_value_change,
            percentage_change: if snapshot1.total_value_usd > U256::zero() {
                let change = total_value_change.as_u128() as f64;
                let base = snapshot1.total_value_usd.as_u128() as f64;
                (change / base) * 100.0
            } else {
                0.0
            },
        }
    }

    pub async fn get_portfolio_summary(&self) -> Result<PortfolioSummary> {
        let mut total_value_usd = U256::zero();
        let mut chain_breakdown = Vec::new();
        let mut asset_breakdown: BTreeMap<H160, AssetBreakdownEntry> = BTreeMap::new();

        for balance in self.current_balances.values() {
            total_value_usd = total_value_usd.saturating_add(balance.value_usd);

            if let Some(chain) = chain_breakdown
                .iter_mut()
                .find(|chain: &&mut ChainSummary| chain.chain_id == balance.chain_id)
            {
                chain.total_value_usd = chain.total_value_usd.saturating_add(balance.value_usd);
                chain.positions_count += 1;
            } else {
                chain_breakdown.push(ChainSummary {
                    chain_id: balance.chain_id,
                    total_value_usd: balance.value_usd,
                    positions_count: 1,
                    gas_efficiency_score: 1.0,
                });
            }

            let entry = asset_breakdown.entry(balance.asset).or_insert((
                format!("Asset_{:?}", balance.asset),
                U256::zero(),
                Vec::new(),
            ));
            entry.1 = entry.1.saturating_add(balance.amount);
            entry.2.push((balance.chain_id, balance.amount));
        }

        let asset_breakdown = asset_breakdown
            .into_iter()
            .map(
                |(asset_address, (symbol, total_amount, chains_distribution))| AssetSummary {
                    asset_address,
                    symbol,
                    total_amount,
                    total_value_usd: self
                        .normalizer
                        .normalize_to_usd(&asset_address, total_amount)
                        .unwrap_or(U256::zero()),
                    chains_distribution,
                },
            )
            .collect();

        Ok(PortfolioSummary {
            total_value_usd,
            chain_breakdown,
            asset_breakdown,
            risk_score: 0.5,
            rebalance_needed: false,
            active_arbitrage_ops: 0,
        })
    }

    fn generate_snapshot_id(&self, timestamp: u64, positions: &[PositionBalance]) -> H256 {
        use sp_core::hashing::blake2_256;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.snapshots.len().to_le_bytes());
        bytes.extend_from_slice(&self.config.chain_configs.len().to_le_bytes());
        bytes.extend_from_slice(&self.normalizer.last_update().to_le_bytes());
        bytes.extend_from_slice(&u256_bytes(&self.config.risk_config.max_position_size_usd));

        for position in positions {
            bytes.extend_from_slice(position.position_id.as_bytes());
            bytes.extend_from_slice(position.asset.as_fixed_bytes());
            bytes.extend_from_slice(&position.chain_id.to_le_bytes());
            bytes.extend_from_slice(&u256_bytes(&position.amount));
            bytes.extend_from_slice(&u256_bytes(&position.value_usd));
        }

        H256::from(blake2_256(&bytes))
    }

    pub fn last_update(&self) -> u64 {
        self.normalizer.last_update()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub from_timestamp: u64,
    pub to_timestamp: u64,
    pub added: Vec<PositionBalance>,
    pub removed: Vec<PositionBalance>,
    pub changed: Vec<PositionDiff>,
    pub total_value_change: U256,
    pub percentage_change: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionDiff {
    pub position_id: PositionId,
    pub old_amount: U256,
    pub new_amount: U256,
    pub old_value_usd: U256,
    pub new_value_usd: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub total_value_usd: U256,
    pub chain_breakdown: Vec<ChainSummary>,
    pub asset_breakdown: Vec<AssetSummary>,
    pub risk_score: f64,
    pub rebalance_needed: bool,
    pub active_arbitrage_ops: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSummary {
    pub chain_id: u64,
    pub total_value_usd: U256,
    pub positions_count: usize,
    pub gas_efficiency_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSummary {
    pub asset_address: H160,
    pub symbol: String,
    pub total_amount: U256,
    pub total_value_usd: U256,
    pub chains_distribution: Vec<(u64, U256)>,
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn inventory_snapshot_id(
    timestamp: u64,
    balances: &[InventoryBalance],
    obligations: &[RouteObligation],
) -> H256 {
    use sp_core::hashing::blake2_256;

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&timestamp.to_le_bytes());
    bytes.extend_from_slice(&balances.len().to_le_bytes());
    bytes.extend_from_slice(&obligations.len().to_le_bytes());

    for balance in balances {
        bytes.extend_from_slice(&balance.key.chain_id.to_le_bytes());
        bytes.extend_from_slice(balance.key.asset.as_fixed_bytes());
        bytes.extend_from_slice(&u256_bytes(&balance.available));
        bytes.extend_from_slice(&u256_bytes(&balance.reserved));
        bytes.extend_from_slice(&u256_bytes(&balance.pending_out));
        bytes.extend_from_slice(&u256_bytes(&balance.pending_in));
    }

    for obligation in obligations {
        bytes.extend_from_slice(obligation.route_id.as_bytes());
        bytes.extend_from_slice(obligation.lane_id.as_bytes());
        bytes.extend_from_slice(&obligation.source.chain_id.to_le_bytes());
        bytes.extend_from_slice(obligation.source.asset.as_fixed_bytes());
        bytes.extend_from_slice(&obligation.destination.chain_id.to_le_bytes());
        bytes.extend_from_slice(obligation.destination.asset.as_fixed_bytes());
        bytes.extend_from_slice(&u256_bytes(&obligation.source_amount));
        bytes.extend_from_slice(&u256_bytes(&obligation.destination_amount));
        bytes.push(obligation.status as u8);
    }

    H256::from(blake2_256(&bytes))
}

fn u256_bytes(value: &U256) -> [u8; 32] {
    let bytes = value.to_big_endian();
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::InventoryBand;

    #[test]
    fn test_usd_normalizer() {
        let mut normalizer = UsdNormalizer::new().unwrap();
        let asset = H160::random();
        let price = U256::from(1_000_000_000_000_000_000u128);

        normalizer.update_price(asset, price);

        let amount = U256::from(2_000_000_000_000_000_000u128);
        let value = normalizer.normalize_to_usd(&asset, amount).unwrap();

        assert_eq!(value, U256::from(2_000_000_000_000_000_000u128));
    }

    #[test]
    fn test_accounting_engine() {
        let mut engine = AccountingEngine::new().unwrap();
        let position_id = PositionId::new();
        let asset = H160::random();
        let amount = U256::from(1_000_000_000_000_000_000u128);

        engine.register_stablecoin(asset);
        engine
            .update_balance(position_id.clone(), asset, amount, 1)
            .unwrap();

        let balance = engine.get_balance(&position_id).unwrap();
        assert_eq!(balance.amount, amount);
    }

    #[test]
    fn test_inventory_reserve_release_cycle() {
        let asset = H160::repeat_byte(0x11);
        let route_id = H256::from_low_u64_be(7);
        let lane_id = H256::from_low_u64_be(9);
        let mut inventory = InventoryManager::new();
        inventory.set_available_balance(1, asset, U256::from(1_000u64));
        inventory.set_inventory_band(
            1,
            asset,
            InventoryBand {
                critical_min: U256::from(10u64),
                min: U256::from(20u64),
                target: U256::from(100u64),
                max: U256::from(200u64),
            },
        );

        let obligation = inventory
            .reserve(InventoryReservationRequest {
                route_id,
                lane_id,
                source_chain: 1,
                source_asset: asset,
                source_amount: U256::from(300u64),
                destination_chain: 137,
                destination_asset: H160::repeat_byte(0x22),
                destination_amount: U256::from(297u64),
            })
            .unwrap();

        assert_eq!(obligation.status, ObligationStatus::Reserved);
        let balance = inventory.balance(1, asset).unwrap();
        assert_eq!(balance.available, U256::from(700u64));
        assert_eq!(balance.reserved, U256::from(300u64));

        inventory.release_reservation(&route_id).unwrap();
        let balance = inventory.balance(1, asset).unwrap();
        assert_eq!(balance.available, U256::from(1_000u64));
        assert_eq!(balance.reserved, U256::zero());
        assert_eq!(
            inventory.obligation(&route_id).unwrap().status,
            ObligationStatus::Released
        );
    }

    #[test]
    fn test_inventory_rejects_insufficient_balance() {
        let asset = H160::repeat_byte(0x33);
        let mut inventory = InventoryManager::new();
        inventory.set_available_balance(10, asset, U256::from(25u64));

        let error = inventory
            .reserve(InventoryReservationRequest {
                route_id: H256::from_low_u64_be(10),
                lane_id: H256::from_low_u64_be(11),
                source_chain: 10,
                source_asset: asset,
                source_amount: U256::from(50u64),
                destination_chain: 56,
                destination_asset: H160::repeat_byte(0x44),
                destination_amount: U256::from(49u64),
            })
            .unwrap_err();

        match error {
            PositionManagerError::InsufficientInventory(message) => {
                assert!(message.contains("requested=50"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_inventory_pending_transitions() {
        let source_asset = H160::repeat_byte(0x55);
        let destination_asset = H160::repeat_byte(0x66);
        let route_id = H256::from_low_u64_be(12);
        let mut inventory = InventoryManager::new();
        inventory.set_available_balance(1, source_asset, U256::from(500u64));

        inventory
            .reserve(InventoryReservationRequest {
                route_id,
                lane_id: H256::from_low_u64_be(13),
                source_chain: 1,
                source_asset,
                source_amount: U256::from(200u64),
                destination_chain: 8453,
                destination_asset,
                destination_amount: U256::from(198u64),
            })
            .unwrap();

        inventory.mark_pending_out(&route_id).unwrap();
        let source_balance = inventory.balance(1, source_asset).unwrap();
        assert_eq!(source_balance.reserved, U256::zero());
        assert_eq!(source_balance.pending_out, U256::from(200u64));

        inventory.mark_pending_in(&route_id).unwrap();
        let source_balance = inventory.balance(1, source_asset).unwrap();
        let destination_balance = inventory.balance(8453, destination_asset).unwrap();
        assert_eq!(source_balance.pending_out, U256::zero());
        assert_eq!(destination_balance.pending_in, U256::from(198u64));

        inventory.settle_inbound(&route_id).unwrap();
        let destination_balance = inventory.balance(8453, destination_asset).unwrap();
        assert_eq!(destination_balance.pending_in, U256::zero());
        assert_eq!(destination_balance.available, U256::from(198u64));
        assert_eq!(
            inventory.obligation(&route_id).unwrap().status,
            ObligationStatus::Settled
        );
    }

    #[test]
    fn test_inventory_snapshot_surface() {
        let asset = H160::repeat_byte(0x77);
        let mut inventory = InventoryManager::new();
        inventory.set_available_balance(42161, asset, U256::from(123u64));

        let snapshot = inventory.snapshot();
        assert_eq!(snapshot.balances.len(), 1);
        assert_eq!(snapshot.balances[0].key.chain_id, 42161);
        assert_eq!(snapshot.balances[0].available, U256::from(123u64));
    }
}
