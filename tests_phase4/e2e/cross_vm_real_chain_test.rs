//! Real-chain integration test for cross-VM swaps
//!
//! Run with: cargo test --test cross_vm_real_chain_test -- --nocapture
//! Requires dev node: ./target/release/x3-node --dev

#[cfg(test)]
mod tests {
    use std::time::Duration;

    const RPC_HTTP: &str = "http://127.0.0.1:9944";
    const RPC_WS: &str = "ws://127.0.0.1:9944";

    fn is_node_running() -> bool {
        std::net::TcpStream::connect_timeout(
            &"127.0.0.1:9944".parse().unwrap(),
            Duration::from_millis(500),
        )
        .is_ok()
    }

    #[tokio::test]
    async fn test_cross_vm_connects() {
        if !is_node_running() {
            println!(
                "⚠ Dev node not running - skipping. Start with: ./target/release/x3-node --dev"
            );
            return;
        }

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
        println!("✅ system_health OK: {:?}", body["result"]);
    }

    #[tokio::test]
    async fn test_cross_vm_rpc_method_exists() {
        if !is_node_running() {
            println!("⚠ Dev node not running - skipping");
            return;
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let res = client
            .post(RPC_HTTP)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "rpc_methods",
                "params": [],
                "id": 1
            }))
            .send()
            .await
            .expect("Failed to connect");

        let body: serde_json::Value = res.json().await.unwrap();
        let methods = body["result"]["methods"]
            .as_array()
            .expect("Methods should be array");
        let method_names: Vec<&str> = methods.iter().filter_map(|m| m.as_str()).collect();

        // Verify all three cross-VM RPC methods are registered
        assert!(
            method_names.contains(&"x3_submitCrossVmTransaction"),
            "x3_submitCrossVmTransaction should be registered. Found: {:?}",
            method_names
                .iter()
                .filter(|m| m.starts_with("x3_"))
                .collect::<Vec<_>>()
        );
        println!("✅ x3_submitCrossVmTransaction method registered");

        assert!(
            method_names.contains(&"x3_submitSvmTransaction"),
            "x3_submitSvmTransaction should be registered"
        );
        println!("✅ x3_submitSvmTransaction method registered");

        assert!(
            method_names.contains(&"x3_submitX3vmTransaction"),
            "x3_submitX3vmTransaction should be registered"
        );
        println!("✅ x3_submitX3vmTransaction method registered");
    }

    #[tokio::test]
    async fn test_cross_vm_submit_evm_only() {
        if !is_node_running() {
            println!("⚠ Dev node not running - skipping");
            return;
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Simple EVM-only transaction (no SVM payload)
        // This is a minimal test tx - real swaps need proper signing
        let res = client
            .post(RPC_HTTP)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "x3_submitCrossVmTransaction",
                "params": [{
                    "evm_payload": "0x00",  // Minimal payload
                    "atomic": false
                }],
                "id": 1
            }))
            .send()
            .await
            .expect("Failed to connect");

        let body: serde_json::Value = res.json().await.unwrap();
        // We expect either a result (success) or an error (which is fine for invalid payload)
        // The key is that the RPC method exists and responds
        if body.get("error").is_some() {
            println!("✅ x3_submitCrossVmTransaction responds (error expected for minimal payload): {:?}", body["error"]["message"]);
        } else {
            println!(
                "✅ x3_submitCrossVmTransaction successful: {:?}",
                body["result"]
            );
        }
    }

    #[tokio::test]
    async fn test_cross_vm_submit_atomic() {
        if !is_node_running() {
            println!("⚠ Dev node not running - skipping");
            return;
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // Atomic cross-VM transaction (EVM + SVM payloads)
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
            .expect("Failed to connect");

        let body: serde_json::Value = res.json().await.unwrap();
        // Cross-VM path should now be enabled (not returning "not available" error)
        if let Some(err) = body.get("error") {
            let msg = err["message"].as_str().unwrap_or("");
            assert!(
                !msg.contains("not available"),
                "Cross-VM should be enabled. Got: {msg}"
            );
            println!(
                "✅ Atomic cross-VM accepted (execution error expected for test payload): {msg}"
            );
        } else {
            println!(
                "✅ Atomic cross-VM transaction submitted: {:?}",
                body["result"]
            );
        }
    }

    #[tokio::test]
    async fn test_websocket_connection() {
        if !is_node_running() {
            println!("⚠ Dev node not running - skipping");
            return;
        }

        use tokio_tungstenite::connect_async;

        match connect_async(RPC_WS).await {
            Ok((ws_stream, _)) => {
                println!("✅ WebSocket connection established");
                drop(ws_stream);
            }
            Err(e) => {
                println!("⚠ WebSocket connection failed: {e}");
                // Not a hard failure - some test environments may not have WS
            }
        }
    }

    #[tokio::test]
    async fn test_svm_submit() {
        if !is_node_running() {
            println!("⚠ Dev node not running - skipping");
            return;
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // SVM-only transaction submission
        let res = client
            .post(RPC_HTTP)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "x3_submitSvmTransaction",
                "params": [{
                    "svm_payload": "0x0102",  // Minimal SVM payload
                    "caller": "0x0000000000000000000000000000000000000000"
                }],
                "id": 1
            }))
            .send()
            .await
            .expect("Failed to connect");

        let body: serde_json::Value = res.json().await.unwrap();

        if body.get("error").is_some() {
            println!("✅ x3_submitSvmTransaction responds with error (expected for minimal payload): {:?}", body["error"]["message"]);
        } else {
            println!(
                "✅ x3_submitSvmTransaction successful: {:?}",
                body["result"]
            );
        }
    }

    #[tokio::test]
    async fn test_x3vm_submit() {
        if !is_node_running() {
            println!("⚠ Dev node not running - skipping");
            return;
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        // X3VM submission (should indicate it's part of Comit protocol)
        let res = client
            .post(RPC_HTTP)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "x3_submitX3vmTransaction",
                "params": [{
                    "x3vm_payload": "0x01"
                }],
                "id": 1
            }))
            .send()
            .await
            .expect("Failed to connect");

        let body: serde_json::Value = res.json().await.unwrap();

        if let Some(err) = body.get("error") {
            let msg = err["message"].as_str().unwrap_or("");
            assert!(
                msg.contains("Comit") || msg.contains("Comit"),
                "X3VM should indicate Comit protocol usage. Got: {msg}"
            );
            println!("✅ x3_submitX3vmTransaction correctly indicates Comit protocol: {msg}");
        } else {
            panic!("x3_submitX3vmTransaction should return error guidance (X3VM is part of Comit)");
        }
    }
}
