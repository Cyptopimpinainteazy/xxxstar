use crate::{
    cli::{
        AtomicSwapSubcommand, Cli, ComitSubcommand, Commands, InspectSubcommand, KeysSubcommand,
        ValidatorSubcommand,
    },
    service,
};
use clap::Parser;
use codec::Encode;
#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use log::{error, info, warn};
use sc_cli::{Error as CliError, Result as CliResult, SubstrateCli};
#[cfg(feature = "runtime-benchmarks")]
use x3_chain_runtime::opaque::Block;

use crate::logging;

/// The account argument `x3_getCanonicalBalance` takes: 32 bytes of hex.
///
/// The CLI documents `--account` as "SS58 or hex format", and every call site
/// passed whatever the user typed straight through — including the SS58 literal
/// in the Comit query — so a correctly formed request was impossible: the node
/// decodes the parameter as hex and rejected all of them.
fn account_to_hex32(account: &str) -> Result<String, String> {
    if let Some(hex_part) = account
        .strip_prefix("0x")
        .or_else(|| account.strip_prefix("0X"))
    {
        let bytes = hex::decode(hex_part).map_err(|e| format!("invalid hex account: {e}"))?;
        if bytes.len() != 32 {
            return Err(format!("account hex must be 32 bytes, got {}", bytes.len()));
        }
        return Ok(format!("0x{}", hex::encode(bytes)));
    }

    use sp_core::crypto::Ss58Codec;
    let public = sp_runtime::AccountId32::from_ss58check(account)
        .map_err(|e| format!("invalid SS58 address: {e}"))?;
    Ok(format!("0x{}", hex::encode(public.as_ref() as &[u8])))
}

// ── Key management ──────────────────────────────────────────────────────────
//
// `keys generate|verify|insert|list` used to print "In a full implementation,
// this would use sp_core crypto / For now, show a placeholder" and exit 0. That
// is worse than an error on the mainnet path it exists for: `install-validator.sh`
// and `scripts/mainnet/genesis_ceremony.sh` both tell an operator to run these
// commands to produce the Aura/GRANDPA authorities that go into
// `X3_PRODUCTION_AUTHORITIES`, and `production_config()` refuses to build a
// genesis without those keys. A command that prints advice and succeeds leaves
// the operator with no key and an empty `--seed` file.

/// The block-authoring/finality key types this CLI understands.
///
/// `aura` and `imonline` are sr25519; `grandpa` is ed25519. The four-character
/// forms (`aura`, `gran`, `imon`) are what `KeyTypeId` stores in the keystore,
/// so both spellings are accepted and the long ones are not silently truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyScheme {
    Sr25519,
    Ed25519,
}

struct KeyTypeSpec {
    /// The 4-byte keystore key type (`aura`, `gran`, `imon`).
    key_type: sp_core::crypto::KeyTypeId,
    scheme: KeyScheme,
}

fn resolve_key_type(name: &str) -> Result<KeyTypeSpec, String> {
    use sp_core::crypto::KeyTypeId;
    let (id, scheme) = match name.to_ascii_lowercase().as_str() {
        "aura" => (KeyTypeId(*b"aura"), KeyScheme::Sr25519),
        "grandpa" | "gran" => (KeyTypeId(*b"gran"), KeyScheme::Ed25519),
        "imonline" | "imon" => (KeyTypeId(*b"imon"), KeyScheme::Sr25519),
        other => {
            return Err(format!(
                "unknown key type '{other}': expected aura, grandpa or imonline \
                 (or the 4-character keystore ids aura, gran, imon)"
            ))
        }
    };
    Ok(KeyTypeSpec {
        key_type: id,
        scheme,
    })
}

/// Derive a public key from a secret URI, in the scheme that key type uses.
fn public_from_suri(scheme: KeyScheme, suri: &str) -> Result<Vec<u8>, String> {
    use sp_core::{crypto::SecretStringError, Pair};
    match scheme {
        KeyScheme::Sr25519 => sp_core::sr25519::Pair::from_string(suri, None)
            .map(|p| p.public().0.to_vec())
            .map_err(|e: SecretStringError| format!("invalid sr25519 secret URI: {e:?}")),
        KeyScheme::Ed25519 => sp_core::ed25519::Pair::from_string(suri, None)
            .map(|p| p.public().0.to_vec())
            .map_err(|e: SecretStringError| format!("invalid ed25519 secret URI: {e:?}")),
    }
}

/// Generate a fresh keypair and return `(public, secret_seed_hex)`.
///
/// The secret is returned because a generated key that the operator cannot save
/// is a key the operator cannot use: `keys generate` without `--seed` has to
/// disclose the seed or it has done nothing.
fn generate_keypair(scheme: KeyScheme) -> (Vec<u8>, String) {
    use sp_core::Pair;
    match scheme {
        KeyScheme::Sr25519 => {
            let (pair, seed) = sp_core::sr25519::Pair::generate();
            (pair.public().0.to_vec(), format!("0x{}", hex::encode(seed)))
        }
        KeyScheme::Ed25519 => {
            let (pair, seed) = sp_core::ed25519::Pair::generate();
            (pair.public().0.to_vec(), format!("0x{}", hex::encode(seed)))
        }
    }
}

/// Render a 32-byte public key as SS58 (prefix 42, the default the SDK's
/// `from_ss58check` reads and the one `X3_PRODUCTION_AUTHORITIES` expects).
fn public_to_ss58(public: &[u8]) -> Result<String, String> {
    if public.len() != 32 {
        return Err(format!("public key must be 32 bytes, got {}", public.len()));
    }
    use sp_core::crypto::Ss58Codec;
    let mut raw = [0u8; 32];
    raw.copy_from_slice(public);
    Ok(sp_runtime::AccountId32::from(raw).to_ss58check())
}

/// Parse `--public` the way the other commands accept accounts: SS58 or hex.
fn parse_public_key(value: &str) -> Result<Vec<u8>, String> {
    if let Some(hex_part) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        let bytes = hex::decode(hex_part).map_err(|e| format!("invalid hex public key: {e}"))?;
        if bytes.len() != 32 {
            return Err(format!(
                "hex public key must be 32 bytes, got {}",
                bytes.len()
            ));
        }
        return Ok(bytes);
    }
    use sp_core::crypto::Ss58Codec;
    let account = sp_runtime::AccountId32::from_ss58check(value)
        .map_err(|e| format!("invalid SS58 public key: {e}"))?;
    Ok((account.as_ref() as &[u8]).to_vec())
}

/// Where a `keys insert`/`keys list` writes when `--keystore-path` is absent.
///
/// This is the path a running node uses for the same `--base-path`/`--chain`:
/// `<base-path>/chains/<chain-id>/keystore` (see `BasePath::config_dir`). The
/// base path defaults to the same project directory the node itself defaults to,
/// so the operator does not have to reproduce it by hand.
fn default_keystore_path(
    base_path: &Option<std::path::PathBuf>,
    chain: &Option<String>,
) -> Result<std::path::PathBuf, String> {
    use sc_service::config::BasePath;
    // Same default the node itself computes: a `BasePath` built from the
    // executable name, then `chains/<chain-id>` inside it.
    let base = match base_path {
        Some(p) => BasePath::new(p.clone()),
        None => BasePath::from_project("", "", "x3-chain-node"),
    };
    let chain_id = chain.clone().unwrap_or_else(|| "dev".to_string());
    let spec = crate::chain_spec::load_spec(&chain_id)?;
    Ok(base.config_dir(spec.id()).join("keystore"))
}

