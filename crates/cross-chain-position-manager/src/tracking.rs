//! Position tracking and monitoring system
//!
//! This module provides comprehensive tracking capabilities for cross-chain positions:
//! - Real-time balance monitoring across all chains
//! - Position state tracking and updates
//! - Performance metrics collection
//! - Event-driven updates
//! - Fast state diffing for efficient updates

use crate::accounting::AccountingEngine;
use crate::adapters::UniversalChainAdapter;
use crate::error::{PositionManagerError, Result};
use crate::events::{ChainEvent, Event, EventBus, PositionEvent};
use crate::state::PositionStateManager;
use crate::types::*;
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;
use std::collections::HashMap;

/// Position tracker for monitoring cross-chain positions
#[derive(Debug, Clone)]
pub struct PositionTracker {
    /// Position state manager
    state_manager: PositionStateManager,
    /// Event bus for notifications
    event_bus: EventBus,
    /// Chain adapter for external chains
    chain_adapters: UniversalChainAdapter,
    /// Tracking configuration
    config: PositionTrackerConfig,
    /// Active tracking sessions
    tracking_sessions: HashMap<PositionId, TrackingSession>,
    /// Last update timestamps
    last_updates: HashMap<PositionId, u64>,
}

/// Configuration for position tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionTrackerConfig {
    /// Update interval in milliseconds
    pub update_interval_ms: u64,
    /// Maximum positions to track concurrently
    pub max_concurrent_positions: usize,
    /// Enable real-time updates
    pub real_time_updates: bool,
    /// Batch update size
    pub batch_size: usize,
    /// Enable performance metrics collection
    pub collect_metrics: bool,
    /// Enable event notifications
    pub enable_events: bool,
}

impl Default for PositionTrackerConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 5000, // 5 seconds
            max_concurrent_positions: 1000,
            real_time_updates: true,
            batch_size: 50,
            collect_metrics: true,
            enable_events: true,
        }
    }
}

/// Active tracking session for a position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingSession {
    pub position_id: PositionId,
    pub chain_id: u64,
    pub asset: H160,
    pub start_time: u64,
    pub last_update: u64,
    pub update_count: u64,
    pub status: TrackingStatus,
    pub metrics: PositionMetrics,
}

/// Tracking status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackingStatus {
    Active,
    Paused,
    Error,
    Completed,
}

/// Position metrics for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionMetrics {
    pub total_updates: u64,
    pub last_balance: U256,
    pub balance_changes: Vec<BalanceChange>,
    pub performance_samples: Vec<PerformanceSample>,
    pub error_count: u64,
    pub uptime_percentage: f64,
}

/// Balance change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceChange {
    pub timestamp: u64,
    pub old_balance: U256,
    pub new_balance: U256,
    pub change_amount: U256,
    pub change_percentage: f64,
}

/// Performance sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSample {
    pub timestamp: u64,
    pub balance_usd: U256,
    pub pnl: U256,
    pub roi: f64,
}

impl PositionTracker {
    /// Create a new position tracker
    pub fn new(config: &PositionTrackerConfig) -> Result<Self> {
        let state_manager = PositionStateManager::new(config)?;
        let event_bus = EventBus::new();
        let chain_adapters = UniversalChainAdapter::new(config)?;

        Ok(Self {
            state_manager,
            event_bus,
            chain_adapters,
            config: config.clone(),
            tracking_sessions: HashMap::new(),
            last_updates: HashMap::new(),
        })
    }

    /// Start tracking all positions across all chains
    pub async fn start(&mut self) -> Result<()> {
        log::info!(
            "Starting position tracker with {}ms interval",
            self.config.update_interval_ms
        );

        // Start background tracking task
        if self.config.real_time_updates {
            self.start_background_tracking().await?;
        }

        Ok(())
    }

    /// Stop tracking
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping position tracker");

        // Stop background tasks
        for session in self.tracking_sessions.values_mut() {
            session.status = TrackingStatus::Completed;
        }

