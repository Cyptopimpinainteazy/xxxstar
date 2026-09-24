use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;
use tempfile::tempdir;

use gpu_swarm::admin::AdminState;
use gpu_swarm::admin::{ensure_config_dir, ensure_local_token, run_admin};

#[test]
fn admin_register_and_login_flow() {
    // setup a temp config dir and point XDG_CONFIG_HOME to it
    let dir = tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());

    // pick a free port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    // shared state
    let state = std::sync::Arc::new(tokio::sync::Mutex::new(AdminState::default()));

    // start admin server on thread
    let addr = format!("127.0.0.1:{}", port).parse().unwrap();
    thread::spawn(move || {
        // run admin (it spawns tiny_http and runs forever)
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move { run_admin(state, addr).await });
    });

    // Wait for server to start
    thread::sleep(Duration::from_millis(200));

    let client = Client::new();
    // Should have token file
    let token = ensure_local_token().unwrap();

    // POST /api/login with token
    let url = format!("http://127.0.0.1:{}/api/login", port);
    let res: serde_json::Value = client
        .post(&url)
        .json(&serde_json::json!({"token": token}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(res.get("ok").and_then(|v| v.as_bool()), Some(true));

    // POST /api/register
    let url2 = format!("http://127.0.0.1:{}/api/register", port);
    let reg: serde_json::Value = client.post(&url2).send().unwrap().json().unwrap();
    assert!(reg.get("mnemonic").is_some());
    assert!(reg.get("address").is_some());

    // GET /api/state
    let url3 = format!("http://127.0.0.1:{}/api/state", port);
    let state: serde_json::Value = client.get(&url3).send().unwrap().json().unwrap();
    assert!(state.get("wallet_address").is_some());
}
