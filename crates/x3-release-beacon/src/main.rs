use std::{
    collections::BTreeSet, env, error::Error, net::SocketAddr, path::PathBuf, time::Duration,
};
use x3_release_beacon::{
    exchange::{bind_exchange, serve_release_beacon_exchange, ReleaseBeaconExchange},
    publisher::{run_release_beacon_publisher, ReleaseBeaconSigner},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("exchange") => run_exchange(&args[2..]).await,
        Some("publish") => run_publisher(&args[2..]).await,
        _ => Err(
            "usage: x3-release-beacon exchange --listen ADDR --store PATH \
             --authorized-key HEX [--authorized-key HEX ...] | publish --rpc-url URL \
             --exchange-url URL --attestor-id ID [--poll-ms N]"
                .into(),
        ),
    }
}

async fn run_exchange(args: &[String]) -> Result<(), Box<dyn Error>> {
    let listen = required_flag(args, "--listen")?.parse::<SocketAddr>()?;
    let store = PathBuf::from(required_flag(args, "--store")?);
    let keys = repeated_flag(args, "--authorized-key")?
        .into_iter()
        .map(|value| decode_32(&value))
        .collect::<Result<BTreeSet<_>, String>>()?;
    if keys.is_empty() {
        return Err("exchange requires at least one --authorized-key".into());
    }
    let exchange = ReleaseBeaconExchange::open(store, keys)?;
    println!("release beacon exchange listening on {listen}");
    serve_release_beacon_exchange(bind_exchange(listen).await?, exchange).await?;
    Ok(())
}

async fn run_publisher(args: &[String]) -> Result<(), Box<dyn Error>> {
    let attestor_id = required_flag(args, "--attestor-id")?;
    let rpc_url = required_flag(args, "--rpc-url")?;
    let exchange_url = required_flag(args, "--exchange-url")?;
    let poll_ms = flag(args, "--poll-ms")
        .unwrap_or_else(|| "2000".into())
        .parse::<u64>()?;
    if poll_ms == 0 {
        return Err("--poll-ms must be greater than zero".into());
    }
    let seed = env::var("X3_RELEASE_BEACON_PRIVATE_KEY_HEX")
        .map_err(|_| "X3_RELEASE_BEACON_PRIVATE_KEY_HEX is required")?;
    let signer = ReleaseBeaconSigner::from_hex(attestor_id, &seed)?;
    println!(
        "release beacon publisher starting as {} ({})",
        signer.attestor_id(),
        hex::encode(signer.public_key())
    );
    run_release_beacon_publisher(
        signer,
        rpc_url,
        exchange_url,
        Duration::from_millis(poll_ms),
    )
    .await;
    Ok(())
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].clone())
}

fn required_flag(args: &[String], name: &str) -> Result<String, String> {
    flag(args, name).ok_or_else(|| format!("missing required flag {name}"))
}

fn repeated_flag(args: &[String], name: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    for window in args.windows(2) {
        if window[0] == name {
            values.push(window[1].clone());
        }
    }
    Ok(values)
}

fn decode_32(value: &str) -> Result<[u8; 32], String> {
    let trimmed = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    let bytes = hex::decode(trimmed).map_err(|error| error.to_string())?;
    bytes
        .try_into()
        .map_err(|bytes: Vec<u8>| format!("key {value} is {} bytes, expected 32", bytes.len()))
}