/// Entry point that runs the CLI and dispatches the requested command.
pub fn run() -> CliResult<()> {
    // Initialize colorful logger with emojis
    logging::init();
    let cli = Cli::parse();

    match &cli.subcommand {
        Some(Commands::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `build-spec`: {e}");
                e
            })?;

            runner.sync_run(|config| {
                info!("Building X3 Chain chain specification (raw: {})", cmd.raw);
                cmd.run(config.chain_spec, config.network).map_err(|e| {
                    error!("`build-spec` command failed: {e}");
                    e
                })
            })
        }
        Some(Commands::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `check-block`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Checking blocks with the current runtime logic");
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `check-block`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = partial;

                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Commands::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `export-blocks`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Exporting blocks to file");
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `export-blocks`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    ..
                } = partial;

                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Commands::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `export-state`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Exporting full runtime state snapshot");
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `export-state`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    ..
                } = partial;

                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Commands::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `import-blocks`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Importing blocks into the local database");
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `import-blocks`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = partial;

                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Commands::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `purge-chain`: {e}");
                e
            })?;

            runner.sync_run(|config| {
                info!("Purging local database for X3 Chain");
                cmd.run(config.database).map_err(|e| {
                    error!("`purge-chain` command failed: {e}");
                    e
                })
            })
        }
        Some(Commands::Revert(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `revert`: {e}");
                e
            })?;

            runner.async_run(|config| {
                info!("Reverting chain state by {:?} blocks", cmd.num);
                let partial = service::new_partial(&config).map_err(|e| {
                    error!("Unable to build partial components for `revert`: {e}");
                    CliError::Service(e)
                })?;

                let sc_service::PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = partial;

                Ok((cmd.run(client, backend, None), task_manager))
            })
        }
        #[cfg(feature = "runtime-benchmarks")]
        Some(Commands::Benchmark(cmd)) => {
            let cmd = cmd.as_ref();
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `benchmark`: {e}");
                e
            })?;

            runner.sync_run(|config| {
                info!("Executing runtime benchmarks");
                match cmd {
                    BenchmarkCmd::Pallet(cmd) => {
                        if !cfg!(feature = "runtime-benchmarks") {
                            return Err(
                                "Runtime benchmarking wasn't enabled when building the node. \
                                You can enable it with `--features runtime-benchmarks`."
                                    .into(),
                            );
                        }
                        cmd.run_with_spec::<sp_runtime::traits::HashingFor<Block>, sp_io::SubstrateHostFunctions>(Some(config.chain_spec))
                    }
                    BenchmarkCmd::Block(cmd) => {
                        let partial = service::new_partial(&config).map_err(|e| {
                            error!("Unable to build partial components for `benchmark block`: {e}");
                            CliError::Service(e)
                        })?;
                        let sc_service::PartialComponents { client, .. } = partial;
                        cmd.run(client)
                    }
                    BenchmarkCmd::Storage(cmd) => {
                        let partial = service::new_partial(&config).map_err(|e| {
                            error!(
                                "Unable to build partial components for `benchmark storage`: {e}"
                            );
                            CliError::Service(e)
                        })?;
                        let sc_service::PartialComponents {
                            client, backend, ..
                        } = partial;
                        let db = backend.expose_db();
                        let storage = backend.expose_storage();
                        let shared_trie_cache = backend.expose_shared_trie_cache();
                        cmd.run(config, client, db, storage, shared_trie_cache)
                    }
                    BenchmarkCmd::Machine(cmd) => {
                        cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
                    }
                    BenchmarkCmd::Overhead(_) | BenchmarkCmd::Extrinsic(_) => Err(
                        "Overhead/Extrinsic benchmarking is not wired for x3-chain-node yet."
                            .into(),
                    ),
                }
                .map_err(|e| {
                    error!("`benchmark` command failed: {e}");
                    e
                })
            })
        }
        #[cfg(feature = "try-runtime")]
        Some(Commands::TryRuntime) => {
            println!("try-runtime CLI was removed in polkadot-sdk stable2512.");
            println!("Use Chopsticks for runtime upgrade testing:");
            println!("  ./scripts/run-chopsticks.sh upgrade");
            Ok(())
        }
        #[cfg(not(feature = "try-runtime"))]
        Some(Commands::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
            You can enable it with `--features try-runtime`."
            .into()),
        Some(Commands::AtomicSwap(cmd)) => {
            match &cmd.command {
                AtomicSwapSubcommand::Simulate {
                    token_in,
                    token_out,
                    amount,
                    slippage_bps,
                    rpc_url,
                } => {
                    info!("Simulating atomic swap trade...");
                    info!("  Token In:  {:?}", token_in);
                    info!("  Token Out: {:?}", token_out);
                    info!("  Amount:    {}", amount);
                    info!("  Slippage:  {} bps", slippage_bps);
                    info!("  RPC URL:   {}", rpc_url);

                    println!("\n=== Atomic Swap Simulation ===");
                    println!("Token In:     0x{}", hex::encode(token_in.as_bytes()));
                    println!("Token Out:    0x{}", hex::encode(token_out.as_bytes()));
                    println!("Amount In:    {}", amount);
                    println!(
                        "Slippage:     {} bps ({}%)",
                        slippage_bps,
                        *slippage_bps as f64 / 100.0
                    );
                    println!();

                    // Make RPC call to atomicTrade_simulate
                    match make_rpc_call(
                        rpc_url,
                        "atomicTrade_simulate",
                        serde_json::json!([
                            format!("0x{}", hex::encode(token_in.as_bytes())),
                            format!("0x{}", hex::encode(token_out.as_bytes())),
                            amount.to_string(),
                            slippage_bps
                        ]),
                    ) {
                        Ok(result) => {
                            println!("--- Simulation Result ---");
                            if let Some(obj) = result.as_object() {
                                println!(
                                    "Success:           {}",
                                    obj.get("success")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false)
                                );
                                println!(
                                    "Estimated Output:  {}",
                                    obj.get("estimatedOutput")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("0")
                                );
                                println!(
                                    "Price Impact:      {} bps",
                                    obj.get("priceImpactBps")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0)
                                );
                                println!(
                                    "EVM Gas:           {}",
                                    obj.get("evmGas").and_then(|v| v.as_u64()).unwrap_or(0)
                                );
                                println!(
                                    "SVM Compute:       {}",
                                    obj.get("svmCompute").and_then(|v| v.as_u64()).unwrap_or(0)
                                );
                            } else {
                                println!("Raw result: {}", result);
                            }
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Simulation Failed ---");
                            println!("RPC endpoint unavailable or returned an error.");
                            println!("No mock output is emitted in shipping builds.");
                            println!("Start a node and retry to get live simulation results.");
                            return Err(format!(
                                "atomicTrade_simulate failed against {}: {}",
                                rpc_url, e
                            )
                            .into());
                        }
                    }

                    Ok(())
                }
                AtomicSwapSubcommand::Price {
                    token_a,
                    token_b,
                    rpc_url,
                } => {
                    info!("Querying price data for token pair...");

                    println!("\n=== Price Data Query ===");
                    println!("Token A:  0x{}", hex::encode(token_a.as_bytes()));
                    println!("Token B:  0x{}", hex::encode(token_b.as_bytes()));
                    println!("RPC URL:  {}", rpc_url);
                    println!();

                    // Make RPC call to atomicTrade_getPriceData
                    match make_rpc_call(
                        rpc_url,
                        "atomicTrade_getPriceData",
                        serde_json::json!([
                            format!("0x{}", hex::encode(token_a.as_bytes())),
                            format!("0x{}", hex::encode(token_b.as_bytes()))
                        ]),
                    ) {
                        Ok(result) => {
                            println!("--- Price Data ---");
                            if let Some(obj) = result.as_object() {
                                println!(
                                    "TWAP Price:        {}",
                                    obj.get("twapPrice").and_then(|v| v.as_str()).unwrap_or("0")
                                );
                                println!(
                                    "Latest Price:      {}",
                                    obj.get("latestPrice")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("0")
                                );
                                println!(
                                    "Observations:      {}",
                                    obj.get("observationCount")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0)
                                );
                                println!(
                                    "Last Updated:      {}",
                                    obj.get("lastUpdated")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("N/A")
                                );
                            } else {
                                println!("Raw result: {}", result);
                            }
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Price Data Query Failed ---");
                            println!("RPC endpoint unavailable or returned an error.");
                            println!("No mock output is emitted in shipping builds.");
                            println!("Start a node and submit observations before retrying.");
                            return Err(format!(
                                "atomicTrade_getPriceData failed against {}: {}",
                                rpc_url, e
                            )
                            .into());
                        }
                    }

                    Ok(())
                }
                AtomicSwapSubcommand::EstimateCost { legs, vm_types } => {
                    info!("Estimating execution costs...");

                    println!("\n=== Execution Cost Estimate ===");
                    println!("Trade Legs: {}", legs);
                    println!("VM Types:   {:?}", vm_types);
                    println!();

                    let mut evm_gas: u64 = 0;
                    let mut svm_compute: u64 = 0;

                    for (i, vm_type) in vm_types.iter().enumerate() {
                        match vm_type.to_lowercase().as_str() {
                            "evm" => {
                                evm_gas += 150_000;
                                println!("  Leg {}: EVM      +150,000 gas", i + 1);
                            }
                            "svm" => {
                                svm_compute += 200_000;
                                println!("  Leg {}: SVM      +200,000 compute units", i + 1);
                            }
                            "crossvm" => {
                                evm_gas += 200_000;
                                svm_compute += 250_000;
                                println!(
                                    "  Leg {}: CrossVM  +200,000 gas, +250,000 compute",
                                    i + 1
                                );
                            }
                            other => {
                                warn!("Unknown VM type '{}', skipping", other);
                            }
                        }
                    }

                    println!();
                    println!("--- Total Estimates ---");
                    println!("EVM Gas:         {}", evm_gas);
                    println!("SVM Compute:     {}", svm_compute);

                    // Rough cost estimates (assuming 20 gwei gas price, $3000 ETH)
                    let evm_cost_usd = evm_gas as f64 * 20.0 * 1e-9 * 3000.0;
                    println!(
                        "Est. EVM Cost:   ${:.4} (at 20 gwei, $3000/ETH)",
                        evm_cost_usd
                    );

                    Ok(())
                }
            }
        }
        Some(Commands::Comit(cmd)) => {
            match &cmd.command {
                ComitSubcommand::Query { comit_id, rpc_url } => {
                    info!("Querying Comit transaction...");
                    info!("  Comit ID: {:?}", comit_id);
                    info!("  RPC URL:  {}", rpc_url);

                    println!("\n=== Comit Transaction Query ===");
                    println!("Comit ID:  0x{}", hex::encode(comit_id.as_bytes()));
                    println!("RPC URL:   {}", rpc_url);
                    println!();

                    // There is no Comit status endpoint to query: the node
                    // registers none, and this used to send Alice's balance
                    // query under the old `atlasKernel_` name and print whatever
                    // came back as "Comit Status". Say what is true instead.
                    println!("--- Comit Status ---");
                    println!("This node exposes no Comit status query over RPC.");
                    println!("  Comit id: 0x{}", hex::encode(comit_id.as_bytes()));
                    println!("  RPC URL:  {}", rpc_url);
                    println!();
                    println!("Use the runtime's own view of the operation:");
                    println!("  x3-chain-node inspect account --account <the comit account>");

                    Ok(())
                }
                ComitSubcommand::Balance {
                    account,
                    asset_id,
                    rpc_url,
                } => {
                    info!("Querying canonical balance...");
                    info!("  Account:  {}", account);
                    info!("  Asset ID: {}", asset_id);
                    info!("  RPC URL:  {}", rpc_url);

                    println!("\n=== Canonical Balance Query ===");
                    println!("Account:   {}", account);
                    println!("Asset ID:  {}", asset_id);
                    println!("RPC URL:   {}", rpc_url);
                    println!();

                    // `x3_getCanonicalBalance` is the registered method, and it
                    // takes the account as 32 bytes of hex.
                    let account_hex = match account_to_hex32(account) {
                        Ok(hex) => hex,
                        Err(e) => {
                            println!("--- Canonical Balance Query Failed ---");
                            println!("Error: {e}");
                            return Ok(());
                        }
                    };
                    match make_rpc_call(
                        rpc_url,
                        "x3_getCanonicalBalance",
                        serde_json::json!([account_hex, asset_id]),
                    ) {
                        Ok(result) => {
                            println!("--- Balance ---");
                            println!("Balance: {}", result);
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Balance Query Failed ---");
                            println!("Error: {}", e);
                            println!();
                            println!("Note: Ensure a node is running on {}", rpc_url);
                        }
                    }

                    Ok(())
                }
                ComitSubcommand::Authorized { rpc_url } => {
                    info!("Querying authorized accounts...");
                    info!("  RPC URL: {}", rpc_url);

                    println!("\n=== Authorized Accounts ===");
                    println!("RPC URL:  {}", rpc_url);
                    println!();

                    // The same registered method as `inspect authorities`; the
                    // authorized set is the second field it returns.
                    match make_rpc_call(rpc_url, "x3_getKernelBridgeState", serde_json::json!([])) {
                        Ok(result) => {
                            println!("--- Authorized Accounts ---");
                            if let Some(arr) =
                                result.get("authorized_accounts").and_then(|v| v.as_array())
                            {
                                if arr.is_empty() {
                                    println!("No authorized accounts found.");
                                } else {
                                    for (i, account) in arr.iter().enumerate() {
                                        println!("{}. {}", i + 1, account);
                                    }
                                }
                            } else {
                                println!("Result: {}", result);
                            }
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Authorized Query Failed ---");
                            println!("Error: {}", e);
                            println!();
                            println!("Note: Ensure a node is running on {}", rpc_url);
                        }
                    }

                    Ok(())
                }
            }
        }
        Some(Commands::Keys(cmd)) => {
            match &cmd.command {
                KeysSubcommand::Generate {
                    key_type,
                    seed,
                    output,
                } => {
                    let spec = resolve_key_type(key_type)?;

                    // With `--seed` the operator already holds the secret, so the
                    // output is the public key only. Without it a fresh keypair is
                    // generated here and the secret is printed, because a key the
                    // operator cannot save is a key the operator cannot use.
                    let (public, generated_secret) = match seed {
                        Some(suri) => (public_from_suri(spec.scheme, suri)?, None),
                        None => {
                            let (public, secret) = generate_keypair(spec.scheme);
                            (public, Some(secret))
                        }
                    };
                    let ss58 = public_to_ss58(&public)?;

                    match output.to_ascii_lowercase().as_str() {
                        "ss58" => {
                            println!("{ss58}");
                        }
                        "hex" => {
                            println!("0x{}", hex::encode(&public));
                        }
                        "json" => {
                            let scheme = match spec.scheme {
                                KeyScheme::Sr25519 => "sr25519",
                                KeyScheme::Ed25519 => "ed25519",
                            };
                            let mut obj = serde_json::json!({
                                "keyType": key_type,
                                "scheme": scheme,
                                "publicKey": format!("0x{}", hex::encode(&public)),
                                "ss58Address": ss58,
                            });
                            if let Some(secret) = &generated_secret {
                                obj["secretSeed"] = serde_json::Value::String(secret.clone());
                            }
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&obj)
                                    .map_err(|e| format!("could not render JSON: {e}"))?
                            );
                        }
                        other => {
                            return Err(format!(
                                "unknown --output '{other}': expected ss58, hex or json"
                            )
                            .into())
                        }
                    }

                    if let Some(secret) = generated_secret {
                        // stderr, so `--output json` stays machine-readable on stdout.
                        eprintln!();
                        eprintln!("SAVE THIS SECRET — it is not recoverable and is not stored:");
                        eprintln!("  {secret}");
                    }
                    eprintln!();
                    eprintln!("SS58 address (use this in X3_PRODUCTION_AUTHORITIES): {ss58}");
                    Ok(())
                }
                KeysSubcommand::Insert {
                    key_type,
                    seed,
                    keystore_path,
                } => {
                    let spec = resolve_key_type(key_type)?;
                    let path = match keystore_path {
                        Some(p) => p.clone(),
                        None => default_keystore_path(
                            &cli.run.shared_params.base_path,
                            &cli.run.shared_params.chain,
                        )?,
                    };
                    let public = public_from_suri(spec.scheme, seed)?;
                    let keystore = sc_keystore::LocalKeystore::open(path.clone(), None)
                        .map_err(|e| format!("could not open keystore {path:?}: {e}"))?;
                    use sp_keystore::Keystore as _;
                    keystore.insert(spec.key_type, seed, &public).map_err(|_| {
                        format!(
                            "the keystore refused this {key_type} key: the secret URI is \
                                 not valid for the {} scheme (use `keys verify` to check it)",
                            match spec.scheme {
                                KeyScheme::Sr25519 => "sr25519",
                                KeyScheme::Ed25519 => "ed25519",
                            }
                        )
                    })?;
                    println!("{}", public_to_ss58(&public)?);
                    eprintln!("inserted {} key into {}", key_type, path.display());
                    Ok(())
                }
                KeysSubcommand::List { keystore_path } => {
                    let path = match keystore_path {
                        Some(p) => p.clone(),
                        None => default_keystore_path(
                            &cli.run.shared_params.base_path,
                            &cli.run.shared_params.chain,
                        )?,
                    };
                    let keystore = sc_keystore::LocalKeystore::open(path.clone(), None)
                        .map_err(|e| format!("could not open keystore {path:?}: {e}"))?;
                    use sp_keystore::Keystore as _;
                    println!("keystore: {}", path.display());
                    let mut total = 0usize;
                    for name in ["aura", "grandpa", "imonline"] {
                        let spec = resolve_key_type(name)?;
                        let keys = keystore
                            .keys(spec.key_type)
                            .map_err(|e| format!("could not read {name} keys: {e}"))?;
                        for key in &keys {
                            println!("{name}: {}", public_to_ss58(key)?);
                        }
                        total += keys.len();
                    }
                    if total == 0 {
                        println!("(no keys in this keystore)");
                    }
                    Ok(())
                }
                KeysSubcommand::Verify {
                    key_type,
                    public,
                    seed,
                } => {
                    let spec = resolve_key_type(key_type)?;
                    let derived = public_from_suri(spec.scheme, seed)?;
                    let claimed = parse_public_key(public)?;
                    if derived == claimed {
                        println!("MATCH {key_type} {}", public_to_ss58(&derived)?);
                    } else {
                        // A mismatch is the answer this command exists to give, so it
                        // exits non-zero: a shell `keys verify || die` must fail.
                        eprintln!(
                            "MISMATCH\n  --public: {}\n  derived:  {}",
                            public_to_ss58(&claimed)?,
                            public_to_ss58(&derived)?
                        );
                        return Err(format!(
                            "the secret does not derive the supplied {key_type} public key"
                        )
                        .into());
                    }
                    Ok(())
                }
            }
        }
        Some(Commands::Validator(cmd)) => match &cmd.command {
            ValidatorSubcommand::Rotate {
                rpc_url,
                suri,
                aura_seed,
                grandpa_seed,
                submit,
            } => run_validator_rotate(rpc_url, suri, aura_seed, grandpa_seed, *submit),
            ValidatorSubcommand::Register {
                rpc_url,
                suri,
                second,
                account,
                due_at,
                submit,
            } => run_validator_register(rpc_url, suri, second, account, *due_at, *submit),
        },
        Some(Commands::Inspect(cmd)) => {
            match &cmd.command {
                InspectSubcommand::Account {
                    account,
                    rpc_url,
                    output,
                } => {
                    info!("Inspecting account...");
                    info!("  Account: {}", account);
                    info!("  RPC URL: {}", rpc_url);
                    info!("  Output:  {}", output);

                    println!("\n=== Account Inspection ===");
                    println!("Account:   {}", account);
                    println!("RPC URL:   {}", rpc_url);
                    println!("Output:    {}", output);
                    println!();

                    // The registered method, with the account the node can
                    // decode: `--account` documents "SS58 or hex", and this used
                    // to pass the user's string through as if it were hex.
                    let account_hex = match account_to_hex32(account) {
                        Ok(hex) => hex,
                        Err(e) => {
                            println!("--- Account Inspection Failed ---");
                            println!("Error: {e}");
                            return Err(query_failed("account inspection", rpc_url, e));
                        }
                    };
                    match make_rpc_call(
                        rpc_url,
                        "x3_getCanonicalBalance",
                        serde_json::json!([account_hex, 0]),
                    ) {
                        Ok(result) => {
                            println!("--- Account Balances ---");
                            let balance = result
                                .get("balance")
                                .and_then(|v| v.as_str())
                                .unwrap_or("<no balance field>");
                            println!("Native X3 (Asset 0): {}", balance);

                            // The native balance above is the one this command has
                            // always asked for. Everything else the account holds in
                            // another asset was previously left to a note about
                            // enumeration — so the per-asset balances are queried here
                            // for every asset the registry actually returns.
                            match scan_asset_metadata(rpc_url) {
                                Ok(assets) => {
                                    let others: Vec<_> = assets
                                        .iter()
                                        .filter(|meta| {
                                            meta.get("asset_id").and_then(|v| v.as_u64()) != Some(0)
                                        })
                                        .collect();
                                    if others.is_empty() {
                                        println!(
                                            "No other asset ids returned metadata in {ASSET_SCAN_MIN}..={ASSET_SCAN_MAX}."
                                        );
                                    } else {
                                        println!(
                                            "--- Balances for registered assets ({ASSET_SCAN_MIN}..={ASSET_SCAN_MAX}) ---"
                                        );
                                        for meta in others {
                                            let id = meta
                                                .get("asset_id")
                                                .and_then(|v| v.as_u64())
                                                .unwrap_or(0);
                                            let symbol = meta
                                                .get("symbol")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("<no symbol>");
                                            match make_rpc_call(
                                                rpc_url,
                                                "x3_getCanonicalBalance",
                                                serde_json::json!([account_hex, id]),
                                            ) {
                                                Ok(balance) => {
                                                    let value = balance
                                                        .get("balance")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("<no balance field>");
                                                    println!("  {id} {symbol}: {value}");
                                                }
                                                Err(e) => {
                                                    println!("  {id} {symbol}: <query failed: {e}>")
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!();
                                    println!("Note: could not enumerate assets ({e}).");
                                }
                            }
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Account Inspection Failed ---");
                            println!("Error: {}", e);
                            println!();
                            println!("Note: Ensure a node is running on {}", rpc_url);
                            return Err(query_failed("account inspection", rpc_url, e));
                        }
                    }

                    Ok(())
                }
                InspectSubcommand::Asset { asset_id, rpc_url } => {
                    info!("Inspecting asset...");
                    info!("  Asset ID: {}", asset_id);
                    info!("  RPC URL:  {}", rpc_url);

                    println!("\n=== Asset Inspection ===");
                    println!("Asset ID:  {}", asset_id);
                    println!("RPC URL:   {}", rpc_url);
                    println!();

                    // Registered by the node for this command; the runtime's
                    // `get_asset_metadata` answers it. The CLI used to ask for
                    // `atlasKernel_getAssetMetadata`, which no node serves.
                    match make_rpc_call(
                        rpc_url,
                        "x3_getAssetMetadata",
                        serde_json::json!([asset_id]),
                    ) {
                        Ok(result) => {
                            println!("--- Asset Metadata ---");
                            if let Some(obj) = result.as_object() {
                                println!(
                                    "Symbol:   {}",
                                    obj.get("symbol").and_then(|v| v.as_str()).unwrap_or("N/A")
                                );
                                println!(
                                    "Decimals: {}",
                                    obj.get("decimals").and_then(|v| v.as_u64()).unwrap_or(0)
                                );
                            } else {
                                println!("Result: {}", result);
                            }
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Asset Inspection Failed ---");
                            println!("Error: {}", e);
                            println!();
                            println!("Note: Ensure a node is running on {}", rpc_url);
                            return Err(query_failed("asset inspection", rpc_url, e));
                        }
                    }

                    Ok(())
                }
                InspectSubcommand::Assets { rpc_url, output } => {
                    info!("Listing all assets...");
                    info!("  RPC URL: {}", rpc_url);
                    info!("  Output:  {}", output);

                    println!("\n=== Asset Registry ===");
                    println!("RPC URL:  {}", rpc_url);
                    println!("Output:   {}", output);
                    println!();

                    // This used to print a hardcoded list — "0: X3 (native token,
                    // 12 decimals) / 1: ETH / 2: SOL / 3: USDC" — under the heading
                    // "Registered Assets", with the note that enumeration was not
                    // supported. An operator reading that cannot tell chain state
                    // from a comment in the binary. The runtime has no "list all
                    // assets" API, so the honest query is `x3_getAssetMetadata` over
                    // a bounded id range, printing only what the chain answers.
                    println!(
                        "--- Registered Assets (x3_getAssetMetadata, ids {ASSET_SCAN_MIN}..={ASSET_SCAN_MAX}) ---"
                    );
                    let mut assets: Vec<serde_json::Value> = Vec::new();
                    match scan_asset_metadata(rpc_url) {
                        Ok(found) => {
                            for meta in &found {
                                let id = meta.get("asset_id").and_then(|v| v.as_u64()).unwrap_or(0);
                                let symbol = meta
                                    .get("symbol")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("<no symbol>");
                                let decimals =
                                    meta.get("decimals").and_then(|v| v.as_u64()).unwrap_or(0);
                                println!("  {id}: {symbol} ({decimals} decimals)");
                                assets.push(meta.clone());
                            }
                            if found.is_empty() {
                                // Not an error: a chain with nothing registered answers
                                // `null` for every id, and that is the truthful answer.
                                println!("  (the chain returned no asset metadata in that range)");
                            }
                        }
                        Err(e) => {
                            println!("--- Asset Query Failed ---");
                            println!("Error: {e}");
                            println!();
                            println!("Note: Ensure a node is running on {rpc_url}");
                            return Err(query_failed("asset enumeration", rpc_url, e));
                        }
                    }

                    if output.eq_ignore_ascii_case("json") {
                        let payload = serde_json::json!({
                            "scanned_ids": { "from": ASSET_SCAN_MIN, "to": ASSET_SCAN_MAX },
                            "assets": assets,
                        });
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&payload)
                                .map_err(|e| format!("could not render JSON: {e}"))?
                        );
                    }
                    Ok(())
                }
                InspectSubcommand::Authorities { rpc_url } => {
                    info!("Querying authority set...");
                    info!("  RPC URL: {}", rpc_url);

                    println!("\n=== Authority Set ===");
                    println!("RPC URL:  {}", rpc_url);
                    println!();

                    // `x3_getKernelBridgeState` is the method the node
                    // registers; it carries the authority set and the authorized
                    // accounts. This used to call `atlasKernel_getAuthorities`,
                    // a name from before the kernel rename, so every node
                    // answered "Method not found".
                    match make_rpc_call(rpc_url, "x3_getKernelBridgeState", serde_json::json!([])) {
                        Ok(result) => {
                            println!("--- Current Authorities ---");
                            if let Some(arr) = result.get("authorities").and_then(|v| v.as_array())
                            {
                                if arr.is_empty() {
                                    println!("No authorities found.");
                                } else {
                                    for (i, authority) in arr.iter().enumerate() {
                                        println!("{}. {}", i + 1, authority);
                                    }
                                    println!();
                                    println!("Total: {} authorities", arr.len());
                                }
                            } else {
                                println!("Result: {}", result);
                            }
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Authorities Query Failed ---");
                            println!("Error: {}", e);
                            println!();
                            println!("Note: Ensure a node is running on {}", rpc_url);
                        }
                    }

                    Ok(())
                }
                InspectSubcommand::ChainInfo { rpc_url } => {
                    info!("Querying chain information...");
                    info!("  RPC URL: {}", rpc_url);

                    println!("\n=== Chain Information ===");
                    println!("RPC URL:  {}", rpc_url);
                    println!();

                    // Make RPC call to get block number
                    match make_rpc_call(rpc_url, "eth_blockNumber", serde_json::json!([])) {
                        Ok(block_number) => {
                            println!("--- Chain Status ---");
                            println!("Block Number: {}", block_number);
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Chain Info Failed ---");
                            println!("Error: {}", e);
                        }
                    }

                    // Make RPC call to get chain ID
                    match make_rpc_call(rpc_url, "eth_chainId", serde_json::json!([])) {
                        Ok(chain_id) => {
                            println!("Chain ID:     {}", chain_id);
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                        }
                    }

                    println!();
                    println!("Note: Ensure a node is running on {}", rpc_url);

                    Ok(())
                }
            }
        }
        None => {
            let runner = cli.create_runner(&cli.run).map_err(|e| {
                error!("Failed to initialize runner for node execution: {e}");
                e
            })?;
            let feature_flags = service::NodeFeatureFlags {
                enable_parallel_proposer: cli.features.enable_parallel_proposer,
                enable_flash_finality: cli.features.enable_flash_finality,
                enable_poh: cli.features.enable_poh,
                enable_atomic_kernel: cli.features.enable_atomic_kernel,
                gpu_required: cli.features.gpu_required,
                enable_gpu_validator: cli.features.enable_gpu_validator,
            };

            runner.run_node_until_exit(|config| async move {
                let role = config.role;
                info!("Starting X3 Chain node as {:?}", role);
                service::new_full_with_atomic_gateway::<sc_network::NetworkWorker<_, _>>(
                    config,
                    feature_flags,
                    cli.features.atomic_gateway_uri.clone(),
                )
                .map_err(|e| {
                    error!("X3 Chain node terminated with an error: {e}");
                    CliError::Service(e)
                })
            })
        }
    }
}

