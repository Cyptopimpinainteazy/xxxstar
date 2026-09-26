//! The supply invariant, proven on every validator's own view under distributed traffic.
//!
//! The public-testnet row says "Supply invariants: proven under distributed traffic". The kernel
//! pallet carries the invariant (`pallets/x3-kernel/src/invariant.rs`: for every asset,
//! `circulating + bridge_locked + pending_transfer + external_locked == total_issued`), but a
//! pallet unit test is one process holding one copy of the ledger — it says nothing about whether
//! three validators that each accepted a *different* signed extrinsic end up at the same
//! conserved state.
//!
//! This test boots the built-in three-validator `local3` chain, drives fee-burning X3 comits into
//! all three validators at once (one thread per validator, so the three streams are concurrent and
//! no two threads share an account nonce), and then, **on every validator separately, at one
//! finalized block**, reads the two sides of the conservation identity straight off that
//! validator's own state:
//!
//! ```text
//!   Σ over every account in System::Account of (free + reserved)
//!        ==
//!   AtlasKernelRuntimeApi::get_total_issuance()            // pallet_balances' TotalIssuance
//! ```
//!
//! The left side is read by enumerating `System::Account` with `state_getKeys` at the finalized
//! block hash and decoding each `AccountInfo`; the right side through the runtime API a client
//! calls. Both are read with that block hash as the `at` argument, so the two halves describe the
//! same block. Between them the check pins the whole conservation transition: a comit fee is
//! withdrawn from the caller and burned (`pallet_x3_kernel`'s `charge_fee` drops the
//! `NegativeImbalance`), so the caller's balance and the total issuance must fall together by the
//! same amount. The test requires that movement to be real — the callers' balances and the total
//! issuance both have to drop, and every comit has to leave an execution receipt readable from a
//! validator that never received the extrinsic — because a "conserved" chain that never moved
//! anything has proven nothing.
//!
//! ## Why there is a second, corrupted chain
//!
//! A conservation check that always passes is not evidence. The last phase takes the same `local3`
//! genesis as a raw chain spec, adds exactly one unit to `Balances::TotalIssuance` in a **scratch
//! copy** (a fresh raw spec, never the operator's chain or data dirs), boots one node on it, and
//! runs the *same* `check_conserved` used above. It must report the violation and name the delta.
//! The scratch copy is then discarded.
//!
//! ```text
//! env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test supply_invariant_distributed \
//!   -- --ignored --nocapture --test-threads=1
//! ```

use codec::{Decode, Encode};
use serde_json::Value;
use sp_core::H256;
use std::fs::File;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use x3_atomic_swap::RpcClient;
use x3_chain_node::x3vm_runtime_signer::X3RuntimeSigner;
use x3_common::bytecode::MAGIC as X3BC_MAGIC;

/// The chain id the signer stamps into `SignedPayload`. It is a label: the genesis hash the
/// signature actually commits to is read from the live node by `X3RuntimeSigner`.
const CHAIN_ID: &str = "x3-local3";

// Three validators, six ports, deliberately away from the other local3 gates on this box
// (`x3lang_network_receipt` holds 19954-19956 / 30389-30391, the runtime-upgrade rehearsal and the
// single-node live tests hold the rest of the 1994x block). They are checked free before anything
// is spawned: a second validator set on these ports answers `system_health` from somebody else's
// node and then wedges.
const ALICE_RPC: u16 = 19964;
const BOB_RPC: u16 = 19965;
const CHARLIE_RPC: u16 = 19966;
const ALICE_P2P: u16 = 30394;
const BOB_P2P: u16 = 30395;
const CHARLIE_P2P: u16 = 30396;
/// The single node the negative control boots from the corrupted scratch spec.
const SCRATCH_RPC: u16 = 19967;
const SCRATCH_P2P: u16 = 30397;

/// Three authorities need all three online to reach GRANDPA's 2/3, so boots on this chain are
/// slower than a dev node's; every wait below is sized for a debug build of a 1.4 GB binary.
const NODE_BOOT_TIMEOUT: Duration = Duration::from_secs(300);
const CONSENSUS_TIMEOUT: Duration = Duration::from_secs(300);
const RECEIPT_TIMEOUT: Duration = Duration::from_secs(300);
/// How long one comit may take to be included and dispatched before a stream gives up. Blocks on
/// this chain are sub-second, so this is generous; a timeout here means the comit was rejected.
const KERNEL_NONCE_TIMEOUT: Duration = Duration::from_secs(120);
/// `build-spec --raw` on the release-shaped runtime emits a ~17 MB JSON document; give it room.
const BUILD_SPEC_TIMEOUT: Duration = Duration::from_secs(300);

/// Finality has to be past genesis for every validator: a chain whose finalized head is block 0 has
/// agreed on nothing, and a supply read at block 0 is a genesis-state read.
const MIN_FINALIZED: u64 = 3;
/// Three validators, each connected to the other two.
const EXPECTED_PEERS: u64 = 2;

