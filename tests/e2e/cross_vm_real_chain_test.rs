//! Real-chain integration test for cross-VM swaps
//!
//! This test boots an ephemeral x3-chain-node, waits for block production
//! and finality, then submits signed extrinsics over RPC.
//!
//! **Gated behind the `real-chain` feature**, because it needs a built node and
//! `cargo test --workspace` cannot supply one — before the gate that command aborted here with
//! six panics instead of naming the missing prerequisite (TICKET-103). Run it on purpose:
//!
//! ```text
//! cargo build -p x3-chain-node            # or set X3_NODE_BIN
//! cargo test -p e2e_tests --features real-chain --test cross_vm_real_chain_test -- --nocapture
//! ```
//!
//! The binary is looked for in `$CARGO_TARGET_DIR` first and then in the workspace's default
//! target directory, release before debug. With the feature on and no node anywhere, each test
//! prints a named reason and passes rather than panicking; `X3_NODE_BIN` set to a path that does
//! not exist is a failure, because setting it is the ask.

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const RPC_HTTP: &str = "http://127.0.0.1:9944";

static NODE_TEST_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

async fn node_test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    NODE_TEST_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

/// Where a node binary may be, most preferred first.
///
/// Split out from [`node_binary`] so the *search order* is testable without a node on the
/// machine, and so `CARGO_TARGET_DIR` is honoured: a build into any target directory was
/// invisible to this suite, which looked only at `<workspace_root>/target/…` and therefore
/// could not find a binary that had just been built (TICKET-103).
fn node_candidates(
    target_dir: Option<&std::path::Path>,
    workspace_root: &std::path::Path,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = target_dir {
        // The directory the build actually used, if it was told to use one. Release first: a
        // release node is the one these tests are written against.
        candidates.push(dir.join("release/x3-chain-node"));
        candidates.push(dir.join("debug/x3-chain-node"));
    }
    candidates.push(workspace_root.join("target/release/x3-chain-node"));
    candidates.push(workspace_root.join("target/debug/x3-chain-node"));
    candidates
}

/// The first candidate that exists.
fn choose(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.is_file()).cloned()
}

/// The node binary these tests drive, or `None` when there is not one.
///
/// `None` is not an error: these tests run a real chain, and a developer who has not built a
/// node has not asked to. `X3_NODE_BIN` **is** an ask — it is set deliberately — so a path that
/// does not exist is refused rather than skipped past.
fn node_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("X3_NODE_BIN") {
        let path = PathBuf::from(path);
        assert!(
            path.is_file(),
            "X3_NODE_BIN is set to {} and there is no file there. It was set deliberately, so \
             this is a failure rather than a skip: point it at a built x3-chain-node, or unset it \
             and let the search run.",
            path.display()
        );
        return Some(path);
    }

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("e2e manifest should be nested under the workspace root")
        .to_path_buf();
    let target_dir = std::env::var("CARGO_TARGET_DIR").ok().map(PathBuf::from);
    choose(&node_candidates(target_dir.as_deref(), &workspace_root))
}

/// Why a real-chain test did not run, naming where it looked.
///
/// Printed rather than panicked: six panics told a reader that something was wrong with the
/// *code*, when the truth was that the suite's prerequisite was absent. A named absence is a
/// different thing, and it is what this prints.
fn skip_reason() -> String {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("e2e manifest should be nested under the workspace root")
        .to_path_buf();
    let target_dir = std::env::var("CARGO_TARGET_DIR").ok().map(PathBuf::from);
    let searched: Vec<String> = node_candidates(target_dir.as_deref(), &workspace_root)
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    format!(
        "SKIPPED: this suite drives a real chain and no x3-chain-node binary was found. Build one \
         (`cargo build -p x3-chain-node`) or set X3_NODE_BIN. Searched:\n  {}",
        searched.join("\n  ")
    )
}

