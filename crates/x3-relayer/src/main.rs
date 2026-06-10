//! X3 Relayer — Watches external chains for deposit events and submits proofs to X3.
//!
//! Also watches X3 for withdrawal events and submits release proofs to external gateways.
//!
//! # Architecture
//! - EVM Chain Watcher: polls gateway contracts for DepositLocked events
//! - X3 Chain Watcher: polls X3 for WithdrawalRequested events
//! - Proof Builder: constructs Merkle inclusion proofs from event data
//! - Submitter: sends proofs to X3/external gateway via RPC
//!
//! # Configuration
//! Set via environment variables (see config section below) or config file.

use std::collections::HashMap;
use std::time::Duration;

// ── Configuration ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct RelayerConfig {
    /// X3 node RPC endpoint
    x3_rpc: String,
    /// External EVM chain RPC endpoints (chain_id -> url)
    evm_rpcs: HashMap<u64, String>,
    /// External gateway contract addresses (chain_id -> address)
    gateway_addresses: HashMap<u64, String>,
    /// X3 kernel bridge contract address (on X3 EVM)
    x3_bridge_address: String,
    /// Number of confirmations required per chain (chain_id -> count)
    confirmations: HashMap<u64, u64>,
    /// Poll interval in seconds
    poll_interval_secs: u64,
    /// Maximum retries for failed proof submissions
    max_retries: u32,
    /// Private key for submitting transactions
    relayer_private_key: String,
    /// Database path for tracking processed events
    db_path: String,
}

impl RelayerConfig {
    fn from_env() -> Self {
        Self {
            x3_rpc: std::env::var("X3_RPC").unwrap_or_else(|_| "ws://127.0.0.1:9944".into()),
            evm_rpcs: {
                let mut m = HashMap::new();
                if let Ok(url) = std::env::var("ETH_RPC") {
                    m.insert(1, url);
                }
                if let Ok(url) = std::env::var("BASE_RPC") {
                    m.insert(8453, url);
                }
                if let Ok(url) = std::env::var("ARB_RPC") {
                    m.insert(42161, url);
                }
                m
            },
            gateway_addresses: {
                let mut m = HashMap::new();
                if let Ok(addr) = std::env::var("ETH_GATEWAY") {
                    m.insert(1, addr);
                }
                if let Ok(addr) = std::env::var("BASE_GATEWAY") {
                    m.insert(8453, addr);
                }
                if let Ok(addr) = std::env::var("ARB_GATEWAY") {
                    m.insert(42161, addr);
                }
                m
            },
            x3_bridge_address: std::env::var("X3_BRIDGE").unwrap_or_default(),
            confirmations: {
                let mut m = HashMap::new();
                m.insert(1, 64); // Ethereum: 64 confirmations
                m.insert(8453, 32); // Base: 32 confirmations
                m.insert(42161, 32); // Arbitrum: 32 confirmations
                m
            },
            poll_interval_secs: std::env::var("POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            max_retries: std::env::var("MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            relayer_private_key: std::env::var("RELAYER_KEY").unwrap_or_default(),
            db_path: std::env::var("DB_PATH").unwrap_or_else(|_| "/tmp/x3-relayer.db".into()),
        }
    }
}

// ── Event Types ─────────────────────────────────────────────────────────────

/// A deposit event from an external EVM gateway
#[derive(Debug, Clone)]
struct ExternalDepositEvent {
    /// The chain ID where the deposit occurred
    chain_id: u64,
    /// The message ID (keccak256 of deposit details)
    message_id: [u8; 32],
    /// The ERC20 token address
    token_address: Vec<u8>,
    /// The depositor address
    depositor: Vec<u8>,
    /// The X3 recipient (SCALE-encoded)
    x3_recipient: Vec<u8>,
    /// The deposit amount
    amount: u128,
    /// The nonce used
    nonce: u128,
    /// Block number on the source chain
    block_number: u64,
    /// Transaction hash on the source chain
    tx_hash: [u8; 32],
}

/// A withdrawal event from X3 (observed on X3 chain)
#[derive(Debug, Clone)]
struct X3WithdrawalEvent {
    /// The message ID
    message_id: [u8; 32],
    /// The asset ID on X3
    asset_id: [u8; 32],
    /// The X3 sender address
    sender: Vec<u8>,
    /// The external chain recipient
    recipient: Vec<u8>,
    /// The amount
    amount: u128,
    /// The destination chain ID
    destination_chain_id: u64,
    /// X3 block number
    x3_block_number: u64,
}

// ── Relayer logic ───────────────────────────────────────────────────────────

struct Relayer {
    config: RelayerConfig,
    /// Track processed events to avoid double-processing
    processed_deposits: HashMap<[u8; 32], bool>,
    processed_withdrawals: HashMap<[u8; 32], bool>,
}

impl Relayer {
    fn new(config: RelayerConfig) -> Self {
        Self {
            config,
            processed_deposits: HashMap::new(),
            processed_withdrawals: HashMap::new(),
        }
    }

