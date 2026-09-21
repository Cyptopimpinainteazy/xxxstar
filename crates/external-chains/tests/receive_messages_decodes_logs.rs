//! `BaseAdapter::receive_messages` against a JSON-RPC endpoint.
//!
//! The adapter used to run `eth_getLogs`, throw the logs away and return
//! `Ok(vec![])` — an empty queue no matter what the chain said. It now decodes
//! the OP-Stack `SentMessage` event, and this test drives the whole path: HTTP
//! request, JSON-RPC response, log parsing, ABI decoding into a `ChainMessage`.
//!
//! The endpoint is a stub rather than a real chain: what is under test is the
//! adapter's request shape and decoding, and a stub can state a log the way a
//! node would (including one no node would emit, which is how the malformed case
//! is exercised). The `eth_getLogs` *request* the stub receives is asserted, so a
//! regression that stops filtering by address or topic fails here.

use sp_core::{keccak_256, H160, U256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use x3_external_chains::adapter::ChainAdapter;
use x3_external_chains::chains::arbitrum::{l2_to_l1_tx_topic, ArbitrumAdapter, ARBSYS_ADDRESS};
use x3_external_chains::chains::base::{BaseAdapter, L2_CROSS_DOMAIN_MESSENGER};
use x3_external_chains::{ChainConfig, ChainType};

/// The OP-Stack messenger predeploy every query must filter on.
fn messenger() -> H160 {
    L2_CROSS_DOMAIN_MESSENGER
}

fn sent_message_topic() -> [u8; 32] {
    keccak_256(b"SentMessage(address,address,uint256,uint256,uint256,bytes)")
}

/// `abi.encode(address sender, uint256 value, uint256 nonce, uint256 gas, bytes message)`
fn sent_message_data(sender: H160, value: U256, nonce: u64, gas: u64, message: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    let mut word = [0u8; 32];
    word[12..].copy_from_slice(sender.as_bytes());
    data.extend_from_slice(&word);
    data.extend_from_slice(&value.to_big_endian());
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&nonce.to_be_bytes());
    data.extend_from_slice(&word);
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&gas.to_be_bytes());
    data.extend_from_slice(&word);
    let mut word = [0u8; 32];
    word[31] = 160;
    data.extend_from_slice(&word);
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&(message.len() as u64).to_be_bytes());
    data.extend_from_slice(&word);
    data.extend_from_slice(message);
    data.extend_from_slice(&vec![0u8; (32 - message.len() % 32) % 32]);
    data
}

/// A JSON-RPC log entry, as `eth_getLogs` returns it.
fn log_json(target: H160, sender: H160, message: &[u8], data_override: Option<Vec<u8>>) -> String {
    let mut target_topic = [0u8; 32];
    target_topic[12..].copy_from_slice(target.as_bytes());
    let data = data_override
        .unwrap_or_else(|| sent_message_data(sender, U256::from(1_000u64), 7, 90_000, message));
    format!(
        r#"{{"address":"0x{}","topics":["0x{}","0x{}"],"data":"0x{}","blockNumber":"0x63","transactionHash":"0x{}","logIndex":"0x0"}}"#,
        hex::encode(messenger().as_bytes()),
        hex::encode(sent_message_topic()),
        hex::encode(target_topic),
        hex::encode(&data),
        hex::encode([0x44u8; 32])
    )
}

/// Start a stub JSON-RPC endpoint. Returns its URL and the requests it saw.
fn spawn_stub(logs_json: String) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("addr"));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_for_thread = Arc::clone(&seen);

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let read = stream.read(&mut chunk).unwrap_or(0);
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                let text = String::from_utf8_lossy(&buffer);
                if let Some(head_end) = text.find("\r\n\r\n") {
                    let content_length = text
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length: ")
                                .or_else(|| line.strip_prefix("Content-Length: "))
                        })
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if buffer.len() >= head_end + 4 + content_length {
                        break;
                    }
                }
            }

            let body = String::from_utf8_lossy(&buffer)
                .split("\r\n\r\n")
                .nth(1)
                .unwrap_or("")
                .to_string();
            seen_for_thread.lock().unwrap().push(body.clone());

            let result = if body.contains("eth_blockNumber") {
                "\"0x64\"".to_string()
            } else if body.contains("eth_getLogs") {
                logs_json.clone()
            } else if body.contains("eth_getBlockByNumber") {
                r#"{"timestamp":"0x6553f100"}"#.to_string()
            } else {
                "null".to_string()
            };
            let payload = format!(r#"{{"jsonrpc":"2.0","id":1,"result":{result}}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                payload.len(),
                payload
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    (url, seen)
}

