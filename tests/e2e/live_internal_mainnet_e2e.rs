//! Live-node internal mainnet E2E lane.
//!
//! This suite requires a real local node when `X3_E2E_REQUIRE_NODE=1`.
//! Without strict mode it will skip when no node is reachable on 127.0.0.1:9944.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use reqwest::Client;
use serde_json::{json, Value};
use sp_core::Pair;
use x3_asset_kernel_types::convert_amount;
use x3_fees::EconomicEngine;
use x3_packet_standard::packet::{Packet, PacketError, Sequence, StreamKey};
use x3_packet_standard::replay::ReplayGuard;
use x3_packet_standard::timeout::{evaluate as evaluate_timeout, TimeoutOutcome};
use x3_proof::{AgentIdentity, DeterministicHasher, ExecutionProof, ProofVerifier};
use x3_slash::{SlashConfig, SlashReason, SlashingEngine};

const RPC_HTTP: &str = "http://127.0.0.1:9944";

fn strict_live_required() -> bool {
    std::env::var("X3_E2E_REQUIRE_NODE")
        .map(|v| {
            let lower = v.to_ascii_lowercase();
            lower == "1" || lower == "true" || lower == "yes"
        })
        .unwrap_or(false)
}

fn node_is_reachable() -> bool {
    let addr: SocketAddr = "127.0.0.1:9944".parse().expect("valid socket");
    TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok()
}

fn chain_id(s: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let n = s.len().min(32);
    out[..n].copy_from_slice(&s[..n]);
    out
}

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

struct LiveRpc {
    client: Client,
}

impl LiveRpc {
    async fn maybe_new() -> TestResult<Option<Self>> {
        if node_is_reachable() {
            let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
            return Ok(Some(Self { client }));
        }

        if strict_live_required() {
            return Err("live node required but not reachable on 127.0.0.1:9944".into());
        }

        Ok(None)
    }

    async fn rpc(&self, method: &str, params: Value) -> TestResult<Value> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let res = self.client.post(RPC_HTTP).json(&payload).send().await?;
        let body: Value = res.json().await?;

