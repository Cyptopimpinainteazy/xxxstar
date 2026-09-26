//! X3Lang proven across the validator set, not just on the node that took the submission.
//!
//! `node/tests/x3vm_live_lifecycle.rs` proves the whole X3Lang path — `.x3` source, the compiler,
//! X3BC, `submit_comit_v2`, finality, and the stored execution receipt — on **one** node. The
//! public-testnet row is narrower than that: the artifact has to travel through *consensus*, so a
//! validator that never saw the submission has to finalize the same block and answer the same
//! receipt for it.
//!
//! This test boots the built-in three-validator `local3` chain (Alice/Bob/Charlie), submits the
//! compiled program to **Alice's** RPC endpoint only, and then does every read against
//! **Charlie** — a node that was never handed the extrinsic:
//!
//! 1. Charlie finalizes the block the extrinsic landed in and reports the same block hash at that
//!    height as Alice and Bob (so the three are on one chain, not three lookalikes).
//! 2. Charlie's own copy of that block contains the signed extrinsic, and the dispatch event at
//!    its index is `ExtrinsicSuccess` — read out of Charlie's `System::Events` at that block hash.
//! 3. Charlie answers the execution receipt from finalized state, both through `state_getStorage`
//!    and through the `AtlasKernelRuntimeApi_get_x3_execution_receipt` runtime API a client uses.
//! 4. All three validators return a byte-identical receipt at the same block hash.
//!
//! The receipt is also required to be *absent* before the program was ever submitted and for a
//! comit id nobody submitted, because a read that answers regardless of what was executed has
//! proven nothing.
//!
//! Two things about this box are load-bearing for a *consensus* test and are handled explicitly
//! below. The three validators are spawned from one frozen copy of the node binary, because this
//! tree is rebuilt by other gates and agents while tests run and the runtime blob is part of the
//! genesis state: two builds of the same code have two genesis hashes, so a set whose processes
//! were exec'd from different builds can never peer (`Genesis mismatch`) and never finalizes. And
//! the six ports are probed before anything is spawned, because a second validator set on the same
//! ports does not fail — it answers `system_health` on somebody else's node and then wedges.
//!
//! ```text
//! env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3lang_network_receipt \
//!   -- --ignored --nocapture --test-threads=1
//! ```

use codec::{Decode, Encode};
use serde_json::Value;
use sp_core::H256;
use std::fs::File;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use x3_atomic_swap::{RpcClient, X3ExtrinsicSigner};
use x3_chain_node::x3vm_runtime_signer::X3RuntimeSigner;
use x3_common::bytecode::{HEADER_LEN, MAGIC as X3BC_MAGIC};

/// The chain id the signer stamps into `SignedPayload`. It is a label: the genesis hash the
/// signature actually commits to is read from the live node by `X3RuntimeSigner`.
const CHAIN_ID: &str = "x3-local3";

// Three validators, six ports. These are deliberately away from the single-node live test
// (19944/30379) so the two gates can share a box without binding a port the other one holds.
// They are checked to be free before anything is spawned — see `assert_ports_free`.
const ALICE_RPC: u16 = 19954;
const BOB_RPC: u16 = 19955;
const CHARLIE_RPC: u16 = 19956;
const ALICE_P2P: u16 = 30389;
const BOB_P2P: u16 = 30390;
const CHARLIE_P2P: u16 = 30391;

/// Three authorities need all three online to reach GRANDPA's 2/3, so boots on this chain are
/// slower than a dev node's; every wait below is sized for a debug build of a 1.4 GB binary.
const NODE_BOOT_TIMEOUT: Duration = Duration::from_secs(300);
const CONSENSUS_TIMEOUT: Duration = Duration::from_secs(300);
const FINALITY_TIMEOUT: Duration = Duration::from_secs(300);

/// Finality has to be past genesis for every validator: a chain whose finalized head is block 0
/// has not agreed on anything, and a receipt read at block 0 would be a genesis-state read.
const MIN_FINALIZED: u64 = 3;
/// Three validators, each connected to the other two.
const EXPECTED_PEERS: u64 = 2;

