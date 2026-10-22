use futures_util::StreamExt;
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use std::{error::Error, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    let endpoint = std::env::args()
        .nth(1)
        .ok_or("loopback endpoint required")?;
    if !is_loopback_endpoint(&endpoint) {
        return Err("this verification probe only permits loopback endpoints".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let client =
            tokio::time::timeout(Duration::from_secs(10), PubsubClient::new(&endpoint)).await??;
        let (mut slots, unsubscribe) =
            tokio::time::timeout(Duration::from_secs(10), client.slot_subscribe()).await??;
        let sample = tokio::time::timeout(Duration::from_secs(10), slots.next())
            .await?
            .ok_or("slot stream ended before a real notification")?;
        if sample.root > sample.slot || sample.parent > sample.slot {
            return Err("inconsistent slot notification".into());
        }
        println!(
            "slot={} parent={} root={}",
            sample.slot, sample.parent, sample.root
        );
        drop(slots);
        unsubscribe().await;
        tokio::time::timeout(Duration::from_secs(5), client.shutdown()).await??;
        Ok::<(), Box<dyn Error>>(())
    })
}

fn is_loopback_endpoint(endpoint: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(endpoint) else {
        return false;
    };
    let permitted = matches!(
        (url.scheme(), url.host_str()),
        ("wss", Some("localhost")) | ("ws", Some("127.0.0.1"))
    );
    permitted
        && url.username().is_empty()
        && url.password().is_none()
        && url.socket_addrs(|| None).is_ok_and(|addresses| {
            !addresses.is_empty() && addresses.iter().all(|address| address.ip().is_loopback())
        })
}

#[cfg(test)]
mod tests {
    use super::is_loopback_endpoint;

    #[test]
    fn loopback_endpoint_accepts_local_probe_addresses() {
        assert!(is_loopback_endpoint("wss://localhost:12345/"));
        assert!(is_loopback_endpoint("ws://127.0.0.1:12345/"));
    }

    #[test]
    fn loopback_endpoint_rejects_authority_prefix_bypasses() {
        for endpoint in [
            "wss://localhost:12345@outside.example:443/",
            "ws://127.0.0.1:12345@outside.example/",
            "wss://outside.example/",
            "https://localhost:12345/",
        ] {
            assert!(!is_loopback_endpoint(endpoint), "{endpoint}");
        }
    }
}