        if let Some(err) = body.get("error") {
            return Err(format!("rpc {method} error: {err}").into());
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| format!("rpc {method} missing result").into())
    }

    async fn finalized_number(&self) -> TestResult<u64> {
        let hash = self
            .rpc("chain_getFinalizedHead", json!([]))
            .await?
            .as_str()
            .ok_or("chain_getFinalizedHead result not string")?
            .to_string();

        let header = self.rpc("chain_getHeader", json!([hash])).await?;
        let number_hex = header
            .get("number")
            .and_then(Value::as_str)
            .ok_or("chain_getHeader missing hex number")?;

        let stripped = number_hex.trim_start_matches("0x");
        Ok(u64::from_str_radix(stripped, 16)?)
    }

    async fn wait_for_finalized_advance(&self, from: u64, timeout: Duration) -> TestResult<u64> {
        let deadline = Instant::now() + timeout;
        loop {
            let now = self.finalized_number().await?;
            if now > from {
                return Ok(now);
            }
            if Instant::now() >= deadline {
                return Err(
                    format!("timeout waiting for finalized head to advance from {from}").into(),
                );
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
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

#[tokio::test]
async fn live_node_progress_and_required_methods() -> TestResult {
    let Some(rpc) = LiveRpc::maybe_new().await? else {
        println!("skipping live test: node not running and strict mode disabled");
        return Ok(());
    };

    let methods = rpc.rpc("rpc_methods", json!([])).await?;
    let names = methods
        .get("methods")
        .and_then(Value::as_array)
        .ok_or("rpc_methods missing methods array")?;
    let names: Vec<&str> = names.iter().filter_map(Value::as_str).collect();

    for required in [
        "x3_submitCrossVmTransaction",
        "x3_verify_signature",
        "x3_weight_meter",
    ] {
        assert!(
            names.contains(&required),
            "required rpc method missing: {required}"
        );
    }

    let start = rpc.finalized_number().await?;
    let end = rpc
        .wait_for_finalized_advance(start, Duration::from_secs(20))
        .await?;
    assert!(end > start, "finalized head did not advance");

    Ok(())
}

#[tokio::test]
async fn live_bridge_proof_crypto_and_full_accounting_paths() -> TestResult {
    let Some(rpc) = LiveRpc::maybe_new().await? else {
        println!("skipping live test: node not running and strict mode disabled");
        return Ok(());
    };

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
    ProofVerifier::verify_proof(&proof)?;

    let msg_hex = format!("0x{}", hex::encode(proof.proof_hash));
    let secret = [7u8; 32];
    let secret_hex = format!("0x{}", hex::encode(secret));
    let signer = sp_core::ed25519::Pair::from_seed(&secret);
    let pubkey_hex = format!("0x{}", hex::encode(signer.public().0));

    let signature_hex = rpc
        .rpc("x3_sign_ed25519", json!([msg_hex, secret_hex]))
        .await?
        .as_str()
        .ok_or("x3_sign_ed25519 did not return hex signature")?
        .to_string();

    let verified = rpc
        .rpc(
            "x3_verify_signature",
            json!([
                format!("0x{}", hex::encode(proof.proof_hash)),
                signature_hex,
                pubkey_hex,
                "ed25519"
            ]),
        )
        .await?
        .as_bool()
        .ok_or("x3_verify_signature did not return bool")?;
    assert!(verified, "node must verify bridge proof signature");

    let mut fee_engine = EconomicEngine::new(1_000);
    let mut slash_engine = SlashingEngine::new(SlashConfig::default());

    let mut locked_by_asset: BTreeMap<&'static str, u128> = BTreeMap::new();
    let mut minted_canonical_by_asset: BTreeMap<&'static str, u128> = BTreeMap::new();
    let mut slash_events = 0u32;

    let assets = [
        ("USDC", 6u8, 12u8, 50_000_000u128, 100_000_000u128, 30u128),
        ("SOL", 9u8, 12u8, 3_000_000_000u128, 10_000_000_000u128, 20u128),
        ("X3", 12u8, 18u8, 1_000_000_000_000u128, 5_000_000_000_000u128, 15u128),
        // This one should fail conversion and trigger slash/insurance path.
        ("BAD_DECIMAL", 6u8, 4u8, 1_234_567u128, 5_000_000u128, 0u128),
    ];

    for (symbol, from_dec, to_dec, lock_amount, mint_cap_daily, fee_bps) in assets {
        *locked_by_asset.entry(symbol).or_default() += lock_amount;

        let (burn_fee, validator_fee) =
            fee_engine.process_transaction(symbol.as_bytes().to_vec(), 0.82);
        let network_fee = burn_fee.saturating_add(validator_fee) as u128;
        let route_fee = lock_amount.saturating_mul(fee_bps) / 10_000;

        let after_fees = match lock_amount.checked_sub(route_fee.saturating_add(network_fee)) {
            Some(v) => v,
            None => {
                let bond = slash_engine.post_bond(
                    AgentIdentity {
                        pubkey: [0x55; 32],
                        ephemeral: false,
                    },
                    2_000_000,
                    100,
                    None,
                )?;
                let ev = slash_engine.execute_slash(
                    AgentIdentity {
                        pubkey: [0x55; 32],
                        ephemeral: false,
                    },
                    bond,
                    SlashReason::InvalidProof {
                        proof_hash: proof.proof_hash,
                    },
                    101,
                )?;
                fee_engine.slash_validator(ev.amount_slashed);
                slash_events += 1;
                continue;
            }
        };

        let Some(_minted_target) = convert_amount(after_fees, from_dec, to_dec) else {
            let bond = slash_engine.post_bond(
                AgentIdentity {
                    pubkey: [0x66; 32],
                    ephemeral: false,
                },
                2_000_000,
                110,
                None,
            )?;
            let ev = slash_engine.execute_slash(
                AgentIdentity {
                    pubkey: [0x66; 32],
                    ephemeral: false,
                },
                bond,
                SlashReason::InvalidProof {
                    proof_hash: proof.proof_hash,
                },
                111,
            )?;
            fee_engine.slash_validator(ev.amount_slashed);
            slash_events += 1;
            continue;
        };

        let total_minted = minted_canonical_by_asset
            .entry(symbol)
            .and_modify(|v| *v = v.saturating_add(after_fees))
            .or_insert(after_fees);
        assert!(
            *total_minted <= mint_cap_daily,
            "mint cap exceeded for {symbol}"
        );
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
        "slashing insurance pool should receive contributions from slash events"
    );
    assert!(slash_events >= 1, "expected at least one slash event in edge paths");

    Ok(())
}

#[tokio::test]
async fn live_timeout_expiry_failure_path() -> TestResult {
    let Some(rpc) = LiveRpc::maybe_new().await? else {
        println!("skipping live test: node not running and strict mode disabled");
        return Ok(());
    };

    let now = rpc.finalized_number().await?;
    let timeout_height = now.saturating_add(1);
    let packet = make_packet(1, timeout_height);

    let at_or_after_timeout = rpc
        .wait_for_finalized_advance(now, Duration::from_secs(20))
        .await?;

    let outcome = evaluate_timeout(&packet, at_or_after_timeout, 0);
    assert_eq!(outcome, TimeoutOutcome::ExpiredHeight);

    Ok(())
}

#[tokio::test]
async fn live_reordered_delivery_and_duplicate_ack_failures() -> TestResult {
    let Some(rpc) = LiveRpc::maybe_new().await? else {
        println!("skipping live test: node not running and strict mode disabled");
        return Ok(());
    };

    let now = rpc.finalized_number().await?;
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

    Ok(())
}