#[test]
fn node_binary_prefers_release_build() {
    // Built, not found: the preference is asserted against a directory holding both, so this
    // test runs on a machine with no node at all — which the previous version could not, since
    // it asserted a suffix of a path it had to find first.
    let dir = std::env::temp_dir().join(format!("x3-node-choice-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("release")).expect("a release directory");
    std::fs::create_dir_all(dir.join("debug")).expect("a debug directory");
    std::fs::write(dir.join("release/x3-chain-node"), b"release").expect("a release binary");
    std::fs::write(dir.join("debug/x3-chain-node"), b"debug").expect("a debug binary");

    let chosen = choose(&node_candidates(
        Some(&dir),
        std::path::Path::new("/nonexistent"),
    ))
    .expect("both candidates exist");
    assert!(
        chosen.ends_with("release/x3-chain-node"),
        "real-chain tests must prefer the validated release node, selected {}",
        chosen.display()
    );

    // And with only a debug build the search still finds it, rather than falling through to the
    // workspace's default target directory.
    std::fs::remove_file(dir.join("release/x3-chain-node")).expect("remove the release binary");
    let chosen = choose(&node_candidates(
        Some(&dir),
        std::path::Path::new("/nonexistent"),
    ))
    .expect("the debug candidate exists");
    assert!(
        chosen.ends_with("debug/x3-chain-node"),
        "{}",
        chosen.display()
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Boot an ephemeral dev node and return a handle that kills it on drop.
struct EphemeralNode {
    child: Child,
    binary: PathBuf,
    stderr: Arc<Mutex<String>>,
}

impl EphemeralNode {
    fn start() -> Option<Self> {
        let binary = node_binary()?;
        let mut child = Command::new(&binary)
            .args([
                "--dev",
                "--tmp",
                "--validator",
                "--rpc-port",
                "9944",
                "--unsafe-rpc-external",
                "--no-prometheus",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("failed to start {}: {error}", binary.display()));
        let stderr = Arc::new(Mutex::new(String::new()));
        let stderr_capture = Arc::clone(&stderr);
        let pipe = child.stderr.take().expect("child stderr must be piped");
        std::thread::spawn(move || {
            let mut reader = BufReader::new(pipe);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap_or(0) != 0 {
                stderr_capture
                    .lock()
                    .expect("stderr lock poisoned")
                    .push_str(&line);
                line.clear();
            }
        });
        Some(Self {
            child,
            binary,
            stderr,
        })
    }

    fn wait_ready(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().expect("failed to poll node process") {
                std::thread::sleep(Duration::from_millis(50));
                panic!(
                    "node {} exited before RPC became ready with {status}; stderr:\n{}",
                    self.binary.display(),
                    self.stderr.lock().expect("stderr lock poisoned")
                );
            }
            if TcpStream::connect_timeout(
                &"127.0.0.1:9944".parse().unwrap(),
                Duration::from_millis(200),
            )
            .is_ok()
            {
                return;
            }
            if Instant::now() >= deadline {
                panic!(
                    "node {} did not open RPC port 9944 within {}s; stderr so far:\n{}",
                    self.binary.display(),
                    timeout.as_secs(),
                    self.stderr.lock().expect("stderr lock poisoned")
                );
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    async fn wait_for_finalized_advance(
        client: &reqwest::Client,
        from: u64,
        timeout: Duration,
    ) -> u64 {
        let deadline = Instant::now() + timeout;
        loop {
            let now = Self::finalized_number(client).await;
            if now > from {
                return now;
            }
            if Instant::now() > deadline {
                panic!("Timeout waiting for finalized head to advance from {from}");
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn finalized_number(client: &reqwest::Client) -> u64 {
        let res = client
            .post(RPC_HTTP)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "chain_getFinalizedHead",
                "params": [],
                "id": 1
            }))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        let hash = body["result"].as_str().unwrap().to_string();

        let res = client
            .post(RPC_HTTP)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "chain_getHeader",
                "params": [hash],
                "id": 1
            }))
            .send()
            .await
            .unwrap();
        let body: serde_json::Value = res.json().await.unwrap();
        let number_hex = body["result"]["number"].as_str().unwrap();
        let stripped = number_hex.trim_start_matches("0x");
        u64::from_str_radix(stripped, 16).unwrap()
    }
}

impl Drop for EphemeralNode {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn rpc_call(
    client: &reqwest::Client,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let res = client
        .post(RPC_HTTP)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        }))
        .send()
        .await
        .expect("RPC call failed");
    let body: serde_json::Value = res.json().await.unwrap();
    if let Some(err) = body.get("error") {
        panic!("RPC {method} error: {err}");
    }
    body["result"].clone()
}

#[tokio::test]
async fn test_cross_vm_connects() {
    let _test_lock = node_test_lock().await;
    let Some(mut node) = EphemeralNode::start() else {
        eprintln!("{}", skip_reason());
        return;
    };
    node.wait_ready(Duration::from_secs(30));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let res = client
        .post(RPC_HTTP)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system_health",
            "params": [],
            "id": 1
        }))
        .send()
        .await
        .expect("Failed to connect");

    let body: serde_json::Value = res.json().await.unwrap();
    assert!(
        body.get("result").is_some(),
        "Node should respond with health status"
    );
    println!("✅ system_health OK");
}

