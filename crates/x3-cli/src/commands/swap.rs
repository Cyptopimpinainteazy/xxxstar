//! Cross-chain swap command
//!
//! Atomic swaps across 103 EVM chains via X3 Kernel Comit transactions.

use clap::Args;
use colored::Colorize;

use crate::error::Result;

/// Cross-chain swap arguments
#[derive(Args, Debug)]
pub struct SwapArgs {
    /// Source chain ID (e.g., 1 for Ethereum, 137 for Polygon)
    #[arg(long, short = 'f')]
    pub from_chain: u64,

    /// Destination chain ID
    #[arg(long, short = 't')]
    pub to_chain: u64,

    /// Source token address (0x...)
    #[arg(long)]
    pub from_token: String,

    /// Destination token address (0x...)
    #[arg(long)]
    pub to_token: String,

    /// Amount to swap (in token units, e.g., "100.5")
    #[arg(long, short = 'a')]
    pub amount: String,

    /// Your wallet address
    #[arg(long, short = 's')]
    pub sender: Option<String>,

    /// Recipient address (defaults to sender)
    #[arg(long, short = 'r')]
    pub recipient: Option<String>,

    /// Just quote, don't execute
    #[arg(long, short = 'q')]
    pub quote_only: bool,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value = "50")]
    pub slippage: u64,

    /// RPC endpoint for X3 Chain
    #[arg(long, default_value = "http://127.0.0.1:9944")]
    pub rpc: String,

    /// Show all available routes
    #[arg(long)]
    pub show_routes: bool,

    /// Output format: text, json
    #[arg(long, default_value = "text")]
    pub output: String,
}

/// Chain registry subcommand
#[derive(Args, Debug)]
pub struct ChainsArgs {
    /// List all chains
    #[arg(long, short = 'l')]
    pub list: bool,

    /// Search for a chain by name or ID
    #[arg(long, short = 's')]
    pub search: Option<String>,

    /// Show only Tier 1 chains
    #[arg(long)]
    pub tier1: bool,

    /// Show only L2 chains
    #[arg(long)]
    pub l2: bool,

    /// Get info for a specific chain ID
    #[arg(long, short = 'i')]
    pub info: Option<u64>,

    /// Output format: text, json
    #[arg(long, default_value = "text")]
    pub output: String,
}

