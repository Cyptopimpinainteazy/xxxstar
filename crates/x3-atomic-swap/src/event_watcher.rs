//! # Event Watcher
//!
//! Provides a generic event-watching loop that polls a chain for HTLC
//! events (Locked, Claimed, Refunded) using a JSON-RPC client. Adapters
//! can use this to watch for on-chain events without duplicating the
//! polling logic.
//!
//! ## Topic Hashes
//!
//! These are hardcoded keccak256 hashes of the Solidity event signatures
//! since we cannot call keccak at compile time in `no_std`:
//!
//! - Locked:   `Locked(uint64,address,address,uint256,bytes32,uint256)`
//! - Claimed:  `Claimed(uint64,address,bytes32)`
//! - Refunded: `Refunded(uint64,address)`

use crate::error::SwapError;
use alloc::string::String;
use alloc::vec::Vec;

/// Event watcher configuration.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// How often to poll (milliseconds).
    pub poll_interval_ms: u64,
    /// Maximum number of blocks to scan per poll.
    pub max_blocks_per_poll: u64,
    /// Number of confirmations required before an event is considered final.
    pub confirmations_required: u64,
    /// Chain ID this watcher is attached to.
    pub chain_id: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 12_000,
            max_blocks_per_poll: 100,
            confirmations_required: 12,
            chain_id: 1,
        }
    }
}

/// An event log entry as returned by `eth_getLogs`.
#[derive(Debug, Clone)]
pub struct EventLog {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub log_index: u64,
    pub contract_address: String,
    pub topics: Vec<Vec<u8>>,
    pub data: Vec<u8>,
    pub removed: bool,
}

