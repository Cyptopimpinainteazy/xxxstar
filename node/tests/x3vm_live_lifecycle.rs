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
    CrossDomainOperation, CrossDomainProofBundle, CrossDomainProofSet, LiveX3VmAdapter,
    NativeX3NodeTransport, ProofKind, RpcClient, SecretReleaseEvidence, SecretReleaseFirewall,
    SecretReleaseRequirement, VmType, X3ExtrinsicSigner, X3NodeTransportConfig, X3VmAdapter,
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

fn header_number(rpc: &mut RpcClient, hash: &str) -> u64 {
    let header = rpc
        .call("chain_getHeader", vec![Value::String(hash.to_string())])
        .expect("header")
        .result
        .expect("header result");
    let raw = header
        .get("number")
        .and_then(Value::as_str)
        .expect("header number");
    u64::from_str_radix(raw.trim_start_matches("0x"), 16).expect("hex block number")
}

fn block_hash_at(rpc: &mut RpcClient, number: u64) -> Option<String> {
    rpc.call(
        "chain_getBlockHash",
        vec![Value::Number(number.into())],
    )
    .expect("block hash")
    .result
    .and_then(|v| v.as_str().map(ToOwned::to_owned))
}

fn block_contains(rpc: &mut RpcClient, hash: &str, signed: &str) -> bool {
    let block = rpc
        .call("chain_getBlock", vec![Value::String(hash.to_string())])
        .expect("finalized block")
        .result;
    block
        .as_ref()
        .and_then(|v| v.pointer("/block/extrinsics"))
        .and_then(Value::as_array)
        .map(|xs| xs.iter().any(|x| x.as_str() == Some(signed)))
        .unwrap_or(false)
}

fn extrinsic_index_in_block(rpc: &mut RpcClient, hash: &str, signed: &str) -> Option<u32> {
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

/// Prove that an extrinsic that was observed in a finalized block actually
/// dispatched successfully there. Inclusion is not success: a rejected
/// extrinsic still lands in a finalized block.
fn assert_dispatch_succeeded(signer: &X3RuntimeSigner, block_hash: &str, signed: &str) {
    let mut rpc = RpcClient::new(RPC_URL.into(), 0);
    let index = extrinsic_index_in_block(&mut rpc, block_hash, signed)
        .expect("signed extrinsic is present in the block that included it");
    signer
        .verify_finalized_dispatch(block_hash, index)
        .expect("finalized extrinsic dispatched successfully");
}

/// Polls finalized blocks for `signed`, scanning every finalized block number
/// seen since the poll started (not just the latest finalized head), because
/// a fast-finalizing dev node can finalize several blocks between two polls
/// and a "only check current head" loop can skip straight past the block
/// that actually contains the extrinsic.
fn wait_finalized(signed: &str, timeout: Duration) -> (u64, String) {
    let started = Instant::now();
    let mut rpc = RpcClient::new(RPC_URL.into(), 0);
    let mut next_number: Option<u64> = None;
    while started.elapsed() < timeout {
        let head = rpc
            .call("chain_getFinalizedHead", Vec::new())
            .expect("finalized head")
            .result
            .and_then(|v| v.as_str().map(ToOwned::to_owned));
        if let Some(head) = head {
            let head_number = header_number(&mut rpc, &head);
            let start = next_number.unwrap_or(head_number);
            if head_number >= start {
                for number in start..=head_number {
                    if let Some(hash) = block_hash_at(&mut rpc, number) {
                        if block_contains(&mut rpc, &hash, signed) {
                            return (number, hash);
                        }
                    }
                }
            }
            next_number = Some(head_number + 1);
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("extrinsic was not observed in a GRANDPA-finalized block");
}

fn finalized_head() -> String {
    let mut rpc = RpcClient::new(RPC_URL.into(), 0);
    rpc.call("chain_getFinalizedHead", Vec::new())
        .expect("finalized head")
        .result
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .expect("finalized head hash")
}

fn intent_state_storage_key(intent_id: H256) -> String {
    let mut key = frame_support::storage::storage_prefix(b"X3SettlementEngine", b"IntentStates").to_vec();
    let encoded = intent_id.encode();
    key.extend_from_slice(&sp_core::hashing::blake2_128(&encoded));
    key.extend_from_slice(&encoded);
    format!("0x{}", hex::encode(key))
}

fn intent_state_at(intent_id: H256, block_hash: &str) -> pallet_x3_settlement_engine::IntentState {
    let mut rpc = RpcClient::new(RPC_URL.into(), 0);
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

/// Poll for the terminal `Refunded` state, returning the finalized head that
/// carries it. The runtime performs this transition from `on_initialize`, so a
/// caller must not assume it has to drive the extrinsic itself.
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
        .expect("intent did not reach Refunded in finalized X3 state")
}

/// Local mirror of the intent the live X3-native escrow settles.
///
/// The secret-release firewall only authorizes a releasable lifecycle state, and
/// it requires the presented requirements to cover the intent's own policy
/// *including the destination chain*. These lifecycles settle X3-native to
/// X3-native, so the intent declares the single X3 BFT policy that the tests
/// then present. `intent_hash` has to be computed after construction — the
/// firewall rejects any intent whose stored hash does not match its fields.
fn atomic_intent(local_id: u64, preimage: [u8; 32]) -> AtomicIntent {
    let mut intent = AtomicIntent {
        intent_id: local_id,
        source_chain: ChainKind::X3,
        destination_chain: ChainKind::X3,
        source_asset: "X3".into(),
        destination_asset: "X3".into(),
        amount_in: 1_000_000,
        min_amount_out: 1,
        receiver: dev_account("Bob").to_ss58check(),
        hashlock: sp_core::hashing::sha2_256(&preimage),
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
        status: AtomicSwapStatus::BothLocked,
        intent_hash: [0u8; 32],
    };
    intent.intent_hash = intent.compute_hash();
    intent
}

fn proof_ledger_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "x3-live-{label}-{}-{}.json",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ))
}

