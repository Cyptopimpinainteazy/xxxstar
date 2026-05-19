use crate::{
    cli::{
        AtomicSwapSubcommand, Cli, ComitSubcommand, Commands, InspectSubcommand, KeysSubcommand,
    },
    service,
};
use clap::Parser;
#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use log::{error, info, warn};
use sc_cli::{Error as CliError, Result as CliResult, SubstrateCli};
#[cfg(feature = "try-runtime")]
use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
#[cfg(any(feature = "runtime-benchmarks", feature = "try-runtime"))]
use x3_chain_runtime::opaque::Block;

use crate::logging;

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
                        cmd.run::<Block, sp_io::SubstrateHostFunctions>(config)
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
                        cmd.run(config, client, db, storage)
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
        Some(Commands::TryRuntime(cmd)) => {
            let runner = cli.create_runner(cmd).map_err(|e| {
                error!("Failed to initialize runner for `try-runtime`: {e}");
                e
            })?;

            runner.async_run(|config| {
                let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                let task_manager =
                    sc_service::TaskManager::new(config.tokio_handle.clone(), registry).map_err(
                        |e| CliError::Service(sc_service::Error::Prometheus(e)),
                    )?;
                let info_provider =
                    try_runtime_cli::block_building_info::substrate_info(x3_chain_runtime::SLOT_DURATION);

                Ok((
                    cmd.run::<
                        Block,
                        ExtendedHostFunctions<
                            sp_io::SubstrateHostFunctions,
                            <service::AtlasSphereExecutorDispatch as NativeExecutionDispatch>::ExtendHostFunctions,
                        >,
                        _,
                    >(Some(info_provider)),
                    task_manager,
                ))
            })
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

                    // Make RPC call to atlasKernel_getCanonicalBalance as a proxy query
                    // In production, this would query a dedicated Comit status endpoint
                    match make_rpc_call(
                        rpc_url,
                        "atlasKernel_getCanonicalBalance",
                        serde_json::json!(["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", 0]),
                    ) {
                        Ok(result) => {
                            println!("--- Comit Status ---");
                            println!("Note: Full Comit query endpoint not yet implemented.");
                            println!("Showing canonical balance query as example:");
                            println!("Balance: {}", result);
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Comit Query Failed ---");
                            println!("Error: {}", e);
                            println!();
                            println!("Note: Ensure a node is running on {}", rpc_url);
                        }
                    }

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

                    // Make RPC call to atlasKernel_getCanonicalBalance
                    match make_rpc_call(
                        rpc_url,
                        "atlasKernel_getCanonicalBalance",
                        serde_json::json!([account, asset_id]),
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

                    // Make RPC call to atlasKernel_getAuthorizedAccounts
                    match make_rpc_call(
                        rpc_url,
                        "atlasKernel_getAuthorizedAccounts",
                        serde_json::json!([]),
                    ) {
                        Ok(result) => {
                            println!("--- Authorized Accounts ---");
                            if let Some(arr) = result.as_array() {
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
                    seed: _,
                    output,
                } => {
                    info!("Generating keypair...");
                    info!("  Key Type: {}", key_type);
                    info!("  Output:   {}", output);

                    println!("\n=== Key Generation ===");
                    println!("Key Type:  {}", key_type);
                    println!("Output:    {}", output);
                    println!();

                    // In a full implementation, this would use sp_core crypto
                    // For now, show a placeholder
                    println!("--- Generated Keypair ---");
                    println!("Note: Full key generation requires sp_core integration.");
                    println!("Use `subkey` tool for production key generation:");
                    println!("  subkey generate --scheme sr25519");
                    println!();
                    println!("Key type mapping:");
                    println!("  aura    -> sr25519 (block authoring)");
                    println!("  grandpa -> ed25519 (finality)");
                    println!("  imonline -> sr25519 (heartbeat)");

                    Ok(())
                }
                KeysSubcommand::Insert {
                    key_type,
                    seed: _,
                    keystore_path,
                } => {
                    info!("Inserting key into keystore...");
                    info!("  Key Type: {}", key_type);
                    if let Some(path) = keystore_path {
                        info!("  Keystore: {:?}", path);
                    }

                    println!("\n=== Key Insertion ===");
                    println!("Key Type:  {}", key_type);
                    if let Some(path) = keystore_path {
                        println!("Keystore:  {:?}", path);
                    } else {
                        println!("Keystore:  (default)");
                    }
                    println!();

                    // In a full implementation, this would insert into the keystore
                    println!("--- Key Insertion ---");
                    println!("Note: Full keystore insertion requires node integration.");
                    println!("Use the node's keystore directly or `subkey` for testing.");

                    Ok(())
                }
                KeysSubcommand::List { keystore_path } => {
                    info!("Listing keystore contents...");
                    if let Some(path) = keystore_path {
                        info!("  Keystore: {:?}", path);
                    }

                    println!("\n=== Keystore Contents ===");
                    if let Some(path) = keystore_path {
                        println!("Keystore:  {:?}", path);
                    } else {
                        println!("Keystore:  (default)");
                    }
                    println!();

                    // In a full implementation, this would list keys from the keystore
                    println!("--- Keys ---");
                    println!("Note: Full keystore listing requires node integration.");

                    Ok(())
                }
                KeysSubcommand::Verify {
                    key_type,
                    public,
                    seed: _,
                } => {
                    info!("Verifying keypair...");
                    info!("  Key Type: {}", key_type);
                    info!("  Public:   {}", public);

                    println!("\n=== Keypair Verification ===");
                    println!("Key Type:  {}", key_type);
                    println!("Public:    {}", public);
                    println!();

                    // In a full implementation, this would verify the keypair
                    println!("--- Verification ---");
                    println!("Note: Full key verification requires sp_core integration.");

                    Ok(())
                }
            }
        }
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

                    // Make RPC call to atlasKernel_getCanonicalBalance for native asset
                    match make_rpc_call(
                        rpc_url,
                        "atlasKernel_getCanonicalBalance",
                        serde_json::json!([account, 0]),
                    ) {
                        Ok(result) => {
                            println!("--- Account Balances ---");
                            println!("Native X3 (Asset 0): {}", result);

                            // In a full implementation, this would iterate through all assets
                            println!();
                            println!("Note: Full account inspection requires asset enumeration.");
                        }
                        Err(e) => {
                            warn!("RPC call failed: {}", e);
                            println!("--- Account Inspection Failed ---");
                            println!("Error: {}", e);
                            println!();
                            println!("Note: Ensure a node is running on {}", rpc_url);
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

                    // Make RPC call to atlasKernel_getAssetMetadata
                    match make_rpc_call(
                        rpc_url,
                        "atlasKernel_getAssetMetadata",
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

                    // In a full implementation, this would enumerate all assets
                    println!("--- Registered Assets ---");
                    println!("Note: Full asset enumeration requires runtime API support.");
                    println!("Known assets:");
                    println!("  0: X3 (native token, 12 decimals)");
                    println!("  1: ETH (18 decimals)");
                    println!("  2: SOL (9 decimals)");
                    println!("  3: USDC (6 decimals)");

                    Ok(())
                }
                InspectSubcommand::Authorities { rpc_url } => {
                    info!("Querying authority set...");
                    info!("  RPC URL: {}", rpc_url);

                    println!("\n=== Authority Set ===");
                    println!("RPC URL:  {}", rpc_url);
                    println!();

                    // Make RPC call to atlasKernel_getAuthorities
                    match make_rpc_call(
                        rpc_url,
                        "atlasKernel_getAuthorities",
                        serde_json::json!([]),
                    ) {
                        Ok(result) => {
                            println!("--- Current Authorities ---");
                            if let Some(arr) = result.as_array() {
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
                let role = config.role.clone();
                info!("Starting X3 Chain node as {:?}", role);
                service::new_full::<sc_network::NetworkWorker<_, _>>(config, feature_flags).map_err(|e| {
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
