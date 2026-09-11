use codec::{Decode, Encode};
use serde_json::Value;
use sp_core::{crypto::Ss58Codec, Pair as _, H256};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use x3_atomic_swap::intent::{
    AtomicIntent, AtomicSwapStatus, ChainKind, FinalityLevel, FinalityRequirement, RefundPath,
    RouteMode,
};
use x3_atomic_swap::{
    LiveEvmExecutor, LiveX3VmAdapter, NativeX3NodeTransport, RpcClient, VmType,
    X3NodeTransportConfig, X3VmAdapter,
};
use x3_chain_node::x3vm_runtime_signer::X3RuntimeSigner;
use x3_chain_runtime::{AccountId, Signature};

const X3_RPC: &str = "http://127.0.0.1:19945";
const EVM_RPC: &str = "http://127.0.0.1:18545";

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
            "19945",
            "--port",
            "30380",
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

/// Scans every finalized block number seen since the poll started (not just
/// the latest finalized head), since a fast-finalizing dev node can finalize
/// several blocks between two polls and skip past the block that actually
/// contains the extrinsic.
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

fn parse_address(value: &str) -> [u8; 20] {
    let bytes = hex::decode(value.trim_start_matches("0x")).expect("20-byte EVM address hex");
    assert_eq!(bytes.len(), 20);
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    out
}

fn htlc_id(
    sender: [u8; 20],
    recipient: [u8; 20],
    hashlock: [u8; 32],
    count: u64,
) -> [u8; 32] {
    let mut encoded = Vec::with_capacity(20 + 20 + 32 + 32);
    encoded.extend_from_slice(&sender);
    encoded.extend_from_slice(&recipient);
    encoded.extend_from_slice(&hashlock);
    let mut count_word = [0u8; 32];
    count_word[24..].copy_from_slice(&count.to_be_bytes());
    encoded.extend_from_slice(&count_word);
    sp_core::hashing::keccak_256(&encoded)
}

fn evm_call(method: &str, params: Vec<Value>) -> Value {
    let mut rpc = RpcClient::new(EVM_RPC.into(), 0);
    rpc.call(method, params)
        .unwrap_or_else(|e| panic!("EVM RPC {method} failed: {e}"))
        .result
        .unwrap_or(Value::Null)
}

fn evm_htlc_count(contract: [u8; 20]) -> u64 {
    let selector = &sp_core::hashing::keccak_256(b"htlcCount()")[..4];
    let result = evm_call(
        "eth_call",
        vec![
            serde_json::json!({
                "to": format!("0x{}", hex::encode(contract)),
                "data": format!("0x{}", hex::encode(selector)),
            }),
            Value::String("latest".into()),
        ],
    );
    let raw = result.as_str().expect("htlcCount eth_call hex");
    u64::from_str_radix(raw.trim_start_matches("0x"), 16).expect("htlcCount u64")
}