/// Make an HTTP JSON-RPC call to a running node.
///
/// Returns the result field from the JSON-RPC response, or an error if the call fails.
fn make_rpc_call(
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    // Parse URL to extract host, port, and path
    let url = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let (host_port, path) = if let Some(idx) = url.find('/') {
        (&url[..idx], &url[idx..])
    } else {
        (url, "/")
    };

    let (host, port) = if let Some(idx) = host_port.find(':') {
        (
            &host_port[..idx],
            host_port[idx + 1..].parse::<u16>().unwrap_or(9944),
        )
    } else {
        (host_port, 9944u16)
    };

    // Build JSON-RPC request
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });
    let body = request_body.to_string();

    // Build HTTP request
    let http_request = format!(
        "POST {} HTTP/1.1\r\n\
         Host: {}:{}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        path,
        host,
        port,
        body.len(),
        body
    );

    // Connect and send request
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect_timeout(
        &addr
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?,
        Duration::from_secs(5),
    )
    .map_err(|e| format!("Connection failed: {}", e))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| format!("Failed to set timeout: {}", e))?;

    stream
        .write_all(http_request.as_bytes())
        .map_err(|e| format!("Failed to send request: {}", e))?;

    // Read response
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Parse HTTP response - find JSON body after headers
    let body_start = response
        .find("\r\n\r\n")
        .ok_or("Invalid HTTP response: no body separator")?;
    let json_body = &response[body_start + 4..];

    // Parse JSON-RPC response
    let rpc_response: serde_json::Value =
        serde_json::from_str(json_body).map_err(|e| format!("Invalid JSON response: {}", e))?;

    // Check for error
    if let Some(error) = rpc_response.get("error") {
        return Err(format!(
            "RPC error: {}",
            error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
        ));
    }

    // Return result
    rpc_response
        .get("result")
        .cloned()
        .ok_or_else(|| "No result in response".to_string())
}