/// The program under test: short on purpose, because what is being proven is that value-moving
/// comits execute and that the ledger they move through stays conserved, not that the test can
/// write a long program.
const PROGRAM_SOURCE: &str = "fn main() -> i64 {\n    return 42;\n}\n";
/// The fee each comit declares. The kernel charges `required_fee <= fee`, so declaring a large
/// ceiling does not over-charge; it just has to clear `MIN_FEE`.
const COMIT_FEE: u128 = 1_000_000;
/// How many comits each of the three submitting accounts drives. Comfortably under the pallet's
/// `MAX_SUBMISSIONS_PER_BLOCK` (10) so the streams are limited by consensus, not by the rate limit.
const COMITS_PER_ACCOUNT: u64 = 5;
/// Base for the per-account comit ids. Each stream's ids are `SEED ^ (account << 32) ^ index`, so a
/// replayed id would be a `DuplicateComitId` error rather than a second, silent execution.
const COMIT_ID_SEED: u64 = 0x0078_3373_7570_7001;

/// `Balances::TotalIssuance`. Two `twox_128` prefixes concatenated: a `StorageValue` has no
/// key suffix. Computed rather than pasted, so a layout change is a compile-time/run-time failure
/// instead of a check that reads `0x`.
fn total_issuance_key() -> Vec<u8> {
    let key = frame_support::storage::storage_prefix(b"Balances", b"TotalIssuance").to_vec();
    debug_assert_eq!(key.len(), 32);
    key
}

/// Whether something actually answers on this loopback port, as opposed to an unbound-but-unusable
/// socket left behind by a connection that is still being reaped.
fn port_has_listener(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

fn rpc_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// One JSON-RPC call to one validator. The error is returned rather than swallowed so a caller can
/// tell "the node refused" from "the node never answered" — the two have different fixes.
///
/// A success carrying `null` is an *answer*, not an error: `state_getStorage` answers `null` for a
/// key the chain never wrote.
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

fn genesis_hash(port: u16) -> String {
    block_hash_at(port, 0).unwrap_or_else(|| panic!(":{port} has no genesis hash"))
}

/// The storage key of one account's `frame_system::AccountInfo`.
///
/// `System::Account` is a `Blake2_128Concat` map: pallet/item prefix, then the 128-bit hash of the
/// encoded key, then the key itself. Built the same way the runtime signer builds `AtlasKernel
/// Nonces` keys, so there is one rule for `Blake2_128Concat` in this file.
fn account_storage_key(account: &x3_chain_runtime::AccountId) -> String {
    let mut key = frame_support::storage::storage_prefix(b"System", b"Account").to_vec();
    let encoded = account.encode();
    key.extend_from_slice(&sp_core::hashing::blake2_128(&encoded));
    key.extend_from_slice(&encoded);
    format!("0x{}", hex::encode(key))
}

/// Every account the chain has ever written, as `System::Account` keys, at one block.
fn account_keys(port: u16, block: &str) -> Vec<String> {
    let prefix = frame_support::storage::storage_prefix(b"System", b"Account").to_vec();
    let value = rpc_expect(
        port,
        "state_getKeys",
        vec![
            Value::String(format!("0x{}", hex::encode(&prefix))),
            Value::String(block.to_string()),
        ],
    );
    value
        .as_array()
        .unwrap_or_else(|| panic!("state_getKeys on :{port} did not return an array: {value}"))
        .iter()
        .filter_map(|k| k.as_str().map(ToOwned::to_owned))
        .collect()
}

/// The runtime's `AccountInfo`: `frame_system`'s nonce/consumers/providers/sufficients header and
/// `pallet_balances`' free/reserved/frozen/flags data. Named through the runtime's own config types
/// so the decode is the one the chain writes, not a hand-copied byte layout.
type RuntimeAccountInfo = frame_system::AccountInfo<
    <x3_chain_runtime::Runtime as frame_system::Config>::Nonce,
    pallet_balances::AccountData<x3_chain_runtime::Balance>,
>;

/// Decode one `System::Account` value into (free, reserved). `None` means the key vanished between
/// the enumeration and the read, which is a different thing from a zero balance.
fn decode_free_reserved(port: u16, key: &str, raw: &str) -> Result<(u128, u128), String> {
    let bytes = hex::decode(raw.trim_start_matches("0x"))
        .map_err(|e| format!("{key} on :{port} was not hex: {e}"))?;
    let info = RuntimeAccountInfo::decode(&mut &bytes[..])
        .map_err(|e| format!("AccountInfo at {key} on :{port} did not decode: {e}"))?;
    Ok((info.data.free, info.data.reserved))
}

/// What one validator says at one block: how many accounts it holds, what they add up to, and what
/// it reports as total issuance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SupplyView {
    accounts: u64,
    accounted: u128,
    total_issuance: u128,
}

/// Read both sides of the conservation identity from one validator, at one block hash.
///
/// Every read is given the same `at` block, including the runtime API call, so the two halves
/// cannot come from different blocks of a chain that is still authoring.
fn read_supply(port: u16, block: &str) -> Result<SupplyView, String> {
    let keys = account_keys(port, block);
    let mut accounted: u128 = 0;
    let mut counted: u64 = 0;
    for key in &keys {
        let value = rpc_try(
            port,
            "state_getStorage",
            vec![Value::String(key.clone()), Value::String(block.to_string())],
        )?;
        let Some(raw) = value.as_str() else {
            // The key was enumerated at `block` and is gone at `block`: not a zero balance.
            return Err(format!(
                "System::Account key {key} was enumerated at {block} on :{port} but read back as {value}"
            ));
        };
        let (free, reserved) = decode_free_reserved(port, key, raw)?;
        accounted = accounted
            .checked_add(free)
            .and_then(|a| a.checked_add(reserved))
            .ok_or_else(|| format!("account sum overflowed reading :{port} at {block}"))?;
        counted += 1;
    }

    let raw_issuance = rpc_try(
        port,
        "state_call",
        vec![
            Value::String("AtlasKernelRuntimeApi_get_total_issuance".into()),
            Value::String("0x".into()),
            Value::String(block.to_string()),
        ],
    )?;
    let raw_issuance = raw_issuance
        .as_str()
        .ok_or_else(|| format!("get_total_issuance on :{port} did not return a string"))?;
    let issuance_bytes = hex::decode(raw_issuance.trim_start_matches("0x"))
        .map_err(|e| format!("get_total_issuance on :{port} was not hex: {e}"))?;
    let total_issuance = u128::decode(&mut &issuance_bytes[..])
        .map_err(|e| format!("get_total_issuance on :{port} did not decode as u128: {e}"))?;

    Ok(SupplyView {
        accounts: counted,
        accounted,
        total_issuance,
    })
}