fn advance_anvil_time(seconds: u64) {
    evm_call("evm_increaseTime", vec![Value::Number(seconds.into())]);
    evm_call("evm_mine", Vec::new());
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

fn atomic_intent(local_id: u64, preimage: [u8; 32]) -> AtomicIntent {
    AtomicIntent {
        intent_id: local_id,
        source_chain: ChainKind::X3,
        destination_chain: ChainKind::Ethereum,
        source_asset: "X3".into(),
        destination_asset: "ETH".into(),
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
                chain: ChainKind::Ethereum,
                level: FinalityLevel::Confirmations(1),
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
#[ignore = "requires Anvil with AtlasHTLC deployed plus a real local X3 node"]
fn real_x3vm_evm_lock_claim_atomic_lifecycle() {
    let contract = parse_address(&std::env::var("X3_TEST_EVM_HTLC").expect("X3_TEST_EVM_HTLC"));
    let locker_key = std::env::var("X3_TEST_EVM_LOCKER_KEY").expect("X3_TEST_EVM_LOCKER_KEY");
    let claimant_key = std::env::var("X3_TEST_EVM_CLAIMANT_KEY").expect("X3_TEST_EVM_CLAIMANT_KEY");

    let _x3 = spawn_x3_node();
    wait_x3_rpc(Duration::from_secs(180));

    let local_id = 1001u64;
    let preimage = [0x5au8; 32];
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
        &hex::decode(finalized_head.trim_start_matches("0x")).expect("decode finalized head hex"),
    );
    let runtime_intent_id = primary
        .resolve_intent_id(&prepared, finalized_hash)
        .expect("resolve real on-chain intent id");
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
        b"x3-native-crossvm-escrow".to_vec(),
        x3_transport,
    );
    let intent = atomic_intent(local_id, preimage);
    let x3_lock = x3_adapter.lock(&intent).expect("real X3 lock");
    assert_eq!(x3_lock.vm_type, VmType::X3Vm);
    assert!(x3_adapter.finality_status(&x3_lock.tx_id).unwrap().finalized);

    // Runtime settlement currently requires two escrow legs before claim.
    let leg1 = second_leg
        .sign_lock_escrow_leg(
            runtime_intent_id,
            1,
            pallet_x3_settlement_engine::ExternalChainId::X3Native,
            1_000_000,
            b"x3-native-crossvm-leg1".to_vec(),
        )
        .expect("sign X3 second leg");
    assert!(!submit_x3(&leg1).is_empty());
    wait_x3_finalized(&leg1, Duration::from_secs(180));

    let mut evm_locker = LiveEvmExecutor::new(EVM_RPC, 1337, contract, &locker_key)
        .expect("live EVM locker");
    let mut evm_claimant = LiveEvmExecutor::new(EVM_RPC, 1337, contract, &claimant_key)
        .expect("live EVM claimant");
    let sender = parse_address(&evm_locker.signer_address().expect("EVM locker address"));
    let recipient = parse_address(&evm_claimant.signer_address().expect("EVM claimant address"));
    let timelock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_add(180);

    let expected_count = evm_htlc_count(contract).saturating_add(1);
    let evm_lock = evm_locker
        .execute_lock("anvil", recipient, hashlock, timelock, [0u8; 20], 1, 30_000)
        .expect("real EVM lock");
    assert_eq!(evm_lock.vm_type, VmType::Evm);
    assert_eq!(evm_lock.hashlock, x3_lock.hashlock);
    assert!(!evm_lock.tx_id.is_empty());

    let evm_id = htlc_id(sender, recipient, hashlock, expected_count);
    let evm_claim = evm_claimant
        .execute_claim("anvil", evm_id, local_id, preimage, 30_000)
        .expect("real EVM claim");
    assert_eq!(evm_claim.vm_type, VmType::Evm);
    assert_eq!(evm_claim.preimage, preimage);
    assert!(!evm_claim.tx_id.is_empty());

    // The same preimage revealed on the EVM leg must settle the finalized X3 leg.
    let x3_claim = x3_adapter
        .claim(local_id, evm_claim.preimage)
        .expect("X3 claim using EVM-revealed preimage");
    assert_eq!(x3_claim.preimage, preimage);
    assert!(x3_adapter.finality_status(&x3_claim.tx_id).unwrap().finalized);
}


#[test]
#[ignore = "requires Anvil with AtlasHTLC deployed plus a real local X3 node"]
fn real_x3vm_evm_timeout_refund_atomic_lifecycle() {
    let contract = parse_address(&std::env::var("X3_TEST_EVM_HTLC").expect("X3_TEST_EVM_HTLC"));
    let locker_key = std::env::var("X3_TEST_EVM_LOCKER_KEY").expect("X3_TEST_EVM_LOCKER_KEY");
    let claimant_key = std::env::var("X3_TEST_EVM_CLAIMANT_KEY").expect("X3_TEST_EVM_CLAIMANT_KEY");

    let _x3 = spawn_x3_node();
    wait_x3_rpc(Duration::from_secs(180));

    let local_id = 1002u64;
    let preimage = [0x7cu8; 32];
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
        b"x3-native-crossvm-refund-escrow".to_vec(),
        x3_transport,
    );
    let intent = atomic_intent(local_id, preimage);
    let x3_lock = x3_adapter.lock(&intent).expect("real X3 refund-path lock");
    assert!(x3_adapter.finality_status(&x3_lock.tx_id).unwrap().finalized);

    let mut evm_locker = LiveEvmExecutor::new(EVM_RPC, 1337, contract, &locker_key)
        .expect("live EVM locker");
    let mut evm_claimant = LiveEvmExecutor::new(EVM_RPC, 1337, contract, &claimant_key)
        .expect("live EVM claimant");
    let sender = parse_address(&evm_locker.signer_address().expect("EVM locker address"));
    let recipient = parse_address(&evm_claimant.signer_address().expect("EVM claimant address"));
    let expected_count = evm_htlc_count(contract).saturating_add(1);
    let timelock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_add(5);

    let evm_lock = evm_locker
        .execute_lock("anvil", recipient, hashlock, timelock, [0u8; 20], 1, 30_000)
        .expect("real EVM refund-path lock");
    assert_eq!(evm_lock.hashlock, x3_lock.hashlock);

    let evm_id = htlc_id(sender, recipient, hashlock, expected_count);
    advance_anvil_time(10);
    let evm_refund = evm_locker
        .execute_refund("anvil", evm_id, local_id, 30_000)
        .expect("real EVM timeout refund");
    assert_eq!(evm_refund.vm_type, VmType::Evm);
    assert!(!evm_refund.tx_id.is_empty());
    assert!(!evm_refund.block_hash.is_empty());

    let refund_head = wait_for_finalized_refund(runtime_intent_id, Duration::from_secs(180));
    assert!(matches!(
        intent_state_at(runtime_intent_id, &refund_head),
        pallet_x3_settlement_engine::IntentState::Refunded
    ));

    // Once both domains have reached REFUNDED, neither side may cross into
    // CLAIMED. These checks catch a split terminal state.
    evm_claimant
        .execute_claim("anvil", evm_id, local_id, preimage, 30_000)
        .expect_err("EVM claim after refund must fail");
    x3_adapter
        .claim(local_id, preimage)
        .expect_err("X3 claim after finalized refund must fail");
}