/// `validator register`: carry `x3_custody::register_validator_key` through the council.
///
/// The call takes the governance origin. There is no root path on a real chain and `Sudo`
/// has no key on the dev one, so the collective origin is the only route: one council member
/// proposes, another carries it over the threshold, and the motion executes the call.
fn run_validator_register(
    rpc_url: &str,
    proposer_suri: &str,
    second_suri: &str,
    account_ss58: &Option<String>,
    due_at: Option<u32>,
    submit: bool,
) -> CliResult<()> {
    use sp_core::crypto::Ss58Codec;
    use x3_chain_runtime::{AccountId, CustodyKeyRotationPeriod};

    let proposer = crate::validator_rotation::OperatorKey::from_uri(proposer_suri)?;
    let second = crate::validator_rotation::OperatorKey::from_uri(second_suri)?;
    let account = match account_ss58 {
        Some(ss58) => {
            AccountId::from_ss58check(ss58).map_err(|e| format!("invalid account {ss58}: {e:?}"))?
        }
        None => proposer.account(),
    };
    let account_ss58 = account.to_ss58check();

    let genesis_hash = decode_h256(&make_rpc_call(
        rpc_url,
        "chain_getBlockHash",
        serde_json::json!([0]),
    )?)?;
    let current_block = current_block_number(rpc_url)?;
    // Read both nonces from chain state, not from the pool's view of them.
    let proposer_nonce = on_chain_nonce(rpc_url, &proposer.account())?;
    let second_nonce = on_chain_nonce(rpc_url, &second.account())?;
    let due_at = due_at.unwrap_or_else(|| {
        current_block.saturating_add(<CustodyKeyRotationPeriod as frame_support::traits::Get<
            x3_chain_runtime::BlockNumber,
        >>::get())
    });

    // The vote has to name the motion's index, which is the council's proposal count *before*
    // this proposal is inserted.
    let proposal_count_raw = make_rpc_call(
        rpc_url,
        "state_getStorage",
        serde_json::json!([format!(
            "0x{}",
            hex::encode(crate::validator_rotation::council_proposal_count_storage_key())
        )]),
    )?;
    let index: u32 = decode_storage_hex(&proposal_count_raw)?.unwrap_or(0);

    let custody_call = crate::validator_rotation::register_validator_call(account.clone(), due_at);
    let length_bound = crate::validator_rotation::call_length_bound(&custody_call)?;
    let (propose, proposal_hash) =
        proposer.council_propose(custody_call, 2, genesis_hash, proposer_nonce)?;

    println!("proposer:      {}", proposer.account().to_ss58check());
    println!("second:        {}", second.account().to_ss58check());
    println!("validator:     {account_ss58}");
    println!("current block: {current_block}");
    println!("register due:  {due_at}");
    println!(
        "proposal:      0x{} (index {index})",
        hex::encode(proposal_hash)
    );

    if submit {
        let propose_hash = make_rpc_call(
            rpc_url,
            "author_submitExtrinsic",
            serde_json::json!([format!("0x{}", hex::encode(propose.encode()))]),
        )?;
        println!(
            "proposed:      {}",
            propose_hash.as_str().unwrap_or("<non-string>")
        );

        // Wait for the proposal to be *built into a block* before voting. Sending both at once
        // left one of them stuck in the pool indefinitely (measured 2026-09-26): a vote that
        // arrives while the motion it names is not on chain yet is skipped as invalid on every
        // authoring attempt, and nothing in the output said so. The proposer's account index
        // moving past the nonce this extrinsic was built with is the precise "included" signal.
        wait_for_account_index_to_pass(
            rpc_url,
            &proposer.account().to_ss58check(),
            proposer_nonce,
        )?;

        // `propose` does **not** count as an approval: with a threshold of two the motion needs
        // two `vote` calls, which is what a two-member council is. Measured: one vote left
        // `yes: 1, no: 0` in the block's events and the call never executed.
        let proposer_vote = proposer.council_vote(
            proposal_hash,
            index,
            true,
            genesis_hash,
            proposer_nonce.saturating_add(1),
        )?;
        let vote_hash = make_rpc_call(
            rpc_url,
            "author_submitExtrinsic",
            serde_json::json!([format!("0x{}", hex::encode(proposer_vote.encode()))]),
        )?;
        println!(
            "voted:         {} (proposer)",
            vote_hash.as_str().unwrap_or("<non-string>")
        );
        wait_for_account_index_to_pass(
            rpc_url,
            &proposer.account().to_ss58check(),
            proposer_nonce.saturating_add(1),
        )?;

        let second_vote =
            second.council_vote(proposal_hash, index, true, genesis_hash, second_nonce)?;
        let second_hash = make_rpc_call(
            rpc_url,
            "author_submitExtrinsic",
            serde_json::json!([format!("0x{}", hex::encode(second_vote.encode()))]),
        )?;
        println!(
            "voted:         {} (second)",
            second_hash.as_str().unwrap_or("<non-string>")
        );
        wait_for_account_index_to_pass(rpc_url, &second.account().to_ss58check(), second_nonce)?;

        // A motion at its threshold is not executed until someone closes it: `vote` only records
        // the vote in this pallet version. Without this the call was built, included and
        // dispatched, and nothing happened (measured).
        let close = second.council_close(
            proposal_hash,
            index,
            length_bound,
            genesis_hash,
            second_nonce.saturating_add(1),
        )?;
        let close_hash = make_rpc_call(
            rpc_url,
            "author_submitExtrinsic",
            serde_json::json!([format!("0x{}", hex::encode(close.encode()))]),
        )?;
        println!(
            "closed:        {} (executes the motion)",
            close_hash.as_str().unwrap_or("<non-string>")
        );
        wait_for_account_index_to_pass(
            rpc_url,
            &second.account().to_ss58check(),
            second_nonce.saturating_add(1),
        )?;

        // And report success only once the storage this wrote actually holds the entry — the
        // same storage `rotate` refuses to work without.
        let record: Option<pallet_x3_custody::ValidatorKeyRecord<x3_chain_runtime::BlockNumber>> =
            decode_storage_hex(&make_rpc_call(
                rpc_url,
                "state_getStorage",
                serde_json::json!([format!(
                    "0x{}",
                    hex::encode(
                        crate::validator_rotation::validator_key_registry_storage_key(&account)
                    )
                )]),
            )?)?;
        match record {
            Some(record) if record.active => println!(
                "registered:    true (role {:?}, due at block {})",
                record.role, record.rotation_due_at
            ),
            Some(_) => return Err("the registry entry exists but is inactive".into()),
            None => {
                // The extrinsic was included, so the answer is in the block's events: print the
                // council and system events rather than guessing why the call had no effect.
                print_recent_events(rpc_url)?;
                return Err(
                    "the council motion did not write a registry entry: the call was built, \
                     included and dispatched without effect (events above)"
                        .into(),
                );
            }
        }
    } else {
        // A two-member motion is three extrinsics: one proposal and one vote from each member.
        println!("council propose extrinsic, from the proposer (submit with --submit):");
        println!("0x{}", hex::encode(propose.encode()));
        let proposer_vote = proposer.council_vote(
            proposal_hash,
            index,
            true,
            genesis_hash,
            proposer_nonce.saturating_add(1),
        )?;
        println!("council vote extrinsic, from the proposer:");
        println!("0x{}", hex::encode(proposer_vote.encode()));
        println!("council vote extrinsic, from the second member:");
        println!(
            "0x{}",
            hex::encode(
                second
                    .council_vote(proposal_hash, index, true, genesis_hash, second_nonce)?
                    .encode()
            )
        );
    }

    Ok(())
}

