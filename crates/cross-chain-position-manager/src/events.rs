//! Cross-chain event system for position management
//!
//! This module provides:
//! - Event bus for cross-chain events
//! - Position event types
//! - Chain event types
//! - Risk event types

use crate::types::{PositionId, H160, H256, U256};
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;

/// Event bus for publishing and subscribing to events
#[derive(Debug, Clone)]
pub struct EventBus {
    /// Event subscribers
    subscribers: sp_std::collections::btree_map::BTreeMap<EventType, Vec<EventSubscriber>>,
    /// Event history
    event_history: Vec<Event>,
    /// Maximum history size
    max_history_size: usize,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            subscribers: sp_std::collections::btree_map::BTreeMap::new(),
            event_history: Vec::new(),
            max_history_size: 1000,
        }
    }

    /// Subscribe to an event type
    pub fn subscribe(&mut self, event_type: EventType, subscriber: EventSubscriber) {
        let subscribers = self.subscribers.entry(event_type).or_insert_with(Vec::new);
        subscribers.push(subscriber);
    }

    /// Unsubscribe from an event type
    pub fn unsubscribe(&mut self, event_type: &EventType, subscriber_id: &H256) {
        if let Some(subscribers) = self.subscribers.get_mut(event_type) {
            subscribers.retain(|s| s.id != *subscriber_id);
        }
    }

    /// Publish an event
    pub async fn publish(&mut self, event: Event) -> Result<(), EventError> {
        // Store in history
        self.event_history.push(event.clone());
        if self.event_history.len() > self.max_history_size {
            self.event_history.remove(0);
        }

        // Notify subscribers
        if let Some(subscribers) = self.subscribers.get(&event.event_type) {
            for subscriber in subscribers {
                if let Err(e) = subscriber.notify(&event).await {
                    // Log error but don't fail the publish
                    tracing::warn!("Failed to notify subscriber: {:?}", e);
                }
            }
        }

        Ok(())
    }

    /// Get event history
    pub fn get_history(&self, limit: Option<usize>) -> Vec<&Event> {
        let limit = limit.unwrap_or(self.event_history.len());
        let start = if self.event_history.len() > limit {
            self.event_history.len() - limit
        } else {
            0
        };
        self.event_history[start..].iter().collect()
    }

    /// Get events by type
    pub fn get_events_by_type(&self, event_type: &EventType, limit: Option<usize>) -> Vec<&Event> {
        let limit = limit.unwrap_or(self.event_history.len());
        self.event_history
            .iter()
            .filter(|e| e.event_type == *event_type)
            .rev()
            .take(limit)
            .collect()
    }

    /// Clear event history
    pub fn clear_history(&mut self) {
        self.event_history.clear();
    }
}

/// Event subscriber
#[derive(Debug, Clone)]
pub struct EventSubscriber {
    /// Subscriber ID
    pub id: H256,
    /// Subscriber name
    pub name: String,
    /// Callback function (in a real implementation, this would be a function pointer)
    pub callback_id: H256,
}

impl EventSubscriber {
    /// Create a new event subscriber
    pub fn new(name: String, callback_id: H256) -> Self {
        let id = Self::generate_id(&name, callback_id);
        Self {
            id,
            name,
            callback_id,
        }
    }

    /// Notify the subscriber of an event
    pub async fn notify(&self, event: &Event) -> Result<(), EventError> {
        // In a real implementation, this would call the callback function
        tracing::info!(
            "Notifying subscriber {} of event: {:?}",
            self.name,
            event.event_type
        );
        Ok(())
    }

    /// Generate subscriber ID
    fn generate_id(name: &str, callback_id: H256) -> H256 {
        use sp_core::Hasher;
        use sp_runtime::traits::BlakeTwo256;

        let mut hasher = BlakeTwo256::default();
        hasher.hash(name.as_bytes());
        hasher.hash(callback_id.as_bytes());
        H256::from_slice(hasher.finish().as_ref())
    }
}