fn adapter_for(url: &str) -> BaseAdapter {
    let mut config = ChainConfig::for_chain(ChainType::Base);
    config.rpc_url = url.as_bytes().to_vec();
    config.confirmations = 1;
    BaseAdapter::new(config)
}

#[tokio::test]
async fn receive_messages_decodes_a_sent_message_log() {
    let target = H160([0xaa; 20]);
    let sender = H160([0xbb; 20]);
    let (url, seen) = spawn_stub(format!(
        "[{}]",
        log_json(target, sender, b"hello from base", None)
    ));
    let adapter = adapter_for(&url);

    let messages = adapter
        .receive_messages()
        .await
        .expect("a well-formed log decodes");
    assert_eq!(messages.len(), 1, "one log, one message");

    let message = &messages[0];
    assert_eq!(message.source_chain, 8453);
    assert_eq!(message.dest_chain, 1);
    assert_eq!(message.sender, sender);
    assert_eq!(message.recipient, target);
    assert_eq!(message.nonce, 7);
    assert_eq!(message.gas_limit, 90_000);
    assert_eq!(message.value, U256::from(1_000u64));
    assert_eq!(message.payload, b"hello from base".to_vec());
    assert_eq!(message.timestamp, 0x6553_f100);

    // The query has to name the configured messenger and the event signature: a
    // request that filters on neither would decode whatever the chain emitted.
    let requests = seen.lock().unwrap();
    let logs_request = requests
        .iter()
        .find(|body| body.contains("eth_getLogs"))
        .expect("the adapter asked for logs");
    assert!(
        logs_request.contains(&hex::encode(messenger().as_bytes())),
        "eth_getLogs must filter by the configured messenger: {logs_request}"
    );
    assert!(
        logs_request.contains(&hex::encode(sent_message_topic())),
        "eth_getLogs must filter by the SentMessage topic: {logs_request}"
    );
    assert!(
        logs_request.contains("\"fromBlock\"") && logs_request.contains("\"toBlock\""),
        "eth_getLogs must bound its block range: {logs_request}"
    );
}

#[tokio::test]
async fn a_malformed_log_is_refused_rather_than_skipped() {
    // A log with the right signature and a truncated `message`.
    let mut data = sent_message_data(H160([0xbb; 20]), U256::from(1u64), 1, 1, b"payload");
    data.truncate(100);
    let (url, _seen) = spawn_stub(format!(
        "[{}]",
        log_json(H160([0xaa; 20]), H160([0xbb; 20]), b"payload", Some(data))
    ));
    let adapter = adapter_for(&url);

    let result = adapter.receive_messages().await;
    assert!(
        result.is_err(),
        "a truncated log must be an error, not a short queue: {result:?}"
    );
}

#[tokio::test]
async fn the_query_targets_the_messenger_predeploy_not_the_config() {
    // `ChainConfig::for_chain` derives its bridge address from a hash, so a query
    // that filtered on the config would return an empty log set for a chain that
    // does have messages. The emitter is the OP-Stack predeploy, and that is what
    // the request must name — whatever the config says.
    let (url, seen) = spawn_stub("[]".to_string());
    let mut config = ChainConfig::for_chain(ChainType::Base);
    config.rpc_url = url.as_bytes().to_vec();
    config.confirmations = 1;
    config.bridge_contract = H160([0x5f; 20]); // deliberately wrong
    let adapter = BaseAdapter::new(config);

    let messages = adapter
        .receive_messages()
        .await
        .expect("empty is not an error");
    assert!(messages.is_empty());
    let requests = seen.lock().unwrap();
    let logs_request = requests
        .iter()
        .find(|body| body.contains("eth_getLogs"))
        .expect("the adapter asked for logs");
    assert!(
        logs_request.contains(&hex::encode(messenger().as_bytes())),
        "the filter must be the predeploy: {logs_request}"
    );
    assert!(
        !logs_request.contains(&hex::encode([0x5f; 20])),
        "the filter must not be the configured (wrong) address: {logs_request}"
    );
}

#[tokio::test]
async fn a_chain_with_no_logs_answers_with_no_messages() {
    // The honest empty: the query ran, and the chain had nothing.
    let (url, _seen) = spawn_stub("[]".to_string());
    let adapter = adapter_for(&url);
    let messages = adapter
        .receive_messages()
        .await
        .expect("empty is not an error");
    assert!(messages.is_empty());
}

// ── Arbitrum ────────────────────────────────────────────────────────────────