fn run_validator_rotate(
    rpc_url: &str,
    suri: &str,
    aura_seed: &str,
    grandpa_seed: &str,
    submit: bool,
) -> CliResult<()> {
    use sp_core::crypto::Ss58Codec;

    let operator = crate::validator_rotation::OperatorKey::from_uri(suri)?;
    let account = operator.account();
    let account_ss58 = account.to_ss58check();

    // The on-chain custody registry is the single source of truth. A null
    // `ValidatorKeyRegistry` entry means the account is not a registered
    // validator, and rotation must be refused rather than guessed.
    let registry_key = crate::validator_rotation::validator_key_registry_storage_key(&account);
    let registry_raw = make_rpc_call(
        rpc_url,
        "state_getStorage",
        serde_json::json!([format!("0x{}", hex::encode(&registry_key))]),
    )?;

    let record: Option<pallet_x3_custody::ValidatorKeyRecord<x3_chain_runtime::BlockNumber>> =
        decode_storage_hex(&registry_raw)?;
    let record = record.ok_or_else(|| {
        format!("account {account_ss58} is not a registered validator (no custody registry entry)")
    })?;
    if !record.active {
        return Err(format!(
            "account {account_ss58} is registered but its validator key is inactive"
        )
        .into());
    }

    let schedule_key = crate::validator_rotation::key_rotation_schedule_storage_key(&account);
    let schedule_raw = make_rpc_call(
        rpc_url,
        "state_getStorage",
        serde_json::json!([format!("0x{}", hex::encode(&schedule_key))]),
    )?;
    let due_at: Option<x3_chain_runtime::BlockNumber> = decode_storage_hex(&schedule_raw)?;

    let genesis_hash = decode_h256(&make_rpc_call(
        rpc_url,
        "chain_getBlockHash",
        serde_json::json!([0]),
    )?)?;
    let current_block = current_block_number(rpc_url)?;
    let nonce = decode_u32(&make_rpc_call(
        rpc_url,
        "system_accountNextIndex",
        serde_json::json!([account_ss58]),
    )?)?;

    let keys = crate::validator_rotation::session_keys(aura_seed, grandpa_seed)?;
    let aura_ss58 = public_to_ss58(keys.aura.as_ref())?;
    let grandpa_ss58 = public_to_ss58(keys.grandpa.as_ref())?;
    let extrinsic = operator.set_keys(keys, genesis_hash, nonce)?;
    let tx_hex = format!("0x{}", hex::encode(extrinsic.encode()));

    let next_due = current_block.saturating_add(x3_chain_runtime::CustodyKeyRotationPeriod::get());

    println!("operator:      {account_ss58}");
    println!("aura:          {aura_ss58}");
    println!("grandpa:       {grandpa_ss58}");
    println!("current block: {current_block}");
    println!(
        "current due:   {}",
        due_at.map_or("unset".to_string(), |b| b.to_string())
    );
    println!("next due:      {next_due}");

    if submit {
        let tx_hash = make_rpc_call(
            rpc_url,
            "author_submitExtrinsic",
            serde_json::json!([tx_hex]),
        )?;
        println!(
            "submitted:     {}",
            tx_hash.as_str().unwrap_or("<non-string>")
        );
    } else {
        println!("session.set_keys extrinsic (unsigned-encoded, submit with --submit):");
        println!("{tx_hex}");
    }

    Ok(())
}