/// The deliverable, as one function: at `block`, this validator's own account balances must add up
/// to the issuance it reports. `Err` names both sides and the delta, because "invariant violated"
/// without the numbers is not something anyone can act on.
fn check_conserved(port: u16, block: &str) -> Result<SupplyView, String> {
    let view = read_supply(port, block)?;
    if view.accounted != view.total_issuance {
        return Err(format!(
            "supply invariant violated on :{port} at {block}: {} accounts sum to {} but TotalIssuance is {} (delta {})",
            view.accounts,
            view.accounted,
            view.total_issuance,
            view.accounted.abs_diff(view.total_issuance),
        ));
    }
    Ok(view)
}

/// The per-asset supply ledger, read through the runtime API that exposes it.
///
/// Key is the **ledger's** `AssetId` (H256), value is the pallet's own `SupplyLedger`. Read with
/// the block hash as `at`, like everything else here, so every validator is asked about the same
/// block.
fn read_asset_ledger(
    port: u16,
    block: &str,
    asset: x3_asset_kernel_types::AssetId,
) -> Result<Option<x3_asset_kernel_types::SupplyLedger>, String> {
    let raw = rpc_try(
        port,
        "state_call",
        vec![
            Value::String("AtlasKernelRuntimeApi_get_asset_supply_ledger".into()),
            Value::String(format!("0x{}", hex::encode(asset.encode()))),
            Value::String(block.to_string()),
        ],
    )?;
    let raw = raw
        .as_str()
        .ok_or_else(|| format!("get_asset_supply_ledger on :{port} did not return a string"))?;
    let bytes = hex::decode(raw.trim_start_matches("0x"))
        .map_err(|e| format!("get_asset_supply_ledger on :{port} was not hex: {e}"))?;
    Option::<x3_asset_kernel_types::SupplyLedger>::decode(&mut &bytes[..])
        .map_err(|e| format!("get_asset_supply_ledger on :{port} did not decode: {e}"))
}

/// Every asset key the ledger holds, enumerated at `block`.
///
/// The prefix is the pallet's own (`X3SupplyLedger` + `Ledgers`), built with the same
/// `storage_prefix` helper the `System::Account` enumeration uses, so the test cannot drift from
/// the storage layout by hard-coding a hash.
fn ledger_assets(port: u16, block: &str) -> Vec<x3_asset_kernel_types::AssetId> {
    let prefix = frame_support::storage::storage_prefix(b"X3SupplyLedger", b"Ledgers").to_vec();
    let keys = rpc_expect(
        port,
        "state_getKeys",
        vec![
            Value::String(format!("0x{}", hex::encode(&prefix))),
            Value::String(block.to_string()),
        ],
    );
    let Some(keys) = keys.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in keys {
        let Some(key) = key.as_str() else { continue };
        let Ok(bytes) = hex::decode(key.trim_start_matches("0x")) else {
            continue;
        };
        // prefix (32) + Blake2_128Concat (16 byte hash + the 32-byte key)
        if bytes.len() != prefix.len() + 16 + 32 {
            continue;
        }
        let tail = &bytes[prefix.len() + 16..];
        out.push(H256::from_slice(tail));
    }
    out
}

/// One account's free balance at one block, read by its own key rather than by enumeration, so a
/// caller's before/after can be compared without depending on the account list being stable.
fn free_balance_at(port: u16, block: &str, account: &x3_chain_runtime::AccountId) -> u128 {
    let key = account_storage_key(account);
    let value = rpc_expect(
        port,
        "state_getStorage",
        vec![Value::String(key.clone()), Value::String(block.to_string())],
    );
    let raw = value
        .as_str()
        .unwrap_or_else(|| panic!("{key} on :{port} at {block} read back as {value}"));
    decode_free_reserved(port, &key, raw)
        .unwrap_or_else(|e| panic!("{e}"))
        .0
}

/// The storage key of a comit's X3 execution receipt, built the way
/// `x3lang_network_receipt.rs` builds it (`AtlasKernel::X3ExecutionReceipts`), so the receipt proof
/// below reads the same map that test reads.
fn x3_receipt_storage_key(comit_id: H256) -> String {
    let mut key =
        frame_support::storage::storage_prefix(b"AtlasKernel", b"X3ExecutionReceipts").to_vec();
    let encoded = comit_id.encode();
    key.extend_from_slice(&sp_core::hashing::blake2_128(&encoded));
    key.extend_from_slice(&encoded);
    format!("0x{}", hex::encode(key))
}