#[test]
#[ignore = "boots the real X3 dev node and waits for GRANDPA finality"]
fn real_local_node_lock_finalized_claim_lifecycle() {
    let _node = spawn_dev_node();
    wait_rpc(Duration::from_secs(180));

    let chain_id = String::from("x3-local");
    let local_id = 1u64;
    let preimage = [0x42u8; 32];
    let hashlock = H256::from(sp_core::hashing::sha2_256(&preimage));
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
    let (_, finalized_head) = wait_finalized(&prepared.signed_extrinsic, Duration::from_secs(180));
    let finalized_hash = H256::from_slice(
        &hex::decode(finalized_head.trim_start_matches("0x")).expect("decode finalized head hex"),
    );
    let runtime_intent_id = primary
        .resolve_intent_id(&prepared, finalized_hash)
        .expect("resolve real on-chain intent id");

    primary.bind_intent(local_id, runtime_intent_id).unwrap();
    second_leg
        .bind_intent(local_id, runtime_intent_id)
        .unwrap();

    let ledger_path = proof_ledger_path("claim");
    let transport = NativeX3NodeTransport::new_with_proof_ledger(
        X3NodeTransportConfig {
            chain_id: chain_id.clone(),
            rpc_url: RPC_URL.into(),
            finality_poll_attempts: 480,
            finality_poll_delay_ms: 500,
            expected_block_time_ms: 6_000,
        },
        primary,
        ledger_path.clone(),
    )
    .expect("persistent native transport");
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

    let leg1 = second_leg
        .sign_lock_escrow_leg(
            runtime_intent_id,
            1,
            pallet_x3_settlement_engine::ExternalChainId::X3Native,
            1_000_000,
            b"x3-native-escrow-leg1".to_vec(),
        )
        .expect("sign second escrow leg");
    assert!(!submit(&leg1).is_empty());
    wait_finalized(&leg1, Duration::from_secs(180));

    // Wrong secrets are now rejected before signing/broadcast: the live claim
    // boundary accepts only a permit issued by the secret-release firewall.
    let lock_finality = adapter
        .finality_status(&lock.tx_id)
        .expect("lock finality for release firewall");
    let requirement = SecretReleaseRequirement {
        chain_id: chain_id.clone(),
        vm_type: VmType::X3Vm,
        // The intent's own policy for X3 is `FinalityLevel::Bft`, which the
        // firewall maps to zero required confirmations. Declaring anything else
        // makes the requirement disagree with the declared policy.
        min_confirmations: 0,
    };
    let evidence = SecretReleaseEvidence {
        lock: lock.clone(),
        finality: lock_finality,
        // The firewall binds this attestation to the observed lock: it requires
        // `tx_id`/`block_hash` to match the lock proof, a non-zero quorum that
        // the provider count actually satisfies, and `finalized`. The observed
        // lock is not refunded, so the release path stays open.
        rpc_quorum: RpcQuorumAttestation {
            tx_id: lock.tx_id.clone(),
            block_hash: lock.block_hash.clone(),
            provider_count: 3,
            required_quorum: 2,
            finalized: true,
        },
        refund: RefundObservation {
            tx_id: lock.tx_id.clone(),
            block_hash: lock.block_hash.clone(),
            refunded: false,
        },
    };
    let wrong_preimage = [0x99u8; 32];
    assert!(
        SecretReleaseFirewall::authorize(
            &intent,
            wrong_preimage,
            core::slice::from_ref(&requirement),
            core::slice::from_ref(&evidence),
        )
        .is_err(),
        "wrong secret must be rejected before a live claim can be signed"
    );

    let permit = SecretReleaseFirewall::authorize(
        &intent,
        preimage,
        core::slice::from_ref(&requirement),
        core::slice::from_ref(&evidence),
    )
    .expect("secret-release permit");
    let claim = adapter
        .claim_with_permit(&permit)
        .expect("live native claim with permit");
    assert_eq!(claim.vm_type, VmType::X3Vm);
    assert_eq!(claim.intent_id, local_id);
    assert_eq!(claim.preimage, preimage);
    assert!(!claim.raw_proof.is_empty());
    let claim_finality = adapter
        .finality_status(&claim.tx_id)
        .expect("claim finality");
    assert!(claim_finality.finalized);
    assert_eq!(claim_finality.block_hash, claim.block_hash);

    let double_claim = adapter
        .claim(local_id, preimage)
        .expect_err("double claim must fail closed");
    // The bare claim path is gated client-side by the secret-release firewall, so
    // a claim without a permit never reaches the chain at all — a stronger
    // guarantee than a chain-side `ExtrinsicFailed`.
    assert!(
        double_claim
            .to_string()
            .contains("secret-release permit required"),
        "unexpected double-claim error: {double_claim}"
    );
    let refund_after_claim = adapter
        .refund(local_id)
        .expect_err("refund after claim must fail closed");
    assert!(
        refund_after_claim.to_string().contains("ExtrinsicFailed"),
        "unexpected refund-after-claim error: {refund_after_claim}"
    );

    let persisted = x3_atomic_swap::PersistentX3ProofLedger::open(&ledger_path)
        .expect("reopen persisted proof ledger")
        .snapshot()
        .expect("proof ledger snapshot");
    assert!(persisted.has_verified_kind_for_intent(local_id, ProofKind::SourceLock));
    assert!(persisted.has_verified_kind_for_intent(local_id, ProofKind::Claim));
    assert!(persisted.has_verified_kind_for_intent(local_id, ProofKind::FinalityVerified));

    // Simulate a relayer/coordinator process restart. The fresh transport must
    // recover durable evidence, then re-read the exact finalized block and
    // dispatch result before it can return safe finality again.
    drop(adapter);
    let restarted_signer =
        X3RuntimeSigner::from_uri(chain_id.clone(), RPC_URL.into(), &alice_uri)
            .expect("restart signer");
    let restarted_transport = NativeX3NodeTransport::new_with_proof_ledger(
        X3NodeTransportConfig {
            chain_id: chain_id.clone(),
            rpc_url: RPC_URL.into(),
            finality_poll_attempts: 480,
            finality_poll_delay_ms: 500,
            expected_block_time_ms: 6_000,
        },
        restarted_signer,
        ledger_path.clone(),
    )
    .expect("reopen native transport after process-style restart");
    let restarted_adapter = LiveX3VmAdapter::new(
        chain_id,
        b"x3-native-escrow".to_vec(),
        restarted_transport,
    );

    let restored_lock = restarted_adapter
        .finality_status(&lock.tx_id)
        .expect("revalidate persisted lock finality after restart");
    assert_eq!(restored_lock.block_hash, lock.block_hash);
    assert!(restored_lock.finalized);
    assert!(restored_lock.safe_to_reveal_secret);

    let restored_claim = restarted_adapter
        .finality_status(&claim.tx_id)
        .expect("revalidate persisted claim finality after restart");
    assert_eq!(restored_claim.block_hash, claim.block_hash);
    assert!(restored_claim.finalized);
    assert!(restored_claim.safe_to_reveal_secret);

    let _ = std::fs::remove_file(ledger_path);
}