/// Event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventType {
    // Position events
    PositionCreated,
    PositionUpdated,
    PositionMigrated,
    PositionClosed,
    PositionLiquidated,

    // Chain events
    ChainConnected,
    ChainDisconnected,
    ChainError,
    ChainSync,

    // Risk events
    RiskAlert,
    KillSwitchTriggered,
    EmergencyStop,

    // Rebalancing events
    RebalanceTriggered,
    RebalanceStarted,
    RebalanceCompleted,
    RebalanceFailed,

    // Arbitrage events
    ArbitrageOpportunityFound,
    ArbitrageExecuted,
    ArbitrageFailed,

    // Migration events
    MigrationStarted,
    MigrationCompleted,
    MigrationFailed,

    // System events
    SystemStarted,
    SystemStopped,
    SystemError,
}

/// Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event ID
    pub id: H256,
    /// Event type
    pub event_type: EventType,
    /// Event timestamp
    pub timestamp: u64,
    /// Event data
    pub data: EventData,
    /// Source chain (if applicable)
    pub source_chain: Option<u64>,
    /// Target chain (if applicable)
    pub target_chain: Option<u64>,
}

impl Event {
    /// Create a new event
    pub fn new(event_type: EventType, data: EventData) -> Self {
        let id = Self::generate_id(event_type, &data);
        let timestamp = sp_io::offchain::timestamp().unix_millis();

        Self {
            id,
            event_type,
            timestamp,
            data,
            source_chain: None,
            target_chain: None,
        }
    }

    /// Create a new event with chain information
    pub fn with_chains(
        event_type: EventType,
        data: EventData,
        source_chain: Option<u64>,
        target_chain: Option<u64>,
    ) -> Self {
        let mut event = Self::new(event_type, data);
        event.source_chain = source_chain;
        event.target_chain = target_chain;
        event
    }

    /// Generate event ID
    fn generate_id(event_type: EventType, data: &EventData) -> H256 {
        use sp_core::Hasher;
        use sp_runtime::traits::BlakeTwo256;

        let mut hasher = BlakeTwo256::default();
        hasher.hash(&(event_type as u8).to_le_bytes());
        hasher.hash(&sp_io::offchain::timestamp().unix_millis().to_le_bytes());
        hasher.hash(&data.hash().as_bytes());
        H256::from_slice(hasher.finish().as_ref())
    }
}

/// Event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventData {
    /// Position event data
    Position(PositionEventData),
    /// Chain event data
    Chain(ChainEventData),
    /// Risk event data
    Risk(RiskEventData),
    /// Rebalancing event data
    Rebalance(RebalanceEventData),
    /// Arbitrage event data
    Arbitrage(ArbitrageEventData),
    /// Migration event data
    Migration(MigrationEventData),
    /// System event data
    System(SystemEventData),
}

impl EventData {
    /// Get hash of event data
    pub fn hash(&self) -> H256 {
        use sp_core::Hasher;
        use sp_runtime::traits::BlakeTwo256;

        let mut hasher = BlakeTwo256::default();
        match self {
            EventData::Position(data) => {
                hasher.hash(&1u8.to_le_bytes());
                hasher.hash(data.position_id.as_bytes());
                hasher.hash(&data.amount.as_bytes());
            }
            EventData::Chain(data) => {
                hasher.hash(&2u8.to_le_bytes());
                hasher.hash(&data.chain_id.to_le_bytes());
                hasher.hash(data.status.as_bytes());
            }
            EventData::Risk(data) => {
                hasher.hash(&3u8.to_le_bytes());
                hasher.hash(&data.severity.to_le_bytes());
                hasher.hash(data.description.as_bytes());
            }
            EventData::Rebalance(data) => {
                hasher.hash(&4u8.to_le_bytes());
                hasher.hash(&data.rebalance_id.as_bytes());
                hasher.hash(&data.actions_count.to_le_bytes());
            }
            EventData::Arbitrage(data) => {
                hasher.hash(&5u8.to_le_bytes());
                hasher.hash(&data.opportunity_id.as_bytes());
                hasher.hash(&data.profit.as_bytes());
            }
            EventData::Migration(data) => {
                hasher.hash(&6u8.to_le_bytes());
                hasher.hash(&data.migration_id.as_bytes());
                hasher.hash(&data.source_chain.to_le_bytes());
                hasher.hash(&data.target_chain.to_le_bytes());
            }
            EventData::System(data) => {
                hasher.hash(&7u8.to_le_bytes());
                hasher.hash(data.component.as_bytes());
                hasher.hash(data.message.as_bytes());
            }
        }
        H256::from_slice(hasher.finish().as_ref())
    }
}

