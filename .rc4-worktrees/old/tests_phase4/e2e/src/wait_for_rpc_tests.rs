#[cfg(test)]
mod tests {
    use super::super::wait_for_rpc::*;
    use httptest::{matchers::*, responders::*, Expectation, Server};
    use reqwest::Client;
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn success_when_predicate_true() {
        let server = Server::run();
        server.expect(
            Expectation::matching(all_of![request::method_path("POST", "/"), request::body(json_decoded(predicate(|v: &serde_json::Value| {
                v["method"] == "system_health"
            })))]).respond_with(json_encoded(json!({"jsonrpc":"2.0","id":1,"result":{"isSyncing":false}})))
        );

        let client = Client::new();
        let retry = RetryPolicy::default();

        let res = wait_for_rpc_health(&server.url_str(&"/"), "system_health", |v| {
            v.get("result").and_then(|r| r.get("isSyncing")).map(|b| b == &json!(false)).unwrap_or(false)
        }, &client, retry).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn timeout_on_no_success() {
        let server = Server::run();
        // always return empty response
        server.expect(Expectation::matching(request::method_path("POST", "/")).respond_with(status_code(500)));

        let client = Client::new();
        let mut retry = RetryPolicy::default();
        retry.max_elapsed = Duration::from_secs(2);

        let res = wait_for_rpc_health(&server.url_str(&"/"), "system_health", |_v| false, &client, retry).await;

        assert!(res.is_err());
        match res.err().unwrap() {
            crate::tests::wait_for_rpc::WaitError::Timeout(_) => {}
            other => panic!("expected timeout, got {:?}", other),
        }
    }
}