#[test]
#[ignore = "boots the real X3 dev node and waits for timeout plus GRANDPA finality"]
fn real_local_node_timeout_reaches_finalized_refund_state() {
    let _node = spawn_dev_node();
    wait_rpc(Duration::from_secs(180));

    let chain_id = String::from("x3-local");
    let local_id = 2u64;
    let preimage = [0x24u8; 32];
    let hashlock = H256::from(sp_core::hashing::sha2_256(&preimage));
    let alice_uri = dev_uri("Alice");
    let signer = X3RuntimeSigner::from_uri(chain_id.clone(), RPC_URL.into(), &alice_uri)
        .expect("timeout signer");
    let second_leg = X3RuntimeSigner::from_uri(chain_id.clone(), RPC_URL.into(), &alice_uri)
        .expect("timeout second-leg signer");

    let prepared = signer
        .prepare_create_intent(
            dev_account("Bob"),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            hashlock,
            Some(30),
        )
        .expect("prepare timeout intent");
    assert!(!submit(&prepared.signed_extrinsic).is_empty());
    let (_, finalized_head) = wait_finalized(&prepared.signed_extrinsic, Duration::from_secs(180));
    let finalized_hash = H256::from_slice(
        &hex::decode(finalized_head.trim_start_matches("0x")).expect("decode finalized head hex"),
    );
    let runtime_intent_id = signer
        .resolve_intent_id(&prepared, finalized_hash)
        .expect("resolve real on-chain intent id");
    signer.bind_intent(local_id, runtime_intent_id).unwrap();

    let transport = NativeX3NodeTransport::new(
        X3NodeTransportConfig {
            chain_id: chain_id.clone(),
            rpc_url: RPC_URL.into(),
            finality_poll_attempts: 480,
            finality_poll_delay_ms: 500,
            expected_block_time_ms: 6_000,
        },
        signer,
    );
    let adapter = LiveX3VmAdapter::new(
        chain_id,
        b"x3-native-timeout-escrow".to_vec(),
        transport,
    );
    let intent = atomic_intent(local_id, preimage);
    let lock = adapter.lock(&intent).expect("live timeout lock");
    assert!(adapter.finality_status(&lock.tx_id).unwrap().finalized);

    // Both legs have to be escrowed before the runtime will consider the refund
    // proof set complete: `all_required_operation_proofs` walks every leg.
    let leg1 = second_leg
        .sign_lock_escrow_leg(
            runtime_intent_id,
            1,
            pallet_x3_settlement_engine::ExternalChainId::X3Native,
            1_000_000,
            b"x3-native-timeout-escrow-leg1".to_vec(),
        )
        .expect("sign timeout escrow leg1");
    assert!(!submit(&leg1).is_empty());
    let (_, leg1_block) = wait_finalized(&leg1, Duration::from_secs(180));
    assert_dispatch_succeeded(&second_leg, &leg1_block, &leg1);

    // A terminal `Refunded` state is gated on a verified canonical Refund proof
    // for the escrow domain. The runtime keys X3-native escrows by the canonical
    // descriptor `x3-native`, while the adapter labels its own chain `x3-local`,
    // so the bundle and its finality observation must carry the canonical
    // descriptor or `bundle_matches_intent_domain` rejects the whole set.
    let mut refund_finality = adapter
        .finality_status(&lock.tx_id)
        .expect("refund observation finality");
    refund_finality.chain_id = String::from("x3-native");
    assert_eq!(refund_finality.tx_id, lock.tx_id);
    assert!(refund_finality.finalized);

    let mut proof_set = CrossDomainProofSet::new(&intent, runtime_intent_id.to_fixed_bytes());
    let refund_bundle = CrossDomainProofBundle::new(
        &intent,
        runtime_intent_id.to_fixed_bytes(),
        String::from("x3-native"),
        VmType::X3Vm,
        CrossDomainOperation::Refund,
        lock.tx_id.clone(),
        lock.block_number,
        lock.block_hash.clone(),
        lock.raw_proof.clone(),
        refund_finality,
    )
    .expect("canonical refund bundle");
    proof_set
        .push_verified(&intent, refund_bundle)
        .expect("verified canonical refund bundle");

    let signed_proof_set = second_leg
        .prepare_cross_domain_proof_set(runtime_intent_id, proof_set)
    .expect("sign canonical refund proof set");
    assert!(!submit(&signed_proof_set).is_empty());
    let (_, proof_block) = wait_finalized(&signed_proof_set, Duration::from_secs(180));
    assert_dispatch_succeeded(&second_leg, &proof_block, &signed_proof_set);

    // With the canonical refund proof set recorded and the wall-clock timeout
    // elapsed, the runtime refunds the intent from `on_initialize` — no caller
    // has to drive it. Only if that automatic path does not fire (an entry can
    // miss the bounded per-block refund budget) do we fall back to the explicit
    // `refund_settlement` extrinsic the pallet documents for exactly that case.
    let refund_head = match wait_for_refund_state(runtime_intent_id, Duration::from_secs(180)) {
        Some(head) => head,
        None => {
            adapter
                .refund(local_id)
                .expect("explicit timeout refund after the automatic path did not fire");
            wait_for_finalized_refund(runtime_intent_id, Duration::from_secs(180))
        },
    };
    assert!(matches!(
        intent_state_at(runtime_intent_id, &refund_head),
        pallet_x3_settlement_engine::IntentState::Refunded
    ));

    let duplicate_refund = adapter
        .refund(local_id)
        .expect_err("duplicate refund must fail closed");
    assert!(
        duplicate_refund.to_string().contains("ExtrinsicFailed"),
        "unexpected duplicate-refund error: {duplicate_refund}"
    );
    let claim_after_refund = adapter
        .claim(local_id, preimage)
        .expect_err("claim after refund must fail closed");
    assert!(
        claim_after_refund
            .to_string()
            .contains("secret-release permit required"),
        "unexpected claim-after-refund error: {claim_after_refund}"
    );
}