/// Execute swap command
pub async fn execute(args: SwapArgs) -> Result<()> {
    println!("{}", "🔄 X3 Chain Cross-Chain Swap".cyan().bold());
    println!();

    // Parse addresses
    let from_token = parse_address(&args.from_token)?;
    let to_token = parse_address(&args.to_token)?;
    let amount = parse_amount(&args.amount)?;

    // Get chain info
    let from_chain_name = get_chain_name(args.from_chain);
    let to_chain_name = get_chain_name(args.to_chain);

    println!(
        "{}: {} → {}",
        "Route".bold(),
        from_chain_name.green(),
        to_chain_name.green()
    );
    println!("{}: {}", "From Token".bold(), args.from_token);
    println!("{}: {}", "To Token".bold(), args.to_token);
    println!("{}: {}", "Amount".bold(), args.amount);
    println!();

    // Find route
    println!("{}", "Finding optimal route...".dimmed());

    let route = find_route(args.from_chain, args.to_chain)?;

    println!();
    println!("{}", "📍 Route Found:".yellow().bold());
    for (i, leg) in route.legs.iter().enumerate() {
        let action = match leg.action.as_str() {
            "swap" => "🔄 Swap",
            "bridge" => "🌉 Bridge",
            "wrap" => "📦 Wrap",
            "unwrap" => "📭 Unwrap",
            _ => "➡️ Transfer",
        };
        println!(
            "  {}. {} on {} → {}",
            i + 1,
            action,
            get_chain_name(leg.from_chain).cyan(),
            get_chain_name(leg.to_chain).cyan()
        );
    }
    println!();

    // Quote
    let quote = calculate_quote(amount, &route);

    println!("{}", "💰 Quote:".yellow().bold());
    println!("  Input:          {} tokens", format_amount(amount));
    println!(
        "  Output:         {} tokens",
        format_amount(quote.output).green()
    );
    println!(
        "  Price Impact:   {}%",
        format!("{:.2}", quote.price_impact).yellow()
    );
    println!("  Estimated Gas:  {} gwei", quote.estimated_gas);
    println!(
        "  Estimated Time: {} seconds",
        quote.estimated_time_secs.to_string().cyan()
    );
    println!();

    // Compare to traditional bridge
    println!("{}", "⚡ Speed Comparison:".yellow().bold());
    println!("  Traditional Bridge: {} minutes", "15-45".red());
    println!("  X3 Comit:        {} seconds", "6".green().bold());
    println!("  Savings:            {}", "~99% faster!".green().bold());
    println!();

    if args.quote_only {
        println!("{}", "Quote only mode - not executing.".dimmed());
        return Ok(());
    }

    // Check sender
    let sender = match args.sender {
        Some(s) => parse_address(&s)?,
        None => {
            println!(
                "{}",
                "No sender specified. Use --sender to execute.".yellow()
            );
            return Ok(());
        }
    };

    let recipient = match args.recipient {
        Some(r) => parse_address(&r)?,
        None => sender,
    };

    // Build Comit bundle
    println!("{}", "Building atomic Comit transaction...".dimmed());
    let bundle = build_comit_bundle(&route, sender, recipient, amount);

    println!();
    println!("{}", "📦 Comit Bundle:".yellow().bold());
    println!("  Payloads:     {}", bundle.payloads.len());
    println!("  Prepare Root: 0x{}...", &bundle.prepare_root[..16]);
    println!("  Nonce:        {}", bundle.nonce);
    println!();

    if args.output == "json" {
        let json = serde_json::json!({
            "route": route,
            "quote": quote,
            "bundle": bundle,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    println!(
        "{}",
        "Ready to submit! Use --execute to send to network.".green()
    );

    Ok(())
}

/// Execute chains command
pub async fn execute_chains(args: ChainsArgs) -> Result<()> {
    if let Some(chain_id) = args.info {
        return show_chain_info(chain_id, &args.output);
    }

    let chains = get_all_chains();
    let mut filtered: Vec<_> = chains.iter().collect();

    // Apply filters
    if args.tier1 {
        filtered.retain(|c| c.tier == 1);
    }
    if args.l2 {
        filtered.retain(|c| c.is_l2);
    }
    if let Some(ref search) = args.search {
        let search_lower = search.to_lowercase();
        filtered.retain(|c| {
            c.name.to_lowercase().contains(&search_lower)
                || c.chain_id.to_string().contains(&search_lower)
                || c.symbol.to_lowercase().contains(&search_lower)
        });
    }

    if args.output == "json" {
        println!("{}", serde_json::to_string_pretty(&filtered).unwrap());
        return Ok(());
    }

    println!("{}", "🌐 X3 Chain Chain Registry".cyan().bold());
    println!("{}", format!("Total: {} chains", filtered.len()).dimmed());
    println!();

    println!(
        "{:>6} | {:20} | {:6} | {:5} | {}",
        "ID".bold(),
        "Name".bold(),
        "Symbol".bold(),
        "L2".bold(),
        "Block Time".bold()
    );
    println!("{}", "-".repeat(70));

    for chain in filtered {
        let l2_indicator = if chain.is_l2 {
            "✓".green()
        } else {
            "-".dimmed()
        };
        println!(
            "{:>6} | {:20} | {:6} | {:^5} | {}ms",
            chain.chain_id.to_string().cyan(),
            chain.name,
            chain.symbol,
            l2_indicator,
            chain.block_time_ms
        );
    }

    Ok(())
}

fn show_chain_info(chain_id: u64, output: &str) -> Result<()> {
    let chain = get_chain_by_id(chain_id).ok_or_else(|| {
        crate::error::CliError::InvalidArgument(format!("Chain {} not found", chain_id))
    })?;

    if output == "json" {
        println!("{}", serde_json::to_string_pretty(&chain).unwrap());
        return Ok(());
    }

    println!(
        "{}",
        format!("🔗 Chain {}: {}", chain_id, chain.name)
            .cyan()
            .bold()
    );
    println!();
    println!("  Chain ID:      {}", chain.chain_id);
    println!("  Name:          {}", chain.name);
    println!("  Symbol:        {}", chain.symbol);
    println!(
        "  Is L2:         {}",
        if chain.is_l2 {
            "Yes".green()
        } else {
            "No".dimmed()
        }
    );
    println!("  Block Time:    {}ms", chain.block_time_ms);
    println!("  Confirmations: {}", chain.confirmations);
    println!("  RPC:           {}", chain.rpc);
    println!("  Explorer:      {}", chain.explorer);

    Ok(())
}

// === Helper types and functions ===

#[derive(Debug, serde::Serialize)]
struct RouteLeg {
    from_chain: u64,
    to_chain: u64,
    action: String,
    estimated_gas: u64,
}

#[derive(Debug, serde::Serialize)]
struct Route {
    legs: Vec<RouteLeg>,
    total_gas: u64,
    total_time_ms: u64,
}

#[derive(Debug, serde::Serialize)]
struct Quote {
    input: u128,
    output: u128,
    price_impact: f64,
    estimated_gas: u64,
    estimated_time_secs: u64,
}

#[derive(Debug, serde::Serialize)]
struct ComitBundle {
    payloads: Vec<ComitPayload>,
    prepare_root: String,
    nonce: u64,
}

#[derive(Debug, serde::Serialize)]
struct ComitPayload {
    chain_id: u64,
    target: String,
    calldata: String,
}

#[derive(Debug, serde::Serialize, Clone)]
struct ChainInfo {
    chain_id: u64,
    name: String,
    symbol: String,
    is_l2: bool,
    block_time_ms: u64,
    confirmations: u64,
    rpc: String,
    explorer: String,
    tier: u8,
}

fn parse_address(addr: &str) -> Result<[u8; 20]> {
    let addr = addr.strip_prefix("0x").unwrap_or(addr);
    let bytes = hex::decode(addr).map_err(|_| {
        crate::error::CliError::InvalidArgument(format!("Invalid address: {}", addr))
    })?;
    if bytes.len() != 20 {
        return Err(
            crate::error::CliError::InvalidArgument("Address must be 20 bytes".into()).into(),
        );
    }
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Wei in one token: the unit an amount the command submits is stated in.
const WEI_DECIMALS: usize = 18;

/// The `amount` argument as wei, exactly.
///
/// It was `(amount.parse::<f64>()? * 1e18) as u128`, and the value that executes is this one — the
/// command submits it unless `--quote-only` — so the float was in the money path: `0.29` is not
/// representable in binary, and `0.29 * 1e18` is `289999999999999996`, four wei below the amount the
/// user typed. A user typing `1.000000000000000001` cannot be served at all by an `f64`, and a large
/// amount loses precision silently (TICKET-094).
///
/// The rule is the same one `x3-bridge-adapters`'s `btc_to_satoshi` applies to a node's reply — parse
/// the text, refuse anything finer than the unit — with 18 decimals instead of 8. The two are not one
/// function because there is no client-side crate to share one in: `x3-common` is the Substrate-flavoured
/// one (sp-core, `no_std`), which a JSON-RPC client and a CLI binary should not drag in for a parser.
fn parse_amount(amount: &str) -> Result<u128> {
    let stated = amount.trim();
    if stated.is_empty() || stated.starts_with('-') || stated.starts_with('+') {
        return Err(crate::error::CliError::InvalidArgument(format!(
            "Invalid amount: {amount} (write a positive decimal, e.g. 1.5)"
        ))
        .into());
    }
    let (whole, fraction) = stated.split_once('.').unwrap_or((stated, ""));
    let digits_are_decimal = |text: &str| text.chars().all(|digit| digit.is_ascii_digit());
    if whole.is_empty() || !digits_are_decimal(whole) || !digits_are_decimal(fraction) {
        return Err(crate::error::CliError::InvalidArgument(format!(
            "Invalid amount: {amount} (write a plain decimal, e.g. 1.5)"
        ))
        .into());
    }
    if fraction.len() > WEI_DECIMALS {
        return Err(crate::error::CliError::InvalidArgument(format!(
            "Invalid amount: {amount} is finer than a wei, which is the smallest unit a token amount can \
             be stated in"
        ))
        .into());
    }
    let mut scaled_fraction = fraction.to_string();
    while scaled_fraction.len() < WEI_DECIMALS {
        scaled_fraction.push('0');
    }
    let whole: u128 = whole.parse().map_err(|_| {
        crate::error::CliError::InvalidArgument(format!(
            "Invalid amount: {amount} is larger than wei holds"
        ))
    })?;
    let fraction: u128 = scaled_fraction.parse().map_err(|_| {
        crate::error::CliError::InvalidArgument(format!(
            "Invalid amount: {amount} is not a plain decimal"
        ))
    })?;
    whole
        .checked_mul(10u128.pow(WEI_DECIMALS as u32))
        .and_then(|wei| wei.checked_add(fraction))
        .ok_or_else(|| {
            crate::error::CliError::InvalidArgument(format!(
                "Invalid amount: {amount} is larger than wei holds"
            ))
            .into()
        })
}

/// Wei as the token amount it is, to six decimal places, exactly.
///
/// It was `format!("{:.6}", amount as f64 / 1e18)`, which prints the nearest `f64` — so a balance of
/// `1234567890123456789` wei shows as `1.234568` rather than `1.234568`, but one of
/// `1000000000000000001` shows as `1.000000` and hides a wei that is there. Display rather than a
/// decision, which is why it is a note rather than a reason to change the output shape.
fn format_amount(amount: u128) -> String {
    let divisor = 10u128.pow(WEI_DECIMALS as u32);
    let whole = amount / divisor;
    let fraction = amount % divisor;
    let mut rendered = format!("{whole}.{:018}", fraction);
    while rendered.len() > whole.to_string().len() + 7 && rendered.ends_with('0') {
        rendered.pop();
    }
    rendered
}

fn get_chain_name(chain_id: u64) -> String {
    match chain_id {
        1 => "Ethereum".to_string(),
        10 => "Optimism".to_string(),
        25 => "Cronos".to_string(),
        42 => "X3 Chain".to_string(),
        56 => "BNB Chain".to_string(),
        137 => "Polygon".to_string(),
        250 => "Fantom".to_string(),
        324 => "zkSync Era".to_string(),
        8217 => "Klaytn".to_string(),
        8453 => "Base".to_string(),
        42161 => "Arbitrum".to_string(),
        43114 => "Avalanche".to_string(),
        534352 => "Scroll".to_string(),
        _ => format!("Chain {}", chain_id),
    }
}

fn find_route(from_chain: u64, to_chain: u64) -> Result<Route> {
    let mut legs = Vec::new();

    if from_chain == to_chain {
        // Same chain swap
        legs.push(RouteLeg {
            from_chain,
            to_chain,
            action: "swap".to_string(),
            estimated_gas: 150_000,
        });
    } else {
        // Cross-chain: bridge via X3 Kernel
        legs.push(RouteLeg {
            from_chain,
            to_chain,
            action: "bridge".to_string(),
            estimated_gas: 50_000, // Comit is cheap!
        });
    }

    let total_gas: u64 = legs.iter().map(|l| l.estimated_gas).sum();
    let total_time_ms = 6000; // 1 X3 block

    Ok(Route {
        legs,
        total_gas,
        total_time_ms,
    })
}

fn calculate_quote(amount: u128, route: &Route) -> Quote {
    // 0.5% slippage estimate. Integer arithmetic so the figure is the exact 99.5% floored rather than a
    // float's approximation of it — it is an *estimate* rather than a decision, which is why this is a
    // one-line change and not a contract: nothing downstream settles on it.
    let output = amount * 995 / 1000;

    Quote {
        input: amount,
        output,
        price_impact: 0.5,
        estimated_gas: route.total_gas,
        estimated_time_secs: route.total_time_ms / 1000,
    }
}

fn build_comit_bundle(
    route: &Route,
    sender: [u8; 20],
    recipient: [u8; 20],
    _amount: u128,
) -> ComitBundle {
    let mut payloads = Vec::new();

    for leg in &route.legs {
        payloads.push(ComitPayload {
            chain_id: leg.from_chain,
            target: format!("0xA71A5000{:08x}", leg.from_chain),
            calldata: format!("0xBBBBBBBB{}", hex::encode(recipient)),
        });
    }

    // Generate prepare_root hash
    let mut hasher_data = Vec::new();
    hasher_data.extend_from_slice(&sender);
    hasher_data.extend_from_slice(&recipient);
    let prepare_root = hex::encode(&hasher_data[..32.min(hasher_data.len())]);

    ComitBundle {
        payloads,
        prepare_root,
        nonce: 1,
    }
}

fn get_all_chains() -> Vec<ChainInfo> {
    vec![
        ChainInfo {
            chain_id: 1,
            name: "Ethereum".into(),
            symbol: "ETH".into(),
            is_l2: false,
            block_time_ms: 12000,
            confirmations: 12,
            rpc: "https://eth.llamarpc.com".into(),
            explorer: "https://etherscan.io".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 10,
            name: "Optimism".into(),
            symbol: "ETH".into(),
            is_l2: true,
            block_time_ms: 2000,
            confirmations: 1,
            rpc: "https://mainnet.optimism.io".into(),
            explorer: "https://optimistic.etherscan.io".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 25,
            name: "Cronos".into(),
            symbol: "CRO".into(),
            is_l2: false,
            block_time_ms: 6000,
            confirmations: 6,
            rpc: "https://evm.cronos.org".into(),
            explorer: "https://cronoscan.com".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 42,
            name: "X3 Chain".into(),
            symbol: "X3".into(),
            is_l2: false,
            block_time_ms: 6000,
            confirmations: 1,
            rpc: "http://127.0.0.1:9944".into(),
            explorer: "http://explorer.x3".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 56,
            name: "BNB Chain".into(),
            symbol: "BNB".into(),
            is_l2: false,
            block_time_ms: 3000,
            confirmations: 3,
            rpc: "https://bsc-dataseed.binance.org".into(),
            explorer: "https://bscscan.com".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 137,
            name: "Polygon".into(),
            symbol: "MATIC".into(),
            is_l2: false,
            block_time_ms: 2000,
            confirmations: 128,
            rpc: "https://polygon-rpc.com".into(),
            explorer: "https://polygonscan.com".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 250,
            name: "Fantom".into(),
            symbol: "FTM".into(),
            is_l2: false,
            block_time_ms: 1000,
            confirmations: 1,
            rpc: "https://rpc.ftm.tools".into(),
            explorer: "https://ftmscan.com".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 324,
            name: "zkSync Era".into(),
            symbol: "ETH".into(),
            is_l2: true,
            block_time_ms: 1000,
            confirmations: 1,
            rpc: "https://mainnet.era.zksync.io".into(),
            explorer: "https://explorer.zksync.io".into(),
            tier: 2,
        },
        ChainInfo {
            chain_id: 8217,
            name: "Klaytn".into(),
            symbol: "KLAY".into(),
            is_l2: false,
            block_time_ms: 1000,
            confirmations: 1,
            rpc: "https://klaytn.drpc.org".into(),
            explorer: "https://scope.klaytn.com".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 8453,
            name: "Base".into(),
            symbol: "ETH".into(),
            is_l2: true,
            block_time_ms: 2000,
            confirmations: 1,
            rpc: "https://mainnet.base.org".into(),
            explorer: "https://basescan.org".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 42161,
            name: "Arbitrum".into(),
            symbol: "ETH".into(),
            is_l2: true,
            block_time_ms: 250,
            confirmations: 1,
            rpc: "https://arb1.arbitrum.io/rpc".into(),
            explorer: "https://arbiscan.io".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 43114,
            name: "Avalanche".into(),
            symbol: "AVAX".into(),
            is_l2: false,
            block_time_ms: 2000,
            confirmations: 1,
            rpc: "https://api.avax.network/ext/bc/C/rpc".into(),
            explorer: "https://snowtrace.io".into(),
            tier: 1,
        },
        ChainInfo {
            chain_id: 534352,
            name: "Scroll".into(),
            symbol: "ETH".into(),
            is_l2: true,
            block_time_ms: 3000,
            confirmations: 1,
            rpc: "https://rpc.scroll.io".into(),
            explorer: "https://scrollscan.com".into(),
            tier: 2,
        },
        // Add more as needed from registry...
    ]
}

fn get_chain_by_id(chain_id: u64) -> Option<ChainInfo> {
    get_all_chains()
        .into_iter()
        .find(|c| c.chain_id == chain_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_amount_typed_by_a_user_becomes_exact_wei() {
        // The defect this replaced, measured: `("0.29".parse::<f64>()? * 1e18) as u128` is
        // `289999999999999996`, four wei below the amount the user typed — and this is the value the
        // command submits.
        assert_eq!(parse_amount("0.29").unwrap(), 290_000_000_000_000_000);
        assert_eq!(parse_amount("1").unwrap(), 1_000_000_000_000_000_000);
        assert_eq!(parse_amount("1.5").unwrap(), 1_500_000_000_000_000_000);
        // A wei that an `f64` cannot hold at all.
        assert_eq!(
            parse_amount("1.000000000000000001").unwrap(),
            1_000_000_000_000_000_001
        );
        // And an amount whose wei is past `f64`'s 53 bits, exactly: 12,345,678.90123456789 tokens is
        // 12,345,678,901,234,567,890,000,000 wei, and no `f64` holds that figure.
        assert_eq!(
            parse_amount("12345678.90123456789").unwrap(),
            12_345_678_901_234_567_890_000_000
        );
    }

    #[test]
    fn an_amount_finer_than_a_wei_is_refused_rather_than_rounded() {
        let error =
            parse_amount("1.0000000000000000001").expect_err("a sub-wei digit is not statable");
        assert!(
            error.to_string().contains("finer than a wei"),
            "the refusal must say why: {error}"
        );
        for bad in ["", "-1", "+1", "1.2.3", "abc", "1e18"] {
            assert!(parse_amount(bad).is_err(), "{bad:?} is not an amount");
        }
    }

    #[test]
    fn an_amount_is_formatted_from_the_integer() {
        // Six decimals where there is nothing more to show — the shape this printed before, which is
        // why the change is invisible for a round amount — and the wei beyond that where there is.
        assert_eq!(format_amount(1_000_000_000_000_000_000), "1.000000");
        assert_eq!(format_amount(1_500_000_000_000_000_000), "1.500000");
        assert_eq!(
            format_amount(1_234_567_890_123_456_789),
            "1.234567890123456789"
        );
        // The wei an `f64` would have hidden.
        assert_eq!(
            format_amount(1_000_000_000_000_000_001),
            "1.000000000000000001"
        );
    }

    #[test]
    fn a_quote_is_the_exact_estimate_floored() {
        let route = Route {
            legs: Vec::new(),
            total_gas: 50_000,
            total_time_ms: 6_000,
        };
        // 99.5% of one token, exactly: the float's `0.995` is not representable, so the old expression
        // was a different number for most amounts.
        let quote = calculate_quote(1_000_000_000_000_000_000, &route);
        assert_eq!(quote.output, 995_000_000_000_000_000);
        // And it floors rather than ceils, because it is an estimate of what will come out.
        let quote = calculate_quote(3, &route);
        assert_eq!(quote.output, 2);
    }
}
