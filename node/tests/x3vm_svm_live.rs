use codec::{Decode, Encode};
use serde_json::Value;
use sp_core::{crypto::Ss58Codec, Pair as _, H256};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use x3_atomic_swap::intent::{
    AtomicIntent, AtomicSwapStatus, ChainKind, FinalityLevel, FinalityRequirement, RefundPath,
    RouteMode,
};
use x3_atomic_swap::{
    LiveX3VmAdapter, NativeX3NodeTransport, RpcClient, VmType, X3NodeTransportConfig, X3VmAdapter,
};
use x3_chain_node::x3vm_runtime_signer::X3RuntimeSigner;
use x3_chain_runtime::{AccountId, Signature};

const X3_RPC: &str = "http://127.0.0.1:19946";

struct NodeGuard(Child);

impl Drop for NodeGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn dev_uri(name: &str) -> String {
    ["//", name].concat()
}

fn dev_account(name: &str) -> AccountId {
    let pair = sp_core::sr25519::Pair::from_string(&dev_uri(name), None).expect("dev key");
    <Signature as Verify>::Signer::from(pair.public()).into_account()
}

fn spawn_x3_node() -> NodeGuard {
    let child = Command::new(env!("CARGO_BIN_EXE_x3-chain-node"))
        .args([
            "--dev",
            "--tmp",
            "--rpc-port",
            "19946",
            "--port",
            "30381",
            "--no-telemetry",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn x3-chain-node --dev");
    NodeGuard(child)
}

fn wait_x3_rpc(timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let mut rpc = RpcClient::new(X3_RPC.into(), 0);
        if rpc.call("system_health", Vec::new()).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("X3 dev node RPC did not become ready");
}

fn submit_x3(signed: &str) -> String {
    let mut rpc = RpcClient::new(X3_RPC.into(), 0);
    rpc.call(
        "author_submitExtrinsic",
        vec![Value::String(signed.to_string())],
    )
    .expect("submit X3 extrinsic")
    .result
    .and_then(|v| v.as_str().map(ToOwned::to_owned))
    .expect("X3 transaction hash")
}

fn x3_header_number(rpc: &mut RpcClient, hash: &str) -> u64 {
    let header = rpc
        .call("chain_getHeader", vec![Value::String(hash.to_string())])
        .expect("X3 header")
        .result
        .expect("X3 header result");
    let raw = header
        .get("number")
        .and_then(Value::as_str)
        .expect("header number");
    u64::from_str_radix(raw.trim_start_matches("0x"), 16).expect("hex block number")
}

fn x3_block_hash_at(rpc: &mut RpcClient, number: u64) -> Option<String> {
    rpc.call("chain_getBlockHash", vec![Value::Number(number.into())])
        .expect("X3 block hash")
        .result
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
}

fn x3_block_contains(rpc: &mut RpcClient, hash: &str, signed: &str) -> bool {
    rpc.call("chain_getBlock", vec![Value::String(hash.to_string())])
        .expect("X3 finalized block")
        .result
        .as_ref()
        .and_then(|v| v.pointer("/block/extrinsics"))
        .and_then(Value::as_array)
        .map(|xs| xs.iter().any(|x| x.as_str() == Some(signed)))
        .unwrap_or(false)
}

fn wait_x3_finalized(signed: &str, timeout: Duration) -> String {
    let started = Instant::now();
    let mut rpc = RpcClient::new(X3_RPC.into(), 0);
    let mut next_number: Option<u64> = None;
    while started.elapsed() < timeout {
        if let Some(head) = rpc
            .call("chain_getFinalizedHead", Vec::new())
            .expect("X3 finalized head")
            .result
            .and_then(|v| v.as_str().map(ToOwned::to_owned))
        {
            let head_number = x3_header_number(&mut rpc, &head);
            let start = next_number.unwrap_or(head_number);
            if head_number >= start {
                for number in start..=head_number {
                    if let Some(hash) = x3_block_hash_at(&mut rpc, number) {
                        if x3_block_contains(&mut rpc, &hash, signed) {
                            return hash;
                        }
                    }
                }
            }
            next_number = Some(head_number + 1);
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("X3 extrinsic not observed in finalized block");
}

fn svm_rpc_url() -> String {
    std::env::var("X3_TEST_SVM_RPC").expect("X3_TEST_SVM_RPC")
}

fn svm_call(method: &str, params: Vec<Value>) -> Value {
    let mut rpc = RpcClient::new(svm_rpc_url(), 0);
    rpc.call(method, params)
        .unwrap_or_else(|e| panic!("SVM RPC {method} failed: {e}"))
        .result
        .unwrap_or(Value::Null)
}

fn svm_finalized_slot() -> u64 {
    svm_call(
        "getSlot",
        vec![serde_json::json!({"commitment":"finalized"})],
    )
    .as_u64()
    .expect("finalized Solana slot")
}

fn wait_svm_finalized(signature: &str, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let value = svm_call(
            "getSignatureStatuses",
            vec![
                serde_json::json!([signature]),
                serde_json::json!({"searchTransactionHistory":true}),
            ],
        );
        if let Some(status) = value
            .get("value")
            .and_then(Value::as_array)
            .and_then(|xs| xs.first())
            .filter(|v| !v.is_null())
        {
            assert!(
                status.get("err").is_none_or(Value::is_null),
                "Solana transaction failed: {status}"
            );
            if status
                .get("confirmationStatus")
                .and_then(Value::as_str)
                == Some("finalized")
            {
                return;
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("Solana signature did not reach finalized commitment: {signature}");
}

fn wait_svm_slot_past(slot: u64, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if svm_finalized_slot() > slot {
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
    panic!("Solana finalized slot did not advance past timeout slot {slot}");
}

fn finalized_head() -> String {
    let mut rpc = RpcClient::new(X3_RPC.into(), 0);
    rpc.call("chain_getFinalizedHead", Vec::new())
        .expect("X3 finalized head")
        .result
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .expect("X3 finalized head hash")
}

fn intent_state_storage_key(intent_id: H256) -> String {
    let mut key =
        frame_support::storage::storage_prefix(b"X3SettlementEngine", b"IntentStates").to_vec();
    let encoded = intent_id.encode();
    key.extend_from_slice(&sp_core::hashing::blake2_128(&encoded));
    key.extend_from_slice(&encoded);
    format!("0x{}", hex::encode(key))
}

fn intent_state_at(
    intent_id: H256,
    block_hash: &str,
) -> pallet_x3_settlement_engine::IntentState {
    let mut rpc = RpcClient::new(X3_RPC.into(), 0);
    let value = rpc
        .call(
            "state_getStorage",
            vec![
                Value::String(intent_state_storage_key(intent_id)),
                Value::String(block_hash.to_string()),
            ],
        )
        .expect("state_getStorage")
        .result
        .expect("intent state storage result");
    let raw = value.as_str().expect("intent state storage hex");
    let bytes = hex::decode(raw.trim_start_matches("0x")).expect("decode intent state hex");
    pallet_x3_settlement_engine::IntentState::decode(&mut &bytes[..]).expect("decode IntentState")
}

fn wait_for_finalized_refund(intent_id: H256, timeout: Duration) -> String {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let head = finalized_head();
        if matches!(
            intent_state_at(intent_id, &head),
            pallet_x3_settlement_engine::IntentState::Refunded
        ) {
            return head;
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("X3 intent did not reach Refunded in GRANDPA-finalized state");
}

fn run_svm_broadcast_expect_failure(action: &[String], payer_keypair: &str) -> Value {
    let bin = std::env::var("X3_SVM_BROADCAST_BIN").expect("X3_SVM_BROADCAST_BIN");
    let program_id = std::env::var("X3_TEST_SVM_PROGRAM_ID").expect("X3_TEST_SVM_PROGRAM_ID");
    let mut command = Command::new(bin);
    command
        .arg("--rpc")
        .arg(svm_rpc_url())
        .arg("--program-id")
        .arg(program_id)
        .arg("--payer-keypair")
        .arg(payer_keypair)
        .arg("--json");
    for arg in action {
        command.arg(arg);
    }
    let output = command.output().expect("run x3-svm-broadcast failure case");
    let stdout = String::from_utf8(output.stdout).expect("SVM broadcaster stdout utf8");
    assert!(
        !output.status.success(),
        "SVM broadcaster unexpectedly succeeded: {stdout}"
    );
    let value: Value = serde_json::from_str(stdout.trim()).expect("SVM broadcaster failure JSON");
    assert_eq!(value.get("status").and_then(Value::as_str), Some("error"));
    value
}

fn run_svm_broadcast(action: &[String], payer_keypair: &str) -> Value {
    let bin = std::env::var("X3_SVM_BROADCAST_BIN").expect("X3_SVM_BROADCAST_BIN");
    let program_id = std::env::var("X3_TEST_SVM_PROGRAM_ID").expect("X3_TEST_SVM_PROGRAM_ID");
    let mut command = Command::new(bin);
    command
        .arg("--rpc")
        .arg(svm_rpc_url())
        .arg("--program-id")
        .arg(program_id)
        .arg("--payer-keypair")
        .arg(payer_keypair)
        .arg("--json");
    for arg in action {
        command.arg(arg);
    }
    let output = command.output().expect("run x3-svm-broadcast");
    let stdout = String::from_utf8(output.stdout).expect("SVM broadcaster stdout utf8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "SVM broadcaster failed: stdout={stdout} stderr={stderr}"
    );
    serde_json::from_str(stdout.trim()).expect("SVM broadcaster JSON")
}

fn atomic_intent(local_id: u64, preimage: [u8; 32]) -> AtomicIntent {
    AtomicIntent {
        intent_id: local_id,
        source_chain: ChainKind::X3,
        destination_chain: ChainKind::Solana,
        source_asset: "X3".into(),
        destination_asset: "SOL".into(),
        amount_in: 1_000_000,
        min_amount_out: 1,
        receiver: dev_account("Bob").to_ss58check(),
        hashlock: sp_core::hashing::sha2_256(&preimage),
        source_timeout: u64::MAX / 2,
        destination_timeout: u64::MAX / 2,
        finality_requirements: vec![
            FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            },
            FinalityRequirement {
                chain: ChainKind::Solana,
                level: FinalityLevel::Bft,
            },
        ],
        refund_path: RefundPath {
            chain: ChainKind::X3,
            address: dev_account("Alice").to_ss58check(),
            asset: None,
        },
        route_mode: RouteMode::DirectHtlc,
        max_slippage_bps: 100,
        relayer_quorum_requirement: 1,
        status: AtomicSwapStatus::Pending,
        intent_hash: [0u8; 32],
    }
}

#[test]
#[ignore = "requires solana-test-validator + real SBF program/client and boots a real X3 dev node"]
fn real_x3vm_svm_lock_claim_atomic_lifecycle() {
    let payer_keypair =
        std::env::var("X3_TEST_SVM_PAYER_KEYPAIR").expect("X3_TEST_SVM_PAYER_KEYPAIR");
    let claimant_keypair =
        std::env::var("X3_TEST_SVM_CLAIMANT_KEYPAIR").expect("X3_TEST_SVM_CLAIMANT_KEYPAIR");
    let payer_pubkey = std::env::var("X3_TEST_SVM_PAYER_PUBKEY").expect("payer pubkey");
    let claimant_pubkey =
        std::env::var("X3_TEST_SVM_CLAIMANT_PUBKEY").expect("claimant pubkey");

    let _x3 = spawn_x3_node();
    wait_x3_rpc(Duration::from_secs(360));

    let local_id = 2001u64;
    let preimage = [0x6bu8; 32];
    let hashlock = sp_core::hashing::sha2_256(&preimage);
    let chain_id = String::from("x3-local");
    let alice_uri = dev_uri("Alice");
    let primary = X3RuntimeSigner::from_uri(chain_id.clone(), X3_RPC.into(), &alice_uri)
        .expect("X3 primary signer");
    let second_leg = X3RuntimeSigner::from_uri(chain_id.clone(), X3_RPC.into(), &alice_uri)
        .expect("X3 second-leg signer");

    let prepared = primary
        .prepare_create_intent(
            dev_account("Bob"),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            H256::from(hashlock),
            Some(300),
        )
        .expect("prepare X3 intent");
    assert!(!submit_x3(&prepared.signed_extrinsic).is_empty());
    let finalized_head = wait_x3_finalized(&prepared.signed_extrinsic, Duration::from_secs(180));
    let finalized_hash = H256::from_slice(
        &hex::decode(finalized_head.trim_start_matches("0x")).expect("decode finalized head"),
    );
    let runtime_intent_id = primary
        .resolve_intent_id(&prepared, finalized_hash)
        .expect("resolve runtime intent id");
    primary.bind_intent(local_id, runtime_intent_id).unwrap();
    second_leg.bind_intent(local_id, runtime_intent_id).unwrap();

    let x3_transport = NativeX3NodeTransport::new(
        X3NodeTransportConfig {
            chain_id: chain_id.clone(),
            rpc_url: X3_RPC.into(),
            finality_poll_attempts: 480,
            finality_poll_delay_ms: 500,
            expected_block_time_ms: 6_000,
        },
        primary,
    );
    let x3_adapter = LiveX3VmAdapter::new(
        chain_id,
        b"x3-native-svm-escrow".to_vec(),
        x3_transport,
    );
    let intent = atomic_intent(local_id, preimage);

    let x3_lock = x3_adapter.lock(&intent).expect("real X3 lock");
    assert_eq!(x3_lock.vm_type, VmType::X3Vm);
    assert!(x3_adapter.finality_status(&x3_lock.tx_id).unwrap().finalized);

    let leg1 = second_leg
        .sign_lock_escrow_leg(
            runtime_intent_id,
            1,
            pallet_x3_settlement_engine::ExternalChainId::X3Native,
            1_000_000,
            b"x3-native-svm-leg1".to_vec(),
        )
        .expect("sign X3 second leg");
    assert!(!submit_x3(&leg1).is_empty());
    wait_x3_finalized(&leg1, Duration::from_secs(180));

    let swap_id = [0x77u8; 32];
    let timeout_slot = svm_finalized_slot().saturating_add(1_000);
    let lock_args = vec![
        "lock".to_string(),
        "--swap-id".to_string(),
        hex::encode(swap_id),
        "--claimant".to_string(),
        claimant_pubkey.clone(),
        "--refund-authority".to_string(),
        payer_pubkey,
        "--hashlock".to_string(),
        hex::encode(hashlock),
        "--amount".to_string(),
        "500000".to_string(),
        "--timeout-slots".to_string(),
        timeout_slot.to_string(),
    ];
    let svm_lock = run_svm_broadcast(&lock_args, &payer_keypair);
    assert_eq!(svm_lock.get("status").and_then(Value::as_str), Some("submitted"));
    let lock_signature = svm_lock
        .get("signature")
        .and_then(Value::as_str)
        .expect("SVM lock signature");
    wait_svm_finalized(lock_signature, Duration::from_secs(120));

    // Only after the SVM lock reaches FINALIZED do we reveal the preimage.
    let claim_args = vec![
        "claim".to_string(),
        "--swap-id".to_string(),
        hex::encode(swap_id),
        "--preimage".to_string(),
        hex::encode(preimage),
    ];
    let svm_claim = run_svm_broadcast(&claim_args, &claimant_keypair);
    assert_eq!(
        svm_claim.get("status").and_then(Value::as_str),
        Some("submitted")
    );
    let claim_signature = svm_claim
        .get("signature")
        .and_then(Value::as_str)
        .expect("SVM claim signature");
    wait_svm_finalized(claim_signature, Duration::from_secs(120));

    let x3_claim = x3_adapter
        .claim(local_id, preimage)
        .expect("X3 claim using SVM-finalized preimage");
    assert_eq!(x3_claim.preimage, preimage);
    assert!(x3_adapter.finality_status(&x3_claim.tx_id).unwrap().finalized);
}


#[test]
#[ignore = "requires solana-test-validator + real SBF program/client and boots a real X3 dev node"]
fn real_x3vm_svm_timeout_refund_atomic_lifecycle() {
    let payer_keypair =
        std::env::var("X3_TEST_SVM_PAYER_KEYPAIR").expect("X3_TEST_SVM_PAYER_KEYPAIR");
    let claimant_keypair =
        std::env::var("X3_TEST_SVM_CLAIMANT_KEYPAIR").expect("X3_TEST_SVM_CLAIMANT_KEYPAIR");
    let payer_pubkey = std::env::var("X3_TEST_SVM_PAYER_PUBKEY").expect("payer pubkey");
    let claimant_pubkey =
        std::env::var("X3_TEST_SVM_CLAIMANT_PUBKEY").expect("claimant pubkey");

    let _x3 = spawn_x3_node();
    wait_x3_rpc(Duration::from_secs(360));

    let local_id = 2002u64;
    let preimage = [0x8du8; 32];
    let hashlock = sp_core::hashing::sha2_256(&preimage);
    let chain_id = String::from("x3-local");
    let alice_uri = dev_uri("Alice");
    let signer = X3RuntimeSigner::from_uri(chain_id.clone(), X3_RPC.into(), &alice_uri)
        .expect("X3 refund signer");

    let prepared = signer
        .prepare_create_intent(
            dev_account("Bob"),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            H256::from(hashlock),
            Some(20),
        )
        .expect("prepare X3 refund intent");
    assert!(!submit_x3(&prepared.signed_extrinsic).is_empty());
    let finalized_head_hash =
        wait_x3_finalized(&prepared.signed_extrinsic, Duration::from_secs(180));
    let finalized_hash = H256::from_slice(
        &hex::decode(finalized_head_hash.trim_start_matches("0x"))
            .expect("decode finalized head"),
    );
    let runtime_intent_id = signer
        .resolve_intent_id(&prepared, finalized_hash)
        .expect("resolve runtime intent id");
    signer.bind_intent(local_id, runtime_intent_id).unwrap();

    let x3_transport = NativeX3NodeTransport::new(
        X3NodeTransportConfig {
            chain_id: chain_id.clone(),
            rpc_url: X3_RPC.into(),
            finality_poll_attempts: 480,
            finality_poll_delay_ms: 500,
            expected_block_time_ms: 6_000,
        },
        signer,
    );
    let x3_adapter = LiveX3VmAdapter::new(
        chain_id,
        b"x3-native-svm-refund-escrow".to_vec(),
        x3_transport,
    );
    let intent = atomic_intent(local_id, preimage);
    let x3_lock = x3_adapter.lock(&intent).expect("real X3 refund-path lock");
    assert!(x3_adapter.finality_status(&x3_lock.tx_id).unwrap().finalized);

    let swap_id = [0x88u8; 32];
    let timeout_slot = svm_finalized_slot().saturating_add(8);
    let lock_args = vec![
        "lock".to_string(),
        "--swap-id".to_string(),
        hex::encode(swap_id),
        "--claimant".to_string(),
        claimant_pubkey,
        "--refund-authority".to_string(),
        payer_pubkey,
        "--hashlock".to_string(),
        hex::encode(hashlock),
        "--amount".to_string(),
        "500000".to_string(),
        "--timeout-slots".to_string(),
        timeout_slot.to_string(),
    ];
    let svm_lock = run_svm_broadcast(&lock_args, &payer_keypair);
    let lock_signature = svm_lock
        .get("signature")
        .and_then(Value::as_str)
        .expect("SVM refund-path lock signature");
    wait_svm_finalized(lock_signature, Duration::from_secs(120));

    wait_svm_slot_past(timeout_slot, Duration::from_secs(120));
    let refund_args = vec![
        "refund".to_string(),
        "--swap-id".to_string(),
        hex::encode(swap_id),
    ];
    let svm_refund = run_svm_broadcast(&refund_args, &payer_keypair);
    assert_eq!(
        svm_refund.get("status").and_then(Value::as_str),
        Some("submitted")
    );
    let refund_signature = svm_refund
        .get("signature")
        .and_then(Value::as_str)
        .expect("SVM refund signature");
    wait_svm_finalized(refund_signature, Duration::from_secs(120));

    let refund_head = wait_for_finalized_refund(runtime_intent_id, Duration::from_secs(180));
    assert!(matches!(
        intent_state_at(runtime_intent_id, &refund_head),
        pallet_x3_settlement_engine::IntentState::Refunded
    ));

    // A finalized refund on both domains is terminal. SVM must reject any
    // subsequent claim, preventing REFUNDED/CLAIMED split state.
    let claim_args = vec![
        "claim".to_string(),
        "--swap-id".to_string(),
        hex::encode(swap_id),
        "--preimage".to_string(),
        hex::encode(preimage),
    ];
    let failure = run_svm_broadcast_expect_failure(&claim_args, &claimant_keypair);
    assert!(
        failure
            .get("error")
            .and_then(Value::as_str)
            .is_some_and(|msg| msg.contains("refunded") || msg.contains("AlreadyRefunded")),
        "unexpected SVM claim-after-refund error: {failure}"
    );

    x3_adapter
        .claim(local_id, preimage)
        .expect_err("X3 claim after finalized refund must fail");
}