    /// Main relayer loop — polls both external chains and X3
    async fn run(&mut self) {
        println!("[relayer] Starting X3 Relayer...");
        println!("[relayer] X3 RPC: {}", self.config.x3_rpc);
        println!(
            "[relayer] EVM chains configured: {:?}",
            self.config.evm_rpcs.keys()
        );
        println!(
            "[relayer] Poll interval: {}s",
            self.config.poll_interval_secs
        );

        loop {
            // Watch external EVM chains for deposit events
            for chain_id in self.config.evm_rpcs.keys() {
                if let Err(e) = self.watch_evm_deposits(*chain_id).await {
                    eprintln!("[relayer] Error watching EVM chain {}: {:?}", chain_id, e);
                }
            }

            // Watch X3 for withdrawal events
            if let Err(e) = self.watch_x3_withdrawals().await {
                eprintln!("[relayer] Error watching X3 withdrawals: {:?}", e);
            }

            tokio::time::sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }

    /// Poll an external EVM gateway contract for new DepositLocked events
    async fn watch_evm_deposits(&self, chain_id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let rpc_url = self
            .config
            .evm_rpcs
            .get(&chain_id)
            .ok_or("No RPC configured for chain")?;

        let gateway = self
            .config
            .gateway_addresses
            .get(&chain_id)
            .ok_or("No gateway configured for chain")?;

        let confirmations = self
            .config
            .confirmations
            .get(&chain_id)
            .copied()
            .unwrap_or(12);

        // In production, this would:
        // 1. Call eth_getLogs to find DepositLocked events
        // 2. Parse event data into ExternalDepositEvent
        // 3. Check confirmations >= required
        // 4. Build receipt proof (RLP-encoded receipt + merkle proof)
        // 5. Submit proof to X3 via x3_submitDepositProof extrinsic

        println!(
            "[relayer] Watching chain {} gateway {} ({} confirmations required)",
            chain_id, gateway, confirmations
        );

        Ok(())
    }

    /// Watch X3 chain for WithdrawalRequested events
    async fn watch_x3_withdrawals(&self) -> Result<(), Box<dyn std::error::Error>> {
        // In production, this would:
        // 1. Query X3 RPC for WithdrawalRequested events
        // 2. Build X3 finalized proof (block hash + message inclusion proof)
        // 3. Submit release proof to external gateway contract

        println!("[relayer] Watching X3 for withdrawal events...");

        Ok(())
    }

    /// Submit a deposit proof to X3 (calls extrinsic on X3 node)
    async fn submit_deposit_proof_to_x3(
        &self,
        event: &ExternalDepositEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "[relayer] Submitting deposit proof to X3: msg_id={:?}, chain={}, amount={}",
            event.message_id, event.chain_id, event.amount
        );

        // In production, this would:
        // 1. Construct a SCALE-encoded extrinsic
        // 2. Sign with the relayer's private key
        // 3. Submit to X3 via RPC (author_submitAndWatchExtrinsic)
        // 4. Wait for finalization

        Ok(())
    }

    /// Submit a release proof to an external EVM gateway
    async fn submit_release_proof_to_gateway(
        &self,
        event: &X3WithdrawalEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "[relayer] Submitting release proof to chain {}: msg_id={:?}, amount={}",
            event.destination_chain_id, event.message_id, event.amount
        );

        // In production, this would:
        // 1. Build the X3 finalized proof payload
        // 2. Call releaseFromX3() on the external gateway contract
        // 3. Wait for transaction confirmation

        Ok(())
    }
}

// ── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let config = RelayerConfig::from_env();
    let mut relayer = Relayer::new(config);
    relayer.run().await;
}
