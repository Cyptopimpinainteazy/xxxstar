//! `ArbitrumAdapter::send_message` against a real chain.
//!
//! The adapter used to refuse to send, and before that it returned
//! `message.hash()` for a message nobody broadcast. It now signs an EIP-155
//! transaction that calls `ArbSys.sendTxToL1(address,bytes)` and broadcasts it
//! through `eth_sendRawTransaction`.
//!
//! This test spawns `anvil` (foundry) and drives the real path: nonce and gas
//! price read from the chain, a signed transaction, the node's receipt, and the
//! node's own view of the transaction it accepted — `to` is the ArbSys predeploy
//! and `input` is the ABI encoding of `sendTxToL1`, which is the part a stub
//! cannot show.
//!
//! Requires `anvil` on PATH — the same requirement the `EVM contract lifecycle`
//! gate already carries.

use sp_core::{H160, H256, U256};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use x3_external_chains::adapter::{ChainAdapter, ChainConfig, ChainMessage};
use x3_external_chains::chains::arbitrum::{
    send_tx_to_l1_selector, ArbitrumAdapter, ARBSYS_ADDRESS,
};
use x3_external_chains::signer::EvmSigner;
use x3_external_chains::ChainType;

const PORT: u16 = 18947;
const CHAIN_ID: u64 = 31_337;

/// Anvil's first default account. Public test key, funded by the node itself.
const ANVIL_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const ANVIL_ADDRESS: [u8; 20] = hex_literal::hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

/// `sendTxToL1(address,bytes)` per `cast sig`, i.e. keccak-256 of the signature
/// truncated to four bytes. Asserted here so the derived selector is checked
/// against something outside this crate's own hashing rather than against itself.
const SEND_TX_TO_L1_SELECTOR: [u8; 4] = [0x92, 0x8c, 0x16, 0x9a];

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

/// Same call as [`rpc`], but a transport failure comes back as an `Err` so a
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
        // bound its port yet" looks like — so the readiness loop could never
        // actually wait for anything: the first refused connection failed the
        // test instead of being retried. A refused connection is a normal state
        // for the first few hundred milliseconds after spawn, not a verdict.
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
        source_chain: 42_161,
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

fn build_adapter(url: &str) -> ArbitrumAdapter {
    let mut config = ChainConfig::for_chain(ChainType::Arbitrum);
    config.rpc_url = url.as_bytes().to_vec();
    // Anvil's chain id, because it is the EIP-155 field the signature commits to.
    config.chain_type = CHAIN_ID;
    let signer = EvmSigner::from_private_key(ANVIL_KEY).expect("a valid private key");
    assert_eq!(
        signer.address(),
        H160(ANVIL_ADDRESS),
        "the signer must report the address its key derives"
    );
    ArbitrumAdapter::with_signer(config, signer)
}

#[test]
fn the_selector_is_the_one_the_abi_names() {
    assert_eq!(
        send_tx_to_l1_selector(),
        SEND_TX_TO_L1_SELECTOR,
        "keccak-256(\"sendTxToL1(address,bytes)\")[..4]"
    );
}

#[test]
fn the_calldata_is_an_abi_encoded_call() {
    let destination = H160([0xAB; 20]);
    let payload = b"bound for L1";
    let encoded = ArbitrumAdapter::encode_send_tx_to_l1(destination, payload);

    assert_eq!(&encoded[..4], &SEND_TX_TO_L1_SELECTOR, "selector first");
    // Head word 1: destination, left-padded to 32 bytes.
    assert_eq!(&encoded[4..16], &[0u8; 12], "address is left-padded");
    assert_eq!(&encoded[16..36], destination.as_bytes(), "the destination");
    // Head word 2: offset to the bytes tail (two head words in).
    assert_eq!(encoded[36..67], [0u8; 31], "offset is a right-aligned word");
    assert_eq!(encoded[67], 64, "the tail starts after both head words");
    // Tail: length, then the data, padded to a whole word.
    assert_eq!(&encoded[68..92], &[0u8; 24], "length is right-aligned");
    assert_eq!(
        &encoded[92..100],
        &(payload.len() as u64).to_be_bytes(),
        "the payload length"
    );
    assert_eq!(&encoded[100..100 + payload.len()], payload, "the payload");
    assert_eq!(
        encoded.len() % 32,
        4,
        "calldata is the selector plus whole words"
    );
    assert!(
        encoded[100 + payload.len()..].iter().all(|b| *b == 0),
        "the tail padding is zeros"
    );
}

#[tokio::test]
async fn send_message_calls_arbsys_and_broadcasts_a_signed_transaction() {
    let (_anvil, url) = spawn_anvil();
    wait_for_chain(&url).await;

    let adapter = build_adapter(&url);
    assert_eq!(pending_nonce(&url, H160(ANVIL_ADDRESS)).await, 0);

    let payload = b"hello from the Arbitrum adapter";
    let sent = message(payload);
    let expected_input = format!(
        "0x{}",
        hex::encode(ArbitrumAdapter::encode_send_tx_to_l1(
            sent.recipient,
            &sent.payload,
        ))
    );

    let first = adapter
        .send_message(sent)
        .await
        .expect("the send is broadcast");
    assert_ne!(first, H256::zero(), "a broadcast returns its hash");

    // The node has the transaction and mined it. A receipt alone would not say
    // *what* was called, so ask the node for the transaction it accepted.
    let tx = rpc(
        &url,
        "eth_getTransactionByHash",
        serde_json::json!([format!("0x{}", hex::encode(first.as_bytes()))]),
    )
    .await;
    let to = tx["result"]["to"]
        .as_str()
        .expect("the transaction has a `to`");
    assert_eq!(
        to.to_lowercase(),
        format!("0x{}", hex::encode(ARBSYS_ADDRESS.as_bytes())),
        "the call is addressed to the ArbSys predeploy"
    );
    let input = tx["result"]["input"].as_str().expect("a calldata field");
    assert_eq!(
        input.to_lowercase(),
        expected_input,
        "the calldata decodes as sendTxToL1(recipient, payload)"
    );

    let receipt = adapter
        .get_transaction_receipt(first)
        .await
        .expect("receipt query")
        .expect("a mined transaction");
    assert!(receipt.success, "the broadcast transaction succeeded");
    assert!(receipt.block_number > 0, "it landed in a block");
    assert!(
        receipt.gas_used > 21_000,
        "a call carrying calldata costs more than the 21000 base cost, got {}",
        receipt.gas_used
    );
    assert_eq!(pending_nonce(&url, H160(ANVIL_ADDRESS)).await, 1);

    // A second send takes the next nonce *from the chain*: a hardcoded or cached
    // nonce would collide here and the node would reject the transaction.
    let second = adapter
        .send_message(message(b"second"))
        .await
        .expect("a second send is broadcast");
    assert_ne!(second, first, "each send has its own hash");
    assert!(
        adapter
            .get_transaction_receipt(second)
            .await
            .expect("receipt query")
            .expect("a mined transaction")
            .success
    );
    assert_eq!(pending_nonce(&url, H160(ANVIL_ADDRESS)).await, 2);
}

#[tokio::test]
async fn sending_without_a_signer_is_refused() {
    let mut config = ChainConfig::for_chain(ChainType::Arbitrum);
    config.chain_type = CHAIN_ID;
    let adapter = ArbitrumAdapter::new(config);
    let result = adapter.send_message(message(b"unsigned")).await;
    assert!(
        result.is_err(),
        "an adapter without a signer must refuse to send: {result:?}"
    );
}
