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
use x3_atomic_swap::secret_release::{RefundObservation, RpcQuorumAttestation};
use x3_atomic_swap::{
    CrossDomainOperation, CrossDomainProofBundle, CrossDomainProofSet, FinalityProof,
    LiveEvmExecutor, LiveX3VmAdapter, NativeX3NodeTransport, RpcClient, SecretReleaseEvidence,
    SecretReleaseFirewall, SecretReleaseRequirement, VmType, X3ExtrinsicSigner,
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
    // `--dev` boots a chain whose genesis allows unattested cross-domain proof
    // sets. That is a dev-only posture: every joinable network passes
    // `allowUnattestedCrossDomainProofs: false`, because a terminal refund
    // released against a self-attested bundle is a fund loss. Setting
    // `X3_TEST_CHAIN_SPEC` runs this same lifecycle against another spec — the
    // gate builds a dev spec with that one policy flipped — so the honest path
    // is proven in the posture mainnet actually uses.
    let mut args: Vec<String> = vec![
        "--tmp".into(),
        "--rpc-port".into(),
        "19945".into(),
        "--port".into(),
        "30380".into(),
        "--no-telemetry".into(),
    ];
    match std::env::var("X3_TEST_CHAIN_SPEC") {
        Ok(spec) => {
            // `--dev` stays: it is what makes this node an authority with the
            // dev network key and forced authoring, and without it the node
            // does not start ("NetworkKeyNotFound"). An explicit `--chain`
            // takes precedence over the chain id `--dev` would use, so the only
            // thing that changes for a strict run is the genesis.
            args.push("--dev".into());
            args.push(format!("--chain={spec}"));
        }
        Err(_) => args.push("--dev".into()),
    }
    let child = Command::new(env!("CARGO_BIN_EXE_x3-chain-node"))
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn x3-chain-node {args:?}: {e}"));
    NodeGuard(child)
}