/// `abi.encode(address caller, uint256 arbBlockNum, uint256 ethBlockNum,
///             uint256 timestamp, uint256 callvalue, bytes data)`
fn l2_to_l1_data(caller: H160, timestamp: u64, callvalue: U256, data: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    let mut word = [0u8; 32];
    word[12..].copy_from_slice(caller.as_bytes());
    payload.extend_from_slice(&word);
    payload.extend_from_slice(&[0u8; 32]); // arbBlockNum
    payload.extend_from_slice(&[0u8; 32]); // ethBlockNum
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&timestamp.to_be_bytes());
    payload.extend_from_slice(&word);
    payload.extend_from_slice(&callvalue.to_big_endian());
    let mut word = [0u8; 32];
    word[31] = 192; // six head words
    payload.extend_from_slice(&word);
    let mut word = [0u8; 32];
    word[24..].copy_from_slice(&(data.len() as u64).to_be_bytes());
    payload.extend_from_slice(&word);
    payload.extend_from_slice(data);
    payload.extend_from_slice(&vec![0u8; (32 - data.len() % 32) % 32]);
    payload
}

fn l2_to_l1_log_json(destination: H160, position: u64, payload: Vec<u8>) -> String {
    let mut destination_topic = [0u8; 32];
    destination_topic[12..].copy_from_slice(destination.as_bytes());
    let mut position_topic = [0u8; 32];
    position_topic[24..].copy_from_slice(&position.to_be_bytes());
    format!(
        r#"{{"address":"0x{}","topics":["0x{}","0x{}","0x{}","0x{}"],"data":"0x{}","blockNumber":"0x1e","transactionHash":"0x{}","logIndex":"0x0"}}"#,
        hex::encode(ARBSYS_ADDRESS.as_bytes()),
        hex::encode(l2_to_l1_tx_topic()),
        hex::encode(destination_topic),
        hex::encode([0x77u8; 32]),
        hex::encode(position_topic),
        hex::encode(&payload),
        hex::encode([0x44u8; 32])
    )
}

fn arbitrum_adapter_for(url: &str) -> ArbitrumAdapter {
    let mut config = ChainConfig::for_chain(ChainType::Arbitrum);
    config.rpc_url = url.as_bytes().to_vec();
    config.confirmations = 1;
    ArbitrumAdapter::new(config)
}

#[tokio::test]
async fn arbitrum_receive_messages_decodes_an_l2_to_l1_tx_log() {
    let caller = H160([0x11; 20]);
    let destination = H160([0x22; 20]);
    let payload = l2_to_l1_data(caller, 1_700_000_456, U256::from(555u64), b"to L1");
    let (url, seen) = spawn_stub(format!(
        "[{}]",
        l2_to_l1_log_json(destination, 12_345, payload)
    ));
    let adapter = arbitrum_adapter_for(&url);

    let messages = adapter.receive_messages().await.expect("decodes");
    assert_eq!(messages.len(), 1);
    let message = &messages[0];
    assert_eq!(message.source_chain, 42_161);
    assert_eq!(message.dest_chain, 1);
    assert_eq!(message.sender, caller);
    assert_eq!(message.recipient, destination);
    assert_eq!(
        message.nonce, 12_345,
        "position is the identifier this event carries"
    );
    assert_eq!(message.value, U256::from(555u64));
    assert_eq!(message.payload, b"to L1".to_vec());
    assert_eq!(message.timestamp, 1_700_000_456);
    assert_eq!(message.gas_limit, 0, "the event states no gas limit");

    // Filtered by ArbSys, the topic, and a bounded range — not by a configurable
    // address that could be wrong.
    let requests = seen.lock().unwrap();
    let logs_request = requests
        .iter()
        .find(|body| body.contains("eth_getLogs"))
        .expect("the adapter asked for logs");
    assert!(
        logs_request.contains(&hex::encode(ARBSYS_ADDRESS.as_bytes())),
        "eth_getLogs must filter by ArbSys: {logs_request}"
    );
    assert!(
        logs_request.contains(&hex::encode(l2_to_l1_tx_topic())),
        "eth_getLogs must filter by the L2ToL1Tx topic: {logs_request}"
    );
    assert!(logs_request.contains("\"fromBlock\"") && logs_request.contains("\"toBlock\""));
}

#[tokio::test]
async fn arbitrum_refuses_a_malformed_l2_to_l1_tx() {
    let mut payload = l2_to_l1_data(H160([0x11; 20]), 1, U256::zero(), b"payload");
    payload.truncate(120);
    let (url, _seen) = spawn_stub(format!(
        "[{}]",
        l2_to_l1_log_json(H160([0x22; 20]), 1, payload)
    ));
    let adapter = arbitrum_adapter_for(&url);
    assert!(
        adapter.receive_messages().await.is_err(),
        "a truncated L2ToL1Tx must be an error, not a short queue"
    );
}