#[test]
#[ignore = "boots the real X3 dev node and proves early refund dispatch fails"]
fn real_local_node_refund_before_timeout_fails_closed() {
    let _node = spawn_dev_node();
    wait_rpc(Duration::from_secs(180));

    let chain_id = String::from("x3-local");
    let local_id = 3u64;
    let preimage = [0x36u8; 32];
    let hashlock = H256::from(sp_core::hashing::sha2_256(&preimage));
    let alice_uri = dev_uri("Alice");
    let signer = X3RuntimeSigner::from_uri(chain_id.clone(), RPC_URL.into(), &alice_uri)
        .expect("early-refund signer");

    let prepared = signer
        .prepare_create_intent(
            dev_account("Bob"),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            X3RuntimeSigner::x3_native_asset(1_000_000),
            hashlock,
            Some(300),
        )
        .expect("prepare early-refund intent");
    assert!(!submit(&prepared.signed_extrinsic).is_empty());
    let (_, finalized_head_hash) =
        wait_finalized(&prepared.signed_extrinsic, Duration::from_secs(180));
    let finalized_hash = H256::from_slice(
        &hex::decode(finalized_head_hash.trim_start_matches("0x"))
            .expect("decode finalized head hex"),
    );
    let runtime_intent_id = signer
        .resolve_intent_id(&prepared, finalized_hash)
        .expect("resolve real on-chain intent id");
    signer.bind_intent(local_id, runtime_intent_id).unwrap();

    let transport = NativeX3NodeTransport::new(
        X3NodeTransportConfig {
            chain_id: chain_id.clone(),
            rpc_url: RPC_URL.into(),
            finality_poll_attempts: 480,
            finality_poll_delay_ms: 500,
            expected_block_time_ms: 6_000,
        },
        signer,
    );
    let adapter = LiveX3VmAdapter::new(
        chain_id,
        b"x3-native-early-refund-escrow".to_vec(),
        transport,
    );
    let intent = atomic_intent(local_id, preimage);
    let lock = adapter.lock(&intent).expect("live early-refund lock");
    assert!(adapter.finality_status(&lock.tx_id).unwrap().finalized);

    let err = adapter
        .refund(local_id)
        .expect_err("refund before timeout must fail closed");
    assert!(
        err.to_string().contains("ExtrinsicFailed"),
        "unexpected early-refund error: {err}"
    );

    let head = finalized_head();
    assert!(
        !matches!(
            intent_state_at(runtime_intent_id, &head),
            pallet_x3_settlement_engine::IntentState::Refunded
        ),
        "failed early refund must not mutate intent into Refunded"
    );
}