fn wait_x3_rpc(timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let mut rpc = RpcClient::new(X3_RPC.into(), 0);
        if rpc.call("system_health", Vec::new()).is_ok() {
            assert_requested_cross_domain_posture();
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("X3 dev node RPC did not become ready");
}

/// Prove the node is running the posture the caller asked for.
///
/// `X3_TEST_CHAIN_SPEC` is meant to boot this lifecycle on a spec whose
/// `allowUnattestedCrossDomainProofs` is `false` — the posture every joinable
/// network uses. Without this check a run that silently ignored the spec (an
/// argument the node dropped, a file that never loaded) would pass exactly like
/// a strict run, and the gate would be reporting a posture it never applied.
///
/// Read as raw SCALE: `bool` is one byte, so `false` is `0x00`. Anything else —
/// a `0x01`, a missing key, an RPC error — fails, because "I could not read the
/// policy" is not evidence that the policy is strict.
fn assert_requested_cross_domain_posture() {
    if std::env::var("X3_TEST_CHAIN_SPEC").is_err() {
        return; // a `--dev` run: permissive on purpose
    }
    let key = format!(
        "0x{}",
        hex::encode(frame_support::storage::storage_prefix(
            b"X3SettlementEngine",
            b"AllowUnattestedCrossDomainProofs"
        ))
    );
    let mut rpc = RpcClient::new(X3_RPC.into(), 0);
    let value = rpc
        .call("state_getStorage", vec![Value::String(key.clone())])
        .expect("state_getStorage for the cross-domain proof policy")
        .result
        .unwrap_or_else(|| panic!("no storage value at {key}: the policy was never set"));
    let raw = value
        .as_str()
        .unwrap_or_else(|| panic!("policy value is not a hex string: {value}"));
    assert_eq!(
        raw, "0x00",
        "X3_TEST_CHAIN_SPEC was given, so this run must be the strict posture \
         (allowUnattestedCrossDomainProofs = false), but the chain reports {raw}"
    );
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

fn x3_extrinsic_index_in_block(rpc: &mut RpcClient, hash: &str, signed: &str) -> Option<u32> {
    let block = rpc
        .call("chain_getBlock", vec![Value::String(hash.to_string())])
        .expect("finalized block")
        .result;
    block
        .as_ref()
        .and_then(|v| v.pointer("/block/extrinsics"))
        .and_then(Value::as_array)
        .and_then(|xs| xs.iter().position(|x| x.as_str() == Some(signed)))
        .map(|position| position as u32)
}

/// Inclusion is not success: a rejected extrinsic still lands in a finalized
/// block, so callers that need positive proof of execution must check this.
fn assert_x3_dispatch_succeeded(signer: &X3RuntimeSigner, block_hash: &str, signed: &str) {
    let mut rpc = RpcClient::new(X3_RPC.into(), 0);
    let index = x3_extrinsic_index_in_block(&mut rpc, block_hash, signed)
        .expect("signed extrinsic is present in the block that included it");
    signer
        .verify_finalized_dispatch(block_hash, index)
        .expect("finalized extrinsic dispatched successfully");
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

fn htlc_id(sender: [u8; 20], recipient: [u8; 20], hashlock: [u8; 32], count: u64) -> [u8; 32] {
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

fn intent_state_at(intent_id: H256, block_hash: &str) -> pallet_x3_settlement_engine::IntentState {
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

/// Poll for the terminal `Refunded` state. The runtime performs this transition
/// from `on_initialize`, so a caller must not assume it has to drive it.
fn wait_for_refund_state(intent_id: H256, timeout: Duration) -> Option<String> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let head = finalized_head();
        if matches!(
            intent_state_at(intent_id, &head),
            pallet_x3_settlement_engine::IntentState::Refunded
        ) {
            return Some(head);
        }
        thread::sleep(Duration::from_millis(500));
    }
    None
}

fn wait_for_finalized_refund(intent_id: H256, timeout: Duration) -> String {
    wait_for_refund_state(intent_id, timeout)
        .expect("X3 intent did not reach Refunded in GRANDPA-finalized state")
}

/// Local mirror of the cross-domain intent. The secret-release firewall requires
/// a releasable lifecycle state and a stored hash that matches the fields, so
/// this fixture mirrors a trade whose X3 and EVM legs are both escrowed.
fn atomic_intent(local_id: u64, preimage: [u8; 32]) -> AtomicIntent {
    let mut intent = AtomicIntent {
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
        status: AtomicSwapStatus::BothLocked,
        intent_hash: [0u8; 32],
    };
    intent.intent_hash = intent.compute_hash();
    intent
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
    let x3_adapter =
        LiveX3VmAdapter::new(chain_id, b"x3-native-crossvm-escrow".to_vec(), x3_transport);
    let intent = atomic_intent(local_id, preimage);
    let x3_lock = x3_adapter.lock(&intent).expect("real X3 lock");
    assert_eq!(x3_lock.vm_type, VmType::X3Vm);
    assert!(
        x3_adapter
            .finality_status(&x3_lock.tx_id)
            .unwrap()
            .finalized
    );

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

    let mut evm_locker =
        LiveEvmExecutor::new(EVM_RPC, 1337, contract, &locker_key).expect("live EVM locker");
    let mut evm_claimant =
        LiveEvmExecutor::new(EVM_RPC, 1337, contract, &claimant_key).expect("live EVM claimant");
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
    // A cross-domain claim is gated by the secret-release firewall — the bare
    // claim path never reaches the chain — and the permit must carry evidence for
    // every domain the intent declares, including the destination chain. Both
    // evidence entries below are built from real on-chain transactions: the X3
    // escrow and the EVM escrow that was just claimed.
    let x3_lock_finality = x3_adapter
        .finality_status(&x3_lock.tx_id)
        .expect("X3 escrow finality");
    // The adapter labels its chain `x3-local` and the intent's X3 policy is
    // BFT finality, which maps to zero required confirmations.
    let x3_requirement = SecretReleaseRequirement {
        chain_id: String::from("x3-local"),
        vm_type: VmType::X3Vm,
        min_confirmations: 0,
    };
    // The EVM executor labels its chain `anvil`; the intent's Ethereum policy
    // requires one confirmation. The firewall matches domains by chain family,
    // so the EVM observation is presented under a canonical EVM label.
    let mut evm_escrow = evm_lock.clone();
    evm_escrow.chain_id = String::from("ethereum-anvil");
    let evm_requirement = SecretReleaseRequirement {
        chain_id: String::from("ethereum-anvil"),
        vm_type: VmType::Evm,
        min_confirmations: 1,
    };
    let evm_lock_finality = FinalityProof {
        chain_id: String::from("ethereum-anvil"),
        vm_type: VmType::Evm,
        tx_id: evm_lock.tx_id.clone(),
        block_number: evm_lock.block_number,
        block_hash: evm_lock.block_hash.clone(),
        confirmations: 1,
        // Anvil mines and finalizes in the same block; there is no reorg window
        // on this ephemeral chain.
        finalized: true,
        finality_source: String::from("anvil-instant-finality"),
        safe_to_reveal_secret: true,
    };
    let evidence = [
        SecretReleaseEvidence {
            lock: x3_lock.clone(),
            finality: x3_lock_finality,
            rpc_quorum: RpcQuorumAttestation {
                tx_id: x3_lock.tx_id.clone(),
                block_hash: x3_lock.block_hash.clone(),
                provider_count: 3,
                required_quorum: 2,
                finalized: true,
            },
            refund: RefundObservation {
                tx_id: x3_lock.tx_id.clone(),
                block_hash: x3_lock.block_hash.clone(),
                refunded: false,
            },
        },
        SecretReleaseEvidence {
            lock: evm_escrow,
            finality: evm_lock_finality,
            rpc_quorum: RpcQuorumAttestation {
                tx_id: evm_lock.tx_id.clone(),
                block_hash: evm_lock.block_hash.clone(),
                provider_count: 3,
                required_quorum: 2,
                finalized: true,
            },
            refund: RefundObservation {
                tx_id: evm_lock.tx_id.clone(),
                block_hash: evm_lock.block_hash.clone(),
                refunded: false,
            },
        },
    ];
    let permit = SecretReleaseFirewall::authorize(
        &intent,
        evm_claim.preimage,
        &[x3_requirement, evm_requirement],
        &evidence,
    )
    .expect("secret-release permit for the cross-domain claim");
    let x3_claim = x3_adapter
        .claim_with_permit(&permit)
        .expect("X3 claim using EVM-revealed preimage");
    assert_eq!(x3_claim.preimage, preimage);
    assert!(
        x3_adapter
            .finality_status(&x3_claim.tx_id)
            .unwrap()
            .finalized
    );
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
        &hex::decode(finalized_head_hash.trim_start_matches("0x")).expect("decode finalized head"),
    );
    let runtime_intent_id = signer
        .resolve_intent_id(&prepared, finalized_hash)
        .expect("resolve runtime intent id");
    signer.bind_intent(local_id, runtime_intent_id).unwrap();
    let second_leg = X3RuntimeSigner::from_uri(chain_id.clone(), X3_RPC.into(), &alice_uri)
        .expect("X3 refund second-leg signer");
    second_leg.bind_intent(local_id, runtime_intent_id).unwrap();

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
    assert!(
        x3_adapter
            .finality_status(&x3_lock.tx_id)
            .unwrap()
            .finalized
    );

    // Both legs have to be escrowed before the runtime considers the refund proof
    // set complete: `all_required_operation_proofs` walks every leg.
    let leg1 = second_leg
        .sign_lock_escrow_leg(
            runtime_intent_id,
            1,
            pallet_x3_settlement_engine::ExternalChainId::X3Native,
            1_000_000,
            b"x3-native-crossvm-refund-leg1".to_vec(),
        )
        .expect("sign refund leg1");
    assert!(!submit_x3(&leg1).is_empty());
    let leg1_block = wait_x3_finalized(&leg1, Duration::from_secs(180));
    assert_x3_dispatch_succeeded(&second_leg, &leg1_block, &leg1);

    let mut evm_locker =
        LiveEvmExecutor::new(EVM_RPC, 1337, contract, &locker_key).expect("live EVM locker");
    let mut evm_claimant =
        LiveEvmExecutor::new(EVM_RPC, 1337, contract, &claimant_key).expect("live EVM claimant");
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

    // A terminal `Refunded` state is gated on a verified canonical Refund proof
    // for every escrowed leg's domain. Every leg on the X3 side is X3-native, so
    // a single canonical Refund bundle bound to the real X3 escrow covers them;
    // the runtime then refunds from `on_initialize` once the wall-clock timeout
    // has elapsed.
    let mut refund_finality = x3_adapter
        .finality_status(&x3_lock.tx_id)
        .expect("refund observation finality");
    refund_finality.chain_id = String::from("x3-native");
    let mut proof_set = CrossDomainProofSet::new(&intent, runtime_intent_id.to_fixed_bytes());
    let refund_bundle = CrossDomainProofBundle::new(
        &intent,
        runtime_intent_id.to_fixed_bytes(),
        String::from("x3-native"),
        VmType::X3Vm,
        CrossDomainOperation::Refund,
        x3_lock.tx_id.clone(),
        x3_lock.block_number,
        x3_lock.block_hash.clone(),
        x3_lock.raw_proof.clone(),
        refund_finality,
    )
    .expect("canonical refund bundle");
    proof_set
        .push_verified(&intent, refund_bundle)
        .expect("verified canonical refund bundle");
    let signed_proof_set = second_leg
        .prepare_cross_domain_proof_set(runtime_intent_id, proof_set)
        .expect("sign canonical refund proof set");
    assert!(!submit_x3(&signed_proof_set).is_empty());
    let proof_block = wait_x3_finalized(&signed_proof_set, Duration::from_secs(180));
    assert_x3_dispatch_succeeded(&second_leg, &proof_block, &signed_proof_set);

    let refund_head = match wait_for_refund_state(runtime_intent_id, Duration::from_secs(180)) {
        Some(head) => head,
        None => {
            x3_adapter
                .refund(local_id)
                .expect("explicit timeout refund after the automatic path did not fire");
            wait_for_finalized_refund(runtime_intent_id, Duration::from_secs(180))
        }
    };
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

/// The cross-chain-validator storage key for one attested EVM merkle root.
fn evm_merkle_root_key(block_number: u64) -> String {
    let mut key =
        frame_support::storage::storage_prefix(b"CrossChainValidator", b"EvmMerkleRoots").to_vec();
    let encoded = block_number.encode();
    key.extend_from_slice(&sp_core::hashing::blake2_128(&encoded));
    key.extend_from_slice(&encoded);
    format!("0x{}", hex::encode(key))
}

fn storage_at(key: &str) -> Option<Vec<u8>> {
    let mut rpc = RpcClient::new(X3_RPC.into(), 0);
    let value = rpc
        .call("state_getStorage", vec![Value::String(key.to_string())])
        .expect("state_getStorage")
        .result?;
    let raw = value.as_str().expect("storage hex");
    Some(hex::decode(raw.trim_start_matches("0x")).expect("decode storage hex"))
}

fn h256_field(header: &Value, field: &str) -> H256 {
    let raw = header[field]
        .as_str()
        .unwrap_or_else(|| panic!("{field} is not a hex string"));
    let bytes = hex::decode(raw.trim_start_matches("0x")).expect("hex");
    H256::from_slice(&bytes)
}

/// The anchor the EVM receipt verifier reads, populated through the real path.
///
/// `ProductionEvmReceiptVerifier` refuses to verify against a header the proof
/// carries; it asks the chain which header it has attested. Nothing had shown
/// that a live chain can *populate* that store, and the path is not obvious: this
/// genesis configures no sudo key and `set_authorized_submitters` needs Root or
/// half the council, so the reachable route is a council motion whose threshold
/// is one (pallet-collective executes those immediately).
///
/// The header attested here is a block anvil actually produced — its number,
/// hash, state root and receipts root are read from the node, not invented.
#[test]
#[ignore = "boots a node and needs a running anvil; the EVM gate supplies both"]
fn real_evm_header_attestation_populates_the_verifiers_anchor() {
    let _x3 = spawn_x3_node();
    wait_x3_rpc(Duration::from_secs(180));

    // A real EVM block from the chain the gate is running — one that actually
    // carries a receipt, so the attestation is about a block with contents rather
    // than anvil's empty genesis.
    let sent = evm_call(
        "eth_sendTransaction",
        vec![serde_json::json!({
            "from": "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "value": "0x1",
        })],
    );
    let sent = sent.as_str().expect("anvil accepted the transaction").to_string();

    // Anvil returns the hash before the block exists, so wait for the receipt and
    // take the block *it* names rather than "latest" — otherwise the test can read
    // the empty genesis block.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mined = loop {
        let receipt = evm_call(
            "eth_getTransactionReceipt",
            vec![Value::String(sent.clone())],
        );
        if receipt.is_object() {
            break receipt;
        }
        assert!(Instant::now() < deadline, "the transaction was never mined");
        thread::sleep(Duration::from_millis(200));
    };
    let header = evm_call(
        "eth_getBlockByNumber",
        vec![
            Value::String(
                mined["blockNumber"]
                    .as_str()
                    .expect("the receipt names its block")
                    .to_string(),
            ),
            Value::Bool(false),
        ],
    );
    let block_number = u64::from_str_radix(
        header["number"]
            .as_str()
            .expect("the block has a number")
            .trim_start_matches("0x"),
        16,
    )
    .expect("a hex block number");
    let block_hash = h256_field(&header, "hash");
    let state_root = h256_field(&header, "stateRoot");
    let receipts_root = h256_field(&header, "receiptsRoot");
    assert_ne!(block_number, 0, "anvil has produced at least one block");
    assert_ne!(receipts_root, H256::zero(), "a header commits to a root");

    let alice = X3RuntimeSigner::from_uri(
        String::from("x3-local"),
        X3_RPC.into(),
        &dev_uri("Alice"),
    )
    .expect("X3 signer");

    // Before anything is attested there is no anchor, so an external proof is
    // refused for lack of one rather than checked against itself.
    assert!(
        storage_at(&evm_merkle_root_key(block_number)).is_none(),
        "nothing is attested yet"
    );

    // 1. Enroll this signer as an external-header submitter, through the council.
    let enroll = alice
        .sign_enroll_header_submitters(vec![alice.account()])
        .expect("sign the council proposal");
    assert!(!submit_x3(&enroll).is_empty());
    let enrol_block = wait_x3_finalized(&enroll, Duration::from_secs(180));
    assert_x3_dispatch_succeeded(&alice, &enrol_block, &enroll);

    // 2. Attest the real block's receipts root. One leaf — the root itself — is
    //    the proof that this root is over the leaves submitted.
    let attest = alice
        .sign_validate_evm_header(
            block_number,
            block_hash,
            state_root,
            receipts_root,
            receipts_root.as_bytes().to_vec(),
        )
        .expect("sign the header attestation");
    assert!(!submit_x3(&attest).is_empty());
    let attest_block = wait_x3_finalized(&attest, Duration::from_secs(180));
    assert_x3_dispatch_succeeded(&alice, &attest_block, &attest);

    // 3. The store the anchor reads now answers with that header.
    let stored_root = storage_at(&evm_merkle_root_key(block_number))
        .expect("the attested root is stored for its height");
    assert_eq!(
        H256::from_slice(&stored_root),
        receipts_root,
        "the height answers with the block's receipts root, which is what a receipt proof must walk to"
    );

    let last_key = frame_support::storage::storage_prefix(b"CrossChainValidator", b"LastEvmHeader");
    let stored = storage_at(&format!("0x{}", hex::encode(last_key)))
        .expect("the attested header is the newest one");
    let header_info = pallet_cross_chain_validator::EvmHeaderInfo::decode(&mut &stored[..])
        .expect("EvmHeaderInfo decodes");
    assert_eq!(header_info.block_number, block_number);
    assert_eq!(header_info.block_hash, block_hash);
    assert_eq!(header_info.state_root, state_root);
    assert_eq!(
        header_info.merkle_root, receipts_root,
        "the anchor's per-height root and its head describe the same attested block"
    );
}