fn decode_storage_hex<T: codec::Decode>(value: &serde_json::Value) -> Result<Option<T>, String> {
    if value.is_null() {
        return Ok(None);
    }
    let hex_str = value
        .as_str()
        .ok_or_else(|| "state_getStorage returned a non-string value".to_string())?;
    let bytes = decode_hex_bytes(hex_str)?;
    T::decode(&mut &bytes[..])
        .map(Some)
        .map_err(|e| format!("failed to decode storage value: {e}"))
}

fn decode_h256(value: &serde_json::Value) -> Result<sp_core::H256, String> {
    let hex_str = value
        .as_str()
        .ok_or_else(|| "expected a hex string result".to_string())?;
    let bytes = decode_hex_bytes(hex_str)?;
    if bytes.len() != 32 {
        return Err(format!("expected a 32-byte hash, got {}", bytes.len()));
    }
    Ok(sp_core::H256::from_slice(&bytes))
}

fn decode_u32(value: &serde_json::Value) -> Result<u32, String> {
    match value {
        serde_json::Value::Number(n) => n
            .as_u64()
            .map(|v| v as u32)
            .ok_or_else(|| "nonce/index is not an unsigned integer".to_string()),
        serde_json::Value::String(s) => {
            // Substrate encodes a block number as *minimal* hex — `0x2dc`, not `0x02dc` — so
            // half of all block heights have an odd number of digits. Two things went wrong
            // when this was read as SCALE bytes rather than as a number: `hex::decode` refuses
            // an odd digit count ("Odd number of digits"), and `u32::decode` wants exactly four
            // bytes ("Not enough data to fill buffer"), so even `0x5f` failed. Measured on a
            // live dev chain at height 0x2dc on 2026-09-26: `validator rotate` could not read
            // the current block at all.
            let digits = s.trim_start_matches("0x").trim_start_matches("0X");
            let value = u64::from_str_radix(digits, 16)
                .map_err(|e| format!("invalid hex integer {s}: {e}"))?;
            u32::try_from(value).map_err(|_| format!("{s} does not fit in a u32 nonce/index"))
        }
        _ => Err("expected a number or hex string".to_string()),
    }
}

