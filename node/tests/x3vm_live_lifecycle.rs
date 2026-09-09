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

const RPC_URL: &str = "http://127.0.0.1:19944";

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

fn spawn_dev_node() -> NodeGuard {
    let child = Command::new(env!("CARGO_BIN_EXE_x3-chain-node"))
        .args([
            "--dev",
            "--tmp",
            "--rpc-port",
            "19944",
            "--port",
            "30379",
            "--no-telemetry",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn x3-chain-node --dev");
    NodeGuard(child)
}

fn wait_rpc(timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let mut rpc = RpcClient::new(RPC_URL.into(), 0);
        if rpc.call("system_health", Vec::new()).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("X3 dev node RPC did not become ready");
}

fn submit(signed: &str) -> String {
    let mut rpc = RpcClient::new(RPC_URL.into(), 0);
    rpc.call(
        "author_submitExtrinsic",
        vec![Value::String(signed.to_string())],
    )
    .expect("submit extrinsic")
    .result
    .and_then(|v| v.as_str().map(ToOwned::to_owned))
    .expect("transaction hash")
}

fn wait_finalized(signed: &str, timeout: Duration) -> (u64, String) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let mut rpc = RpcClient::new(RPC_URL.into(), 0);
        let head = rpc
            .call("chain_getFinalizedHead", Vec::new())
            .expect("finalized head")
            .result
            .and_then(|v| v.as_str().map(ToOwned::to_owned));
        if let Some(head) = head {
            let block = rpc
                .call("chain_getBlock", vec![Value::String(head.clone())])
                .expect("finalized block")
                .result;
            let included = block
                .as_ref()
                .and_then(|v| v.pointer("/block/extrinsics"))
                .and_then(Value::as_array)
                .map(|xs| xs.iter().any(|x| x.as_str() == Some(signed)))
                .unwrap_or(false);
            if included {
                let header = rpc
                    .call("chain_getHeader", vec![Value::String(head.clone())])
                    .expect("finalized header")
                    .result
                    .expect("header result");
                let raw = header
                    .get("number")
                    .and_then(Value::as_str)
                    .expect("header number");
                let number = u64::from_str_radix(raw.trim_start_matches("0x"), 16)
                    .expect("hex block number");
                return (number, head);
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("extrinsic was not observed in a GRANDPA-finalized block");
}

fn atomic_intent(local_id: u64, preimage: [u8; 32]) -> AtomicIntent {
    AtomicIntent {
        intent_id: local_id,
        source_chain: ChainKind::X3,
        destination_chain: ChainKind::Ethereum,
        source_asset: "X3".into(),
        destination_asset: "X3".into(),
        amount_in: 1_000_000,
        min_amount_out: 1,
        receiver: dev_account("Bob").to_ss58check(),
        hashlock: sp_io::hashing::sha2_256(&preimage),
        source_timeout: u64::MAX / 2,
        destination_timeout: u64::MAX / 2,
        finality_requirements: vec![FinalityRequirement {
            chain: ChainKind::X3,
            level: FinalityLevel::Bft,
        }],
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
#[ignore = "boots the real X3 dev node and waits for GRANDPA finality"]
fn real_local_node_lock_finalized_claim_lifecycle() {
    let _node = spawn_dev_node();
    wait_rpc(Duration::from_secs(60));

    let chain_id = String::from("x3-local");
    let local_id = 1u64;
    let preimage = [0x42u8; 32];
    let hashlock = H256::from(sp_io::hashing::sha2_256(&preimage));
    let alice_uri = dev_uri("Alice");

    let primary = X3RuntimeSigner::from_uri(chain_id.clone(), RPC_URL.into(), &alice_uri)
        .expect("primary signer");
    let second_leg = X3RuntimeSigner::from_uri(chain_id.clone(), RPC_URL.into(), &alice_uri)
        .expect("second-leg signer");

    let prepared = primary
        .prepare_create_intent(
            dev_account("Bob"),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            hashlock,
            Some(300),
        )
        .expect("prepare create_intent");
    assert!(!submit(&prepared.signed_extrinsic).is_empty());
    wait_finalized(&prepared.signed_extrinsic, Duration::from_secs(90));

    primary.bind_intent(local_id, prepared.runtime_intent_id).unwrap();
    second_leg
        .bind_intent(local_id, prepared.runtime_intent_id)
        .unwrap();

    let transport = NativeX3NodeTransport::new(
        X3NodeTransportConfig {
            chain_id: chain_id.clone(),
            rpc_url: RPC_URL.into(),
            finality_poll_attempts: 180,
            finality_poll_delay_ms: 500,
            expected_block_time_ms: 6_000,
        },
        primary,
    );
    let adapter = LiveX3VmAdapter::new(
        chain_id.clone(),
        b"x3-native-escrow".to_vec(),
        transport,
    );
    let intent = atomic_intent(local_id, preimage);

    let lock = adapter.lock(&intent).expect("live native lock");
    assert_eq!(lock.vm_type, VmType::X3Vm);
    assert!(!lock.raw_proof.is_empty());
    let lock_finality = adapter.finality_status(&lock.tx_id).expect("lock finality");
    assert!(lock_finality.finalized);
    assert_eq!(lock_finality.block_hash, lock.block_hash);

    // SettlementEngine requires both escrow legs before a claim. This second
    // real extrinsic satisfies that runtime precondition; leg 0 above remains
    // the X3VmAdapter lock under test.
    let leg1 = second_leg
        .sign_lock_escrow_leg(
            prepared.runtime_intent_id,
            1,
            pallet_x3_settlement_engine::ExternalChainId::X3Native,
            1_000_000,
            b"x3-native-escrow-leg1".to_vec(),
        )
        .expect("sign second escrow leg");
    assert!(!submit(&leg1).is_empty());
    wait_finalized(&leg1, Duration::from_secs(90));

    let claim = adapter.claim(local_id, preimage).expect("live native claim");
    assert_eq!(claim.vm_type, VmType::X3Vm);
    assert_eq!(claim.intent_id, local_id);
    assert_eq!(claim.preimage, preimage);
    assert!(!claim.raw_proof.is_empty());
    let claim_finality = adapter
        .finality_status(&claim.tx_id)
        .expect("claim finality");
    assert!(claim_finality.finalized);
    assert_eq!(claim_finality.block_hash, claim.block_hash);
}