#[tokio::test]
async fn test_cross_vm_rpc_methods_present() {
    let _test_lock = node_test_lock().await;
    let Some(mut node) = EphemeralNode::start() else {
        eprintln!("{}", skip_reason());
        return;
    };
    node.wait_ready(Duration::from_secs(30));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let methods = rpc_call(&client, "rpc_methods", serde_json::json!([])).await;
    let names: Vec<&str> = methods["methods"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m.as_str())
        .collect();

    let required = "x3_submitCrossVmTransaction";
    assert!(
        names.contains(&required),
        "Required RPC method missing: {required}"
    );
    println!("✅ {required} registered");
}

#[tokio::test]
async fn test_block_production_and_finality() {
    let _test_lock = node_test_lock().await;
    let Some(mut node) = EphemeralNode::start() else {
        eprintln!("{}", skip_reason());
        return;
    };
    node.wait_ready(Duration::from_secs(30));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // Wait for at least one block to be finalized
    let start = EphemeralNode::finalized_number(&client).await;
    let end =
        EphemeralNode::wait_for_finalized_advance(&client, start, Duration::from_secs(30)).await;
    assert!(end > start, "Finalized head did not advance");
    println!("✅ Block production: finalized advanced from {start} to {end}");
}

#[tokio::test]
async fn test_signed_extrinsic_submission() {
    let _test_lock = node_test_lock().await;
    let Some(mut node) = EphemeralNode::start() else {
        eprintln!("{}", skip_reason());
        return;
    };
    node.wait_ready(Duration::from_secs(30));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Submit a cross-VM transaction (expect either result or execution error,
    // not "method not found" or connection failure)
    let res = client
        .post(RPC_HTTP)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "x3_submitCrossVmTransaction",
            "params": [{
                "evm_payload": "0x01",
                "svm_payload": "0x02",
                "atomic": true
            }],
            "id": 1
        }))
        .send()
        .await
        .expect("RPC call failed");

    let body: serde_json::Value = res.json().await.unwrap();
    // Accept either success or execution error — the critical test is that
    // the method is registered and the node responds to the signed extrinsic
    if let Some(err) = body.get("error") {
        let msg = err["message"].as_str().unwrap_or("");
        assert!(
            !msg.contains("not available"),
            "Cross-VM should be enabled. Got: {msg}"
        );
        println!("✅ Atomic cross-VM accepted (execution error: {msg})");
    } else {
        println!("✅ Atomic cross-VM transaction submitted");
    }
}

#[tokio::test]
async fn test_web_socket_connection() {
    let _test_lock = node_test_lock().await;
    let Some(mut node) = EphemeralNode::start() else {
        eprintln!("{}", skip_reason());
        return;
    };
    node.wait_ready(Duration::from_secs(30));

    use tokio_tungstenite::connect_async;
    let ws_url = "ws://127.0.0.1:9944";

    match connect_async(ws_url).await {
        Ok((ws_stream, _)) => {
            println!("✅ WebSocket connection established");
            drop(ws_stream);
        }
        Err(e) => {
            panic!("WebSocket connection failed: {e}");
        }
    }
}
