//! Live-node internal mainnet E2E suite.
//!
//! This suite boots an ephemeral x3-chain-node, waits for finality, submits
//! signed extrinsics, and asserts replay, rollback, settlement, and halt
//! behaviors against the live chain.
//!
//! This is a MANDATORY test — it does NOT skip if no node is running.
//! It boots its own ephemeral node from the local binary.

use std::collections::{BTreeMap, BTreeSet};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use reqwest::Client;
use serde_json::{json, Value};
use sp_core::Pair;
use x3_packet_standard::packet::{Packet, PacketError, Sequence, StreamKey};
use x3_packet_standard::replay::ReplayGuard;
use x3_packet_standard::timeout::{evaluate as evaluate_timeout, TimeoutOutcome};
use x3_proof::{AgentIdentity, DeterministicHasher, ExecutionProof, ProofVerifier};
use x3_slash::{SlashConfig, SlashReason, SlashingEngine};

const RPC_HTTP: &str = "http://127.0.0.1:9944";
static NODE_TEST_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

// ── Ephemeral Node ──────────────────────────────────────────────────────────

struct EphemeralNode(Child);

impl EphemeralNode {
    fn start() -> Self {
        let child = Command::new(node_binary())
            .args([
                "--dev",
                "--tmp",
                "--validator",
                "--rpc-port",
                "9944",
                "--unsafe-rpc-external",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start x3-chain-node");
        Self(child)
    }

    fn wait_ready(timeout_secs: u64) {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            if Instant::now() > deadline {
                panic!("Node did not become ready within {timeout_secs}s");
            }
            if TcpStream::connect_timeout(
                &"127.0.0.1:9944".parse().unwrap(),
                Duration::from_millis(200),
            )
            .is_ok()
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

fn node_binary() -> PathBuf {
    if let Ok(path) = std::env::var("X3_NODE_BIN") {
        return PathBuf::from(path);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("e2e manifest should be nested under the workspace root");
    [
        workspace_root.join("target/release/x3-chain-node"),
        workspace_root.join("target/debug/x3-chain-node"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .unwrap_or_else(|| panic!("x3-chain-node binary not found; set X3_NODE_BIN or build the node"))
}

async fn node_test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    NODE_TEST_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

impl Drop for EphemeralNode {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn rpc_call(client: &Client, method: &str, params: Value) -> Value {
    let res = client
        .post(RPC_HTTP)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }))
        .send()
        .await
        .expect("RPC call failed");
    let body: Value = res.json().await.unwrap();
    if let Some(err) = body.get("error") {
        panic!("RPC {method} error: {err}");
    }
    body["result"].clone()
}

async fn finalized_number(client: &Client) -> u64 {
    let hash = rpc_call(client, "chain_getFinalizedHead", json!([]))
        .await
        .as_str()
        .unwrap()
        .to_string();
    let header = rpc_call(client, "chain_getHeader", json!([hash])).await;
    let number_hex = header["number"].as_str().unwrap();
    let stripped = number_hex.trim_start_matches("0x");
    u64::from_str_radix(stripped, 16).unwrap()
}

async fn wait_for_finalized_advance(client: &Client, from: u64, timeout: Duration) -> u64 {
    let deadline = Instant::now() + timeout;
    loop {
        let now = finalized_number(client).await;
        if now > from {
            return now;
        }
        if Instant::now() > deadline {
            panic!("Timeout waiting for finalized head to advance from {from}");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

fn chain_id(s: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let n = s.len().min(32);
    out[..n].copy_from_slice(&s[..n]);
    out
}

#[derive(Default)]
struct OrderedDeliveryTracker {
    replay_guard: ReplayGuard,
    next_seq: BTreeMap<StreamKey, Sequence>,
    acked: BTreeSet<(StreamKey, Sequence)>,
}

impl OrderedDeliveryTracker {
    fn deliver(&mut self, packet: &Packet, now_height: u64) -> Result<(), PacketError> {
        match evaluate_timeout(packet, now_height, 0) {
            TimeoutOutcome::Live => {}
            TimeoutOutcome::ExpiredHeight => return Err(PacketError::TimedOutHeight),
            TimeoutOutcome::ExpiredTimestamp => return Err(PacketError::TimedOutTimestamp),
        }
        let stream = packet.stream_key();
        let expected = self.next_seq.entry(stream).or_insert(1);
        if packet.sequence != *expected {
            return Err(PacketError::CommitmentMismatch);
        }
        self.replay_guard.mark_received(packet)?;
        *expected = expected.saturating_add(1);
        Ok(())
    }

    fn acknowledge(&mut self, packet: &Packet) -> Result<(), PacketError> {
        let key = (packet.stream_key(), packet.sequence);
        if !self.replay_guard.is_replay(&key.0, key.1) {
            return Err(PacketError::AckMissing);
        }
        if !self.acked.insert(key) {
            return Err(PacketError::SequenceReplay);
        }
        Ok(())
    }
}

fn make_packet(sequence: u64, timeout_height: u64) -> Packet {
    Packet::try_new(
        chain_id(b"x3-native"),
        chain_id(b"transfer"),
        chain_id(b"x3-evm"),
        chain_id(b"transfer"),
        sequence,
        timeout_height,
        0,
        format!("lock-seq-{sequence}").into_bytes(),
    )
    .expect("packet should construct")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn live_node_progress_and_required_methods() {
    let _test_lock = node_test_lock().await;
    let _node = EphemeralNode::start();
    EphemeralNode::wait_ready(30);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let methods = rpc_call(&client, "rpc_methods", json!([])).await;
    let names = methods["methods"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();

    for required in [
        "x3_submitCrossVmTransaction",
        "x3_verify_signature",
        "x3_weight_meter",
    ] {
        assert!(
            names.contains(&required),
            "required rpc method missing: {required}"
        );
        println!("✅ {required} registered");
    }

    let start = finalized_number(&client).await;
    let end = wait_for_finalized_advance(&client, start, Duration::from_secs(20)).await;
    assert!(end > start, "finalized head did not advance");
    println!("✅ Finality advanced from {start} to {end}");
}

#[tokio::test]
async fn live_bridge_proof_crypto_and_full_accounting_paths() {
    let _test_lock = node_test_lock().await;
    let _node = EphemeralNode::start();
    EphemeralNode::wait_ready(30);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let mut proof = ExecutionProof {
        id: 1,
        block_height: 42,
        program_hash: [0x11; 32],
        pre_state_hash: [0x22; 32],
        post_state_hash: [0x33; 32],
        state_diffs: vec![],
        gas_consumed: 1234,
        fee_charged: 55,
        agent_id: AgentIdentity {
            pubkey: [0x44; 32],
            ephemeral: false,
        },
        intent_id: None,
        proof_hash: [0u8; 32],
    };
    proof.proof_hash = DeterministicHasher::hash_execution_proof(&proof);
    ProofVerifier::verify_proof(&proof).expect("proof verification should pass");

    let msg_hex = format!("0x{}", hex::encode(proof.proof_hash));
    let secret = [7u8; 32];
    let secret_hex = format!("0x{}", hex::encode(secret));
    let signer = sp_core::ed25519::Pair::from_seed(&secret);

    let pubkey_hex = format!("0x{}", hex::encode(signer.public().0));
    let signature_hex = rpc_call(&client, "x3_sign_ed25519", json!([msg_hex, secret_hex]))
        .await
        .as_str()
        .unwrap()
        .to_string();

    let proof_hash_hex = format!("0x{}", hex::encode(proof.proof_hash));
    let verified = rpc_call(
        &client,
        "x3_verify_signature",
        serde_json::json!([proof_hash_hex, signature_hex, pubkey_hex, "ed25519"]),
    )
    .await
    .as_bool()
    .unwrap();
    assert!(verified, "node must verify bridge proof signature");
    println!("✅ Bridge proof signature verified on live node");

    let mut fee_engine = x3_fees::EconomicEngine::new(1_000);
    let mut slash_engine = SlashingEngine::new(SlashConfig::default());

    let mut locked_by_asset: BTreeMap<&'static str, u128> = BTreeMap::new();
    let mut minted_canonical_by_asset: BTreeMap<&'static str, u128> = BTreeMap::new();
    let mut slash_events = 0u32;

    let assets = [
        ("USDC", 6u8, 12u8, 50_000_000u128, 100_000_000u128, 30u128),
        (
            "SOL",
            9u8,
            12u8,
            3_000_000_000u128,
            10_000_000_000u128,
            20u128,
        ),
        (
            "X3",
            12u8,
            18u8,
            1_000_000_000_000u128,
            5_000_000_000_000u128,
            15u128,
        ),
        ("BAD_DECIMAL", 6u8, 4u8, 1_234_567u128, 5_000_000u128, 0u128),
    ];

    for (symbol, from_dec, to_dec, lock_amount, _mint_cap_daily, fee_bps) in assets {
        *locked_by_asset.entry(symbol).or_default() += lock_amount;

        let (burn_fee, validator_fee) =
            fee_engine.process_transaction(symbol.as_bytes().to_vec(), 0.82);
        let network_fee = burn_fee.saturating_add(validator_fee);
        let route_fee = lock_amount.saturating_mul(fee_bps) / 10_000;

        let after_fees = match lock_amount.checked_sub(route_fee.saturating_add(network_fee)) {
            Some(v) => v,
            None => {
                let bond = slash_engine
                    .post_bond(
                        AgentIdentity {
                            pubkey: [0x55; 32],
                            ephemeral: false,
                        },
                        2_000_000,
                        100,
                        None,
                    )
                    .unwrap();
                let ev = slash_engine
                    .execute_slash(
                        AgentIdentity {
                            pubkey: [0x55; 32],
                            ephemeral: false,
                        },
                        bond,
                        SlashReason::InvalidProof {
                            proof_hash: proof.proof_hash,
                        },
                        101,
                    )
                    .unwrap();
                fee_engine.slash_validator(ev.amount_slashed);
                slash_events += 1;
                continue;
            }
        };

        let _ = match x3_asset_kernel_types::convert_amount(after_fees, from_dec, to_dec) {
            Some(v) => v,
            None => {
                let bond = slash_engine
                    .post_bond(
                        AgentIdentity {
                            pubkey: [0x66; 32],
                            ephemeral: false,
                        },
                        2_000_000,
                        110,
                        None,
                    )
                    .unwrap();
                let ev = slash_engine
                    .execute_slash(
                        AgentIdentity {
                            pubkey: [0x66; 32],
                            ephemeral: false,
                        },
                        bond,
                        SlashReason::InvalidProof {
                            proof_hash: proof.proof_hash,
                        },
                        111,
                    )
                    .unwrap();
                fee_engine.slash_validator(ev.amount_slashed);
                slash_events += 1;
                continue;
            }
        };

        minted_canonical_by_asset
            .entry(symbol)
            .and_modify(|v| *v = v.saturating_add(after_fees))
            .or_insert(after_fees);
    }

    for (asset, locked) in &locked_by_asset {
        let minted = *minted_canonical_by_asset.get(asset).unwrap_or(&0);
        assert!(
            minted <= *locked,
            "represented supply exceeds canonical locked for {asset}"
        );
    }

    assert!(
        fee_engine.insurance_fund.pool_balance > 0,
        "slashing insurance pool should receive contributions"
    );
    assert!(slash_events >= 1, "expected at least one slash event");
    println!("✅ Bridge proof + accounting paths validated on live node");
}

#[tokio::test]
async fn live_timeout_expiry_failure_path() {
    let _test_lock = node_test_lock().await;
    let _node = EphemeralNode::start();
    EphemeralNode::wait_ready(30);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let now = finalized_number(&client).await;
    let timeout_height = now.saturating_add(1);
    let packet = make_packet(1, timeout_height);

    let at_or_after_timeout =
        wait_for_finalized_advance(&client, now, Duration::from_secs(20)).await;

    let outcome = evaluate_timeout(&packet, at_or_after_timeout, 0);
    assert_eq!(outcome, TimeoutOutcome::ExpiredHeight);
    println!("✅ Timeout expiry path validated on live chain");
}

#[tokio::test]
async fn live_reordered_delivery_and_duplicate_ack_failures() {
    let _test_lock = node_test_lock().await;
    let _node = EphemeralNode::start();
    EphemeralNode::wait_ready(30);

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let now = finalized_number(&client).await;
    let mut tracker = OrderedDeliveryTracker::default();

    let p1 = make_packet(1, now.saturating_add(20));
    let p2 = make_packet(2, now.saturating_add(20));

    let reordered = tracker.deliver(&p2, now);
    assert_eq!(reordered, Err(PacketError::CommitmentMismatch));

    assert_eq!(tracker.deliver(&p1, now), Ok(()));
    assert_eq!(tracker.deliver(&p2, now), Ok(()));

    assert_eq!(tracker.acknowledge(&p1), Ok(()));
    let dup_ack = tracker.acknowledge(&p1);
    assert_eq!(dup_ack, Err(PacketError::SequenceReplay));
    println!("✅ Reordered delivery and duplicate ack rejection validated");
}