/// Position event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionEventData {
    pub position_id: PositionId,
    pub asset: H160,
    pub amount: U256,
    pub chain_id: u64,
    pub action: PositionAction,
}

/// Position action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionAction {
    Created,
    Updated,
    Migrated,
    Closed,
    Liquidated,
}

/// Chain event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEventData {
    pub chain_id: u64,
    pub status: String,
    pub block_number: Option<u64>,
    pub gas_price: Option<U256>,
}

/// Risk event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskEventData {
    pub severity: u8,
    pub description: String,
    pub position_id: Option<PositionId>,
    pub chain_id: Option<u64>,
    pub action_taken: Option<String>,
}

/// Rebalancing event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceEventData {
    pub rebalance_id: H256,
    pub actions_count: usize,
    pub total_cost_usd: U256,
    pub status: RebalanceStatus,
}

/// Rebalance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebalanceStatus {
    Triggered,
    Started,
    Completed,
    Failed,
}

/// Arbitrage event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageEventData {
    pub opportunity_id: H256,
    pub profit: U256,
    pub source_chain: u64,
    pub target_chain: u64,
    pub asset: H160,
    pub status: ArbitrageStatus,
}

/// Arbitrage status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArbitrageStatus {
    Found,
    Executing,
    Executed,
    Failed,
}

/// Migration event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationEventData {
    pub migration_id: H256,
    pub source_chain: u64,
    pub target_chain: u64,
    pub position_id: PositionId,
    pub status: MigrationStatus,
}

/// Migration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    Started,
    InProgress,
    Completed,
    Failed,
}

/// System event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEventData {
    pub component: String,
    pub message: String,
    pub level: SystemEventLevel,
}

/// System event level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemEventLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Position event type alias
pub type PositionEvent = Event;

/// Chain event type alias
pub type ChainEvent = Event;

/// Risk event type alias
pub type RiskEvent = Event;

/// Event error
#[derive(Debug, Clone)]
pub enum EventError {
    /// Subscriber not found
    SubscriberNotFound(H256),
    /// Event publish failed
    PublishFailed(String),
    /// Event history full
    HistoryFull,
    /// Invalid event data
    InvalidEventData(String),
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventError::SubscriberNotFound(id) => write!(f, "Subscriber not found: {:?}", id),
            EventError::PublishFailed(msg) => write!(f, "Event publish failed: {}", msg),
            EventError::HistoryFull => write!(f, "Event history is full"),
            EventError::InvalidEventData(msg) => write!(f, "Invalid event data: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus() {
        let mut event_bus = EventBus::new();

        let event = Event::new(
            EventType::PositionCreated,
            EventData::Position(PositionEventData {
                position_id: PositionId::new(),
                asset: H160::random(),
                amount: U256::from(1000),
                chain_id: 1,
                action: PositionAction::Created,
            }),
        );

        // Note: In a real test, we would use tokio::test
        // For now, we just test the structure
        assert_eq!(event.event_type, EventType::PositionCreated);
    }

    #[test]
    fn test_event_subscriber() {
        let subscriber = EventSubscriber::new("TestSubscriber".to_string(), H256::random());

        assert_eq!(subscriber.name, "TestSubscriber");
    }
}
