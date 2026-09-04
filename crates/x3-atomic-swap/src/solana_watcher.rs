//! # Solana Program Event Watcher
//!
//! Polls a Solana RPC endpoint for transaction signatures referencing the
//! HTLC program, fetches parsed transactions, decodes lock/claim/refund
//! instructions, and returns structured `HtlcEvent` records.
//!
//! ## How it works
//! 1. Call `getSignaturesForAddress` to discover recent transactions touching
//!    the HTLC program ID.
//! 2. For each new signature, call `getTransaction` with `jsonParsed` encoding.
//! 3. Parse the transaction instructions to extract Lock/Claim/Refund events.
//! 4. Return decoded `HtlcEvent` records for the relayer to consume.

use crate::error::SwapError;
use crate::event_watcher::HtlcEvent;
use crate::rpc_client::RpcClient;
use alloc::string::String;
use alloc::vec::Vec;

/// Configuration for the Solana program watcher.
#[derive(Debug, Clone)]
pub struct SolanaWatcherConfig {
    /// Solana RPC endpoint URL.
    pub rpc_url: String,
    /// HTLC program ID to watch.
    pub program_id: String,
    /// How many signatures to fetch per poll.
    pub limit: u64,
    /// Required commitment level ("confirmed" or "finalized").
    pub commitment: String,
}

impl Default for SolanaWatcherConfig {
    fn default() -> Self {
        Self {
            rpc_url: String::new(),
            program_id: String::new(),
            limit: 100,
            commitment: "finalized".into(),
        }
    }
}

/// Watches a Solana program for HTLC events via `getSignaturesForAddress`.
#[derive(Debug, Clone)]
pub struct SolanaWatcher {
    pub config: SolanaWatcherConfig,
    pub rpc_client: RpcClient,
    /// Signature high-water mark: only fetch signatures after this one.
    pub last_seen_signature: Option<String>,
    /// Block watermark for polling.
    pub last_polled_slot: u64,
}

impl SolanaWatcher {
    /// Create a new Solana program watcher.
    pub fn new(config: SolanaWatcherConfig) -> Self {
        let rpc_client = RpcClient::new(config.rpc_url.clone(), 0);
        Self {
            config,
            rpc_client,
            last_seen_signature: None,
            last_polled_slot: 0,
        }
    }

