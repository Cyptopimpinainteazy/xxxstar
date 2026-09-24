//! `BaseAdapter::send_message` against a real chain.
//!
//! The adapter used to build `sendMessage` calldata, run it through `eth_call`
//! (which changes no state) and return `message.hash()` — a hash for a transaction
//! that was never broadcast, indistinguishable from a real one. It now signs an
//! EIP-155 transaction with a configured key and broadcasts it.
//!
//! This test spawns `anvil` (foundry) and drives the real path: nonce and gas
//! price read from the chain, a signed transaction, `eth_sendRawTransaction`, and
//! the receipt the node returns. The node is a local one because a send is a
//! write; the point is that every field comes from a chain, which a stub cannot
//! show for a broadcast.
//!
//! Requires `anvil` on PATH — the same requirement the `EVM contract lifecycle`
//! gate already carries.

use sp_core::{H160, H256, U256};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use x3_external_chains::adapter::{ChainAdapter, ChainConfig, ChainMessage};
use x3_external_chains::chains::base::{BaseAdapter, EvmSigner};
use x3_external_chains::ChainType;

const PORT: u16 = 18946;
const CHAIN_ID: u64 = 31_337;

/// Anvil's first default account. Public test key, funded by the node itself.
const ANVIL_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const ANVIL_ADDRESS: [u8; 20] = hex_literal::hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

struct AnvilGuard(Child);

impl Drop for AnvilGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_anvil() -> (AnvilGuard, String) {
    let child = Command::new("anvil")
        .args([
            "--port",
            &PORT.to_string(),
            "--chain-id",
            &CHAIN_ID.to_string(),
            "--silent",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect(
            "anvil (foundry) is required for this test: a send is a write and needs a chain. \
             The `EVM contract lifecycle` gate requires the same tool.",
        );
    (AnvilGuard(child), format!("http://127.0.0.1:{PORT}"))
}

async fn rpc(url: &str, method: &str, params: serde_json::Value) -> serde_json::Value {
    let body = format!(r#"{{"jsonrpc":"2.0","method":"{method}","params":{params},"id":1}}"#);
    let bytes = x3_external_chains::rpc_http::post_json(url, body.as_bytes())
        .await
        .unwrap_or_else(|e| panic!("{method} over HTTP failed: {e}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{method} did not answer JSON: {e}"))
}

/// Same call as [`rpc`], but a transport failure comes back as an `Err` so the
/// readiness loop can retry it. Everything after startup still uses `rpc`, where
/// a failed connection is a real failure.
async fn try_rpc(
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let body = format!(r#"{{"jsonrpc":"2.0","method":"{method}","params":{params},"id":1}}"#);
    let bytes = x3_external_chains::rpc_http::post_json(url, body.as_bytes())
        .await
        .map_err(|e| format!("{method} over HTTP failed: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{method} did not answer JSON: {e}"))
}

async fn wait_for_chain(url: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        // `rpc` panics on a transport error, which is exactly what "anvil has not
        // bound its port yet" looks like — so this loop could never wait for
        // anything: the first refused connection failed the test instead of
        // being retried.
        if let Ok(answer) = try_rpc(url, "eth_chainId", serde_json::json!([])).await {
            if answer.get("result").is_some() {
                return;
            }
        }
        thread::sleep(Duration::from_millis(250));
    }
    panic!("anvil did not answer on {url} within 30s");
}

async fn pending_nonce(url: &str, address: H160) -> u64 {
    let answer = rpc(
        url,
        "eth_getTransactionCount",
        serde_json::json!([format!("0x{}", hex::encode(address.as_bytes())), "pending"]),
    )
    .await;
    let raw = answer["result"].as_str().expect("a nonce string");
    u64::from_str_radix(raw.trim_start_matches("0x"), 16).expect("a hex nonce")
}

fn message(payload: &[u8]) -> ChainMessage {
    ChainMessage {
        source_chain: 8_453,
        dest_chain: 1,
        sender: H160(ANVIL_ADDRESS),
        recipient: H160([0x22; 20]),
        nonce: 0,
        payload: payload.to_vec(),
        value: U256::zero(),
        gas_limit: 100_000,
        timestamp: 0,
    }
}

#[tokio::test]
async fn send_message_broadcasts_a_signed_transaction() {
    let (_anvil, url) = spawn_anvil();
    wait_for_chain(&url).await;

    let mut config = ChainConfig::for_chain(ChainType::Base);
    config.rpc_url = url.as_bytes().to_vec();
    // Anvil's chain id, because it is the EIP-155 field the signature commits to.
    config.chain_type = CHAIN_ID;

    let signer = EvmSigner::from_private_key(ANVIL_KEY).expect("a valid private key");
    assert_eq!(
        signer.address(),
        H160(ANVIL_ADDRESS),
        "the signer must report the address its key derives"
    );
    let adapter = BaseAdapter::with_signer(config, signer);

    assert_eq!(pending_nonce(&url, H160(ANVIL_ADDRESS)).await, 0);

    let first = adapter
        .send_message(message(b"hello from the external-chains adapter"))
        .await
        .expect("the send is broadcast");
    assert_ne!(first, H256::zero(), "a broadcast returns its hash");

    // The node has the transaction and mined it: this is what the old `eth_call`
    // path could never produce.
    let receipt = adapter
        .get_transaction_receipt(first)
        .await
        .expect("receipt query")
        .expect("a mined transaction");
    assert!(
        receipt.success,
        "the broadcast transaction was included and succeeded"
    );
    assert!(receipt.block_number > 0, "it landed in a block");
    assert_eq!(pending_nonce(&url, H160(ANVIL_ADDRESS)).await, 1);

    // A second send takes the next nonce *from the chain*: a hardcoded or cached
    // nonce would collide here and the node would reject the transaction.
    let second = adapter
        .send_message(message(b"second"))
        .await
        .expect("a second send is broadcast");
    assert_ne!(second, first, "each send has its own hash");
    let receipt = adapter
        .get_transaction_receipt(second)
        .await
        .expect("receipt query")
        .expect("a mined transaction");
    assert!(receipt.success);
    assert_eq!(pending_nonce(&url, H160(ANVIL_ADDRESS)).await, 2);
}

#[tokio::test]
async fn sending_without_a_signer_is_refused() {
    // The refusal has to survive the new capability: an adapter built with
    // `new()` holds no key and must not report a transaction it did not send.
    let adapter = BaseAdapter::new(ChainConfig::for_chain(ChainType::Base));
    let result = adapter.send_message(message(b"unsigned")).await;
    assert!(
        result.is_err(),
        "an adapter without a signer must refuse to send: {result:?}"
    );
}