        Ok(())
    }

    /// Track positions across all connected chains
    pub async fn track_all_positions(&self) -> Result<Vec<CrossChainPosition>> {
        log::info!("Tracking positions across all chains");

        let mut all_positions = Vec::new();

        // Get all connected chains
        let chains = self.chain_adapters.get_supported_chains();

        for chain_id in chains {
            match self.track_chain_positions(chain_id).await {
                Ok(positions) => {
                    all_positions.extend(positions);
                }
                Err(e) => {
                    log::warn!("Failed to track positions on chain {}: {}", chain_id, e);
                    if self.config.enable_events {
                        self.event_bus.publish(Event::ChainEvent(ChainEvent {
                            chain_id,
                            event_type: "tracking_failed".to_string(),
                            details: format!("Failed to track positions: {}", e),
                            timestamp: sp_io::offchain::timestamp().unix_millis(),
                        }));
                    }
                }
            }
        }

        // Update state manager
        self.state_manager.update_positions(&all_positions).await?;

        // Publish position update event
        if self.config.enable_events {
            self.event_bus.publish(Event::PositionEvent(PositionEvent {
                event_type: "positions_updated".to_string(),
                details: format!(
                    "Tracked {} positions across {} chains",
                    all_positions.len(),
                    chains.len()
                ),
                timestamp: sp_io::offchain::timestamp().unix_millis(),
            }));
        }

        Ok(all_positions)
    }

    /// Track positions on a specific chain
    async fn track_chain_positions(&self, chain_id: u64) -> Result<Vec<CrossChainPosition>> {
        let mut positions = Vec::new();

        // Get chain adapter
        let adapter = self
            .chain_adapters
            .get_adapter(chain_id)
            .ok_or(PositionManagerError::ChainNotFound(chain_id))?;

        // Get all tracked assets on this chain
        let assets = self.get_tracked_assets(chain_id).await?;

        for asset in assets {
            // Get balance for this asset
            let balance = adapter.get_balance(asset).await?;

            // Create position if balance > 0
            if balance > U256::zero() {
                let position = self
                    .create_position_from_balance(chain_id, asset, balance)
                    .await?;
                positions.push(position);
            }
        }

        log::debug!("Found {} positions on chain {}", positions.len(), chain_id);
        Ok(positions)
    }

    /// Get tracked assets for a chain
    async fn get_tracked_assets(&self, chain_id: u64) -> Result<Vec<H160>> {
        // In a real implementation, this would query the state manager
        // or configuration for assets to track on this chain
        // For now, return some common assets
        Ok(vec![
            H160::from_low_u64_be(0), // Native token
            H160::from_low_u64_be(1), // USDC
            H160::from_low_u64_be(2), // USDT
        ])
    }

    /// Create position from balance information
    async fn create_position_from_balance(
        &self,
        chain_id: u64,
        asset: H160,
        balance: U256,
    ) -> Result<CrossChainPosition> {
        // Get asset info
        let asset_info = self.get_asset_info(chain_id, asset).await?;

        // Create position
        let mut position =
            CrossChainPosition::new(PositionType::Token, chain_id, asset_info, balance);

        // Update USD value
        let usd_value = self.get_usd_value(chain_id, asset, balance).await?;
        position.chain_holdings[0].balance_usd = usd_value;

        // Update tracking session
        let session = TrackingSession {
            position_id: position.id.clone(),
            chain_id,
            asset,
            start_time: sp_io::offchain::timestamp().unix_millis(),
            last_update: sp_io::offchain::timestamp().unix_millis(),
            update_count: 1,
            status: TrackingStatus::Active,
            metrics: PositionMetrics {
                total_updates: 1,
                last_balance: balance,
                balance_changes: Vec::new(),
                performance_samples: Vec::new(),
                error_count: 0,
                uptime_percentage: 100.0,
            },
        };

        self.tracking_sessions.insert(position.id.clone(), session);
        self.last_updates.insert(
            position.id.clone(),
            sp_io::offchain::timestamp().unix_millis(),
        );

        Ok(position)
    }

    /// Get asset information
    async fn get_asset_info(&self, chain_id: u64, asset: H160) -> Result<AssetInfo> {
        // In a real implementation, this would query the asset registry
        // For now, return a default asset info
        Ok(AssetInfo {
            address: asset,
            symbol: "TOKEN".to_string(),
            name: "Token".to_string(),
            decimals: 18,
            is_native: asset == H160::zero(),
            is_stable: false,
            price_source: PriceSource::None,
            coingecko_id: None,
        })
    }

    /// Get USD value of an asset
    async fn get_usd_value(&self, chain_id: u64, asset: H160, amount: U256) -> Result<U256> {
        // In a real implementation, this would query price oracles
        // For now, return a placeholder value
        Ok(amount) // 1:1 ratio for simplicity
    }

    /// Start background tracking task
    async fn start_background_tracking(&self) -> Result<()> {
        // In a real implementation, this would spawn a background task
        // that periodically calls track_all_positions()
        // For now, we'll just log that it would start
        log::info!(
            "Background tracking would start here (interval: {}ms)",
            self.config.update_interval_ms
        );
        Ok(())
    }

    /// Get tracking statistics
    pub fn get_tracking_stats(&self) -> TrackingStats {
        let active_sessions = self
            .tracking_sessions
            .values()
            .filter(|s| s.status == TrackingStatus::Active)
            .count();

        let total_updates = self
            .tracking_sessions
            .values()
            .map(|s| s.metrics.total_updates)
            .sum();

        let avg_uptime = if self.tracking_sessions.is_empty() {
            0.0
        } else {
            self.tracking_sessions
                .values()
                .map(|s| s.metrics.uptime_percentage)
                .sum::<f64>()
                / self.tracking_sessions.len() as f64
        };

        TrackingStats {
            active_sessions,
            total_positions: self.tracking_sessions.len(),
            total_updates,
            average_uptime: avg_uptime,
            error_count: self
                .tracking_sessions
                .values()
                .map(|s| s.metrics.error_count)
                .sum(),
        }
    }

    /// Update position balance
    pub async fn update_position_balance(
        &mut self,
        position_id: &PositionId,
        new_balance: U256,
    ) -> Result<()> {
        if let Some(session) = self.tracking_sessions.get_mut(position_id) {
            let old_balance = session.metrics.last_balance;
            let change = if new_balance > old_balance {
                new_balance - old_balance
            } else {
                old_balance - new_balance
            };

            let change_percentage = if old_balance > U256::zero() {
                (change * U256::from(100)) / old_balance
            } else {
                U256::zero()
            };

            // Record balance change
            session.metrics.balance_changes.push(BalanceChange {
                timestamp: sp_io::offchain::timestamp().unix_millis(),
                old_balance,
                new_balance,
                change_amount: change,
                change_percentage: change_percentage.as_u128() as f64 / 100.0,
            });

            // Update metrics
            session.metrics.last_balance = new_balance;
            session.metrics.total_updates += 1;
            session.last_update = sp_io::offchain::timestamp().unix_millis();

            // Keep only last 100 balance changes
            if session.metrics.balance_changes.len() > 100 {
                session.metrics.balance_changes.remove(0);
            }

            // Publish balance update event
            if self.config.enable_events {
                self.event_bus.publish(Event::PositionEvent(PositionEvent {
                    event_type: "balance_updated".to_string(),
                    details: format!(
                        "Position {} balance updated to {}",
                        position_id, new_balance
                    ),
                    timestamp: sp_io::offchain::timestamp().unix_millis(),
                }));
            }

            Ok(())
        } else {
            Err(PositionManagerError::PositionNotFound(position_id.clone()))
        }
    }

    /// Get position metrics
    pub fn get_position_metrics(&self, position_id: &PositionId) -> Option<&PositionMetrics> {
        self.tracking_sessions
            .get(position_id)
            .map(|session| &session.metrics)
    }

    /// Pause tracking for a position
    pub fn pause_tracking(&mut self, position_id: &PositionId) -> Result<()> {
        if let Some(session) = self.tracking_sessions.get_mut(position_id) {
            session.status = TrackingStatus::Paused;
            Ok(())
        } else {
            Err(PositionManagerError::PositionNotFound(position_id.clone()))
        }
    }

    /// Resume tracking for a position
    pub fn resume_tracking(&mut self, position_id: &PositionId) -> Result<()> {
        if let Some(session) = self.tracking_sessions.get_mut(position_id) {
            session.status = TrackingStatus::Active;
            session.last_update = sp_io::offchain::timestamp().unix_millis();
            Ok(())
        } else {
            Err(PositionManagerError::PositionNotFound(position_id.clone()))
        }
    }

    /// Remove tracking for a position
    pub fn remove_tracking(&mut self, position_id: &PositionId) -> Result<()> {
        if self.tracking_sessions.remove(position_id).is_some() {
            self.last_updates.remove(position_id);
            Ok(())
        } else {
            Err(PositionManagerError::PositionNotFound(position_id.clone()))
        }
    }
}

/// Tracking statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingStats {
    pub active_sessions: usize,
    pub total_positions: usize,
    pub total_updates: u64,
    pub average_uptime: f64,
    pub error_count: u64,
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new(&PositionTrackerConfig::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_tracker_config() {
        let config = PositionTrackerConfig::default();
        assert_eq!(config.update_interval_ms, 5000);
        assert_eq!(config.max_concurrent_positions, 1000);
        assert!(config.real_time_updates);
    }

    #[test]
    fn test_tracking_session() {
        let session = TrackingSession {
            position_id: PositionId::new(),
            chain_id: 1,
            asset: H160::zero(),
            start_time: 1000,
            last_update: 2000,
            update_count: 5,
            status: TrackingStatus::Active,
            metrics: PositionMetrics {
                total_updates: 5,
                last_balance: U256::from(1000),
                balance_changes: Vec::new(),
                performance_samples: Vec::new(),
                error_count: 0,
                uptime_percentage: 100.0,
            },
        };

        assert_eq!(session.status, TrackingStatus::Active);
        assert_eq!(session.metrics.total_updates, 5);
    }
}