/// The program under test. Short on purpose: what is being proven is that the *compiler's* output
/// round-trips through consensus, not that the test can write a long program.
const PROGRAM_SOURCE: &str = "fn main() -> i64 {\n    return 42;\n}\n";
/// What `PROGRAM_SOURCE` returns, and therefore what the receipt has to carry.
const PROGRAM_RETURN_VALUE: i64 = 42;
/// The fee the comit declares, matching the single-node live test's value.
const COMIT_FEE: u128 = 1_000_000;
/// A comit id for *this* test's submission, and one nobody submits (used below to prove the
/// receipt read is keyed rather than a constant).
const COMIT_ID_SEED: u64 = 0x0078_336c_616e_6731;
const UNSENT_COMIT_ID_SEED: u64 = 0x0078_336c_616e_6732;

/// `X3VmAdapter::execute` stamps every X3 receipt with this protocol version
/// (`pallets/x3-kernel/src/adapters.rs`). Naming it here means the assertion fails loudly if the
/// adapter's protocol changes, instead of silently passing on "some nonzero number".
const X3_ADAPTER_PROTOCOL_VERSION: u32 = 1;

fn rpc_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// One JSON-RPC call to one validator. The error is returned rather than swallowed so a caller can
/// tell "the node refused" from "the node never answered" — the two have different fixes.
///
/// A success carrying `null` is an *answer*, not an error: `state_getStorage` answers `null` for a
/// key the chain never wrote, which is exactly the negative result the receipt checks below need to
/// be able to observe. It comes back as `Value::Null` and callers that need a string say so.
fn rpc_try(port: u16, method: &str, params: Vec<Value>) -> Result<Value, String> {
    let mut rpc = RpcClient::new(rpc_url(port), 0);
    let response = rpc
        .call(method, params)
        .map_err(|e| format!("{method} on :{port} failed: {e}"))?;
    Ok(response.result.unwrap_or(Value::Null))
}

fn rpc_expect(port: u16, method: &str, params: Vec<Value>) -> Value {
    rpc_try(port, method, params).unwrap_or_else(|e| panic!("{e}"))
}

fn rpc_string(port: u16, method: &str, params: Vec<Value>) -> String {
    rpc_expect(port, method, params)
        .as_str()
        .unwrap_or_else(|| panic!("{method} on :{port} did not return a string"))
        .to_string()
}

fn peers_of(port: u16) -> u64 {
    rpc_expect(port, "system_health", Vec::new())
        .get("peers")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn header_number(port: u16, hash: &str) -> u64 {
    let raw = rpc_expect(port, "chain_getHeader", vec![Value::String(hash.into())])
        .get("number")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("chain_getHeader on :{port} had no number"))
        .to_string();
    u64::from_str_radix(raw.trim_start_matches("0x"), 16)
        .unwrap_or_else(|e| panic!("header number '{raw}' on :{port} is not hex: {e}"))
}

fn best_number(port: u16) -> u64 {
    let raw = rpc_expect(port, "chain_getHeader", Vec::new())
        .get("number")
        .and_then(Value::as_str)
        .expect("best header number")
        .to_string();
    u64::from_str_radix(raw.trim_start_matches("0x"), 16).expect("hex best number")
}

fn finalized_head(port: u16) -> String {
    rpc_string(port, "chain_getFinalizedHead", Vec::new())
}

fn finalized_number(port: u16) -> u64 {
    header_number(port, &finalized_head(port))
}

/// The canonical hash at a height, as *this* validator sees it. Heights are decimal in this node's
/// JSON-RPC; the hex form is tried second because a client that only understands one of the two
/// should not be able to turn a chain-agreement check into a false failure.
fn block_hash_at(port: u16, number: u64) -> Option<String> {
    let params = [
        Value::Number(number.into()),
        Value::String(format!("0x{number:x}")),
    ];
    for param in params {
        let hash = rpc_try(port, "chain_getBlockHash", vec![param])
            .ok()
            .and_then(|v| v.as_str().map(ToOwned::to_owned));
        if hash.is_some() {
            return hash;
        }
    }
    None
}