/// Whether a validator holds an execution receipt for `comit_id`: the comit was not merely
/// submitted, it executed and its receipt is in consensus state.
fn has_receipt(port: u16, comit_id: H256, block: &str) -> bool {
    rpc_try(
        port,
        "state_getStorage",
        vec![
            Value::String(x3_receipt_storage_key(comit_id)),
            Value::String(block.to_string()),
        ],
    )
    .map(|value| value.as_str().is_some())
    .unwrap_or(false)
}

fn wait_for_receipt(port: u16, comit_id: H256, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let head = finalized_head(port);
        if has_receipt(port, comit_id, &head) {
            return;
        }
        thread::sleep(Duration::from_millis(500));
    }
    panic!(":{port} never stored an execution receipt for comit {comit_id:?} within {timeout:?}");
}

/// Kill every validator on drop, including on a panic inside the test body.
struct ValidatorSet {
    base_path: PathBuf,
    children: Vec<Child>,
}

impl Drop for ValidatorSet {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
        }
        for child in &mut self.children {
            let _ = child.wait();
        }
        if std::thread::panicking() {
            // Keep the logs, the frozen binary and the data dirs so the failure can be read after
            // the fact; print where they are.
            eprintln!(
                "[x3-supply] test panicked — keeping {} for diagnosis",
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
fn gate_ports() -> [u16; 8] {
    [
        ALICE_RPC,
        BOB_RPC,
        CHARLIE_RPC,
        ALICE_P2P,
        BOB_P2P,
        CHARLIE_P2P,
        SCRATCH_RPC,
        SCRATCH_P2P,
    ]
}

/// Refuse to start if any of the eight ports is already bound.
///
/// A second validator set on these ports does not announce itself: the new node fails to bind, the
/// gate's `system_health` probe answers from the *other* set, and the run wedges into a genesis
/// mismatch or a finality timeout minutes later. Measuring the ports first turns that into one
/// sentence naming the port.
fn assert_ports_free() {
    // "Cannot bind" is not the same as "someone is there": nodes killed by a previous run leave
    // their accepted P2P connections in `TIME_WAIT`, and `TcpListener::bind` does not set
    // `SO_REUSEADDR`, so the P2P ports can refuse a bind for a few seconds while nothing is
    // listening. Waiting that out keeps the gate from going red on a port that is merely settling,
    // while a port somebody really holds is reported immediately.
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
                 holds them, so the validators below could never all be ours. This gate is serial \
                 by construction; stop the other run and try again.",
                report.join(", ")
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Freeze the node binary this run will use, and return its path plus `sha256` when the box has a
/// `sha256sum` to compute one with.
///
/// The runtime blob is part of genesis, so two builds of this tree have two genesis hashes. Other
/// gates and agents on this box rebuild the binary while tests run, and a validator set exec'd
/// across two builds rejects its own bootnodes with `Genesis mismatch` (and, for the negative
/// control, the mutated spec must be built by the same binary that later boots it). Copying once
/// and spawning everything from the copy makes the artifact under test a constant for the run, and
/// prints the digest so the evidence names which build was proven.
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

fn spawn_with_args(argv: &[String], log_path: &Path) -> Child {
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    let log =
        File::create(log_path).unwrap_or_else(|e| panic!("create {}: {e}", log_path.display()));
    command.stdout(Stdio::from(log.try_clone().expect("clone log handle")));
    command.stderr(Stdio::from(log));
    command
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", argv[0]))
}

/// The argument vector one validator is booted with. Shared by the three-validator set and the
/// negative control's single node so both are started the same way.
#[allow(clippy::too_many_arguments)] // one flag-shaped row, the same shape `spawn_validator` has
fn validator_argv(
    node_bin: &Path,
    name: &str,
    key_flag: &str,
    node_key_seed: u32,
    rpc_port: u16,
    p2p_port: u16,
    base_path: &Path,
    chain: &str,
    bootnodes: &[String],
) -> Vec<String> {
    let mut argv: Vec<String> = vec![
        node_bin.to_string_lossy().into_owned(),
        "--chain".into(),
        chain.into(),
        key_flag.into(),
        "--base-path".into(),
        base_path.to_string_lossy().into_owned(),
        "--node-key".into(),
        dev_node_key(node_key_seed),
        "--rpc-port".into(),
        rpc_port.to_string(),
        "--port".into(),
        p2p_port.to_string(),
        // `state_getKeys`/`state_getStorage` at a block and the runtime-API `state_call` are unsafe
        // methods; a validator that refuses them cannot be asked the questions this test asks.
        "--rpc-methods".into(),
        "unsafe".into(),
        "--no-mdns".into(),
        "--no-telemetry".into(),
        // Every gate that boots a dev chain binds the default metrics port 9615; this gate asks
        // about consensus state, not metrics, so it stays off that port entirely.
        "--no-prometheus".into(),
    ];
    if !bootnodes.is_empty() {
        argv.push("--bootnodes".into());
        argv.extend(bootnodes.iter().cloned());
    }
    println!(
        "[x3-supply] {name}: rpc :{rpc_port}, p2p :{p2p_port}, log {}",
        base_path.display()
    );
    argv
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

/// The three validators, all on one chain: connected, finalizing past genesis, and agreeing on the
/// canonical hash at a common height.
fn boot_local3(node_bin: &Path, base_path: &Path) -> ValidatorSet {
    std::fs::create_dir_all(base_path).expect("create the validator set's base path");
    let mut children = Vec::new();

    children.push(spawn_with_args(
        &validator_argv(
            node_bin,
            "alice",
            "--alice",
            1,
            ALICE_RPC,
            ALICE_P2P,
            &base_path.join("alice"),
            "local3",
            &[],
        ),
        &base_path.join("alice.log"),
    ));
    wait_for_rpc("alice", ALICE_RPC, NODE_BOOT_TIMEOUT);
    let alice_peer_id = rpc_string(ALICE_RPC, "system_localPeerId", Vec::new());
    let alice_bootnode = format!("/ip4/127.0.0.1/tcp/{ALICE_P2P}/p2p/{alice_peer_id}");

    children.push(spawn_with_args(
        &validator_argv(
            node_bin,
            "bob",
            "--bob",
            2,
            BOB_RPC,
            BOB_P2P,
            &base_path.join("bob"),
            "local3",
            std::slice::from_ref(&alice_bootnode),
        ),
        &base_path.join("bob.log"),
    ));
    wait_for_rpc("bob", BOB_RPC, NODE_BOOT_TIMEOUT);
    let bob_peer_id = rpc_string(BOB_RPC, "system_localPeerId", Vec::new());
    let bob_bootnode = format!("/ip4/127.0.0.1/tcp/{BOB_P2P}/p2p/{bob_peer_id}");

    children.push(spawn_with_args(
        &validator_argv(
            node_bin,
            "charlie",
            "--charlie",
            3,
            CHARLIE_RPC,
            CHARLIE_P2P,
            &base_path.join("charlie"),
            "local3",
            &[alice_bootnode, bob_bootnode],
        ),
        &base_path.join("charlie.log"),
    ));
    wait_for_rpc("charlie", CHARLIE_RPC, NODE_BOOT_TIMEOUT);

    let network = ValidatorSet {
        base_path: base_path.to_path_buf(),
        children,
    };

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
        "[x3-supply] connected: alice={} bob={} charlie={} peers",
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

    let (height, hash) = common_finalized(&ports);
    println!(
        "[x3-supply] consensus: finalized alice={} bob={} charlie={}, all agree on {height}:{hash}",
        finalized[0], finalized[1], finalized[2]
    );

    network
}

/// The highest height every validator has finalized, plus the canonical hash at it — after
/// requiring all three to report the *same* hash there. Three chains running side by side is not
/// one chain; this is the check that makes the supply numbers comparable.
fn common_finalized(ports: &[u16; 3]) -> (u64, String) {
    let heights = [
        finalized_number(ports[0]),
        finalized_number(ports[1]),
        finalized_number(ports[2]),
    ];
    let common = *heights.iter().min().expect("three finalized heights");
    let hashes: Vec<String> = ports
        .iter()
        .map(|port| {
            block_hash_at(*port, common)
                .unwrap_or_else(|| panic!(":{port} has no canonical hash at height {common}"))
        })
        .collect();
    assert!(
        hashes.iter().all(|h| h == &hashes[0]),
        "the validators disagree at finalized height {common}: {hashes:?} (their heads were \
         {heights:?})"
    );
    (common, hashes[0].clone())
}

/// One stream of fee-burning comits, driven from a single account into a single validator.
///
/// One thread owns one account on purpose: `submit_comit_v2` refuses any kernel nonce that is not
/// the account's next one, and the signed extension's account nonce comes from the same pool, so
/// two threads signing for one account would race and the loser would fail for a reason that has
/// nothing to do with supply.
struct Stream {
    name: &'static str,
    uri: &'static str,
    account_index: u64,
    submit_port: u16,
    signer_port: u16,
}

/// A comit that landed, and which validator accepted it.
#[derive(Debug, Clone, Copy)]
struct LandedComit {
    id: H256,
    submit_port: u16,
}

fn run_stream(stream: Stream, program: &[u8]) -> Result<Vec<LandedComit>, String> {
    let signer =
        X3RuntimeSigner::from_uri(CHAIN_ID.into(), rpc_url(stream.signer_port), stream.uri)
            .map_err(|e| format!("build {}'s signer: {e}", stream.name))?;
    let mut landed = Vec::new();
    for index in 0..COMITS_PER_ACCOUNT {
        // `AtlasKernel::Nonces` is the kernel's own replay counter and it only advances when a
        // comit has been *included and dispatched* — a failed dispatch rolls the storage write
        // back. So it is both how the next nonce is chosen (`sign_kernel_submit_comit_v2` reads
        // the same value) and how the previous comit is proven to have run: pace on it, and a
        // refused comit shows up as a timeout naming that comit rather than as a silent gap.
        wait_for_kernel_nonce(&signer, index, stream.name, index)?;
        let comit_id = H256::from_low_u64_be(COMIT_ID_SEED ^ (stream.account_index << 32) ^ index);
        let signed = signer
            .sign_kernel_submit_comit_v2(comit_id, program.to_vec(), COMIT_FEE)
            .map_err(|e| format!("{} comit {index}: sign: {e}", stream.name))?;
        let hash = rpc_try(
            stream.submit_port,
            "author_submitExtrinsic",
            vec![Value::String(signed)],
        )
        .map_err(|e| format!("{} comit {index}: {e}", stream.name))?;
        if hash.as_str().map(str::is_empty).unwrap_or(true) {
            return Err(format!(
                "{} comit {index}: :{} did not return a transaction hash ({hash})",
                stream.name, stream.submit_port
            ));
        }
        landed.push(LandedComit {
            id: comit_id,
            submit_port: stream.submit_port,
        });
    }
    // And the last one too, so returning from this stream means every comit it claimed ran.
    wait_for_kernel_nonce(
        &signer,
        COMITS_PER_ACCOUNT,
        stream.name,
        COMITS_PER_ACCOUNT - 1,
    )?;
    Ok(landed)
}

/// Block until `signer`'s account has a kernel comit nonce of at least `expected`, reading the
/// counter from the chain each time. `who` and `last_index` are only used to make the failure say
/// which comit was refused.
fn wait_for_kernel_nonce(
    signer: &X3RuntimeSigner,
    expected: u64,
    who: &str,
    last_index: u64,
) -> Result<(), String> {
    let started = Instant::now();
    let mut observed = None;
    while started.elapsed() < KERNEL_NONCE_TIMEOUT {
        let nonce = signer
            .kernel_comit_nonce()
            .map_err(|e| format!("{who}: read kernel comit nonce: {e}"))?;
        observed = Some(nonce);
        if nonce >= expected {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(300));
    }
    Err(format!(
        "{who}: comit {last_index} (expecting kernel nonce {expected}) was not dispatched within \
         {KERNEL_NONCE_TIMEOUT:?}; the kernel nonce is still {observed:?}. A comit that is refused \
         rolls its nonce write back, so this is a refused comit, not a slow one"
    ))
}

/// Copy the built `local3` spec, add one unit to `Balances::TotalIssuance` in the raw top, and write
/// the result to `out`. Returns the key and the two values so the caller can report them.
fn write_corrupted_spec(node_bin: &Path, out: &Path) -> (String, u128, u128) {
    let mut command = Command::new(node_bin);
    command.args(["build-spec", "--chain", "local3", "--raw"]);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = command.spawn().expect("spawn build-spec");

    let started = Instant::now();
    let output = child
        .wait_with_output()
        .expect("build-spec should run to completion");
    assert!(
        started.elapsed() < BUILD_SPEC_TIMEOUT,
        "build-spec took {:?}",
        started.elapsed()
    );
    assert!(
        output.status.success(),
        "build-spec --raw failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut spec: Value =
        serde_json::from_slice(&output.stdout).expect("build-spec emits one JSON document");
    let top = spec
        .pointer_mut("/genesis/raw/top")
        .and_then(Value::as_object_mut)
        .expect("a raw spec has genesis.raw.top");

    let key = format!("0x{}", hex::encode(total_issuance_key()));
    let existing = top
        .get(&key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("the raw spec has no TotalIssuance at {key}"));
    let real = u128::decode(&mut &hex::decode(existing.trim_start_matches("0x")).expect("hex")[..])
        .expect("TotalIssuance decodes as u128");
    let corrupted = real + 1;
    top.insert(
        key.clone(),
        Value::String(format!("0x{}", hex::encode(corrupted.encode()))),
    );

    let encoded = serde_json::to_vec(&spec).expect("re-serialise the spec");
    std::fs::write(out, encoded).expect("write the scratch spec");
    (key, real, corrupted)
}

/// The negative control: boot one node from a scratch spec whose `TotalIssuance` is one unit larger
/// than the accounts it starts with, and require the same check to notice.
fn assert_corrupted_ledger_is_caught(node_bin: &Path, base_path: &Path) {
    let spec_path = base_path.join("corrupted-local3.json");
    let (key, real, corrupted) = write_corrupted_spec(node_bin, &spec_path);
    println!(
        "[x3-supply] negative control: scratch spec at {} has {key} = {corrupted} (real {real}, +1)",
        spec_path.display()
    );

    let argv = validator_argv(
        node_bin,
        "scratch",
        "--alice",
        9,
        SCRATCH_RPC,
        SCRATCH_P2P,
        &base_path.join("scratch"),
        spec_path.to_str().expect("utf-8 spec path"),
        &[],
    );
    let mut scratch = ValidatorSet {
        base_path: base_path.join("scratch"),
        children: Vec::new(),
    };
    scratch
        .children
        .push(spawn_with_args(&argv, &base_path.join("scratch.log")));
    wait_for_rpc("scratch", SCRATCH_RPC, NODE_BOOT_TIMEOUT);

    // A single node is 1 of 3 authorities, so GRANDPA can never finalize past 0 here. The
    // corruption is in *genesis* state, and reading at the genesis hash is reading a real block
    // of this chain through the same RPC path the three validators were read through.
    let genesis = genesis_hash(SCRATCH_RPC);
    let view = read_supply(SCRATCH_RPC, &genesis)
        .unwrap_or_else(|e| panic!("reading the corrupted chain failed outright: {e}"));
    assert_eq!(
        view.total_issuance, corrupted,
        "the scratch chain did not keep the injected TotalIssuance"
    );
    assert_ne!(
        view.accounted, view.total_issuance,
        "the corrupted ledger read back as conserved, so the check cannot detect a one-unit \
         accounting error and proves nothing"
    );
    let violation = check_conserved(SCRATCH_RPC, &genesis)
        .expect_err("the corrupted ledger must be reported as a violation");
    assert!(
        violation.contains(&corrupted.to_string())
            && violation.contains(&view.accounted.to_string()),
        "the violation must name both sides (accounted {}, issuance {}): {violation}",
        view.accounted,
        corrupted
    );
    println!("[x3-supply] negative control caught it: {violation}");

    // Scratch copy: discard it. The operator's chain and data dirs were never touched.
    drop(scratch);
    let _ = std::fs::remove_file(&spec_path);
}

#[test]
#[ignore = "boots the three-validator local3 network, drives comits into all three, and proves \
            supply conservation on each; the check is then proven load-bearing on a corrupted \
            scratch ledger"]
fn supply_is_conserved_on_every_validator_under_distributed_traffic() {
    assert_ports_free();

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_nanos();
    let base_path = std::env::temp_dir().join(format!("x3-supply-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&base_path).expect("create base path");
    let (node_bin, node_bin_digest) = freeze_node_binary(&base_path);
    println!(
        "[x3-supply] every node under test runs one frozen artifact: {} (sha256 {})",
        node_bin.display(),
        node_bin_digest.as_deref().unwrap_or("unavailable")
    );
    println!("[x3-supply] base path: {}", base_path.display());

    // -------- baseline: the invariant is not assumed, it is read before any traffic ----------
    // The three validators live in their own subtree: dropping the set at the end of this phase
    // removes their data dirs, and must not take the frozen node binary (or the scratch spec built
    // below) with them — the negative control still needs both.
    let network = boot_local3(&node_bin, &base_path.join("local3"));
    let ports = [ALICE_RPC, BOB_RPC, CHARLIE_RPC];

    let (height0, hash0) = common_finalized(&ports);
    let baseline: Vec<SupplyView> = ports
        .iter()
        .map(|port| {
            check_conserved(*port, &hash0).unwrap_or_else(|e| panic!("baseline check failed: {e}"))
        })
        .collect();
    assert!(
        baseline.iter().all(|v| *v == baseline[0]),
        "the validators disagree about supply at {height0}:{hash0}: {baseline:?}"
    );
    println!(
        "[x3-supply] baseline at {height0}:{} — {} accounts, accounted {}, TotalIssuance {} on all three",
        hash0, baseline[0].accounts, baseline[0].accounted, baseline[0].total_issuance
    );

    // The three submitting accounts, each bound to a different validator, so the traffic is
    // concurrent and lands on more than one validator.
    let streams = [
        Stream {
            name: "alice",
            uri: "//Alice",
            account_index: 0,
            submit_port: ALICE_RPC,
            signer_port: ALICE_RPC,
        },
        Stream {
            name: "bob",
            uri: "//Bob",
            account_index: 1,
            submit_port: BOB_RPC,
            signer_port: BOB_RPC,
        },
        Stream {
            name: "charlie",
            uri: "//Charlie",
            account_index: 2,
            submit_port: CHARLIE_RPC,
            signer_port: CHARLIE_RPC,
        },
    ];

    // Compile the program in the test: this is the compiler's output, not a fixture blob.
    let program = x3_x3_integration::compiler_bridge::compile_source(PROGRAM_SOURCE)
        .expect("the fixture must compile");
    assert!(
        program.starts_with(X3BC_MAGIC),
        "the compiler must emit an X3BC module, not an arbitrary blob"
    );
    println!(
        "[x3-supply] compiled {} bytes of X3BC; driving {COMITS_PER_ACCOUNT} comits per account \
         into alice/bob/charlie concurrently",
        program.len()
    );

    // Read each submitter's pre-traffic balance at the baseline block, by its own key.
    let submitters = [
        ("//Alice", ALICE_RPC, baseline[0]),
        ("//Bob", BOB_RPC, baseline[0]),
        ("//Charlie", CHARLIE_RPC, baseline[0]),
    ];
    let before: Vec<u128> = submitters
        .iter()
        .map(|(uri, port, _)| {
            let signer = X3RuntimeSigner::from_uri(CHAIN_ID.into(), rpc_url(*port), uri)
                .expect("build a balance probe signer");
            free_balance_at(*port, &hash0, &signer.account())
        })
        .collect();
    println!("[x3-supply] submitter balances at {height0}: {before:?}");

    // -------- the distributed traffic ----------
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();
    for stream in streams {
        let tx = tx.clone();
        let program = program.clone();
        handles.push(thread::spawn(move || {
            let name = stream.name;
            let result = run_stream(stream, &program);
            let _ = tx.send((name, result));
        }));
    }
    drop(tx);

    let mut landed: Vec<LandedComit> = Vec::new();
    for handle in handles {
        handle.join().expect("stream thread panicked");
    }
    for (name, result) in rx.iter() {
        match result {
            Ok(mut comits) => {
                println!("[x3-supply] {name}: {} comits accepted", comits.len());
                landed.append(&mut comits);
            }
            Err(e) => panic!("stream {name} failed: {e}"),
        }
    }
    assert_eq!(
        landed.len() as u64,
        3 * COMITS_PER_ACCOUNT,
        "every stream must have landed all of its comits"
    );

    // Each comit has to leave a receipt in consensus state — read from a validator that did *not*
    // receive the submission, so "the extrinsic was accepted" is not confused with "it ran".
    for comit in &landed {
        let observer = ports
            .iter()
            .copied()
            .find(|p| *p != comit.submit_port)
            .expect("a port other than the submitter");
        wait_for_receipt(observer, comit.id, RECEIPT_TIMEOUT);
    }
    println!(
        "[x3-supply] all {} comits have execution receipts on a validator that never saw the \
         submission",
        landed.len()
    );

    // -------- the invariant, on every validator's own view at one finalized block ----------
    let (height1, hash1) = common_finalized(&ports);
    assert!(
        height1 > height0,
        "finality did not advance past the baseline block ({height0} -> {height1})"
    );
    let after: Vec<SupplyView> = ports
        .iter()
        .map(|port| {
            check_conserved(*port, &hash1)
                .unwrap_or_else(|e| panic!("post-traffic check failed: {e}"))
        })
        .collect();
    assert!(
        after.iter().all(|v| *v == after[0]),
        "the validators disagree about supply at {height1}:{hash1}: {after:?}"
    );
    for (index, view) in after.iter().enumerate() {
        println!(
            "[x3-supply] :{} at {height1}:{hash1} — {} accounts, accounted {}, TotalIssuance {} — conserved",
            ports[index], view.accounts, view.accounted, view.total_issuance
        );
    }

    // The traffic has to have moved value, or "conserved" is a statement about a chain that did
    // nothing. The kernel burns its comit fee, so issuance falls, and each submitter paid.
    assert!(
        after[0].total_issuance < baseline[0].total_issuance,
        "no value was destroyed by {} fee-burning comits (TotalIssuance {} -> {})",
        landed.len(),
        baseline[0].total_issuance,
        after[0].total_issuance
    );
    let burned = baseline[0].total_issuance - after[0].total_issuance;
    let mut paid_total = 0u128;
    for (index, ((uri, port, _), before_balance)) in
        submitters.iter().zip(before.iter()).enumerate()
    {
        let signer = X3RuntimeSigner::from_uri(CHAIN_ID.into(), rpc_url(*port), uri)
            .expect("build a balance probe signer");
        let after_balance = free_balance_at(*port, &hash1, &signer.account());
        assert!(
            after_balance < *before_balance,
            "submitter {index} ({uri}) paid nothing: {} -> {}",
            before_balance,
            after_balance
        );
        paid_total += before_balance - after_balance;
        println!(
            "[x3-supply] submitter {index} ({uri}): {} -> {} (paid {})",
            before_balance,
            after_balance,
            before_balance - after_balance
        );
    }
    println!(
        "[x3-supply] TotalIssuance {} -> {} (burned {}); submitter balances fell by {} in total",
        baseline[0].total_issuance, after[0].total_issuance, burned, paid_total
    );
    // Conservation is an equality, so the total fall in account balances must equal the fall in
    // issuance exactly. Checking it directly is what makes the identity load-bearing rather than
    // "the sum happened to be right at both ends".
    assert_eq!(
        baseline[0].accounted - after[0].accounted,
        burned,
        "account balances fell by {} but issuance fell by {burned}: value was created or lost",
        baseline[0].accounted - after[0].accounted
    );

    // -------- the per-asset ledger, through the API that exposes it --------
    //
    // The native identity above is one ledger. The *asset* ledger
    // (`native + evm + svm + external_locked + pending <= canonical`, per asset, in
    // `pallet-x3-supply-ledger`) is a different one, and until this API existed it had no read
    // surface at all. This chain does not create asset records: nothing in these comits touches
    // that pallet — the kernel writes its own `CanonicalLedger` — so the assertion below is on
    // what the read surface *reports*, cross-checked between validators, and it says out loud
    // how many records that is. It is deliberately not written as "the identity holds" when
    // there is nothing to check: creating an asset needs a signed `tokenFactory.createToken`,
    // which `x3-runtime-signer` does not expose yet.
    let mut ledger_records = 0usize;
    for asset in ledger_assets(ports[0], &hash1) {
        let mut views = Vec::new();
        for port in ports {
            let ledger = read_asset_ledger(port, &hash1, asset)
                .unwrap_or_else(|e| panic!("{asset:?}: ledger read failed on :{port}: {e}"));
            let ledger = ledger.unwrap_or_else(|| {
                panic!("{asset:?} is in the ledger's key set but reads as None on :{port}")
            });
            let represented = ledger
                .native_supply
                .checked_add(ledger.evm_supply)
                .and_then(|v| v.checked_add(ledger.svm_supply))
                .and_then(|v| v.checked_add(ledger.external_locked_supply))
                .and_then(|v| v.checked_add(ledger.pending_supply))
                .expect("represented supply overflowed");
            assert!(
                represented <= ledger.canonical_supply,
                "{asset:?} on :{port}: represented {represented} exceeds canonical {}",
                ledger.canonical_supply
            );
            views.push(ledger);
        }
        assert!(
            views.iter().all(|v| *v == views[0]),
            "{asset:?}: validators disagree about the ledger at {height1}:{hash1}"
        );
        ledger_records += 1;
        println!(
            "[x3-supply] asset {asset:?}: native {}, evm {}, svm {}, external_locked {}, pending {}, canonical {} — identity holds and all validators agree",
            views[0].native_supply,
            views[0].evm_supply,
            views[0].svm_supply,
            views[0].external_locked_supply,
            views[0].pending_supply,
            views[0].canonical_supply,
        );
    }
    println!(
        "[x3-supply] per-asset ledger: {ledger_records} record(s) present on this chain, read \
         through AtlasKernelRuntimeApi_get_asset_supply_ledger on every validator"
    );

    // -------- negative control, on a scratch copy of the ledger ----------
    drop(network);
    assert_corrupted_ledger_is_caught(&node_bin, &base_path);

    let _ = std::fs::remove_dir_all(&base_path);
}