    /// Fetch recent signatures for the configured program.
    ///
    /// Calls `getSignaturesForAddress` with the program ID. Filters to
    /// signatures newer than `last_seen_signature`.
    ///
    /// Requires the `std` feature for HTTP transport.
    #[cfg(feature = "std")]
    pub fn get_recent_signatures(&mut self) -> Result<Vec<String>, SwapError> {
        let params = vec![
            serde_json::Value::String(self.config.program_id.clone()),
            serde_json::json!({
                "limit": self.config.limit,
                "commitment": self.config.commitment,
            }),
        ];
        let resp = self.rpc_client.call("getSignaturesForAddress", params)?;

        let signatures: Vec<String> = if let Some(result) = resp.result {
            let arr = result.as_array().cloned().unwrap_or_default();
            arr.iter()
                .filter_map(|entry| {
                    entry
                        .get("signature")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        } else {
            Vec::new()
        };

        // Filter to only new signatures (after last_seen).
        let new_sigs: Vec<String> = if let Some(ref last) = self.last_seen_signature {
            signatures
                .into_iter()
                .take_while(|sig| sig != last)
                .collect()
        } else {
            signatures
        };

        // Update watermark
        if let Some(first) = new_sigs.first() {
            self.last_seen_signature = Some(first.clone());
        }

        Ok(new_sigs)
    }

    /// Fetch a parsed transaction by signature and decode HTLC events.
    ///
    /// Requires the `std` feature for HTTP transport.
    #[cfg(feature = "std")]
    pub fn fetch_and_decode_transaction(
        &mut self,
        signature: &str,
    ) -> Result<Vec<HtlcEvent>, SwapError> {
        let params = vec![
            serde_json::Value::String(signature.to_string()),
            serde_json::json!({
                "encoding": "jsonParsed",
                "commitment": self.config.commitment,
                "maxSupportedTransactionVersion": 0,
            }),
        ];
        let resp = self.rpc_client.call("getTransaction", params)?;

        let tx = match resp.result {
            Some(ref result) if !result.is_null() => result,
            _ => return Ok(Vec::new()),
        };

        self.decode_instructions(tx)
    }

    /// Decode HTLC events from a parsed Solana transaction.
    #[allow(dead_code)]
    fn decode_instructions(&self, tx: &serde_json::Value) -> Result<Vec<HtlcEvent>, SwapError> {
        let mut events = Vec::new();

        let instructions = tx
            .get("transaction")
            .and_then(|t| t.get("message"))
            .and_then(|m| m.get("instructions"))
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();

        let block_number = tx.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);

        let meta = tx.get("meta");
        let tx_hash = tx
            .get("transaction")
            .and_then(|t| t.get("signatures"))
            .and_then(|s| s.get(0))
            .and_then(|s| s.as_str())
            .unwrap_or("unknown")
            .to_string();

        for instruction in &instructions {
            // Check if this instruction targets our program
            let program_id = instruction
                .get("programId")
                .and_then(|p| p.as_str())
                .unwrap_or("");
            if program_id != self.config.program_id {
                continue;
            }

            let parsed = instruction.get("parsed");
            let ix_type = parsed
                .and_then(|p| p.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("");

            match ix_type {
                "lock" => {
                    if let Some(info) = parsed.and_then(|p| p.get("info")) {
                        let intent_id = info.get("swapId").and_then(|v| v.as_u64()).unwrap_or(0);
                        let amount = info
                            .get("amount")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<u128>().ok())
                            .unwrap_or(0);
                        let hashlock_raw =
                            info.get("hashlock").and_then(|v| v.as_str()).unwrap_or("");
                        let mut hashlock = [0u8; 32];
                        if let Ok(decoded) =
                            hex::decode(hashlock_raw.strip_prefix("0x").unwrap_or(hashlock_raw))
                        {
                            let len = decoded.len().min(32);
                            hashlock[..len].copy_from_slice(&decoded[..len]);
                        }

                        events.push(HtlcEvent::Locked {
                            intent_id,
                            sender: Vec::new(),
                            receiver: Vec::new(),
                            amount,
                            hashlock,
                            timeout: 0,
                            contract_address: self.config.program_id.clone(),
                            tx_hash: tx_hash.clone(),
                            block_number,
                        });
                    }
                }
                "claim" => {
                    if let Some(info) = parsed.and_then(|p| p.get("info")) {
                        let intent_id = info.get("swapId").and_then(|v| v.as_u64()).unwrap_or(0);
                        let preimage_raw =
                            info.get("preimage").and_then(|v| v.as_str()).unwrap_or("");
                        let mut preimage = [0u8; 32];
                        if let Ok(decoded) =
                            hex::decode(preimage_raw.strip_prefix("0x").unwrap_or(preimage_raw))
                        {
                            let len = decoded.len().min(32);
                            preimage[..len].copy_from_slice(&decoded[..len]);
                        }

                        events.push(HtlcEvent::Claimed {
                            intent_id,
                            claimer: Vec::new(),
                            preimage,
                            contract_address: self.config.program_id.clone(),
                            tx_hash: tx_hash.clone(),
                            block_number,
                        });
                    }
                }
                "refund" => {
                    if let Some(info) = parsed.and_then(|p| p.get("info")) {
                        let intent_id = info.get("swapId").and_then(|v| v.as_u64()).unwrap_or(0);

                        events.push(HtlcEvent::Refunded {
                            intent_id,
                            refund_address: Vec::new(),
                            contract_address: self.config.program_id.clone(),
                            tx_hash: tx_hash.clone(),
                            block_number,
                        });
                    }
                }
                _ => {}
            }
        }

        // Also check logs in meta for events
        if let Some(log_messages) = meta
            .and_then(|m| m.get("logMessages"))
            .and_then(|l| l.as_array())
        {
            for log in log_messages {
                if let Some(msg) = log.as_str() {
                    if msg.contains("Program log: Locked") {
                        // Additional event decoding from program logs if needed
                    }
                    if msg.contains("Program log: Claimed") {
                        // Additional event decoding from program logs if needed
                    }
                    if msg.contains("Program log: Refunded") {
                        // Additional event decoding from program logs if needed
                    }
                }
            }
        }

        Ok(events)
    }

    /// Poll the program for new events since the last poll.
    ///
    /// This is the main production poll loop for Solana HTLC programs.
    /// Requires the `std` feature for HTTP transport.
    #[cfg(feature = "std")]
    pub fn poll_events(&mut self) -> Result<Vec<HtlcEvent>, SwapError> {
        let signatures = self.get_recent_signatures()?;

        if signatures.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_events = Vec::new();

        // Fetch and decode each new transaction (limit to avoid excessive RPC calls)
        let max_fetch = signatures.len().min(20);
        for sig in signatures.iter().take(max_fetch) {
            match self.fetch_and_decode_transaction(sig) {
                Ok(events) => all_events.extend(events),
                Err(_) => continue, // Skip malformed transactions
            }
        }

        Ok(all_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_watcher_creation() {
        let config = SolanaWatcherConfig {
            rpc_url: "https://api.devnet.solana.com".into(),
            program_id: "HTLC1111111111111111111111111111111111111".into(),
            ..SolanaWatcherConfig::default()
        };
        let watcher = SolanaWatcher::new(config);
        assert_eq!(
            watcher.config.program_id,
            "HTLC1111111111111111111111111111111111111"
        );
        assert_eq!(watcher.config.limit, 100);
        assert_eq!(watcher.config.commitment, "finalized");
        assert!(watcher.last_seen_signature.is_none());
        assert_eq!(watcher.last_polled_slot, 0);
    }

    #[test]
    fn test_solana_watcher_default_config() {
        let config = SolanaWatcherConfig::default();
        assert_eq!(config.rpc_url, "");
        assert_eq!(config.limit, 100);
        assert_eq!(config.commitment, "finalized");
    }

    #[test]
    fn test_solana_watcher_no_program_id() {
        let config = SolanaWatcherConfig {
            rpc_url: "https://api.devnet.solana.com".into(),
            program_id: String::new(),
            ..SolanaWatcherConfig::default()
        };
        let watcher = SolanaWatcher::new(config);
        assert!(watcher.config.program_id.is_empty());
    }
}
