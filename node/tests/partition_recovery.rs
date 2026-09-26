//! A partition, not a crash.
//!
//! The goal's "Crash/restart/partition/recovery: all pass" already had its crash (the
//! `validator failure drill`, which kills and restarts validators) and its restart (the X3Lang
//! and lifecycle tests re-open a real database), but nothing exercised a **network partition**:
//! a validator that is alive, running, and reachable to nobody.
//!
//! This boots the built-in three-validator `local3` chain and then cuts exactly one validator
//! (Charlie) off from the other two at the kernel level, **without killing it** — the difference
//! between "crashed" and "partitioned" is the whole point. It then requires the chain to behave
//! the way a three-authority GRANDPA chain actually must, and to heal on its own.
//!
//! ## What "the way a three-authority chain must" is
//!
//! `node/src/chain_spec.rs::local_three_validator_config` builds `local3` from exactly three
//! `initial_authorities` (Alice, Bob, Charlie). GRANDPA's finality threshold for a voter set of
//! size `n` is `n - (n - 1) / 3`; for `n = 3` that is `3 - 0 = 3`. So a three-authority chain
//! tolerates **zero** faults: two live validators are one short of the threshold, and the honest
//! behaviour is that the two survivors keep *authoring* (Aura is independent of GRANDPA) but
//! **do not finalize** until the third returns. A chain that finalized with two of three would be
//! rounding the 2/3 threshold the unsafe way, and this test treats that as the bug it is.
//!
//! (This corrects the round-2 workstream note that read "with three GRANDPA authorities, two is
//! exactly the 2/3 threshold." Two of three is 66.7%, which is *below* the strict-greater-than
//! threshold GRANDPA needs; the note is off by one authority, and asserting the note would have
//! written a test that cannot pass on a correct chain.)
//!
//! ## The partition lever, and why it is honest here
//!
//! Charlie boots with `--out-peers 0` and no bootnodes, so it never dials outbound: every link it
//! owns is an *inbound* connection to its one P2P port, and Alice and Bob dial in (from their own
//! P2P ports — libp2p reuses the listening port for outgoing connections, observed as
//! `127.0.0.1:30410 <-> 127.0.0.1:30412`). That pins every link between two validators to exactly
//! their two P2P ports, so a cut scoped to Charlie's port is a cut of every link Charlie has.
//!
//! The cut is two operations, because one is not enough on this node:
//!
//! * `iptables` DROP rules on Charlie's P2P port stop the survivors from dialing it back;
//! * `ss -K` forcibly resets the sockets that already exist. A DROP alone black-holes the packets,
//!   and this node does not treat a black hole as a disconnect — measured directly, the connections
//!   stayed `ESTAB` with ~150 KB of unacked data queued and `system_health.peers` unchanged for
//!   90 s.
//!
//! Both run the host's own binaries, reached without root by a privileged, host-network container
//! (`docker run --privileged --network host -v /:/host alpine chroot /host …`); on a host where the
//! test is already privileged it runs them directly. The rules are scoped to the one P2P port and
//! removed on heal and again on drop, so a panic cannot strand the firewall — and the test refuses
//! to start if these tools are not usable, rather than "passing" without a cut.
//!
//! The cut is only believed once the peer counts move: Charlie's peer count must fall to zero and
//! the survivors' to one, and stay there for the whole window. A "partition" that does not change a
//! peer count is checking nothing. Setting `X3_PARTITION_SKIP_RESET=1` disables the socket reset on
//! purpose, to show the check is load-bearing: without it the DROP rules alone leave a stale peer
//! count and the gate fails.
//!
//! ```text
//! env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test partition_recovery \
//!   -- --ignored --nocapture --test-threads=1
//! ```

use serde_json::Value;
use std::fs::File;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

// Six ports, deliberately away from the other local3 gates on this box
// (`x3lang_network_receipt` holds 19954-19956 / 30389-30391, `supply_invariant_distributed` holds
// 19964-19967 / 30394-30397, and the runtime-upgrade rehearsal and the single-node live tests hold
// the rest of the 1994x block). They are checked free before anything is spawned.
const ALICE_RPC: u16 = 19974;
const BOB_RPC: u16 = 19975;
const CHARLIE_RPC: u16 = 19976;
const ALICE_P2P: u16 = 30410;
const BOB_P2P: u16 = 30411;
const CHARLIE_P2P: u16 = 30412;

/// Three authorities need all three online to reach GRANDPA's threshold, so boots on this chain
/// are slower than a dev node's; every wait below is sized for a debug build of a 1.4 GB binary.
const NODE_BOOT_TIMEOUT: Duration = Duration::from_secs(300);
const CONSENSUS_TIMEOUT: Duration = Duration::from_secs(300);
/// How long the two survivors are given to prove they keep authoring while Charlie is cut off,
/// and for Charlie to fall genuinely behind. Sub-second blocks make this generous.
const PARTITION_TIMEOUT: Duration = Duration::from_secs(180);
/// How long the three are given to re-converge and resume finality after the heal.
const CONVERGE_TIMEOUT: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Finality has to be past genesis before the partition: a chain whose finalized head is block 0
/// has agreed on nothing, and "finality stalled at 0" would prove nothing.
const MIN_FINALIZED: u64 = 3;
/// Three validators, each connected to the other two before the cut.
const EXPECTED_PEERS: u64 = 2;
/// The survivors must author at least this many blocks past the fork point while Charlie is cut,
/// and Charlie must be at least this far behind them, before "authoring continued" and "genuinely
/// behind" mean anything.
const MIN_AUTHORED: u64 = 4;
const MIN_BEHIND: u64 = 3;

fn rpc_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// Whether something actually answers on this loopback port, as opposed to an unbound-but-unusable
/// socket left behind by a connection that is still being reaped.
fn port_has_listener(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

/// One JSON-RPC call to one validator. The error is returned rather than swallowed so a caller can
/// tell "the node refused" from "the node never answered" — the two have different fixes.
fn rpc_try(port: u16, method: &str, params: Vec<Value>) -> Result<Value, String> {
    // A plain blocking HTTP call would need a client crate; the node's own `RpcClient` (used by the
    // signer and by `supply_invariant_distributed`) already speaks this exact dialect.
    let mut rpc = x3_atomic_swap::RpcClient::new(rpc_url(port), 0);
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

/// `system_health.peers`: the sync peerset's connected count, which is the number the node itself
/// reports and the same figure `system_peers` is derived from.
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

/// The best (head) block number as *this* validator sees it. `chain_getHeader` with no argument
/// answers for the head, which is what "still authoring" is measured on.
fn best_number(port: u16) -> u64 {
    let raw = rpc_expect(port, "chain_getHeader", Vec::new())
        .get("number")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("best header on :{port} had no number"))
        .to_string();
    u64::from_str_radix(raw.trim_start_matches("0x"), 16)
        .unwrap_or_else(|e| panic!("best header number '{raw}' on :{port} is not hex: {e}"))
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

/// The highest height every validator has finalized, plus the canonical hash at it — after
/// requiring all three to report the *same* hash there. Three chains running side by side is not
/// one chain; this is the check that makes the numbers comparable.
fn common_finalized(ports: &[u16]) -> (u64, String) {
    let heights: Vec<u64> = ports.iter().map(|port| finalized_number(*port)).collect();
    let common = *heights.iter().min().expect("a finalized height to compare");
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
                "[x3-partition] test panicked — keeping {} for diagnosis",
                self.base_path.display()
            );
        } else {
            let _ = std::fs::remove_dir_all(&self.base_path);
        }
    }
}

/// Every port this gate binds, in the order it binds them.
fn gate_ports() -> Vec<u16> {
    vec![
        ALICE_RPC,
        BOB_RPC,
        CHARLIE_RPC,
        ALICE_P2P,
        BOB_P2P,
        CHARLIE_P2P,
    ]
}