fn current_block_number(rpc_url: &str) -> Result<u32, String> {
    let header = make_rpc_call(rpc_url, "chain_getHeader", serde_json::json!([]))?;
    let number = header
        .get("number")
        .ok_or_else(|| "chain_getHeader returned no block number".to_string())?;
    decode_u32(number)
}

fn decode_hex_bytes(hex_str: &str) -> Result<Vec<u8>, String> {
    let hex_str = hex_str.trim_start_matches("0x").trim_start_matches("0X");
    hex::decode(hex_str).map_err(|e| format!("invalid hex: {e}"))
}

/// Block until `account`'s next index has moved past `used`, i.e. an extrinsic signed with that
/// nonce was built into a block.
///
/// Neither `author_submitExtrinsic` (the pool took it) nor `system_accountNextIndex` (the *pool's*
/// view, which advances the moment the transaction is queued) answers this — using the latter made
/// a stuck transaction look included. The on-chain nonce in `System.Account` moves only when a
/// block actually carries the extrinsic, whether or not its dispatch succeeds.
fn wait_for_account_index_to_pass(rpc_url: &str, account_ss58: &str, used: u32) -> CliResult<()> {
    use sp_core::crypto::Ss58Codec;
    use x3_chain_runtime::AccountId;

    let account =
        AccountId::from_ss58check(account_ss58).map_err(|e| format!("invalid account: {e:?}"))?;
    for _ in 0..120 {
        let next = on_chain_nonce(rpc_url, &account)?;
        if next > used {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Err(format!(
        "the extrinsic signed by {account_ss58} with nonce {used} was queued but never built into \
         a block — it is still sitting in the pool"
    )
    .into())
}

/// The account's on-chain nonce, from the runtime's `AccountNonceApi`.
///
/// Distinct from `system_accountNextIndex`, which answers with the *pool's* view of the next
/// index and therefore advances the moment a transaction is queued.
fn on_chain_nonce(rpc_url: &str, account: &x3_chain_runtime::AccountId) -> CliResult<u32> {
    let raw = make_rpc_call(
        rpc_url,
        "state_call",
        serde_json::json!([
            "AccountNonceApi_account_nonce",
            format!("0x{}", hex::encode(account.encode()))
        ]),
    )?;
    let hex_str = raw
        .as_str()
        .ok_or_else(|| "state_call returned a non-string result".to_string())?;
    let bytes = decode_hex_bytes(hex_str)?;
    <x3_chain_runtime::Index as codec::Decode>::decode(&mut &bytes[..])
        .map_err(|e| format!("failed to decode the account nonce: {e}").into())
}

/// Print the council, custody and failed-extrinsic events of the latest block.
///
/// `author_submitExtrinsic` answering with a hash says the pool took the transaction, and the
/// account index moving says a block carried it — neither says the call *worked*. The dispatch
/// result exists only in the block's events, so an operator debugging a governance call that had
/// no effect needs them printed rather than a bare "nothing happened".
fn print_recent_events(rpc_url: &str) -> CliResult<()> {
    use x3_chain_runtime::RuntimeEvent;

    let mut key = sp_core::hashing::twox_128(b"System").to_vec();
    key.extend_from_slice(&sp_core::hashing::twox_128(b"Events"));
    let key_hex = format!("0x{}", hex::encode(&key));

    // The extrinsics under investigation are a block or two behind the head by the time we look,
    // so scan a window of blocks rather than only the newest one.
    let height = current_block_number(rpc_url)?;
    let mut printed = 0usize;
    for number in height.saturating_sub(12)..=height {
        let Ok(hash) = decode_h256(&make_rpc_call(
            rpc_url,
            "chain_getBlockHash",
            serde_json::json!([number]),
        )?) else {
            continue;
        };
        let raw = make_rpc_call(
            rpc_url,
            "state_getStorage",
            serde_json::json!([key_hex, format!("0x{}", hex::encode(hash))]),
        )?;
        let Some(events) = decode_storage_hex::<
            Vec<frame_system::EventRecord<RuntimeEvent, sp_core::H256>>,
        >(&raw)?
        else {
            continue;
        };
        for record in events {
            let interesting = match &record.event {
                RuntimeEvent::System(frame_system::Event::ExtrinsicFailed {
                    dispatch_error,
                    ..
                }) => Some(format!("ExtrinsicFailed: {dispatch_error:?}")),
                RuntimeEvent::Council(council) => Some(format!("Council::{council:?}")),
                RuntimeEvent::X3Custody(custody) => Some(format!("X3Custody::{custody:?}")),
                _ => None,
            };
            if let Some(text) = interesting {
                let text: String = text.chars().take(400).collect();
                println!("events:        block {number}: {text}");
                printed += 1;
            }
        }
    }
    if printed == 0 {
        println!("events:        no council, custody or failure event in the last 12 blocks");
    }
    Ok(())
}

// ── Asset enumeration ───────────────────────────────────────────────────────
//
// The runtime answers `get_asset_metadata(asset_id)` but exposes no "list every
// asset" call, so the CLI asks for ids `ASSET_SCAN_MIN..=ASSET_SCAN_MAX` and
// prints only what the chain returns. The bound is printed next to the results:
// an id outside it is *not claimed to be absent*, which is what the previous
// hardcoded list got wrong in the other direction.
const ASSET_SCAN_MIN: u32 = 0;
const ASSET_SCAN_MAX: u32 = 31;

/// `x3_getAssetMetadata` answers `null` for an unregistered id. A `null` is
/// data; a malformed object is not silently turned into one.
fn parse_asset_metadata(value: &serde_json::Value) -> Option<serde_json::Value> {
    if value.is_null() {
        return None;
    }
    let asset_id = value.get("asset_id")?.as_u64()?;
    let symbol = value.get("symbol")?.as_str()?;
    let decimals = value.get("decimals")?.as_u64()?;
    Some(serde_json::json!({
        "asset_id": asset_id,
        "symbol": symbol,
        "decimals": decimals,
    }))
}

/// Query every id in the scan range, stopping at the first RPC failure.
///
/// Stopping matters: a node that does not serve `x3_getAssetMetadata` must not
/// produce "no assets registered", which is the same answer a healthy empty
/// registry gives.
fn scan_asset_metadata(rpc_url: &str) -> Result<Vec<serde_json::Value>, String> {
    let mut found = Vec::new();
    for asset_id in ASSET_SCAN_MIN..=ASSET_SCAN_MAX {
        let answer = make_rpc_call(
            rpc_url,
            "x3_getAssetMetadata",
            serde_json::json!([asset_id]),
        )?;
        if let Some(metadata) = parse_asset_metadata(&answer) {
            found.push(metadata);
        }
    }
    Ok(found)
}

/// A read-only query that never answered is a failure, not a result.
///
/// Every `inspect` arm used to print the error and `return Ok(())`, so
/// `x3-chain-node inspect account … || exit 1` succeeded against a node that was
/// down, and against a method the node does not serve. A `null` answer from the
/// chain is different: that is data ("no such asset") and stays a success.
fn query_failed(what: &str, rpc_url: &str, error: impl std::fmt::Display) -> CliError {
    CliError::Input(format!(
        "{what} failed: {error}\nNote: ensure a node is running on {rpc_url}"
    ))
}

#[cfg(test)]
mod asset_scan_tests {
    use super::*;

    /// The node reports a block number as *minimal* hex, so half of all heights have an
    /// odd number of digits. `0x2dc` is the value measured from a live dev chain on
    /// 2026-09-26, and it is what made `validator rotate` fail with
    /// `Input("invalid hex: Odd number of digits")`.
    #[test]
    fn a_minimal_hex_block_number_decodes() {
        assert_eq!(decode_u32(&serde_json::json!("0x2dc")).unwrap(), 0x2dc);
        assert_eq!(decode_u32(&serde_json::json!("0x5f")).unwrap(), 0x5f);
        assert_eq!(decode_u32(&serde_json::json!("0x02dc")).unwrap(), 0x2dc);
        assert_eq!(decode_u32(&serde_json::json!(0)).unwrap(), 0);
        assert_eq!(decode_u32(&serde_json::json!("0x0")).unwrap(), 0);
        assert!(decode_u32(&serde_json::json!("0xzz")).is_err());
        assert!(decode_u32(&serde_json::json!("0x1_0000_0000")).is_err());
    }

    #[test]
    fn asset_metadata_is_taken_from_the_chain_answer() {
        let answer = serde_json::json!({
            "asset_id": 7,
            "symbol": "ATLAS",
            "decimals": 18,
        });
        let parsed = parse_asset_metadata(&answer).expect("well-formed metadata");
        assert_eq!(parsed["asset_id"], 7);
        assert_eq!(parsed["symbol"], "ATLAS");
        assert_eq!(parsed["decimals"], 18);
    }

    #[test]
    fn an_unregistered_id_is_null_and_not_an_asset() {
        assert!(parse_asset_metadata(&serde_json::Value::Null).is_none());
    }

    #[test]
    fn half_formed_metadata_is_refused_rather_than_defaulted() {
        // A missing symbol or decimals must not become "" or 0: the CLI would
        // print a fabricated asset, which is exactly the bug this replaced.
        for broken in [
            serde_json::json!({ "asset_id": 1, "decimals": 12 }),
            serde_json::json!({ "asset_id": 1, "symbol": "X3" }),
            serde_json::json!({ "symbol": "X3", "decimals": 12 }),
            serde_json::json!({ "asset_id": "1", "symbol": "X3", "decimals": 12 }),
        ] {
            assert!(
                parse_asset_metadata(&broken).is_none(),
                "{broken} must not be accepted as asset metadata"
            );
        }
    }

    #[test]
    fn the_scan_range_is_bounded_and_printed() {
        // The bound is part of the contract: it is printed with every result so
        // an operator can tell "no assets" from "no assets in this range".
        let scanned: Vec<u32> = (ASSET_SCAN_MIN..=ASSET_SCAN_MAX).collect();
        assert_eq!(scanned.first().copied(), Some(0));
        assert!(
            scanned.len() > 8 && scanned.len() <= 256,
            "a bound of {} ids is not a useful scan",
            scanned.len()
        );
    }
}

#[cfg(test)]
mod key_command_tests {
    use super::*;

    /// `//Alice`'s canonical keys. These are the values every Substrate tool
    /// prints for the dev phrase, so they pin the derivation to the SDK's
    /// rather than to "something 32 bytes long".
    const ALICE_AURA_PUBLIC_HEX: &str =
        "d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d";
    const ALICE_GRANDPA_PUBLIC_HEX: &str =
        "88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee";
    const ALICE_AURA_SS58: &str = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
    const ALICE_GRANDPA_SS58: &str = "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu";

    #[test]
    fn key_type_aliases_resolve_to_the_keystore_ids() {
        for (name, expected_id, expected_scheme) in [
            ("aura", *b"aura", KeyScheme::Sr25519),
            ("AURA", *b"aura", KeyScheme::Sr25519),
            ("grandpa", *b"gran", KeyScheme::Ed25519),
            ("gran", *b"gran", KeyScheme::Ed25519),
            ("imonline", *b"imon", KeyScheme::Sr25519),
            ("imon", *b"imon", KeyScheme::Sr25519),
        ] {
            let spec = resolve_key_type(name).expect("alias should resolve");
            assert_eq!(spec.key_type.0, expected_id, "{name} key type id");
            assert_eq!(spec.scheme, expected_scheme, "{name} scheme");
        }

        // A typo must not silently become a key: the old code accepted any
        // string and printed advice.
        for bad in ["", "sudos", "grnd", "aura2"] {
            assert!(
                resolve_key_type(bad).is_err(),
                "{bad:?} must not resolve to a key type"
            );
        }
    }

    #[test]
    fn dev_phrase_derives_the_canonical_authority_keys() {
        let aura = public_from_suri(KeyScheme::Sr25519, "//Alice").expect("aura key");
        assert_eq!(hex::encode(&aura), ALICE_AURA_PUBLIC_HEX);
        assert_eq!(public_to_ss58(&aura).unwrap(), ALICE_AURA_SS58);

        let grandpa = public_from_suri(KeyScheme::Ed25519, "//Alice").expect("grandpa key");
        assert_eq!(hex::encode(&grandpa), ALICE_GRANDPA_PUBLIC_HEX);
        assert_eq!(public_to_ss58(&grandpa).unwrap(), ALICE_GRANDPA_SS58);
    }

    #[test]
    fn ss58_and_hex_forms_of_the_same_key_agree() {
        let from_ss58 = parse_public_key(ALICE_AURA_SS58).expect("ss58 form");
        let from_hex = parse_public_key(&format!("0x{ALICE_AURA_PUBLIC_HEX}")).expect("hex form");
        assert_eq!(from_ss58, from_hex);
        assert_eq!(hex::encode(&from_ss58), ALICE_AURA_PUBLIC_HEX);

        // Wrong length and wrong alphabet are rejected, not truncated.
        assert!(parse_public_key("0xdead").is_err());
        assert!(parse_public_key("not-an-address").is_err());
        assert!(public_to_ss58(&[0u8; 31]).is_err());
    }

    #[test]
    fn a_generated_key_is_reproducible_from_the_secret_it_prints() {
        // `keys generate` without `--seed` must hand back a secret that derives
        // the same public key, or the operator cannot re-insert the key later.
        let (public, secret) = generate_keypair(KeyScheme::Sr25519);
        assert_eq!(public.len(), 32);
        let rederived = public_from_suri(KeyScheme::Sr25519, &secret).expect("secret re-derives");
        assert_eq!(rederived, public);

        let (public, secret) = generate_keypair(KeyScheme::Ed25519);
        let rederived = public_from_suri(KeyScheme::Ed25519, &secret).expect("secret re-derives");
        assert_eq!(rederived, public);
    }

    #[test]
    fn default_keystore_path_matches_the_layout_a_running_node_uses() {
        let base = Some(std::path::PathBuf::from("/tmp/x3-keys-default-test"));
        let path = default_keystore_path(&base, &Some("dev".to_string())).expect("dev keystore");
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/x3-keys-default-test/chains/x3_chain_dev/keystore")
        );

        // An unknown chain id is a path, and a path that does not exist must
        // surface as an error rather than as a keystore in a made-up directory.
        assert!(default_keystore_path(&base, &Some("/nonexistent/spec.json".into())).is_err());
    }
}
