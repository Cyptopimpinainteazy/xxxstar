//! Route optimization system for cross-chain operations
//!
//! This module provides:
//! - Multi-hop route finding
//! - Gas cost optimization
//! - Slippage-aware routing
//! - Fallback route system
//! - Route simulation

use crate::{PositionManagerConfig, PositionManagerError, Result};
use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

static RESERVATION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryBand {
    pub critical_min: U256,
    pub min: U256,
    pub target: U256,
    pub max: U256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneClass {
    MarketOnly,
    PartnerBacked,
    ProtocolBacked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdTier {
    Healthy,
    Guarded,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaneStatus {
    Active,
    Warning,
    Frozen,
}

impl LaneStatus {
    pub fn allows_firm_execution(self) -> bool {
        !matches!(self, Self::Frozen)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiquiditySourceType {
    ExternalMarket,
    PartnerMm,
    Treasury,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReservationStatus {
    Active,
    Released,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteFirmness {
    Indicative,
    Firm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanePolicy {
    pub lane_id: H256,
    pub source_chain: u64,
    pub target_chain: u64,
    pub source_asset: H160,
    pub target_asset: H160,
    pub lane_class: LaneClass,
    pub status: LaneStatus,
    pub allowed_liquidity_sources: Vec<LiquiditySourceType>,
    pub inventory_band: InventoryBand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReservationRecord {
    pub reservation_id: H256,
    pub route_id: H256,
    pub lane_id: H256,
    pub liquidity_source: LiquiditySourceType,
    pub source_chain: u64,
    pub target_chain: u64,
    pub source_asset: H160,
    pub target_asset: H160,
    pub source_amount: U256,
    pub target_amount: U256,
    pub created_at_ms: u64,
    pub expiry_ts_ms: u64,
    pub status: ReservationStatus,
    pub max_fee_envelope: U256,
    pub solvency_snapshot: H256,
}

impl ReservationRecord {
    pub fn is_active_at(&self, now_ms: u64) -> bool {
        self.status == ReservationStatus::Active && now_ms <= self.expiry_ts_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteExecutionCandidate {
    pub route_id: H256,
    pub route: SwapRoute,
    pub firmness: RouteFirmness,
    pub lane_status: LaneStatus,
    pub reservation: Option<ReservationRecord>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteOptimizationParams {
    pub max_hops: u8,
    pub min_liquidity: U256,
    pub gas_weight: f64,
    pub time_weight: f64,
    pub slippage_weight: f64,
    pub preferred_chains: Vec<u64>,
    pub avoid_chains: Vec<u64>,
}

impl Default for RouteOptimizationParams {
    fn default() -> Self {
        Self {
            max_hops: 2,
            min_liquidity: U256::zero(),
            gas_weight: 0.3,
            time_weight: 0.2,
            slippage_weight: 0.5,
            preferred_chains: Vec::new(),
            avoid_chains: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwapRoute {
    pub source_chain: u64,
    pub target_chain: u64,
    pub source_asset: H160,
    pub target_asset: H160,
    pub amount_in: U256,
    pub amount_out: U256,
    pub hops: Vec<u64>,
    pub gas_estimate: U256,
    pub price_impact_bps: u32,
}

/// Route optimizer for finding optimal paths
#[derive(Debug, Clone)]
pub struct RouteOptimizer {
    /// Supported chains
    supported_chains: Vec<u64>,
    /// DEX routers per chain
    dex_routers: sp_std::collections::btree_map::BTreeMap<u64, Vec<DexRouter>>,
    /// Bridge contracts
    bridge_contracts: sp_std::collections::btree_map::BTreeMap<(u64, u64), BridgeContract>,
    /// Route cache
    route_cache: sp_std::collections::btree_map::BTreeMap<RouteKey, CachedRoute>,
    /// Lane policy registry for firm-route eligibility.
    lane_policies: sp_std::collections::btree_map::BTreeMap<H256, LanePolicy>,
    /// Active and historical reservations.
    reservations: sp_std::collections::btree_map::BTreeMap<H256, ReservationRecord>,
    /// Configuration
    config: PositionManagerConfig,
}

impl RouteOptimizer {
    /// Create a new route optimizer
    pub fn new(config: &PositionManagerConfig) -> Result<Self> {
        Ok(Self {
            supported_chains: config.chain_configs.keys().cloned().collect(),
            dex_routers: sp_std::collections::btree_map::BTreeMap::new(),
            bridge_contracts: sp_std::collections::btree_map::BTreeMap::new(),
            route_cache: sp_std::collections::btree_map::BTreeMap::new(),
            lane_policies: sp_std::collections::btree_map::BTreeMap::new(),
            reservations: sp_std::collections::btree_map::BTreeMap::new(),
            config: config.clone(),
        })
    }

    /// Register or replace a lane policy.
    pub fn upsert_lane_policy(&mut self, policy: LanePolicy) {
        self.lane_policies.insert(policy.lane_id, policy);
    }

    /// Build an execution candidate that is either indicative or firm.
    pub fn build_execution_candidate(
        &mut self,
        route: SwapRoute,
        source_asset: H160,
        target_asset: H160,
        reservation_ttl_ms: u64,
    ) -> Result<RouteExecutionCandidate> {
        let route_id = self.compute_route_id(&route, source_asset, target_asset);
        let lane_id = self.compute_lane_id(
            route.source_chain,
            route.target_chain,
            source_asset,
            target_asset,
        );
        let lane_policy = self.lane_policies.get(&lane_id).cloned();

        let Some(policy) = lane_policy else {
            return Ok(RouteExecutionCandidate {
                route_id,
                route,
                firmness: RouteFirmness::Indicative,
                lane_status: LaneStatus::Warning,
                reservation: None,
                reason: Some("lane policy missing".to_string()),
            });
        };

        if !policy.status.allows_firm_execution() {
            return Ok(RouteExecutionCandidate {
                route_id,
                route,
                firmness: RouteFirmness::Indicative,
                lane_status: policy.status,
                reservation: None,
                reason: Some("lane is frozen for firm execution".to_string()),
            });
        }

        let reservation = self.create_reservation(&route, &policy, route_id, reservation_ttl_ms)?;

        Ok(RouteExecutionCandidate {
            route_id,
            route,
            firmness: RouteFirmness::Firm,
            lane_status: policy.status,
            reservation: Some(reservation),
            reason: None,
        })
    }

    /// Release an existing reservation.
    pub fn release_reservation(&mut self, reservation_id: &H256) -> Result<()> {
        let reservation = self.reservations.get_mut(reservation_id).ok_or_else(|| {
            PositionManagerError::ReservationNotFound(hex::encode(reservation_id))
        })?;
        reservation.status = ReservationStatus::Released;
        Ok(())
    }

    /// Expire an existing reservation.
    pub fn expire_reservation(&mut self, reservation_id: &H256) -> Result<()> {
        let reservation = self.reservations.get_mut(reservation_id).ok_or_else(|| {
            PositionManagerError::ReservationNotFound(hex::encode(reservation_id))
        })?;
        reservation.status = ReservationStatus::Expired;
        Ok(())
    }

    /// Read an existing reservation.
    pub fn reservation(&self, reservation_id: &H256) -> Option<&ReservationRecord> {
        self.reservations.get(reservation_id)
    }

    /// Find optimal route between chains
    pub async fn find_optimal_route(
        &self,
        source_chain: u64,
        target_chain: u64,
        source_asset: H160,
        target_asset: H160,
        amount: U256,
        params: &RouteOptimizationParams,
    ) -> Result<SwapRoute> {
        // Check cache first
        let route_key = RouteKey {
            source_chain,
            target_chain,
            source_asset,
            target_asset,
            amount,
        };

        if let Some(cached) = self.route_cache.get(&route_key) {
            if !cached.is_expired() {
                return Ok(cached.route.clone());
            }
        }

        // Find all possible routes
        let routes = self
            .find_all_routes(
                source_chain,
                target_chain,
                source_asset,
                target_asset,
                amount,
                params,
            )
            .await?;

        // Select optimal route based on parameters
        let optimal_route = self.select_optimal_route(&routes, params)?;

        // Cache the result
        // Note: In a real implementation, we'd use a mutable cache
        // For now, we just return the route

        Ok(optimal_route)
    }

    /// Find all possible routes
    async fn find_all_routes(
        &self,
        source_chain: u64,
        target_chain: u64,
        source_asset: H160,
        target_asset: H160,
        amount: U256,
        params: &RouteOptimizationParams,
    ) -> Result<Vec<SwapRoute>> {
        let mut routes = Vec::new();

        // Direct route (if same chain)
        if source_chain == target_chain {
            let direct_route = self
                .build_direct_route(source_chain, source_asset, target_asset, amount)
                .await?;
            routes.push(direct_route);
        }

        // Single-hop bridge route
        if source_chain != target_chain {
            let bridge_route = self
                .build_bridge_route(
                    source_chain,
                    target_chain,
                    source_asset,
                    target_asset,
                    amount,
                )
                .await?;
            routes.push(bridge_route);
        }

        // Multi-hop routes (through intermediate chains)
        if params.max_hops > 1 {
            let multi_hop_routes = self
                .find_multi_hop_routes(
                    source_chain,
                    target_chain,
                    source_asset,
                    target_asset,
                    amount,
                    params,
                )
                .await?;
            routes.extend(multi_hop_routes);
        }

        // Filter routes based on preferences
        routes = self.filter_routes(routes, params)?;

        Ok(routes)
    }

    /// Build direct swap route
    async fn build_direct_route(
        &self,
        chain_id: u64,
        source_asset: H160,
        target_asset: H160,
        amount: U256,
    ) -> Result<SwapRoute> {
        let dex_router = self.get_dex_router(chain_id)?;

        // Estimate output amount
        let amount_out = self
            .estimate_swap_output(chain_id, source_asset, target_asset, amount)
            .await?;

        // Estimate gas
        let gas_estimate = self
            .estimate_swap_gas(chain_id, source_asset, target_asset, amount)
            .await?;

        Ok(SwapRoute {
            source_chain: chain_id,
            target_chain: chain_id,
            source_asset,
            target_asset,
            amount_in: amount,
            amount_out,
            hops: vec![chain_id],
            gas_estimate,
            price_impact_bps: self.calculate_price_impact_bps(amount, amount_out)?,
        })
    }

    /// Build bridge route
    async fn build_bridge_route(
        &self,
        source_chain: u64,
        target_chain: u64,
        source_asset: H160,
        target_asset: H160,
        amount: U256,
    ) -> Result<SwapRoute> {
        // Get bridge contract
        let bridge = self.get_bridge_contract(source_chain, target_chain)?;

        // Estimate bridge fee
        let bridge_fee = self
            .estimate_bridge_fee(source_chain, target_chain, amount)
            .await?;

        // Estimate output (after bridge fee)
        let amount_out = amount.checked_sub(bridge_fee).unwrap_or(U256::zero());

        // Estimate gas (includes bridge gas)
        let gas_estimate = self.estimate_bridge_gas(source_chain, target_chain).await?;

        Ok(SwapRoute {
            source_chain,
            target_chain,
            source_asset,
            target_asset,
            amount_in: amount,
            amount_out,
            hops: vec![source_chain, target_chain],
            gas_estimate,
            price_impact_bps: self.calculate_price_impact_bps(amount, amount_out)?,
        })
    }

    /// Find multi-hop routes
    async fn find_multi_hop_routes(
        &self,
        source_chain: u64,
        target_chain: u64,
        source_asset: H160,
        target_asset: H160,
        amount: U256,
        params: &RouteOptimizationParams,
    ) -> Result<Vec<SwapRoute>> {
        let mut routes = Vec::new();

        // Find intermediate chains
        let intermediate_chains =
            self.find_intermediate_chains(source_chain, target_chain, params)?;

        for intermediate in intermediate_chains {
            // Skip if in avoid list
            if params.avoid_chains.contains(&intermediate) {
                continue;
            }

            // Build route through intermediate
            let route = self
                .build_multi_hop_route(
                    source_chain,
                    intermediate,
                    target_chain,
                    source_asset,
                    target_asset,
                    amount,
                )
                .await?;

            routes.push(route);
        }

        Ok(routes)
    }

    /// Find intermediate chains for multi-hop routing
    fn find_intermediate_chains(
        &self,
        source_chain: u64,
        target_chain: u64,
        params: &RouteOptimizationParams,
    ) -> Result<Vec<u64>> {
        let mut intermediates = Vec::new();

        for &chain in &self.supported_chains {
            if chain == source_chain || chain == target_chain {
                continue;
            }

            // Check if bridge exists
            if self.has_bridge(source_chain, chain) && self.has_bridge(chain, target_chain) {
                // Check if preferred
                if params.preferred_chains.is_empty() || params.preferred_chains.contains(&chain) {
                    intermediates.push(chain);
                }
            }
        }

        Ok(intermediates)
    }

    /// Build multi-hop route
    async fn build_multi_hop_route(
        &self,
        source_chain: u64,
        intermediate_chain: u64,
        target_chain: u64,
        source_asset: H160,
        target_asset: H160,
        amount: U256,
    ) -> Result<SwapRoute> {
        // First hop: source -> intermediate
        let first_hop_fee = self
            .estimate_bridge_fee(source_chain, intermediate_chain, amount)
            .await?;
        let amount_after_first = amount.checked_sub(first_hop_fee).unwrap_or(U256::zero());

        // Second hop: intermediate -> target
        let second_hop_fee = self
            .estimate_bridge_fee(intermediate_chain, target_chain, amount_after_first)
            .await?;
        let amount_out = amount_after_first
            .checked_sub(second_hop_fee)
            .unwrap_or(U256::zero());

        // Total gas
        let gas_estimate = self
            .estimate_bridge_gas(source_chain, intermediate_chain)
            .await?
            .checked_add(
                self.estimate_bridge_gas(intermediate_chain, target_chain)
                    .await?,
            )
            .unwrap_or(U256::zero());

        Ok(SwapRoute {
            source_chain,
            target_chain,
            source_asset,
            target_asset,
            amount_in: amount,
            amount_out,
            hops: vec![source_chain, intermediate_chain, target_chain],
            gas_estimate,
            price_impact_bps: self.calculate_price_impact_bps(amount, amount_out)?,
        })
    }

    /// Filter routes based on parameters
    fn filter_routes(
        &self,
        routes: Vec<SwapRoute>,
        params: &RouteOptimizationParams,
    ) -> Result<Vec<SwapRoute>> {
        let mut filtered = Vec::new();

        for route in routes {
            // Check minimum liquidity
            if route.amount_out < params.min_liquidity {
                continue;
            }

            // Check if chains are in avoid list
            if route
                .hops
                .iter()
                .any(|chain| params.avoid_chains.contains(chain))
            {
                continue;
            }

            filtered.push(route);
        }

        Ok(filtered)
    }

    /// Select optimal route from candidates
    fn select_optimal_route(
        &self,
        routes: &[SwapRoute],
        params: &RouteOptimizationParams,
    ) -> Result<SwapRoute> {
        if routes.is_empty() {
            return Err(PositionManagerError::NoRoutesFound);
        }

        // Score routes based on parameters
        let mut best_route = routes[0].clone();
        let mut best_score = self.calculate_route_score(&best_route, params)?;

        for route in routes.iter().skip(1) {
            let score = self.calculate_route_score(route, params)?;
            if score > best_score {
                best_score = score;
                best_route = route.clone();
            }
        }

        Ok(best_route)
    }

    /// Calculate route score
    fn calculate_route_score(
        &self,
        route: &SwapRoute,
        params: &RouteOptimizationParams,
    ) -> Result<f64> {
        let mut score = 0.0;

        // Output amount score (higher is better)
        let output_score = route.amount_out.as_u128() as f64;
        score += output_score * params.slippage_weight;

        // Gas cost score (lower is better)
        let gas_score = 1.0 / (route.gas_estimate.as_u128() as f64 + 1.0);
        score += gas_score * params.gas_weight;

        // Hop count score (fewer is better)
        let hop_score = 1.0 / (route.hops.len() as f64);
        score += hop_score * params.time_weight;

        Ok(score)
    }

    /// Estimate swap output
    async fn estimate_swap_output(
        &self,
        chain_id: u64,
        source_asset: H160,
        target_asset: H160,
        amount: U256,
    ) -> Result<U256> {
        // Placeholder - would query DEX for actual output
        Ok(amount) // 1:1 for now
    }

    /// Estimate swap gas
    async fn estimate_swap_gas(
        &self,
        chain_id: u64,
        source_asset: H160,
        target_asset: H160,
        amount: U256,
    ) -> Result<U256> {
        let chain_config = self
            .config
            .chain_configs
            .get(&chain_id)
            .ok_or(PositionManagerError::ChainNotFound(chain_id))?;

        Ok(U256::from(150_000)) // Base swap gas
    }

    /// Estimate bridge fee
    async fn estimate_bridge_fee(
        &self,
        source_chain: u64,
        target_chain: u64,
        amount: U256,
    ) -> Result<U256> {
        // Base fee + percentage
        let base_fee = U256::from(1_000_000_000_000_000u128); // 0.001 ETH
        let percentage_fee = amount
            .checked_mul(U256::from(30)) // 0.3%
            .unwrap_or(U256::zero())
            .checked_div(U256::from(10000))
            .unwrap_or(U256::zero());

        Ok(base_fee.saturating_add(percentage_fee))
    }

    /// Estimate bridge gas
    async fn estimate_bridge_gas(&self, source_chain: u64, target_chain: u64) -> Result<U256> {
        Ok(U256::from(200_000)) // Base bridge gas
    }

    /// Calculate price impact
    fn calculate_price_impact_bps(&self, amount_in: U256, amount_out: U256) -> Result<u32> {
        if amount_in.is_zero() {
            return Ok(0);
        }

        let diff = if amount_in > amount_out {
            amount_in - amount_out
        } else {
            amount_out - amount_in
        };

        let impact_bps = diff
            .checked_mul(U256::from(10_000u64))
            .ok_or(PositionManagerError::ArithmeticOverflow)?
            .checked_div(amount_in)
            .ok_or(PositionManagerError::ArithmeticOverflow)?;
        Ok(impact_bps.as_u32())
    }

    /// Get DEX router for a chain
    fn get_dex_router(&self, chain_id: u64) -> Result<&DexRouter> {
        self.dex_routers
            .get(&chain_id)
            .and_then(|routers| routers.first())
            .ok_or(PositionManagerError::DexRouterNotFound(chain_id))
    }

    /// Get bridge contract between chains
    fn get_bridge_contract(&self, source_chain: u64, target_chain: u64) -> Result<&BridgeContract> {
        self.bridge_contracts
            .get(&(source_chain, target_chain))
            .ok_or(PositionManagerError::BridgeNotFound(
                source_chain,
                target_chain,
            ))
    }

    /// Check if bridge exists between chains
    fn has_bridge(&self, source_chain: u64, target_chain: u64) -> bool {
        self.bridge_contracts
            .contains_key(&(source_chain, target_chain))
    }

    /// Add DEX router
    pub fn add_dex_router(&mut self, chain_id: u64, router: DexRouter) {
        self.dex_routers.entry(chain_id).or_default().push(router);
    }

    /// Add bridge contract
    pub fn add_bridge_contract(
        &mut self,
        source_chain: u64,
        target_chain: u64,
        bridge: BridgeContract,
    ) {
        self.bridge_contracts
            .insert((source_chain, target_chain), bridge);
    }

    fn create_reservation(
        &mut self,
        route: &SwapRoute,
        policy: &LanePolicy,
        route_id: H256,
        reservation_ttl_ms: u64,
    ) -> Result<ReservationRecord> {
        let created_at_ms = current_time_ms();
        let reservation_id =
            H256::from_low_u64_be(RESERVATION_COUNTER.fetch_add(1, Ordering::Relaxed));
        let reservation = ReservationRecord {
            reservation_id,
            route_id,
            lane_id: policy.lane_id,
            liquidity_source: policy
                .allowed_liquidity_sources
                .first()
                .copied()
                .unwrap_or(LiquiditySourceType::ExternalMarket),
            source_chain: route.source_chain,
            target_chain: route.target_chain,
            source_asset: policy.source_asset,
            target_asset: policy.target_asset,
            source_amount: route.amount_in,
            target_amount: route.amount_out,
            created_at_ms,
            expiry_ts_ms: created_at_ms.saturating_add(reservation_ttl_ms),
            status: ReservationStatus::Active,
            max_fee_envelope: route.gas_estimate,
            solvency_snapshot: route_id,
        };

        self.reservations
            .insert(reservation_id, reservation.clone());
        Ok(reservation)
    }

    fn compute_lane_id(
        &self,
        source_chain: u64,
        target_chain: u64,
        source_asset: H160,
        target_asset: H160,
    ) -> H256 {
        let counter = source_chain ^ target_chain;
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&source_chain.to_le_bytes());
        bytes[8..16].copy_from_slice(&target_chain.to_le_bytes());
        bytes[16..20].copy_from_slice(&source_asset.as_fixed_bytes()[0..4]);
        bytes[20..24].copy_from_slice(&target_asset.as_fixed_bytes()[0..4]);
        bytes[24..32].copy_from_slice(&counter.to_le_bytes());
        H256::from(bytes)
    }

    fn compute_route_id(&self, route: &SwapRoute, source_asset: H160, target_asset: H160) -> H256 {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&route.source_chain.to_le_bytes());
        bytes[8..16].copy_from_slice(&route.target_chain.to_le_bytes());
        bytes[16..20].copy_from_slice(&source_asset.as_fixed_bytes()[0..4]);
        bytes[20..24].copy_from_slice(&target_asset.as_fixed_bytes()[0..4]);
        bytes[24..32].copy_from_slice(&route.amount_in.low_u64().to_le_bytes());
        H256::from(bytes)
    }

    /// Simulate route execution
    pub async fn simulate_route(&self, route: &SwapRoute) -> Result<SimulationResult> {
        // Check liquidity
        let liquidity_check = self.check_liquidity(route).await?;

        // Estimate actual output
        let actual_output = self.estimate_actual_output(route).await?;

        // Calculate actual price impact
        let actual_impact = self.calculate_price_impact_bps(route.amount_in, actual_output)?;

        Ok(SimulationResult {
            feasible: liquidity_check,
            estimated_output: actual_output,
            actual_price_impact_bps: actual_impact,
            gas_used: route.gas_estimate,
            warnings: if !liquidity_check {
                vec!["Insufficient liquidity".to_string()]
            } else {
                Vec::new()
            },
        })
    }

    /// Check route liquidity
    async fn check_liquidity(&self, route: &SwapRoute) -> Result<bool> {
        // Placeholder - would check actual DEX liquidity
        Ok(true)
    }

    /// Estimate actual output
    async fn estimate_actual_output(&self, route: &SwapRoute) -> Result<U256> {
        // Placeholder - would get actual quote from DEX
        Ok(route.amount_out)
    }
}

fn current_time_ms() -> u64 {
    #[cfg(test)]
    {
        1_000
    }

    #[cfg(not(test))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// DEX router information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexRouter {
    pub chain_id: u64,
    pub router_address: H160,
    pub name: String,
    pub version: String,
}

/// Bridge contract information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeContract {
    pub source_chain: u64,
    pub target_chain: u64,
    pub contract_address: H160,
    pub name: String,
    pub fee_percentage: f64,
}

/// Route cache key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RouteKey {
    pub source_chain: u64,
    pub target_chain: u64,
    pub source_asset: H160,
    pub target_asset: H160,
    pub amount: U256,
}

/// Cached route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRoute {
    pub route: SwapRoute,
    pub timestamp: u64,
    pub ttl_ms: u64,
}

impl CachedRoute {
    /// Check if cache entry is expired
    pub fn is_expired(&self) -> bool {
        let now = current_time_ms();
        now > self.timestamp + self.ttl_ms
    }
}

/// Simulation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub feasible: bool,
    pub estimated_output: U256,
    pub actual_price_impact_bps: u32,
    pub gas_used: U256,
    pub warnings: Vec<String>,
}

/// Execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub route: SwapRoute,
    pub steps: Vec<ExecutionStep>,
    pub total_gas: U256,
    pub estimated_time_ms: u64,
}

/// Execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_type: StepType,
    pub chain_id: u64,
    pub contract: H160,
    pub data: Vec<u8>,
    pub value: U256,
    pub gas_estimate: U256,
}

/// Step types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepType {
    Approve,
    Swap,
    Bridge,
    Claim,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lane_id() -> H256 {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&1u64.to_le_bytes());
        bytes[8..16].copy_from_slice(&137u64.to_le_bytes());
        bytes[16..20].copy_from_slice(&H160::repeat_byte(0x11).as_fixed_bytes()[0..4]);
        bytes[20..24].copy_from_slice(&H160::repeat_byte(0x22).as_fixed_bytes()[0..4]);
        bytes[24..32].copy_from_slice(&(1u64 ^ 137u64).to_le_bytes());
        H256::from(bytes)
    }

    fn sample_policy(status: LaneStatus) -> LanePolicy {
        LanePolicy {
            lane_id: sample_lane_id(),
            source_chain: 1,
            target_chain: 137,
            source_asset: H160::repeat_byte(0x11),
            target_asset: H160::repeat_byte(0x22),
            lane_class: LaneClass::MarketOnly,
            status,
            allowed_liquidity_sources: vec![LiquiditySourceType::ExternalMarket],
            inventory_band: InventoryBand {
                critical_min: U256::from(10u64),
                min: U256::from(20u64),
                target: U256::from(100u64),
                max: U256::from(200u64),
            },
        }
    }

    fn sample_route() -> SwapRoute {
        SwapRoute {
            source_chain: 1,
            target_chain: 137,
            source_asset: H160::repeat_byte(0x11),
            target_asset: H160::repeat_byte(0x22),
            amount_in: U256::from(1000u64),
            amount_out: U256::from(995u64),
            hops: vec![1, 137],
            gas_estimate: U256::from(100_000u64),
            price_impact_bps: 50,
        }
    }

    #[test]
    fn test_route_optimizer() {
        let config = PositionManagerConfig::default();
        let optimizer = RouteOptimizer::new(&config).unwrap();

        assert_eq!(optimizer.supported_chains.len(), config.chain_configs.len());
    }

    #[test]
    fn test_cached_route_expiry() {
        let route = SwapRoute {
            source_chain: 1,
            target_chain: 137,
            source_asset: H160::zero(),
            target_asset: H160::zero(),
            amount_in: U256::from(1000),
            amount_out: U256::from(1000),
            hops: vec![1, 137],
            gas_estimate: U256::from(100_000),
            price_impact_bps: 10,
        };

        let cached = CachedRoute {
            route,
            timestamp: 0,
            ttl_ms: 600,
        };

        assert!(cached.is_expired());
    }

    #[test]
    fn test_frozen_lane_stays_indicative() {
        let config = PositionManagerConfig::default();
        let mut optimizer = RouteOptimizer::new(&config).unwrap();
        optimizer.upsert_lane_policy(sample_policy(LaneStatus::Frozen));

        let candidate = optimizer
            .build_execution_candidate(
                sample_route(),
                H160::repeat_byte(0x11),
                H160::repeat_byte(0x22),
                5_000,
            )
            .unwrap();

        assert_eq!(candidate.firmness, RouteFirmness::Indicative);
        assert!(candidate.reservation.is_none());
        assert_eq!(candidate.lane_status, LaneStatus::Frozen);
    }

    #[test]
    fn test_active_lane_gets_reservation() {
        let config = PositionManagerConfig::default();
        let mut optimizer = RouteOptimizer::new(&config).unwrap();
        optimizer.upsert_lane_policy(sample_policy(LaneStatus::Active));

        let candidate = optimizer
            .build_execution_candidate(
                sample_route(),
                H160::repeat_byte(0x11),
                H160::repeat_byte(0x22),
                5_000,
            )
            .unwrap();

        assert_eq!(candidate.firmness, RouteFirmness::Firm);
        let reservation = candidate.reservation.expect("reservation expected");
        assert_eq!(reservation.status, ReservationStatus::Active);
        assert!(reservation.is_active_at(2_000));
        assert!(optimizer.reservation(&reservation.reservation_id).is_some());
    }

    #[test]
    fn test_release_and_expire_reservation() {
        let config = PositionManagerConfig::default();
        let mut optimizer = RouteOptimizer::new(&config).unwrap();
        optimizer.upsert_lane_policy(sample_policy(LaneStatus::Active));

        let candidate = optimizer
            .build_execution_candidate(
                sample_route(),
                H160::repeat_byte(0x11),
                H160::repeat_byte(0x22),
                10,
            )
            .unwrap();

        let reservation_id = candidate.reservation.unwrap().reservation_id;
        optimizer.release_reservation(&reservation_id).unwrap();
        assert_eq!(
            optimizer.reservation(&reservation_id).unwrap().status,
            ReservationStatus::Released
        );

        optimizer.expire_reservation(&reservation_id).unwrap();
        assert_eq!(
            optimizer.reservation(&reservation_id).unwrap().status,
            ReservationStatus::Expired
        );
    }
}