fn block_extrinsics(port: u16, hash: &str) -> Vec<String> {
    rpc_expect(port, "chain_getBlock", vec![Value::String(hash.into())])
        .pointer("/block/extrinsics")
        .and_then(Value::as_array)
        .map(|xs| {
            xs.iter()
                .filter_map(|x| x.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn extrinsic_index_in_block(port: u16, hash: &str, signed: &str) -> Option<u32> {
    block_extrinsics(port, hash)
        .iter()
        .position(|x| x == signed)
        .map(|position| position as u32)
}

/// Poll finalized blocks for `signed`, scanning every height finalized since the previous poll.
///
/// Scanning the *range* rather than only the current head matters: a block can be produced and
/// finalized between two polls, and a check that only ever looks at the newest finalized head
/// would step over the block that actually contains the extrinsic.
fn wait_finalized(port: u16, signed: &str, timeout: Duration) -> (u64, String) {
    let started = Instant::now();
    let mut next_number: Option<u64> = None;
    while started.elapsed() < timeout {
        if let Ok(head) = rpc_try(port, "chain_getFinalizedHead", Vec::new()) {
            if let Some(head) = head.as_str() {
                let head_number = header_number(port, head);
                let start = next_number.unwrap_or(head_number);
                if head_number >= start {
                    for number in start..=head_number {
                        if let Some(hash) = block_hash_at(port, number) {
                            if block_extrinsics(port, &hash).iter().any(|x| x == signed) {
                                return (number, hash);
                            }
                        }
                    }
                }
                next_number = Some(head_number + 1);
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!(":{port} did not finalize the submitted extrinsic within {timeout:?}");
}

/// The storage key of a comit's X3 execution receipt.
///
/// `AtlasKernel::X3ExecutionReceipts` is a `Blake2_128Concat` map: pallet/item prefix, then the
/// 128-bit hash of the encoded key, then the key itself.
fn x3_receipt_storage_key(comit_id: H256) -> String {
    let mut key =
        frame_support::storage::storage_prefix(b"AtlasKernel", b"X3ExecutionReceipts").to_vec();
    let encoded = comit_id.encode();
    key.extend_from_slice(&sp_core::hashing::blake2_128(&encoded));
    key.extend_from_slice(&encoded);
    format!("0x{}", hex::encode(key))
}

/// Read a comit's X3 receipt from one validator, at one block. `None` means that chain stored no
/// receipt for that comit at that block — which is a different thing from an RPC failure, and this
/// distinguishes them on purpose.
fn x3_receipt_at(
    port: u16,
    comit_id: H256,
    block_hash: &str,
) -> Option<pallet_x3_kernel::ExecutionReceipt> {
    let result = rpc_try(
        port,
        "state_getStorage",
        vec![
            Value::String(x3_receipt_storage_key(comit_id)),
            Value::String(block_hash.to_string()),
        ],
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let raw = result.as_str()?;
    let bytes = hex::decode(raw.trim_start_matches("0x")).expect("decode receipt hex");
    Some(
        pallet_x3_kernel::ExecutionReceipt::decode(&mut &bytes[..])
            .expect("decode ExecutionReceipt"),
    )
}

/// The same receipt through the runtime API a *client* calls (`state_call`), so the accessor is
/// proven reachable by name over the wire and not merely declared in the runtime.
fn x3_receipt_via_runtime_api(
    port: u16,
    comit_id: H256,
    block_hash: &str,
) -> Option<pallet_x3_kernel::ExecutionReceipt> {
    let encoded_input = comit_id.as_bytes().to_vec().encode();
    let result = rpc_try(
        port,
        "state_call",
        vec![
            Value::String("AtlasKernelRuntimeApi_get_x3_execution_receipt".into()),
            Value::String(format!("0x{}", hex::encode(encoded_input))),
            Value::String(block_hash.to_string()),
        ],
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let raw = match result.as_str() {
        Some(raw) => raw.to_string(),
        // The API returns `Option<Vec<u8>>`; a null result means the chain has no receipt for this
        // comit at this block, which is the same answer the storage read gives.
        None if result.is_null() => return None,
        None => panic!("state_call on :{port} did not return a hex string: {result}"),
    };
    let bytes = hex::decode(raw.trim_start_matches("0x")).expect("decode state_call hex");
    let encoded_result: Option<Vec<u8>> =
        Decode::decode(&mut &bytes[..]).expect("the API returns Option<Vec<u8>>");
    encoded_result.map(|encoded| {
        pallet_x3_kernel::ExecutionReceipt::decode(&mut &encoded[..])
            .expect("the API's payload decodes as ExecutionReceipt")
    })
}

/// Kill every validator on drop, including on a panic inside the test body.
struct Local3Network {
    base_path: PathBuf,
    node_bin_digest: Option<String>,
    children: Vec<Child>,
}

impl Drop for Local3Network {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
        }
        for child in &mut self.children {
            let _ = child.wait();
        }
        if std::thread::panicking() {
            // Keep the logs, the frozen binary and the three data dirs so the failure can be read
            // after the fact; print where they are.
            eprintln!(
                "[x3-lang-net] test panicked — keeping {} for diagnosis",
                self.base_path.display()
            );
        } else {
            let _ = std::fs::remove_dir_all(&self.base_path);
        }
    }
}

fn dev_node_key(seed: u32) -> String {
    // A deterministic 32-byte libp2p key per validator, so the three identities are stable across
    // runs and a failure is reproducible. `--alice` supplies session keys, not a network identity.
    format!("{seed:064x}")
}

/// Every port this gate binds, in the order it binds them.
fn gate_ports() -> [u16; 6] {
    [
        ALICE_RPC,
        BOB_RPC,
        CHARLIE_RPC,
        ALICE_P2P,
        BOB_P2P,
        CHARLIE_P2P,
    ]
}

/// Refuse to start if any of the six ports is already bound.
///
/// A second validator set on these ports does not announce itself: the new node fails to bind, the
/// gate's `system_health` probe answers from the *other* set, and the run wedges into a genesis
/// mismatch or a finality timeout minutes later. Measuring the ports first turns that into one
/// sentence naming the port, which is the difference between a five-second fix and a long hunt.
fn assert_ports_free() {
    // "Cannot bind" is not the same as "someone is there": the three nodes just killed by a
    // previous run leave their accepted P2P connections in `TIME_WAIT`, and `TcpListener::bind`
    // does not set `SO_REUSEADDR`, so the P2P ports can refuse a bind for a few seconds while
    // nothing at all is listening. Waiting that out keeps this gate from going red on a port that
    // is merely settling, while a port somebody really holds is still reported immediately.
    const SETTLE_TIMEOUT: Duration = Duration::from_secs(20);
    const POLL_INTERVAL: Duration = Duration::from_millis(500);
    let started = Instant::now();
    loop {
        let mut live = Vec::new();
        let mut settling = Vec::new();
        for port in gate_ports() {
            if let Err(error) = TcpListener::bind(("127.0.0.1", port)) {
                let entry = format!("{port} ({error})");
                if port_has_listener(port) {
                    live.push(entry);
                } else {
                    settling.push(entry);
                }
            }
        }
        if live.is_empty() && settling.is_empty() {
            return;
        }
        if !live.is_empty() || started.elapsed() >= SETTLE_TIMEOUT {
            let mut report = live;
            report.extend(settling);
            panic!(
                "this gate's ports are already bound: {}. Another validator set (or a stale node) \
                 holds them, so the three validators below could never all be ours. This gate is \
                 serial by construction; stop the other run and try again.",
                report.join(", ")
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Whether something actually answers on this loopback port, as opposed to an unbound-but-unusable
/// socket left behind by a connection that is still being reaped.
fn port_has_listener(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

/// Freeze the node binary this run will use, and return its path plus `sha256` when the box has a
/// `sha256sum` to compute one with.
///
/// The runtime blob is part of genesis, so two builds of this tree have two genesis hashes. Other
/// gates and agents on this box rebuild the binary while tests run, and a validator set exec'd
/// across two builds rejects its own bootnodes with `Genesis mismatch`. Copying once and spawning
/// all three from the copy makes the artifact under test a constant for the run, and prints the
/// digest so the evidence names which build was proven.
fn freeze_node_binary(base_path: &Path) -> (PathBuf, Option<String>) {
    let source = PathBuf::from(env!("CARGO_BIN_EXE_x3-chain-node"));
    let frozen = base_path.join("x3-chain-node");
    std::fs::copy(&source, &frozen).unwrap_or_else(|e| {
        panic!(
            "freeze the node binary {} -> {}: {e}",
            source.display(),
            frozen.display()
        )
    });
    let digest = Command::new("sha256sum")
        .arg(&frozen)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|line| line.split_whitespace().next().map(ToOwned::to_owned));
    (frozen, digest)
}

#[allow(clippy::too_many_arguments)]
fn spawn_validator(
    name: &str,
    node_bin: &Path,
    key_flag: &str,
    node_key_seed: u32,
    rpc_port: u16,
    p2p_port: u16,
    base_path: &Path,
    log_path: &Path,
    bootnodes: &[String],
) -> Child {
    let mut command = Command::new(node_bin);
    command.args([
        "--chain",
        "local3",
        key_flag,
        "--base-path",
        base_path.to_str().expect("utf-8 base path"),
        "--node-key",
        &dev_node_key(node_key_seed),
        "--rpc-port",
        &rpc_port.to_string(),
        "--port",
        &p2p_port.to_string(),
        // `state_getStorage`/`state_call` at a block and `author_submitExtrinsic` are all unsafe
        // methods; a validator that refuses them cannot be asked the questions this test asks.
        "--rpc-methods",
        "unsafe",
        "--no-mdns",
        "--no-telemetry",
        // Every gate that boots a dev chain binds the default metrics port 9615; this gate asks
        // about consensus state, not metrics, so it stays off that port entirely.
        "--no-prometheus",
    ]);
    if !bootnodes.is_empty() {
        command.arg("--bootnodes").args(bootnodes);
    }
    let log = File::create(log_path).unwrap_or_else(|e| panic!("create {name} log: {e}"));
    command.stdout(Stdio::from(log.try_clone().expect("clone log handle")));
    command.stderr(Stdio::from(log));
    let child = command
        .spawn()
        .unwrap_or_else(|e| panic!("spawn validator {name}: {e}"));
    println!(
        "[x3-lang-net] {name}: rpc :{rpc_port}, p2p :{p2p_port}, log {}",
        log_path.display()
    );
    child
}

fn wait_for_rpc(name: &str, port: u16, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        // A node mid-initialisation can answer the HTTP request without a usable result; only a
        // real `system_health` payload counts as "up".
        if rpc_try(port, "system_health", Vec::new())
            .map(|value| value.get("peers").is_some())
            .unwrap_or(false)
        {
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!("{name} (: {port}) did not answer system_health within {timeout:?}");
}

/// Boot Alice, Bob and Charlie on the built-in `local3` chain and wait until the three have
/// connected, finalized past genesis, and agree on the canonical hash at a common height.
///
/// The orchestration mirrors `scripts/local-network-smoke.sh` (that script is the repository's own
/// bring-up for this chain); the difference is what happens after: this test drives a compiled
/// program through the network instead of stopping at "they agree".
fn boot_local3() -> Local3Network {
    assert_ports_free();

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_nanos();
    let base_path =
        std::env::temp_dir().join(format!("x3-lang-net-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&base_path).expect("create base path");
    let (node_bin, node_bin_digest) = freeze_node_binary(&base_path);
    println!(
        "[x3-lang-net] all three validators run one frozen artifact: {} (sha256 {})",
        node_bin.display(),
        node_bin_digest.as_deref().unwrap_or("unavailable")
    );

    let mut children = Vec::new();

    children.push(spawn_validator(
        "alice",
        &node_bin,
        "--alice",
        1,
        ALICE_RPC,
        ALICE_P2P,
        &base_path.join("alice"),
        &base_path.join("alice.log"),
        &[],
    ));
    wait_for_rpc("alice", ALICE_RPC, NODE_BOOT_TIMEOUT);
    let alice_peer_id = rpc_string(ALICE_RPC, "system_localPeerId", Vec::new());
    let alice_bootnode = format!("/ip4/127.0.0.1/tcp/{ALICE_P2P}/p2p/{alice_peer_id}");

    children.push(spawn_validator(
        "bob",
        &node_bin,
        "--bob",
        2,
        BOB_RPC,
        BOB_P2P,
        &base_path.join("bob"),
        &base_path.join("bob.log"),
        std::slice::from_ref(&alice_bootnode),
    ));
    wait_for_rpc("bob", BOB_RPC, NODE_BOOT_TIMEOUT);
    let bob_peer_id = rpc_string(BOB_RPC, "system_localPeerId", Vec::new());
    let bob_bootnode = format!("/ip4/127.0.0.1/tcp/{BOB_P2P}/p2p/{bob_peer_id}");

    children.push(spawn_validator(
        "charlie",
        &node_bin,
        "--charlie",
        3,
        CHARLIE_RPC,
        CHARLIE_P2P,
        &base_path.join("charlie"),
        &base_path.join("charlie.log"),
        &[alice_bootnode, bob_bootnode],
    ));
    wait_for_rpc("charlie", CHARLIE_RPC, NODE_BOOT_TIMEOUT);

    let network = Local3Network {
        base_path,
        node_bin_digest,
        children,
    };
    println!("[x3-lang-net] base path: {}", network.base_path.display());

    // they have to find each other
    let started = Instant::now();
    let ports = [ALICE_RPC, BOB_RPC, CHARLIE_RPC];
    let mut peers = [0u64; 3];
    while started.elapsed() < CONSENSUS_TIMEOUT {
        for (index, port) in ports.iter().enumerate() {
            peers[index] = peers_of(*port);
        }
        if peers.iter().all(|p| *p >= EXPECTED_PEERS) {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
    assert!(
        peers.iter().all(|p| *p >= EXPECTED_PEERS),
        "the three validators never all connected \
         (alice={} bob={} charlie={}, expected at least {EXPECTED_PEERS} each)",
        peers[0],
        peers[1],
        peers[2]
    );
    println!(
        "[x3-lang-net] connected: alice={} bob={} charlie={} peers",
        peers[0], peers[1], peers[2]
    );

    // and agree on finalized history
    let started = Instant::now();
    let mut finalized = [0u64; 3];
    while started.elapsed() < CONSENSUS_TIMEOUT {
        for (index, port) in ports.iter().enumerate() {
            finalized[index] = finalized_number(*port);
        }
        if finalized.iter().all(|n| *n >= MIN_FINALIZED) {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
    assert!(
        finalized.iter().all(|n| *n >= MIN_FINALIZED),
        "finality did not reach {MIN_FINALIZED} on every validator \
         (alice={} bob={} charlie={})",
        finalized[0],
        finalized[1],
        finalized[2]
    );

    let common = *finalized.iter().min().expect("three finalized heights");
    let hashes: Vec<String> = ports
        .iter()
        .map(|port| {
            block_hash_at(*port, common)
                .unwrap_or_else(|| panic!(":{port} has no canonical hash at height {common}"))
        })
        .collect();
    assert!(
        hashes.iter().all(|h| h == &hashes[0]),
        "the validators disagree at finalized height {common}: {hashes:?}"
    );
    println!(
        "[x3-lang-net] consensus: finalized alice={} bob={} charlie={}, all agree on {common}:{}",
        finalized[0], finalized[1], finalized[2], hashes[0]
    );

    network
}

/// Submit a signed extrinsic to one validator and return the transaction hash it reports.
fn submit_to(port: u16, signed: &str) -> String {
    rpc_string(
        port,
        "author_submitExtrinsic",
        vec![Value::String(signed.to_string())],
    )
}

/// A signer used purely as an RPC-backed *verifier* against one validator's view of a block.
///
/// `verify_finalized_dispatch` reads `System::Events` at a block hash from the endpoint the signer
/// was built with, so pointing a signer at the observer is how the observer's own copy of the
/// dispatch event gets read. No transaction is signed with it.
fn observer_at(port: u16) -> X3RuntimeSigner {
    X3RuntimeSigner::from_uri(CHAIN_ID.into(), rpc_url(port), "//Charlie")
        .expect("build an observer signer")
}

#[test]
#[ignore = "boots the three-validator local3 network and proves a compiled .x3 receipt across it"]
fn a_compiled_x3_receipt_is_readable_from_a_validator_that_did_not_submit_it() {
    let comit_id = H256::from_low_u64_be(COMIT_ID_SEED);
    let unsent_comit_id = H256::from_low_u64_be(UNSENT_COMIT_ID_SEED);

    let network = boot_local3();

    // The receipt must not exist before the program is submitted. Anchored at the observer's
    // pre-submission finalized head: without this, a read that always answers would look like a
    // pass after the fact.
    let pre_submission_head = finalized_head(CHARLIE_RPC);
    assert!(
        x3_receipt_at(CHARLIE_RPC, comit_id, &pre_submission_head).is_none(),
        "Charlie already had a receipt for comit {comit_id:?} before anything was submitted"
    );
    println!(
        "[x3-lang-net] pre-submission: no receipt at Charlie's finalized head {pre_submission_head}"
    );

    // Compile the program in the test: this is the compiler's output, not a fixture blob.
    let program = x3_x3_integration::compiler_bridge::compile_source(PROGRAM_SOURCE)
        .expect("the fixture must compile");
    assert!(
        program.starts_with(X3BC_MAGIC),
        "the compiler must emit an X3BC module, not an arbitrary blob"
    );
    assert!(
        program.len() > HEADER_LEN,
        "an X3BC module is more than its {HEADER_LEN}-byte header"
    );
    println!(
        "[x3-lang-net] compiled {} bytes of X3BC from {} bytes of .x3 source",
        program.len(),
        PROGRAM_SOURCE.len()
    );

    let alice = X3RuntimeSigner::from_uri(CHAIN_ID.into(), rpc_url(ALICE_RPC), "//Alice")
        .expect("build Alice's signer against her own RPC endpoint");

    // Submit to Alice's endpoint and to no other. Everything below reads Charlie.
    let signed = alice
        .sign_kernel_submit_comit_v2(comit_id, program, COMIT_FEE)
        .expect("sign the compiled comit");
    let tx_hash = submit_to(ALICE_RPC, &signed);
    assert!(!tx_hash.is_empty(), "Alice's node must return a tx hash");
    println!("[x3-lang-net] submitted to alice only: {tx_hash}");

    // The observer has to finalize the block that contains it.
    let (number, hash) = wait_finalized(CHARLIE_RPC, &signed, FINALITY_TIMEOUT);
    println!("[x3-lang-net] charlie finalized {number}:{hash}");

    // All three have to hold *that* block — the same hash at the same height — or the receipt below
    // would only be Charlie's view of a private fork.
    for (name, port) in [("alice", ALICE_RPC), ("bob", BOB_RPC)] {
        let their_hash = block_hash_at(port, number)
            .unwrap_or_else(|| panic!("{name} has no canonical hash at height {number}"));
        assert_eq!(
            their_hash, hash,
            "{name} finalized a different block at height {number} than charlie"
        );
        assert!(
            block_extrinsics(port, &hash).iter().any(|x| x == &signed),
            "{name} does not carry the submitted extrinsic in block {hash}"
        );
        assert!(
            finalized_number(port) >= number,
            "{name} has not finalized height {number} yet"
        );
    }
    println!("[x3-lang-net] alice, bob and charlie all finalized {number}:{hash}");

    // The observer's own dispatch event, not the submitter's, has to say success: a rejected
    // extrinsic is still included in a finalized block, so inclusion alone is not the claim.
    let index = extrinsic_index_in_block(CHARLIE_RPC, &hash, &signed)
        .expect("Charlie's copy of the block contains the signed extrinsic");
    observer_at(CHARLIE_RPC)
        .verify_finalized_dispatch(&hash, index)
        .expect("Charlie's System::Events shows ExtrinsicSuccess at that index");
    println!("[x3-lang-net] charlie's dispatch event at index {index}: ExtrinsicSuccess");

    // The receipt, read from the validator that never saw the submission.
    let receipt = x3_receipt_at(CHARLIE_RPC, comit_id, &hash)
        .expect("an accepted X3 comit must store its execution receipt");
    assert!(
        receipt.success,
        "the fixture returns, so the receipt must succeed"
    );
    assert_eq!(
        receipt.return_data,
        PROGRAM_RETURN_VALUE.to_le_bytes().to_vec(),
        "the receipt must carry the value the compiled program returns"
    );
    // Gas has to be metered *and* inside the chain's own rule: `submit_comit_v2` hands
    // `T::X3Adapter::execute` exactly `DefaultX3GasLimit`, so a receipt above it would mean the
    // adapter ignored the limit it was given (and a zero would mean nothing was metered at all).
    let x3_gas_limit = <<x3_chain_runtime::Runtime as pallet_x3_kernel::Config>::DefaultX3GasLimit
        as frame_support::traits::Get<u64>>::get();
    assert!(
        receipt.gas_used > 0 && receipt.gas_used <= x3_gas_limit,
        "the receipt must report gas inside the chain's X3 limit ({x3_gas_limit}), \
         not {}",
        receipt.gas_used
    );
    assert_eq!(
        receipt.version,
        pallet_x3_kernel::EXECUTION_RECEIPT_VERSION,
        "the receipt must be stamped with the kernel's receipt version"
    );
    assert_eq!(
        receipt.protocol_version, X3_ADAPTER_PROTOCOL_VERSION,
        "the receipt must be stamped with the X3 adapter's protocol version"
    );
    assert!(
        receipt.from.is_empty() && receipt.to.is_empty() && receipt.value == 0,
        "an X3VM receipt is not an EVM transfer: it must not invent sender, target or value"
    );

    // And the client-facing accessor agrees, again from the observer.
    let via_api = x3_receipt_via_runtime_api(CHARLIE_RPC, comit_id, &hash)
        .expect("the runtime API must return the receipt Charlie stored");
    assert_eq!(
        via_api.encode(),
        receipt.encode(),
        "state_getStorage and the runtime API must describe the same receipt"
    );

    // Every validator holds the same receipt bytes at the same block.
    for (name, port) in [("alice", ALICE_RPC), ("bob", BOB_RPC)] {
        let theirs = x3_receipt_at(port, comit_id, &hash)
            .unwrap_or_else(|| panic!("{name} stored no receipt for the comit charlie executed"));
        assert_eq!(
            theirs.encode(),
            receipt.encode(),
            "{name}'s receipt differs from charlie's for the same block"
        );
    }
    println!("[x3-lang-net] all three validators return an identical receipt at {hash}");

    // And the read is keyed rather than a constant: nobody submitted this comit id, and the receipt
    // did not exist in the state finalized before the submission.
    assert!(
        x3_receipt_at(CHARLIE_RPC, unsent_comit_id, &hash).is_none(),
        "Charlie answered a receipt for a comit nobody submitted"
    );
    assert!(
        x3_receipt_at(CHARLIE_RPC, comit_id, &pre_submission_head).is_none(),
        "the receipt must not be readable at a block finalized before the submission"
    );

    println!(
        "[x3-lang-net] PASS — comit {comit_id:?} finalized at {number}:{hash} on a 3-validator \
         local3 network running the artifact sha256 {}, \
         return_data={:?}, gas_used={}, version={}, protocol_version={}, \
         best blocks alice={} bob={} charlie={}",
        network.node_bin_digest.as_deref().unwrap_or("unavailable"),
        receipt.return_data,
        receipt.gas_used,
        receipt.version,
        receipt.protocol_version,
        best_number(ALICE_RPC),
        best_number(BOB_RPC),
        best_number(CHARLIE_RPC),
    );
}