/// Refuse to start if any of `ports` is already bound.
///
/// A second validator set on these ports does not announce itself: the new node fails to bind, the
/// gate's `system_health` probe answers from the *other* set, and the run wedges into a genesis
/// mismatch or a finality timeout minutes later. Measuring the ports first turns that into one
/// sentence naming the port.
fn assert_ports_free(ports: &[u16]) {
    const SETTLE_TIMEOUT: Duration = Duration::from_secs(20);
    let started = Instant::now();
    loop {
        let mut live = Vec::new();
        let mut settling = Vec::new();
        for port in ports {
            if let Err(error) = TcpListener::bind(("127.0.0.1", *port)) {
                let entry = format!("{port} ({error})");
                if port_has_listener(*port) {
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
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Freeze the node binary this run will use, and return its path plus `sha256` when the box has a
/// `sha256sum` to compute one with.
///
/// The runtime blob is part of genesis, so two builds of this tree have two genesis hashes. Other
/// gates and agents on this box rebuild the binary while tests run, and a validator set exec'd
/// across two builds rejects its own bootnodes with `Genesis mismatch`. Copying once and spawning
/// everything from the copy makes the artifact under test a constant for the run, and prints the
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

fn dev_node_key(seed: u32) -> String {
    // A deterministic 32-byte libp2p key per validator, so the three identities are stable across
    // runs and a failure is reproducible. `--alice` supplies session keys, not a network identity.
    format!("{seed:064x}")
}

/// The argument vector one validator is booted with.
///
/// `out_peers == 0` is what makes the isolated validator's cut complete: with no outbound slots it
/// dials nobody, so every link it owns is an inbound socket on its single P2P port, and dropping
/// that port cuts it in both directions. The other two keep normal slot budgets.
#[allow(clippy::too_many_arguments)]
fn validator_argv(
    node_bin: &Path,
    name: &str,
    key_flag: &str,
    node_key_seed: u32,
    rpc_port: u16,
    p2p_port: u16,
    out_peers: u32,
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
        // Bind the P2P listener to IPv4 loopback only. With the default (0.0.0.0 + [::]) the node
        // also listens on `::1`, and a peer that dials the IPv6 address would ride a path the
        // IPv4 firewall rules below never see — an incomplete cut that would look like a flaky
        // partition. Pinning the listener to `/ip4/127.0.0.1/...` makes every link IPv4.
        "--listen-addr".into(),
        format!("/ip4/127.0.0.1/tcp/{p2p_port}"),
        "--out-peers".into(),
        out_peers.to_string(),
        "--rpc-port".into(),
        rpc_port.to_string(),
        // `system_health` and the block/consensus queries below are served on the safe set, but the
        // validator refuses nothing this gate needs once unsafe methods are allowed, and a node
        // that answers "method not found" cannot be asked whether it is behind.
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
        "[x3-partition] {name}: rpc :{rpc_port}, p2p :{p2p_port}, out-peers {out_peers}, log {}",
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
        std::thread::sleep(POLL_INTERVAL);
    }
    panic!("{name} (: {port}) did not answer system_health within {timeout:?}");
}

/// Poll `check` until it returns `Ok`, or until `timeout`; on timeout return the last error so the
/// failure names what was still untrue.
fn wait_until<T, E: std::fmt::Display>(
    what: &str,
    timeout: Duration,
    mut check: impl FnMut() -> Result<T, E>,
) -> T {
    let started = Instant::now();
    let mut last = None;
    while started.elapsed() < timeout {
        match check() {
            Ok(value) => return value,
            Err(error) => last = Some(error.to_string()),
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    panic!(
        "timed out after {timeout:?} waiting for {what}; last observation: {}",
        last.as_deref().unwrap_or("<never observed>")
    );
}

/// Boot the three validators on one chain.
///
/// Charlie is booted **first**, with no bootnodes and `--out-peers 0`, so it dials nobody and its
/// only peers are the ones Alice and Bob dial in. Alice and Bob are then booted knowing Charlie's
/// address (and Bob additionally knows Alice), so both connect in to Charlie and to each other.
/// This is the topology that makes a port-scoped cut a complete partition.
fn boot_local3(node_bin: &Path, base_path: &Path) -> ValidatorSet {
    std::fs::create_dir_all(base_path).expect("create the validator set's base path");
    let mut children = Vec::new();

    // Charlie first: it must be listening before Alice and Bob can dial it, and it must dial none.
    children.push(spawn_with_args(
        &validator_argv(
            node_bin,
            "charlie",
            "--charlie",
            3,
            CHARLIE_RPC,
            CHARLIE_P2P,
            0,
            &base_path.join("charlie"),
            "local3",
            &[],
        ),
        &base_path.join("charlie.log"),
    ));
    wait_for_rpc("charlie", CHARLIE_RPC, NODE_BOOT_TIMEOUT);
    let charlie_peer_id = rpc_string(CHARLIE_RPC, "system_localPeerId", Vec::new());
    let charlie_bootnode = format!("/ip4/127.0.0.1/tcp/{CHARLIE_P2P}/p2p/{charlie_peer_id}");

    // Alice dials Charlie.
    children.push(spawn_with_args(
        &validator_argv(
            node_bin,
            "alice",
            "--alice",
            1,
            ALICE_RPC,
            ALICE_P2P,
            25,
            &base_path.join("alice"),
            "local3",
            std::slice::from_ref(&charlie_bootnode),
        ),
        &base_path.join("alice.log"),
    ));
    wait_for_rpc("alice", ALICE_RPC, NODE_BOOT_TIMEOUT);
    let alice_peer_id = rpc_string(ALICE_RPC, "system_localPeerId", Vec::new());
    let alice_bootnode = format!("/ip4/127.0.0.1/tcp/{ALICE_P2P}/p2p/{alice_peer_id}");

    // Bob dials Charlie and Alice, completing the triangle of inbound links to Charlie.
    children.push(spawn_with_args(
        &validator_argv(
            node_bin,
            "bob",
            "--bob",
            2,
            BOB_RPC,
            BOB_P2P,
            25,
            &base_path.join("bob"),
            "local3",
            &[charlie_bootnode, alice_bootnode],
        ),
        &base_path.join("bob.log"),
    ));
    wait_for_rpc("bob", BOB_RPC, NODE_BOOT_TIMEOUT);

    let network = ValidatorSet {
        base_path: base_path.to_path_buf(),
        children,
    };

    let ports = [ALICE_RPC, BOB_RPC, CHARLIE_RPC];

    // They have to find each other.
    let peers = wait_until("all three validators to connect", CONSENSUS_TIMEOUT, || {
        let peers = [
            peers_of(ALICE_RPC),
            peers_of(BOB_RPC),
            peers_of(CHARLIE_RPC),
        ];
        if peers.iter().all(|p| *p >= EXPECTED_PEERS) {
            Ok(peers)
        } else {
            Err(format!(
                "alice={} bob={} charlie={}",
                peers[0], peers[1], peers[2]
            ))
        }
    });
    println!(
        "[x3-partition] connected: alice={} bob={} charlie={} peers",
        peers[0], peers[1], peers[2]
    );

    // And agree on finalized history.
    let finalized = wait_until(
        &format!("finality to reach {MIN_FINALIZED} on every validator"),
        CONSENSUS_TIMEOUT,
        || {
            let finalized = [
                finalized_number(ALICE_RPC),
                finalized_number(BOB_RPC),
                finalized_number(CHARLIE_RPC),
            ];
            if finalized.iter().all(|n| *n >= MIN_FINALIZED) {
                Ok(finalized)
            } else {
                Err(format!(
                    "alice={} bob={} charlie={}",
                    finalized[0], finalized[1], finalized[2]
                ))
            }
        },
    );
    let (height, hash) = common_finalized(&ports);
    println!(
        "[x3-partition] consensus: finalized alice={} bob={} charlie={}, all agree on {height}:{hash}",
        finalized[0], finalized[1], finalized[2]
    );

    network
}

/// How the gate reaches the host's own networking tools: directly when it is already privileged,
/// otherwise through the host's own binaries inside a privileged, host-network container. The
/// container is a thin wrapper — it mounts the host root and `chroot`s into it, so the program that
/// runs is the host's and the namespace it edits is the host's: no packages are installed and no
/// network is needed.
#[derive(Clone, Copy)]
enum Shell {
    Direct,
    DockerChroot,
}

impl Shell {
    fn run(&self, program: &str, args: &[String]) -> Result<String, String> {
        let output = match self {
            Shell::Direct => Command::new(program).args(args).output(),
            Shell::DockerChroot => {
                let mut full: Vec<String> = vec![
                    "run",
                    "--rm",
                    "--privileged",
                    "--network",
                    "host",
                    "-v",
                    "/:/host",
                    "alpine",
                    "chroot",
                    "/host",
                ]
                .into_iter()
                .map(ToOwned::to_owned)
                .collect();
                full.push(program.to_string());
                full.extend(args.iter().cloned());
                Command::new("docker").args(&full).output()
            }
        }
        .map_err(|e| format!("spawn {program}: {e}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

fn first_existing(candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|path| Path::new(path).exists())
        .map(|path| path.to_string())
}

/// The host tools a partition needs, and the way this process can reach them.
///
/// Two tools, deliberately, because neither alone is a partition on this node:
///
/// * `ss -K` forcibly resets the connections that already exist. It is what makes the cut *take*:
///   a DROP alone black-holes packets, and this node does not treat a black hole as a disconnect.
///   Measured directly during development: after a DROP the survivors and Charlie kept the
///   connection in `ESTAB` with ~150 KB of unacked data queued and `system_health.peers` unchanged
///   for 90 s. `ss -K` closes the sockets on both ends, which both nodes observe immediately.
/// * `iptables` then stops the reset from healing: with the DROP rules in place the survivors
///   cannot dial Charlie back, and Charlie is booted with `--out-peers 0`, so it never dials them.
///
/// The rules are scoped to the one P2P port this gate owns; `ss -K` is scoped to sockets whose
/// local or peer port is that port. A listening socket on that port survives `ss -K`, so the heal
/// only has to remove the DROP rules.
struct NetTools {
    shell: Shell,
    iptables: String,
    ss: String,
}

impl NetTools {
    fn detect() -> Self {
        let iptables =
            first_existing(&["/usr/sbin/iptables", "/usr/bin/iptables", "/sbin/iptables"])
                .unwrap_or_else(|| {
                    panic!("no iptables binary found (looked in /usr/sbin, /usr/bin, /sbin)")
                });
        let ss = first_existing(&["/usr/bin/ss", "/sbin/ss", "/usr/sbin/ss"])
            .unwrap_or_else(|| panic!("no `ss` binary found; cannot force dead connections down"));

        let probe: Vec<String> = ["-t", "filter", "-L", "INPUT", "-n"]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect();
        let shell = if Shell::Direct.run(&iptables, &probe).is_ok() {
            println!("[x3-partition] netfilter: {iptables} + {ss} (direct, privileged)");
            Shell::Direct
        } else {
            match Shell::DockerChroot.run(&iptables, &probe) {
                Ok(_) => {
                    println!(
                        "[x3-partition] netfilter: {iptables} + {ss} via privileged host-network \
                         container"
                    );
                    Shell::DockerChroot
                }
                Err(error) => panic!(
                    "no usable firewall for the partition: direct iptables failed and the \
                     docker-chroot fallback failed too ({error}). This gate needs netfilter and a \
                     way to reset sockets it can reach; it refuses to run a 'partition' it cannot \
                     install rather than report one that did not happen."
                ),
            }
        };
        NetTools {
            shell,
            iptables,
            ss,
        }
    }

    fn ipt(&self, args: &[String]) -> Result<String, String> {
        self.shell.run(&self.iptables, args)
    }

    /// One DROP rule for the isolated port, in one direction on one chain.
    fn drop_rule(port: u16, op: &str, chain: &str, side: &str) -> Vec<String> {
        vec![
            "-t".into(),
            "filter".into(),
            op.into(),
            chain.into(),
            "-p".into(),
            "tcp".into(),
            side.into(),
            port.to_string(),
            "-j".into(),
            "DROP".into(),
        ]
    }

    /// Every DROP rule for the isolated port, both directions, on both chains.
    fn drop_rules(port: u16, op: &str) -> Vec<Vec<String>> {
        vec![
            Self::drop_rule(port, op, "INPUT", "--dport"),
            Self::drop_rule(port, op, "INPUT", "--sport"),
            Self::drop_rule(port, op, "OUTPUT", "--dport"),
            Self::drop_rule(port, op, "OUTPUT", "--sport"),
        ]
    }

    /// Remove any rules a previous run may have left, so a crash cannot wedge the next attempt.
    /// `-D` on a rule that is not present reports "no such rule"; that is the desired end state.
    fn clean(&self, port: u16) {
        for rule in Self::drop_rules(port, "-D") {
            let _ = self.ipt(&rule);
        }
    }

    /// Reset every established TCP connection that touches the isolated port, on both ends.
    /// `dport` matches the survivors' sockets (their peer is Charlie); `sport` matches Charlie's
    /// (its own local port). Listening sockets survive `ss -K`.
    ///
    /// `X3_PARTITION_SKIP_RESET=1` skips the reset. That is a deliberate control: it reproduces the
    /// "DROP rules only" state in which this node black-holes the packets but keeps reporting the
    /// peer, so the gate's own cut-confirmation must fail. It exists to prove the peer-count check
    /// is load-bearing, not to make the gate easier to pass.
    fn reset_connections(&self, port: u16) {
        if std::env::var("X3_PARTITION_SKIP_RESET").is_ok() {
            println!(
                "[x3-partition] X3_PARTITION_SKIP_RESET set: leaving live sockets up (control)"
            );
            return;
        }
        let filters: [Vec<String>; 2] = [
            vec![
                "-K".into(),
                "dst".into(),
                "127.0.0.1".into(),
                "dport".into(),
                "=".into(),
                format!(":{port}"),
            ],
            vec![
                "-K".into(),
                "src".into(),
                "127.0.0.1".into(),
                "sport".into(),
                "=".into(),
                format!(":{port}"),
            ],
        ];
        for filter in filters {
            self.shell
                .run(&self.ss, &filter)
                .unwrap_or_else(|e| panic!("ss {filter:?} failed: {e}"));
        }
    }

    /// Install the cut: prove the DROP rules are in place with `-C`, then reset the live
    /// connections so the cut takes effect now rather than after a TCP timeout.
    fn cut(&self, port: u16) {
        self.clean(port);
        let rules = Self::drop_rules(port, "-I");
        for rule in &rules {
            self.ipt(rule)
                .unwrap_or_else(|e| panic!("install cut rule {rule:?}: {e}"));
        }
        for rule in &rules {
            let mut check = rule.clone();
            check[2] = "-C".into();
            self.ipt(&check)
                .unwrap_or_else(|e| panic!("cut rule {rule:?} is not in place after install: {e}"));
        }
        self.reset_connections(port);
        // A second pass: the first reset may race a socket that was mid-accept when the rules went
        // in, letting it be re-accepted before the DROP is consulted.
        std::thread::sleep(Duration::from_millis(500));
        self.reset_connections(port);
        println!("[x3-partition] cut installed, live sockets reset for p2p port {port}");
    }

    fn heal(&self, port: u16) {
        for rule in Self::drop_rules(port, "-D") {
            self.ipt(&rule)
                .unwrap_or_else(|e| panic!("remove cut rule {rule:?}: {e}"));
        }
        println!("[x3-partition] cut removed for p2p port {port}");
    }
}

/// Removes the cut on drop, so a panic between `cut` and `heal` cannot leave the host firewall
/// modified. The heal itself is idempotent.
struct PartitionGuard {
    tools: NetTools,
    p2p_port: u16,
}

impl Drop for PartitionGuard {
    fn drop(&mut self) {
        self.tools.clean(self.p2p_port);
    }
}

/// What one validator says at one instant, for the phase being observed.
#[derive(Debug, Clone, Copy)]
struct View {
    best: u64,
    finalized: u64,
    peers: u64,
}

fn view(port: u16) -> View {
    View {
        best: best_number(port),
        finalized: finalized_number(port),
        peers: peers_of(port),
    }
}

const ALICE: usize = 0;
const BOB: usize = 1;
const CHARLIE: usize = 2;
const ALL: [usize; 3] = [ALICE, BOB, CHARLIE];

#[test]
#[ignore = "boots the three-validator local3 network, cuts one validator off at the kernel level \
            while it stays alive, and requires the two survivors to keep authoring but refuse to \
            finalize until the partition heals"]
fn network_partition_isolates_one_validator_and_the_chain_reconverges() {
    assert_ports_free(&gate_ports());
    let tools = NetTools::detect();

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_nanos();
    let base_path =
        std::env::temp_dir().join(format!("x3-partition-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&base_path).expect("create base path");
    let (node_bin, node_bin_digest) = freeze_node_binary(&base_path);
    println!(
        "[x3-partition] every node under test runs one frozen artifact: {} (sha256 {})",
        node_bin.display(),
        node_bin_digest.as_deref().unwrap_or("unavailable")
    );
    println!("[x3-partition] base path: {}", base_path.display());

    // -------- phase 1: three validators, connected and finalizing --------
    let network = boot_local3(&node_bin, &base_path.join("local3"));
    let ports = [ALICE_RPC, BOB_RPC, CHARLIE_RPC];
    let before: [View; 3] = [view(ALICE_RPC), view(BOB_RPC), view(CHARLIE_RPC)];
    let (height0, hash0) = common_finalized(&ports);
    println!(
        "[x3-partition] baseline at {height0}:{} — best alice={} bob={} charlie={}; peers alice={} bob={} charlie={}",
        hash0,
        before[ALICE].best,
        before[BOB].best,
        before[CHARLIE].best,
        before[ALICE].peers,
        before[BOB].peers,
        before[CHARLIE].peers
    );

    // -------- phase 2: cut Charlie off without killing it --------
    let guard = PartitionGuard {
        tools,
        p2p_port: CHARLIE_P2P,
    };
    guard.tools.cut(CHARLIE_P2P);

    // The cut is only believed once the peer counts move: Charlie to zero (it has no outbound
    // sockets to hide behind), the survivors to one. A "partition" that does not change a count is
    // checking nothing.
    let cut_peers = wait_until(
        "the cut to show as peer counts (charlie 0, others 1)",
        PARTITION_TIMEOUT,
        || {
            let peers = [
                peers_of(ALICE_RPC),
                peers_of(BOB_RPC),
                peers_of(CHARLIE_RPC),
            ];
            if peers[CHARLIE] == 0 && peers[ALICE] >= 1 && peers[BOB] >= 1 {
                Ok(peers)
            } else {
                Err(format!(
                    "alice={} bob={} charlie={}",
                    peers[ALICE], peers[BOB], peers[CHARLIE]
                ))
            }
        },
    );
    println!(
        "[x3-partition] cut confirmed: peers alice={} bob={} charlie={} (charlie is alive but \
         reachable to nobody)",
        cut_peers[ALICE], cut_peers[BOB], cut_peers[CHARLIE]
    );

    // Charlie must still be a running node, not a corpse: its RPC must answer. This is what makes
    // the phase a partition rather than a crash.
    let charlie_alive = rpc_expect(CHARLIE_RPC, "system_health", Vec::new());
    assert!(
        charlie_alive.get("peers").is_some(),
        "charlie did not answer system_health during the partition: {charlie_alive}"
    );

    // The fork point and the finality watermark, read now that the cut is in effect: the three had
    // all synced with each other up to here, and the block each has finalized is the watermark that
    // must not move again while one authority is gone. A short settle first lets any justification
    // already in flight land, so the watermark is not captured mid-round.
    std::thread::sleep(Duration::from_secs(2));
    let now = [view(ALICE_RPC), view(BOB_RPC), view(CHARLIE_RPC)];
    let fork = *[now[ALICE].best, now[BOB].best, now[CHARLIE].best]
        .iter()
        .min()
        .expect("three best heights");
    let frozen = [
        now[ALICE].finalized,
        now[BOB].finalized,
        now[CHARLIE].finalized,
    ];
    let freeze_floor = *frozen.iter().max().expect("three watermarks");
    println!(
        "[x3-partition] fork point ≈ {fork}; finality watermark alice={} bob={} charlie={}",
        frozen[ALICE], frozen[BOB], frozen[CHARLIE]
    );

    // -------- phase 3: the survivors keep authoring, the chain refuses to finalize --------
    // Wait until the two survivors have clearly moved their heads past the fork point and Charlie
    // has fallen behind them. If authoring also stopped, this times out and the test fails with the
    // numbers rather than asserting something it did not observe.
    let during = wait_until(
        "the two survivors to author past the fork point while charlie falls behind",
        PARTITION_TIMEOUT,
        || {
            let v = [view(ALICE_RPC), view(BOB_RPC), view(CHARLIE_RPC)];
            let survivors_authored = v[ALICE].best.min(v[BOB].best).saturating_sub(fork);
            let behind = v[ALICE].best.saturating_sub(v[CHARLIE].best);
            if survivors_authored >= MIN_AUTHORED && behind >= MIN_BEHIND {
                Ok(v)
            } else {
                Err(format!(
                    "authored={survivors_authored} (need {MIN_AUTHORED}), behind={behind} (need \
                     {MIN_BEHIND}); best alice={} bob={} charlie={}",
                    v[ALICE].best, v[BOB].best, v[CHARLIE].best
                ))
            }
        },
    );
    println!(
        "[x3-partition] during: best alice={} bob={} charlie={}; finalized alice={} bob={} charlie={}; \
         peers alice={} bob={} charlie={}",
        during[ALICE].best,
        during[BOB].best,
        during[CHARLIE].best,
        during[ALICE].finalized,
        during[BOB].finalized,
        during[CHARLIE].finalized,
        during[ALICE].peers,
        during[BOB].peers,
        during[CHARLIE].peers
    );

    // (a) The two survivors are still authoring — this is "alive and running", and it is why the
    //     isolated node being "behind" is meaningful rather than everyone standing still.
    for who in [ALICE, BOB] {
        assert!(
            during[who].best >= fork + MIN_AUTHORED,
            "survivor {who} stopped authoring: best {} did not move {MIN_AUTHORED} past the fork \
             point {fork}",
            during[who].best
        );
    }
    // (b) The isolated validator is alive and genuinely behind the survivors' head, not merely
    //     disconnected at the same height.
    assert!(
        during[CHARLIE].best + MIN_BEHIND <= during[ALICE].best,
        "charlie was not genuinely behind: its best head {} vs alice's {}",
        during[CHARLIE].best,
        during[ALICE].best
    );
    // (c) The safety property. Three authorities, one cut off, is one short of GRANDPA's threshold
    //     (n - (n-1)/3 = 3 for n = 3). The correct chain finalizes NOTHING here. If any validator
    //     advanced its finalized head past its watermark, it finalized a block two-thirds of the
    //     authorities never voted to keep.
    for who in ALL {
        assert_eq!(
            during[who].finalized, frozen[who],
            "finality advanced while one of three authorities was isolated and could not reach the \
             threshold: validator {who} finalized {} -> {} — a three-authority chain that finalizes \
             on two is rounding the 2/3 threshold the unsafe way",
            frozen[who], during[who].finalized
        );
    }
    // (d) The survivors stay connected to each other; only Charlie is isolated.
    assert!(
        during[ALICE].peers >= 1 && during[BOB].peers >= 1,
        "the survivors lost each other too (alice={} bob={} peers): the cut was not scoped to the \
         isolated validator",
        during[ALICE].peers,
        during[BOB].peers
    );
    assert_eq!(
        during[CHARLIE].peers, 0,
        "charlie was not actually isolated: it still reports {} peers",
        during[CHARLIE].peers
    );
    println!(
        "[x3-partition] partition proven: two survivors authoring past {fork} (alice={} bob={}), \
         charlie alive and behind ({}), and no validator finalized past {freeze_floor} — the 3-authority \
         threshold held",
        during[ALICE].best, during[BOB].best, during[CHARLIE].best
    );

    // -------- phase 4: heal, and require real convergence --------
    guard.tools.heal(CHARLIE_P2P);

    let healed = wait_until(
        "the three validators to reconnect after the heal",
        CONVERGE_TIMEOUT,
        || {
            let peers = [
                peers_of(ALICE_RPC),
                peers_of(BOB_RPC),
                peers_of(CHARLIE_RPC),
            ];
            if peers.iter().all(|p| *p >= EXPECTED_PEERS) {
                Ok(peers)
            } else {
                Err(format!(
                    "alice={} bob={} charlie={}",
                    peers[ALICE], peers[BOB], peers[CHARLIE]
                ))
            }
        },
    );
    println!(
        "[x3-partition] healed: peers alice={} bob={} charlie={}",
        healed[ALICE], healed[BOB], healed[CHARLIE]
    );

    // Convergence is two facts: they agree on a finalized height above the frozen point, and they
    // agree on the canonical hash there. Charlie has to reorg onto the survivors' branch (two of
    // three votes outweighed its one), not merely relabel its own.
    let converged = wait_until(
        "the three to agree on a finalized block past the freeze and resume finality",
        CONVERGE_TIMEOUT,
        || {
            let heights = [
                finalized_number(ALICE_RPC),
                finalized_number(BOB_RPC),
                finalized_number(CHARLIE_RPC),
            ];
            let common = *heights.iter().min().expect("three finalized heights");
            if common <= freeze_floor {
                return Err(format!(
                    "finality has not resumed past the freeze: finalized alice={} bob={} \
                     charlie={} (frozen at {freeze_floor})",
                    heights[0], heights[1], heights[2]
                ));
            }
            let hashes: Vec<String> = ports
                .iter()
                .map(|port| {
                    block_hash_at(*port, common)
                        .unwrap_or_else(|| panic!(":{port} has no hash at {common}"))
                })
                .collect();
            if !hashes.iter().all(|h| h == &hashes[0]) {
                return Err(format!(
                    "the validators disagree at height {common}: {hashes:?}"
                ));
            }
            Ok((common, hashes[0].clone(), heights))
        },
    );
    let (conv_height, conv_hash, heights) = converged;
    println!(
        "[x3-partition] converged: all three finalized {conv_height}:{conv_hash} \
         (alice={} bob={} charlie={}), past the freeze at {freeze_floor}",
        heights[0], heights[1], heights[2]
    );
    assert!(
        conv_height > freeze_floor,
        "finality did not resume past the freeze point ({freeze_floor})"
    );

    // And finality keeps moving — "converged at one block" is not "finalizing again".
    let resumed = wait_until(
        "finality to keep advancing after the heal",
        CONVERGE_TIMEOUT,
        || {
            let heights = [
                finalized_number(ALICE_RPC),
                finalized_number(BOB_RPC),
                finalized_number(CHARLIE_RPC),
            ];
            let common = *heights.iter().min().expect("three finalized heights");
            if common > conv_height {
                Ok(common)
            } else {
                Err(format!(
                    "finalized alice={} bob={} charlie={}",
                    heights[0], heights[1], heights[2]
                ))
            }
        },
    );
    println!(
        "[x3-partition] finality resumed: {freeze_floor} (frozen) -> {conv_height} (converged) -> \
         {resumed} (still advancing) on all three"
    );

    drop(guard);
    drop(network);
    let _ = std::fs::remove_dir_all(&base_path);
}

// =====================================================================================
// Partition *tolerance* at seven authorities
// =====================================================================================
//
// The test above proves the honest behaviour of a **three**-authority set: isolating one authority
// must stall finality, because three need all three votes (`n - (n-1)/3 = 3`). That is a safety
// property, but it is the opposite of what a public testnet needs to show — a network that keeps
// finalizing with one validator down. Only a set of four or more can lose one authority and still
// reach the threshold, so this second gate boots a generated `N`-authority network (default seven,
// where the threshold is five of seven) and proves that isolating exactly one validator leaves the
// survivors **finalizing**.
//
// It reuses the pieces above — `NetTools::cut`/`heal`, `PartitionGuard`, `view`, `wait_until`,
// `freeze_node_binary`, the port pre-flight — rather than a second harness. Two things are
// different from the three-authority test, both measured on this box:
//
// * The network is booted from a **generated** spec and identity set
//   (`scripts/testnet/build-x3-testnet-spec.py`, then `x3_testnet_up.sh`), never the built-in dev
//   keys, so it is the topology a real testnet actually runs.
// * The spec's `bootNodes` name `P2P_BASE + i - 1`, so the builder and the launcher must agree on
//   `P2P_BASE` or every node dials a port nobody listens on and the mesh collapses through the one
//   address `run-7-validators-local.sh` passes on the command line. (`P2P_BASE` is threaded into
//   both below for that reason.) With them aligned, all `N` nodes form a full mesh and each reports
//   `N - 1` peers.
//
// The cut is scoped to the isolated validator's single P2P port. That is complete here because this
// node reuses its listening port for outbound dials: every established socket between two
// validators is `127.0.0.1:<a> <-> 127.0.0.1:<b>` where `<a>`/`<b>` are their listening ports
// (measured: six sockets on each of the seven ports, one per peer). So the DROP rules plus the
// socket reset remove every link the isolated validator owns, in both directions.
//
// There is one more thing to get right, and it cost a run: a validator booted with the launcher's
// normal outbound budget that is cut off **re-dials** its peers on a fresh *ephemeral* source port.
// The DROP rules are scoped to the isolated P2P port, so an ephemeral-port dial matches neither
// direction and the validator re-syncs — measured: peer counts fell to `[5,5,5,0,5,5,5]` at the
// cut and the victim was level with the survivors again 180 s later. So before the cut the victim
// is restarted exactly the way the three-authority test boots Charlie: `--out-peers 0` and a spec
// whose only bootnode is unreachable. It then dials nobody, every link it owns is an inbound
// connection to its single P2P port, and a cut of that port is a real partition.
//
// ```text
// env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test partition_recovery \
//   -- --ignored --nocapture --test-threads=1 seven_authority
// ```

/// The seven-authority gate's ports. Deliberately clear of every other gate on this box: `local3`
/// holds 19974-19976 / 30410-30412, `x3lang_network_receipt` 19954-19956 / 30389-30391,
/// `supply_invariant_distributed` 19964-19967 / 30394-30397, and the default RPC block 9944-9950.
/// The launcher derives `rpc = RPC_BASE + i - 1` and `p2p = P2P_BASE + i - 1` for `i` in `1..=N`.
const SEVEN_RPC_BASE: u16 = 19984;
const SEVEN_P2P_BASE: u16 = 30420;
/// Prometheus is off (`--no-prometheus`), so these are never bound; the base is still passed so a
/// future metrics mode cannot silently land on another gate's port.
const SEVEN_PROM_BASE: u16 = 9620;
/// The launcher refuses more than seven authorities; the floor is four (below that, one isolation
/// is already a majority loss and the tolerance claim is false).
const SEVEN_MAX: usize = 7;
/// Which authority (1-based) is isolated, matching the three-authority test's "Charlie" role.
const SEVEN_VICTIM: usize = 4;
/// Finality has to be past genesis before the cut, and the survivors have to push it at least this
/// far past the watermark while the victim is cut off before "they kept finalizing" means anything.
const SEVEN_MIN_FINALIZED: u64 = 3;
const SEVEN_AFTER_CUT_FINALITY: u64 = 10;
/// The isolated validator's head must be at least this far behind the slowest survivor.
const SEVEN_VICTIM_LAG: u64 = 8;
/// How far behind the best survivor the restarted validator may be and still count as caught up.
/// It has to be caught up before the cut, or it keeps applying a backlog it fetched while
/// reconnecting after the cut and its finalized head is not a meaningful watermark.
const SEVEN_CATCHUP_DELTA: u64 = 5;

fn seven_authority_count() -> usize {
    let count = std::env::var("X3_PARTITION_VALIDATORS")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(SEVEN_MAX);
    assert!(
        (4..=SEVEN_MAX).contains(&count),
        "X3_PARTITION_VALIDATORS={count} cannot show the property this gate proves: n authorities \
         need n - (n-1)/3 votes, so three need all three (a single isolated authority stalls \
         finality, which the local3 test asserts) and the launcher refuses more than {SEVEN_MAX}"
    );
    count
}

fn seven_rpc_ports(count: usize) -> Vec<u16> {
    (0..count).map(|i| SEVEN_RPC_BASE + i as u16).collect()
}

fn seven_p2p_ports(count: usize) -> Vec<u16> {
    (0..count).map(|i| SEVEN_P2P_BASE + i as u16).collect()
}

/// `system_health.peers` for a node that may not be up (when one is being restarted), so a caller
/// can distinguish "not answering yet" from "answers with a count".
fn peers_try(port: u16) -> Result<u64, String> {
    rpc_try(port, "system_health", Vec::new()).and_then(|value| {
        value
            .get("peers")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("system_health on :{port} had no peers field"))
    })
}

/// Wait until a port is no longer bound, so a restarted validator can take it back.
fn wait_port_free(port: u16, what: &str, timeout: Duration) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if !port_has_listener(port) {
            return;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    panic!("{what}: port {port} was still held after {timeout:?}");
}

/// Stop one validator the launcher started, by the pid file it wrote, and wait for its RPC and P2P
/// ports to be released. `SIGKILL` matches the three-authority test's `Child::kill`.
fn stop_validator(net_dir: &Path, index: usize, rpc_port: u16, p2p_port: u16) {
    let pid_file = net_dir.join("pids").join(format!("node-{index}.pid"));
    let pid = std::fs::read_to_string(&pid_file)
        .unwrap_or_else(|e| panic!("read {}: {e}", pid_file.display()));
    let pid = pid.trim().to_string();
    let _ = Command::new("kill")
        .arg("-9")
        .arg(&pid)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    wait_port_free(
        rpc_port,
        &format!("stopping validator {index}"),
        Duration::from_secs(60),
    );
    wait_port_free(
        p2p_port,
        &format!("stopping validator {index}"),
        Duration::from_secs(60),
    );
}

/// Rewrite a copy of the run's chain spec so its only bootnode is unreachable. A Live spec must
/// carry at least one bootnode, but it need not be reachable; the point is that the validator
/// booted from this copy dials nobody while the survivors keep dialing *it* from their own spec.
fn write_inbound_only_spec(chain_spec: &Path, out: &Path) {
    let text = std::fs::read_to_string(chain_spec)
        .unwrap_or_else(|e| panic!("read {}: {e}", chain_spec.display()));
    let mut spec: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("parse {}: {e}", chain_spec.display()));
    let first = spec
        .get("bootNodes")
        .and_then(Value::as_array)
        .and_then(|bootnodes| bootnodes.first())
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{} carries no bootNodes to rewrite", chain_spec.display()))
        .to_string();
    let unreachable = first
        .split_once("/tcp/")
        .and_then(|(head, tail)| tail.split_once("/p2p/").map(|(_, peer)| (head, peer)))
        .map(|(head, peer)| format!("{head}/tcp/1/p2p/{peer}"))
        .unwrap_or_else(|| panic!("bootnode {first} is not an /ip4…/tcp…/p2p/… multiaddr"));
    *spec
        .get_mut("bootNodes")
        .expect("bootNodes exists, read just above") =
        Value::Array(vec![Value::String(unreachable)]);
    std::fs::write(
        out,
        serde_json::to_string_pretty(&spec).expect("re-serialize the rewritten chain spec"),
    )
    .unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
}

/// Relaunch one validator from the frozen binary on the same base path, keystore and network key,
/// but inbound-only: `--out-peers 0` and no `--bootnodes`. The pid file is rewritten so the set's
/// `Drop` still tears this process down; the returned `Child` is held by the set so it is reaped.
#[allow(clippy::too_many_arguments)]
fn relaunch_inbound_only(
    node_bin: &Path,
    net_dir: &Path,
    keys_dir: &Path,
    index: usize,
    rpc_port: u16,
    p2p_port: u16,
    victim_spec: &Path,
    log_path: &Path,
) -> Child {
    let seed_file = keys_dir.join(format!("validator-{index}.suri"));
    let seed_text = std::fs::read_to_string(&seed_file)
        .unwrap_or_else(|e| panic!("read {}: {e}", seed_file.display()));
    let seed = seed_text
        .lines()
        .find_map(|line| line.strip_prefix("seed="))
        .unwrap_or_else(|| panic!("{} has no `seed=` line", seed_file.display()))
        .to_string();
    let key_file = keys_dir.join(format!("validator-{index}.nodekey"));
    let node_key = std::fs::read_to_string(&key_file)
        .unwrap_or_else(|e| panic!("read {}: {e}", key_file.display()));
    let node_key = node_key.trim().to_string();

    let mut command = Command::new(node_bin);
    command
        .env("X3_DEV_SEED", seed)
        .arg("--chain")
        .arg(victim_spec)
        .arg("--base-path")
        .arg(net_dir.join(format!("node-{index}")))
        .arg("--name")
        .arg(format!("x3-testnet-node-{index:02}"))
        .arg("--rpc-port")
        .arg(rpc_port.to_string())
        .arg("--rpc-methods=Unsafe")
        .arg("--rpc-cors=all")
        .arg("--disable-log-color")
        .arg("--listen-addr")
        .arg(format!("/ip4/127.0.0.1/tcp/{p2p_port}"))
        .arg("--no-mdns")
        .arg("--no-telemetry")
        .arg("--no-prometheus")
        .arg("--validator")
        .arg("--force-authoring")
        .arg("--allow-private-ip")
        .arg("--node-key")
        .arg(node_key)
        .arg("--out-peers")
        .arg("0");
    let log =
        File::create(log_path).unwrap_or_else(|e| panic!("create {}: {e}", log_path.display()));
    command.stdout(Stdio::from(log.try_clone().expect("clone log handle")));
    command.stderr(Stdio::from(log));
    let child = command
        .spawn()
        .unwrap_or_else(|e| panic!("relaunch validator {index} inbound-only: {e}"));
    std::fs::write(
        net_dir.join("pids").join(format!("node-{index}.pid")),
        child.id().to_string(),
    )
    .unwrap_or_else(|e| panic!("write the relaunched validator's pid file: {e}"));
    child
}

/// Run a helper script from the repository root, with `envs` set, and hand back its stdout. A
/// non-zero exit or a spawn failure panics with everything the script printed, so a broken spec
/// build or launcher is a readable failure rather than a mysterious timeout.
fn run_script(root: &Path, program: &str, args: &[String], envs: &[(&str, String)]) -> String {
    let mut command = Command::new(program);
    command.args(args).current_dir(root);
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("run {program} {args:?} in {}: {e}", root.display()));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        panic!(
            "{program} {args:?} failed ({})\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
            output.status
        );
    }
    stdout
}

/// The generated validator set, killed on drop — on success *and* on panic — by the pid files the
/// launcher wrote. `pkill -f` is scoped to this run's base path, never a bare
/// `pkill -f x3-chain-node`, which would kill the nodes of every other gate on this box.
struct TestnetSet {
    run_dir: PathBuf,
    net_dir: PathBuf,
    /// The inbound-only replacement for the isolated validator, once it has been restarted. Held so
    /// `Drop` can reap it rather than leave a zombie.
    relaunched: Option<Child>,
}

impl TestnetSet {
    fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    fn net_dir(&self) -> &Path {
        &self.net_dir
    }

    /// The sanitized copy of the spec the launcher actually boots (`CHAIN_SPEC_RUN`).
    fn chain_spec(&self) -> PathBuf {
        self.net_dir.join("chain-spec.json")
    }

    fn keys_dir(&self) -> PathBuf {
        self.run_dir.join("spec").join("validator-keys")
    }
}

impl Drop for TestnetSet {
    fn drop(&mut self) {
        if let Some(mut child) = self.relaunched.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Ok(entries) = std::fs::read_dir(self.net_dir.join("pids")) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !(name.starts_with("node-") && name.ends_with(".pid")) {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(entry.path()) {
                    if let Ok(pid) = text.trim().parse::<u32>() {
                        let _ = Command::new("kill")
                            .arg("-9")
                            .arg(pid.to_string())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                }
            }
        }
        // A pid file can be stale (a `--only` restart rewrites one), so sweep by the base path too.
        // The pattern deliberately does not begin with `-`: `pkill` would read `--base-path` as an
        // option rather than a pattern.
        let _ = Command::new("pkill")
            .arg("-9")
            .arg("-f")
            .arg(format!("base-path {}/node-", self.net_dir.display()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if std::thread::panicking() {
            eprintln!(
                "[x3-partition7] test panicked — keeping {} for diagnosis",
                self.run_dir.display()
            );
        } else {
            let _ = std::fs::remove_dir_all(&self.run_dir);
        }
    }
}

/// Build a generated `count`-authority plain Live spec and boot it with the testnet launcher.
///
/// The guard is constructed *before* anything is spawned, so a panic while the launcher is starting
/// nodes still tears the ones that did start down. The launcher exits once every node answers RPC,
/// so a successful return means `count` validators are running.
fn boot_generated_network(
    root: &Path,
    node_bin: &Path,
    run_dir: &Path,
    count: usize,
) -> TestnetSet {
    let net_dir = run_dir.join("net");
    let spec_dir = run_dir.join("spec");
    let log_dir = run_dir.join("logs");
    std::fs::create_dir_all(&net_dir).expect("create the generated network's base dir");
    std::fs::create_dir_all(&spec_dir).expect("create the generated network's spec dir");

    let network = TestnetSet {
        run_dir: run_dir.to_path_buf(),
        net_dir: net_dir.clone(),
        relaunched: None,
    };
    let node_bin = node_bin.to_string_lossy().into_owned();
    let chain_spec = spec_dir.join("x3-testnet-plain.json");
    let keys_dir = spec_dir.join("validator-keys");

    println!(
        "[x3-partition7] building a {count}-authority spec from generated keys (never dev keys)"
    );
    run_script(
        root,
        "python3",
        &[
            "scripts/testnet/build-x3-testnet-spec.py".to_string(),
            count.to_string(),
        ],
        &[
            ("X3_NODE_BIN", node_bin.clone()),
            ("OUT_DIR", spec_dir.to_string_lossy().into_owned()),
            // Must equal the launcher's P2P_BASE below, or the spec's bootNodes name ports nobody
            // listens on and the mesh collapses.
            ("P2P_BASE", SEVEN_P2P_BASE.to_string()),
        ],
    );
    assert!(
        chain_spec.is_file(),
        "the spec builder did not write {}",
        chain_spec.display()
    );

    println!("[x3-partition7] booting {count} validators via x3_testnet_up.sh...");
    let stdout = run_script(
        root,
        "bash",
        &[
            "scripts/testnet/x3_testnet_up.sh".to_string(),
            "--skip-build".to_string(),
            "--node-bin".to_string(),
            node_bin,
        ],
        &[
            ("COUNT", count.to_string()),
            ("RPC_BASE", SEVEN_RPC_BASE.to_string()),
            ("P2P_BASE", SEVEN_P2P_BASE.to_string()),
            ("PROM_BASE", SEVEN_PROM_BASE.to_string()),
            ("BASE_DIR", net_dir.to_string_lossy().into_owned()),
            ("CHAIN_SPEC", chain_spec.to_string_lossy().into_owned()),
            ("KEYS_DIR", keys_dir.to_string_lossy().into_owned()),
            ("LOG_DIR", log_dir.to_string_lossy().into_owned()),
            ("SKIP_BUILD", "1".to_string()),
        ],
    );
    let started = stdout
        .lines()
        .filter(|line| line.contains("Started x3-testnet-node-"))
        .count();
    assert_eq!(
        started, count,
        "the launcher reported {started} of {count} validators started; refusing to test a network \
         that is not all there"
    );

    network
}

#[test]
#[ignore = "boots a generated seven-authority network, isolates one validator at the kernel level \
            while it stays alive, and requires the six survivors to keep finalizing — the \
            tolerance a three-authority set cannot have and a public testnet needs"]
fn seven_authority_network_keeps_finalizing_with_one_validator_isolated() {
    let count = seven_authority_count();
    let victim = SEVEN_VICTIM - 1;
    assert!(
        victim < count,
        "the victim index {SEVEN_VICTIM} is outside a {count}-authority set"
    );
    let rpc_ports = seven_rpc_ports(count);
    let p2p_ports = seven_p2p_ports(count);
    let mut all_ports = rpc_ports.clone();
    all_ports.extend(p2p_ports.iter().copied());
    assert_ports_free(&all_ports);
    let tools = NetTools::detect();

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_nanos();
    let run_dir =
        std::env::temp_dir().join(format!("x3-partition7-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&run_dir).expect("create the run directory");
    let (node_bin, node_bin_digest) = freeze_node_binary(&run_dir);
    println!(
        "[x3-partition7] every validator runs one frozen artifact: {} (sha256 {})",
        node_bin.display(),
        node_bin_digest.as_deref().unwrap_or("unavailable")
    );
    println!("[x3-partition7] run directory: {}", run_dir.display());
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the node crate has a parent (the repository root)")
        .to_path_buf();

    // -------- phase 1: N authorities, fully connected and finalizing --------
    let mut network = boot_generated_network(&root, &node_bin, &run_dir, count);
    for (i, port) in rpc_ports.iter().enumerate() {
        wait_for_rpc(&format!("node{}", i + 1), *port, NODE_BOOT_TIMEOUT);
    }

    let expected_peers = (count - 1) as u64;
    let peers = wait_until(
        &format!("all {count} validators to see each other"),
        CONSENSUS_TIMEOUT,
        || {
            let peers: Vec<u64> = rpc_ports.iter().map(|port| peers_of(*port)).collect();
            if peers.iter().all(|n| *n >= expected_peers) {
                Ok(peers)
            } else {
                Err(format!("peers {peers:?} (want {expected_peers} each)"))
            }
        },
    );
    println!("[x3-partition7] connected: {count} validators, peers {peers:?}");

    let finals = wait_until(
        &format!("finality past genesis on all {count} validators"),
        CONSENSUS_TIMEOUT,
        || {
            let finals: Vec<u64> = rpc_ports
                .iter()
                .map(|port| finalized_number(*port))
                .collect();
            if finals.iter().all(|n| *n >= SEVEN_MIN_FINALIZED) {
                Ok(finals)
            } else {
                Err(format!("finalized {finals:?}"))
            }
        },
    );
    let (height0, hash0) = common_finalized(&rpc_ports);
    println!(
        "[x3-partition7] consensus: all {count} agree on {height0}:{hash0} (finalized {finals:?})"
    );

    // -------- phase 1b: put the victim on an inbound-only footing --------
    // A port-scoped cut is only complete if the isolated validator cannot dial out. With the
    // launcher's normal outbound budget it re-dials on a fresh ephemeral source port the DROP rules
    // do not match and re-syncs (measured — see the note at the top of this section). Restarting it
    // with `--out-peers 0` and an unreachable-only spec, exactly as the three-authority test boots
    // Charlie, makes every link it owns an inbound connection to its single P2P port.
    let victim_rpc = rpc_ports[victim];
    let victim_p2p = p2p_ports[victim];
    let victim_spec = run_dir.join("chain-spec-inbound-only.json");
    println!(
        "[x3-partition7] restarting validator {} inbound-only (--out-peers 0, unreachable \
         bootnode) so the cut can be complete",
        victim + 1
    );
    stop_validator(network.net_dir(), victim + 1, victim_rpc, victim_p2p);
    write_inbound_only_spec(&network.chain_spec(), &victim_spec);
    let victim_child = relaunch_inbound_only(
        &node_bin,
        network.net_dir(),
        &network.keys_dir(),
        victim + 1,
        victim_rpc,
        victim_p2p,
        &victim_spec,
        &network.run_dir().join("victim-inbound-only.log"),
    );
    network.relaunched = Some(victim_child);
    let rejoin = wait_until(
        &format!(
            "validator {} to rejoin with {expected_peers} inbound peers",
            victim + 1
        ),
        CONVERGE_TIMEOUT,
        || {
            let mut peers = Vec::with_capacity(count);
            for port in &rpc_ports {
                match peers_try(*port) {
                    Ok(n) => peers.push(n),
                    Err(error) => return Err(format!("{error}; peers so far {peers:?}")),
                }
            }
            if peers.iter().all(|n| *n >= expected_peers) {
                Ok(peers)
            } else {
                Err(format!("peers {peers:?} (want {expected_peers} each)"))
            }
        },
    );
    println!(
        "[x3-partition7] validator {} rejoined inbound-only: peers {rejoin:?}",
        victim + 1
    );

    // It must be caught up before it is cut, or the backlog it fetched while reconnecting (blocks
    // and finality justifications already in hand) keeps advancing its finalized head after the cut.
    // Measured: a validator left ~160 blocks behind kept finalizing from a 519 watermark up to 653
    // once it was cut off, which is not the isolated validator finalizing on its own.
    let caught_up = wait_until(
        &format!("validator {} to catch up before it is cut off", victim + 1),
        CONVERGE_TIMEOUT,
        || {
            let finals: Vec<u64> = rpc_ports
                .iter()
                .map(|port| finalized_number(*port))
                .collect();
            let best_survivor = finals
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != victim)
                .map(|(_, n)| *n)
                .max()
                .expect("at least one survivor");
            if finals[victim] + SEVEN_CATCHUP_DELTA >= best_survivor {
                Ok(finals)
            } else {
                Err(format!(
                    "validator {} finalized {} against the best survivor {best_survivor}",
                    victim + 1,
                    finals[victim]
                ))
            }
        },
    );
    println!(
        "[x3-partition7] validator {} caught up before the cut: finalized {caught_up:?}",
        victim + 1
    );

    // -------- phase 2: cut exactly one validator off, without killing it --------
    let guard = PartitionGuard {
        tools,
        p2p_port: victim_p2p,
    };
    guard.tools.cut(victim_p2p);
    println!(
        "[x3-partition7] cut validator {} (rpc :{victim_rpc}, p2p :{victim_p2p}) off at the kernel \
         level; it stays alive",
        victim + 1
    );

    // The cut is only believed once the peer counts move: the survivor that lost the link drops by
    // one; the isolated validator reaches zero. A "partition" that does not change a count is
    // checking nothing, so a cut that does not show fails here by name.
    let survivor_peers = (count - 2) as u64;
    let cut_peers = wait_until(
        &format!("the cut to show as peer counts (victim 0, each survivor {survivor_peers})"),
        PARTITION_TIMEOUT,
        || {
            let mut observed = Vec::with_capacity(count);
            for (i, port) in rpc_ports.iter().enumerate() {
                let n = peers_of(*port);
                observed.push(n);
                if i == victim {
                    if n != 0 {
                        return Err(format!(
                            "the isolated validator still reports {n} peer(s); peer counts \
                             {observed:?}"
                        ));
                    }
                } else if n != survivor_peers {
                    return Err(format!(
                        "survivor {} reports {n} peers, want {survivor_peers}; peer counts \
                         {observed:?}",
                        i + 1
                    ));
                }
            }
            Ok(observed)
        },
    );
    println!("[x3-partition7] cut confirmed: peer counts {cut_peers:?}");

    // The isolated validator is a running node, not a corpse.
    let alive = rpc_expect(victim_rpc, "system_health", Vec::new());
    assert!(
        alive.get("peers").is_some(),
        "the isolated validator stopped answering system_health: {alive}"
    );

    // The watermark the survivors must push past and the victim must not. Read only once the
    // victim's finalized head has stopped moving, so a backlog it fetched while reconnecting cannot
    // advance it after the cut and be mistaken for the isolated validator finalizing on its own.
    let victim_frozen = wait_until(
        "the isolated validator's finalized head to stop moving",
        PARTITION_TIMEOUT,
        || {
            let before = finalized_number(victim_rpc);
            std::thread::sleep(Duration::from_secs(5));
            let after = finalized_number(victim_rpc);
            if before == after {
                Ok(before)
            } else {
                Err(format!("finalized moved {before} -> {after}"))
            }
        },
    );
    let at_cut: Vec<View> = rpc_ports.iter().map(|port| view(*port)).collect();
    let freeze_floor = at_cut
        .iter()
        .map(|v| v.finalized)
        .max()
        .expect("a finalized watermark");
    let fork = at_cut.iter().map(|v| v.best).min().expect("a fork point");
    println!(
        "[x3-partition7] fork point ≈ {fork}; finality watermark {freeze_floor}; validator {} \
         frozen at {victim_frozen}",
        victim + 1
    );

    // -------- phase 3: the survivors keep finalizing while the victim falls behind --------
    let during = wait_until(
        &format!(
            "the survivors to finalize past {} while the isolated validator falls {SEVEN_VICTIM_LAG} \
             behind",
            freeze_floor + SEVEN_AFTER_CUT_FINALITY
        ),
        PARTITION_TIMEOUT,
        || {
            let views: Vec<View> = rpc_ports.iter().map(|port| view(*port)).collect();
            let survivor_finalized = views
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != victim)
                .map(|(_, v)| v.finalized)
                .min()
                .expect("at least one survivor");
            let survivor_best = views
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != victim)
                .map(|(_, v)| v.best)
                .min()
                .expect("at least one survivor");
            let behind = survivor_best.saturating_sub(views[victim].best);
            if survivor_finalized >= freeze_floor + SEVEN_AFTER_CUT_FINALITY
                && behind >= SEVEN_VICTIM_LAG
            {
                Ok(views)
            } else {
                Err(format!(
                    "survivors finalized {survivor_finalized} (want ≥ {}); victim behind by \
                     {behind} (want ≥ {SEVEN_VICTIM_LAG}); best {survivor_best}",
                    freeze_floor + SEVEN_AFTER_CUT_FINALITY
                ))
            }
        },
    );
    println!(
        "[x3-partition7] during: finalized {:?}; best {:?}; peers {:?}",
        during.iter().map(|v| v.finalized).collect::<Vec<_>>(),
        during.iter().map(|v| v.best).collect::<Vec<_>>(),
        during.iter().map(|v| v.peers).collect::<Vec<_>>(),
    );

    // (a) The survivors kept finalizing — the property a testnet needs, and the one a three-set
    //     cannot have.
    for (i, observed) in during.iter().enumerate() {
        if i == victim {
            continue;
        }
        assert!(
            observed.finalized >= freeze_floor + SEVEN_AFTER_CUT_FINALITY,
            "survivor {} stopped finalizing: {} did not reach {} (watermark {freeze_floor})",
            i + 1,
            observed.finalized,
            freeze_floor + SEVEN_AFTER_CUT_FINALITY
        );
        assert!(
            observed.peers >= survivor_peers,
            "survivor {} lost a peer it should have kept: {} < {survivor_peers}",
            i + 1,
            observed.peers
        );
    }
    // (b) The isolated validator is isolated: zero peers, alive, frozen, and behind.
    assert_eq!(
        during[victim].peers, 0,
        "the isolated validator was not isolated: it reports {} peers",
        during[victim].peers
    );
    assert!(
        during[victim].finalized <= victim_frozen,
        "the isolated validator finalized past its watermark without the other authorities: {} -> \
         {}",
        victim_frozen,
        during[victim].finalized
    );
    let slowest_survivor_best = during
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != victim)
        .map(|(_, v)| v.best)
        .min()
        .expect("at least one survivor");
    assert!(
        during[victim].best + SEVEN_VICTIM_LAG <= slowest_survivor_best,
        "the isolated validator did not fall behind: its head {} vs the slowest survivor's {}",
        during[victim].best,
        slowest_survivor_best
    );
    println!(
        "[x3-partition7] tolerance proven: {} of {count} survivors finalized past {freeze_floor} \
         while validator {} stayed alive at {victim_frozen} and fell {} behind",
        count - 1,
        victim + 1,
        slowest_survivor_best - during[victim].best
    );

    // -------- phase 4: heal, and require real convergence --------
    guard.tools.heal(victim_p2p);

    let healed = wait_until(
        &format!("all {count} validators to reconnect after the heal"),
        CONVERGE_TIMEOUT,
        || {
            let peers: Vec<u64> = rpc_ports.iter().map(|port| peers_of(*port)).collect();
            if peers.iter().all(|n| *n >= expected_peers) {
                Ok(peers)
            } else {
                Err(format!("peers {peers:?} (want {expected_peers} each)"))
            }
        },
    );
    println!("[x3-partition7] healed: peers {healed:?}");

    let converged = wait_until(
        "all validators to agree on a finalized block past the freeze",
        CONVERGE_TIMEOUT,
        || {
            let heights: Vec<u64> = rpc_ports
                .iter()
                .map(|port| finalized_number(*port))
                .collect();
            let common = *heights.iter().min().expect("a finalized height to compare");
            if common <= freeze_floor {
                return Err(format!(
                    "finality has not resumed past the freeze {freeze_floor}: {heights:?}"
                ));
            }
            let hashes: Vec<String> = rpc_ports
                .iter()
                .map(|port| {
                    block_hash_at(*port, common)
                        .unwrap_or_else(|| panic!(":{} has no hash at {common}", port))
                })
                .collect();
            if !hashes.iter().all(|h| h == &hashes[0]) {
                return Err(format!(
                    "the validators disagree at height {common}: {hashes:?}"
                ));
            }
            Ok((common, hashes[0].clone(), heights))
        },
    );
    let (conv_height, conv_hash, heights) = converged;
    assert!(
        conv_height > freeze_floor,
        "finality did not resume past the freeze point ({freeze_floor})"
    );
    println!(
        "[x3-partition7] converged: all {count} finalized {conv_height}:{conv_hash} (heights \
         {heights:?}), past the freeze {freeze_floor}"
    );

    let resumed = wait_until(
        "finality to keep advancing after the heal",
        CONVERGE_TIMEOUT,
        || {
            let heights: Vec<u64> = rpc_ports
                .iter()
                .map(|port| finalized_number(*port))
                .collect();
            let common = *heights.iter().min().expect("a finalized height to compare");
            if common > conv_height {
                Ok(common)
            } else {
                Err(format!("finalized {heights:?}"))
            }
        },
    );
    println!(
        "[x3-partition7] finality resumed: {freeze_floor} (frozen) -> {conv_height} (converged) -> \
         {resumed} (still advancing) on all {count}"
    );

    drop(guard);
    drop(network);
}