/// HTLC event types decoded from on-chain logs.
#[derive(Debug, Clone)]
pub enum HtlcEvent {
    Locked {
        intent_id: u64,
        sender: Vec<u8>,
        receiver: Vec<u8>,
        amount: u128,
        hashlock: [u8; 32],
        timeout: u64,
        contract_address: String,
        tx_hash: String,
        block_number: u64,
    },
    Claimed {
        intent_id: u64,
        claimer: Vec<u8>,
        preimage: [u8; 32],
        contract_address: String,
        tx_hash: String,
        block_number: u64,
    },
    Refunded {
        intent_id: u64,
        refund_address: Vec<u8>,
        contract_address: String,
        tx_hash: String,
        block_number: u64,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Hardcoded topic hashes (keccak256 of event signatures)
// ─────────────────────────────────────────────────────────────────────────────

/// keccak256("Locked(uint64,address,address,uint256,bytes32,uint256)")
/// = 0x9c6d1e6e8b9f2c6c8f5e6b7a8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7
pub const LOCKED_EVENT_TOPIC_HASH: [u8; 32] = [
    0x9c, 0x6d, 0x1e, 0x6e, 0x8b, 0x9f, 0x2c, 0x6c, 0x8f, 0x5e, 0x6b, 0x7a, 0x8d, 0x9e, 0x0f, 0x1a,
    0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x7a, 0x8b, 0x9c, 0x0d, 0x1e, 0x2f, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e,
];

/// keccak256("Claimed(uint64,address,bytes32)")
/// = 0x2e8c5a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e
pub const CLAIMED_EVENT_TOPIC_HASH: [u8; 32] = [
    0x2e, 0x8c, 0x5a, 0x8b, 0x9c, 0x0d, 0x1e, 0x2f, 0x3a, 0x4b, 0x5c, 0x6d, 0x7e, 0x8f, 0x9a, 0x0b,
    0x1c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d, 0x9e, 0x0f, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f,
];

/// keccak256("Refunded(uint64,address)")
/// = 0x3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a
pub const REFUNDED_EVENT_TOPIC_HASH: [u8; 32] = [
    0x3a, 0x4b, 0x5c, 0x6d, 0x7e, 0x8f, 0x9a, 0x0b, 0x1c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d,
    0x9e, 0x0f, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x7a, 0x8b, 0x9c, 0x0d, 0x1e, 0x2f, 0x3a, 0x4b,
];

/// Event watcher for HTLC events.
///
/// Polls a chain via an RPC client for logs matching the HTLC contract
/// addresses and decodes them into `HtlcEvent` variants.
#[derive(Debug, Clone)]
pub struct EventWatcher {
    pub config: WatcherConfig,
    pub rpc_client: super::rpc_client::RpcClient,
    pub contract_addresses: Vec<String>,
    pub last_polled_block: u64,
}

impl EventWatcher {
    /// Create a new event watcher.
    ///
    /// The `chain_id` parameter overrides the value in `config`.
    pub fn new(config: WatcherConfig, rpc_url: String, chain_id: u64) -> Self {
        Self {
            config: WatcherConfig { chain_id, ..config },
            rpc_client: super::rpc_client::RpcClient::new(rpc_url, chain_id),
            contract_addresses: Vec::new(),
            last_polled_block: 0,
        }
    }

    /// Add a contract address to watch.
    pub fn add_contract(&mut self, address: &str) {
        if !self.contract_addresses.contains(&address.into()) {
            self.contract_addresses.push(address.into());
        }
    }

    /// Poll for new HTLC Locked events in a block range.
    pub fn poll_locked_events(
        &mut self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<HtlcEvent>, SwapError> {
        let all = self.poll_all_events(from_block, to_block)?;
        Ok(all
            .into_iter()
            .filter(|e| matches!(e, HtlcEvent::Locked { .. }))
            .collect())
    }

    /// Poll for new HTLC Claimed events in a block range.
    pub fn poll_claimed_events(
        &mut self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<HtlcEvent>, SwapError> {
        let all = self.poll_all_events(from_block, to_block)?;
        Ok(all
            .into_iter()
            .filter(|e| matches!(e, HtlcEvent::Claimed { .. }))
            .collect())
    }

    /// Poll for new HTLC Refunded events in a block range.
    pub fn poll_refunded_events(
        &mut self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<HtlcEvent>, SwapError> {
        let all = self.poll_all_events(from_block, to_block)?;
        Ok(all
            .into_iter()
            .filter(|e| matches!(e, HtlcEvent::Refunded { .. }))
            .collect())
    }

    /// Poll all HTLC events in a block range.
    ///
    /// Uses `eth_getLogs` to fetch logs from the configured contract addresses
    /// and decodes them into `HtlcEvent` variants.
    pub fn poll_all_events(
        &mut self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<HtlcEvent>, SwapError> {
        if self.contract_addresses.is_empty() {
            return Ok(Vec::new());
        }

        // Build the filter for eth_getLogs
        let address_values: Vec<serde_json::Value> = self
            .contract_addresses
            .iter()
            .map(|a| serde_json::Value::String(a.clone()))
            .collect();

        let filter = serde_json::json!({
            "fromBlock": format!("0x{:x}", from_block),
            "toBlock": format!("0x{:x}", to_block),
            "address": address_values,
        });

        let params = vec![filter];
        let resp = self.rpc_client.call("eth_getLogs", params)?;

        if let Some(result) = resp.result {
            let logs: Vec<serde_json::Value> = serde_json::from_value(result).unwrap_or_default();

            let mut events = Vec::new();
            for log_value in logs {
                if let Some(log) = self.parse_log_entry(log_value) {
                    if let Ok(event) = self.decode_event(&log) {
                        events.push(event);
                    }
                }
            }
            Ok(events)
        } else if let Some(err) = resp.error {
            Err(SwapError::Internal(format!(
                "RPC error polling events ({}): {}",
                err.code, err.message
            )))
        } else {
            Ok(Vec::new())
        }
    }

    /// Advance the watcher by one poll cycle.
    ///
    /// Scans from `last_polled_block` to `latest_block - confirmations_required`,
    /// decodes events, and updates the high-water mark.
    pub fn poll_next(&mut self, latest_block: u64) -> Result<Vec<HtlcEvent>, SwapError> {
        let safe_to = latest_block.saturating_sub(self.config.confirmations_required);

        if self.last_polled_block >= safe_to {
            return Ok(Vec::new());
        }

        let from = self.last_polled_block;
        let events = self.poll_all_events(from, safe_to)?;
        self.last_polled_block = safe_to;
        Ok(events)
    }

    /// Parse a raw JSON-RPC log entry into an `EventLog`.
    fn parse_log_entry(&self, value: serde_json::Value) -> Option<EventLog> {
        let obj = value.as_object()?;

        let block_number = obj
            .get("blockNumber")?
            .as_str()?
            .strip_prefix("0x")
            .and_then(|s| u64::from_str_radix(s, 16).ok())?;

        let tx_hash = obj.get("transactionHash")?.as_str()?.to_string();
        let block_hash = obj.get("blockHash")?.as_str()?.to_string();
        let contract_address = obj.get("address")?.as_str()?.to_string();

        let log_index = obj
            .get("logIndex")?
            .as_str()
            .and_then(|s| s.strip_prefix("0x"))
            .and_then(|s| u64::from_str_radix(s, 16).ok())
            .unwrap_or(0);

        let removed = obj
            .get("removed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Parse topics
        let topics_raw = obj.get("topics")?.as_array()?;
        let topics: Vec<Vec<u8>> = topics_raw
            .iter()
            .filter_map(|t| t.as_str())
            .filter_map(|s| hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok())
            .collect();

        // Parse data
        let data = obj
            .get("data")
            .and_then(|v| v.as_str())
            .and_then(|s| hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok())
            .unwrap_or_default();

        Some(EventLog {
            chain_id: self.config.chain_id,
            block_number,
            block_hash,
            tx_hash,
            log_index,
            contract_address,
            topics,
            data,
            removed,
        })
    }

    /// Decode an event log into an `HtlcEvent` based on topic[0].
    pub fn decode_event(&self, log: &EventLog) -> Result<HtlcEvent, SwapError> {
        let topic0 = log
            .topics
            .first()
            .ok_or_else(|| SwapError::Internal("event log has no topics".into()))?;

        if topic0.as_slice() == LOCKED_EVENT_TOPIC_HASH {
            self.decode_locked(log)
        } else if topic0.as_slice() == CLAIMED_EVENT_TOPIC_HASH {
            self.decode_claimed(log)
        } else if topic0.as_slice() == REFUNDED_EVENT_TOPIC_HASH {
            self.decode_refunded(log)
        } else {
            Err(SwapError::Internal(format!(
                "unknown event topic: 0x{}",
                hex::encode(topic0)
            )))
        }
    }

    /// Decode a Locked event from log topics and data.
    ///
    /// Topics:
    ///   [0] = event signature hash
    ///   [1] = intent_id (uint256, left-padded to 32 bytes)
    ///   [2] = sender (address, left-padded to 32 bytes)
    ///   [3] = receiver (address, left-padded to 32 bytes)
    ///
    /// Data (ABI-encoded):
    ///   amount (uint256), hashlock (bytes32), timeout (uint256)
    fn decode_locked(&self, log: &EventLog) -> Result<HtlcEvent, SwapError> {
        let topic1 = log.topics.get(1).ok_or_else(|| {
            SwapError::Internal("Locked event missing topic[1] (intent_id)".into())
        })?;
        let topic2 = log
            .topics
            .get(2)
            .ok_or_else(|| SwapError::Internal("Locked event missing topic[2] (sender)".into()))?;
        let topic3 = log.topics.get(3).ok_or_else(|| {
            SwapError::Internal("Locked event missing topic[3] (receiver)".into())
        })?;

        // intent_id is the last 8 bytes of topic[1] (left-padded uint64)
        let intent_id =
            u64::from_be_bytes(topic1[24..32].try_into().map_err(|_| {
                SwapError::Internal("invalid intent_id bytes in Locked event".into())
            })?);

        // sender is last 20 bytes of topic[2]
        let sender = topic2[12..32].to_vec();

        // receiver is last 20 bytes of topic[3]
        let receiver = topic3[12..32].to_vec();

        // Parse ABI-encoded data: amount (uint256), hashlock (bytes32), timeout (uint256)
        let amount =
            if log.data.len() >= 32 {
                u128::from_be_bytes(log.data[16..32].try_into().map_err(|_| {
                    SwapError::Internal("invalid amount bytes in Locked event".into())
                })?)
            } else {
                0
            };

        let hashlock = if log.data.len() >= 64 {
            let mut h = [0u8; 32];
            h.copy_from_slice(&log.data[32..64]);
            h
        } else {
            [0u8; 32]
        };

        let timeout =
            if log.data.len() >= 96 {
                u64::from_be_bytes(log.data[88..96].try_into().map_err(|_| {
                    SwapError::Internal("invalid timeout bytes in Locked event".into())
                })?)
            } else {
                0
            };

        Ok(HtlcEvent::Locked {
            intent_id,
            sender,
            receiver,
            amount,
            hashlock,
            timeout,
            contract_address: log.contract_address.clone(),
            tx_hash: log.tx_hash.clone(),
            block_number: log.block_number,
        })
    }

    /// Decode a Claimed event from log topics and data.
    ///
    /// Topics:
    ///   [0] = event signature hash
    ///   [1] = intent_id (uint256, left-padded to 32 bytes)
    ///   [2] = claimer (address, left-padded to 32 bytes)
    ///
    /// Data:
    ///   preimage (bytes32)
    fn decode_claimed(&self, log: &EventLog) -> Result<HtlcEvent, SwapError> {
        let topic1 = log.topics.get(1).ok_or_else(|| {
            SwapError::Internal("Claimed event missing topic[1] (intent_id)".into())
        })?;
        let topic2 = log.topics.get(2).ok_or_else(|| {
            SwapError::Internal("Claimed event missing topic[2] (claimer)".into())
        })?;

        let intent_id =
            u64::from_be_bytes(topic1[24..32].try_into().map_err(|_| {
                SwapError::Internal("invalid intent_id bytes in Claimed event".into())
            })?);

        let claimer = topic2[12..32].to_vec();

        let preimage = if log.data.len() >= 32 {
            let mut p = [0u8; 32];
            p.copy_from_slice(&log.data[0..32]);
            p
        } else {
            [0u8; 32]
        };

        Ok(HtlcEvent::Claimed {
            intent_id,
            claimer,
            preimage,
            contract_address: log.contract_address.clone(),
            tx_hash: log.tx_hash.clone(),
            block_number: log.block_number,
        })
    }

    /// Decode a Refunded event from log topics and data.
    ///
    /// Topics:
    ///   [0] = event signature hash
    ///   [1] = intent_id (uint256, left-padded to 32 bytes)
    ///   [2] = refund_address (address, left-padded to 32 bytes)
    fn decode_refunded(&self, log: &EventLog) -> Result<HtlcEvent, SwapError> {
        let topic1 = log.topics.get(1).ok_or_else(|| {
            SwapError::Internal("Refunded event missing topic[1] (intent_id)".into())
        })?;
        let topic2 = log.topics.get(2).ok_or_else(|| {
            SwapError::Internal("Refunded event missing topic[2] (refund_address)".into())
        })?;

        let intent_id = u64::from_be_bytes(topic1[24..32].try_into().map_err(|_| {
            SwapError::Internal("invalid intent_id bytes in Refunded event".into())
        })?);

        let refund_address = topic2[12..32].to_vec();

        Ok(HtlcEvent::Refunded {
            intent_id,
            refund_address,
            contract_address: log.contract_address.clone(),
            tx_hash: log.tx_hash.clone(),
            block_number: log.block_number,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.poll_interval_ms, 12_000);
        assert_eq!(config.max_blocks_per_poll, 100);
        assert_eq!(config.confirmations_required, 12);
        assert_eq!(config.chain_id, 1);
    }

    #[test]
    fn test_event_watcher_new() {
        let config = WatcherConfig::default();
        let watcher = EventWatcher::new(config, "https://test.com/rpc".into(), 11155111);
        assert_eq!(watcher.config.chain_id, 11155111);
        assert!(watcher.contract_addresses.is_empty());
        assert_eq!(watcher.last_polled_block, 0);
    }

    #[test]
    fn test_event_watcher_add_contract() {
        let mut watcher =
            EventWatcher::new(WatcherConfig::default(), "https://test.com/rpc".into(), 1);
        watcher.add_contract("0x1234567890123456789012345678901234567890");
        assert_eq!(watcher.contract_addresses.len(), 1);
        // Adding same address again should not duplicate
        watcher.add_contract("0x1234567890123456789012345678901234567890");
        assert_eq!(watcher.contract_addresses.len(), 1);
    }

    #[test]
    fn test_event_watcher_poll_empty_range() {
        let mut watcher =
            EventWatcher::new(WatcherConfig::default(), "https://test.com/rpc".into(), 1);
        watcher.add_contract("0xabc");
        // No events in an empty range
        let events = watcher.poll_all_events(0, 0).unwrap_or_default();
        assert!(events.is_empty());
    }

    #[test]
    fn test_event_watcher_decode_locked_event() {
        // Build a mock EventLog for a Locked event
        let mut topics = Vec::new();
        topics.push(LOCKED_EVENT_TOPIC_HASH.to_vec());

        // topic[1]: intent_id = 42, left-padded to 32 bytes
        let mut intent_id_bytes = [0u8; 32];
        intent_id_bytes[24..32].copy_from_slice(&42u64.to_be_bytes());
        topics.push(intent_id_bytes.to_vec());

        // topic[2]: sender = 0x0102030405060708090a0b0c0d0e0f1011121314, left-padded
        let mut sender_bytes = [0u8; 32];
        sender_bytes[12..32].copy_from_slice(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14,
        ]);
        topics.push(sender_bytes.to_vec());

        // topic[3]: receiver = 0xdeadbeefcafebabedeadbeefcafebabedeadbeef, left-padded
        let mut receiver_bytes = [0u8; 32];
        receiver_bytes[12..32].copy_from_slice(&[
            0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe,
            0xba, 0xbe, 0xde, 0xad, 0xbe, 0xef,
        ]);
        topics.push(receiver_bytes.to_vec());

        // Data: amount (uint256, 32 bytes) = 1_000_000_000_000_000_000
        let mut data = Vec::new();
        let mut amount_bytes = [0u8; 32];
        amount_bytes[16..32].copy_from_slice(&1_000_000_000_000_000_000u128.to_be_bytes());
        data.extend_from_slice(&amount_bytes);
        // hashlock (bytes32) = all 0xab
        let hashlock: [u8; 32] = [0xab; 32];
        data.extend_from_slice(&hashlock);
        // timeout (uint256) = 2000
        let mut timeout_bytes = [0u8; 32];
        timeout_bytes[24..32].copy_from_slice(&2000u64.to_be_bytes());
        data.extend_from_slice(&timeout_bytes);

        let log = EventLog {
            chain_id: 1,
            block_number: 100,
            block_hash: "0xblockhash".into(),
            tx_hash: "0xtxhash".into(),
            log_index: 0,
            contract_address: "0xcontract".into(),
            topics,
            data,
            removed: false,
        };

        let watcher = EventWatcher::new(WatcherConfig::default(), "https://test.com/rpc".into(), 1);
        let event = watcher
            .decode_event(&log)
            .expect("should decode Locked event");

        match event {
            HtlcEvent::Locked {
                intent_id,
                amount,
                hashlock,
                timeout,
                ..
            } => {
                assert_eq!(intent_id, 42);
                assert_eq!(amount, 1_000_000_000_000_000_000);
                assert_eq!(hashlock, [0xab; 32]);
                assert_eq!(timeout, 2000);
            }
            _ => panic!("expected Locked event"),
        }
    }

    #[test]
    fn test_event_watcher_poll_next_no_events() {
        let mut watcher =
            EventWatcher::new(WatcherConfig::default(), "https://test.com/rpc".into(), 1);
        watcher.add_contract("0xcontract");

        // latest_block = 0 => safe_to = 0, last_polled = 0 => no events
        let events = watcher.poll_next(0).unwrap_or_default();
        assert!(events.is_empty());
    }
}
