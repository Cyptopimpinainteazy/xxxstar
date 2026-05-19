use codec::Encode;
use sc_service::GenericChainSpec;
use sc_service::{ChainSpec as ServiceChainSpec, ChainType};
use serde::Deserialize;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::crypto::Ss58Codec;
use sp_core::{sr25519, Pair, Public, H160};
#[cfg(feature = "frontier")]
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::{collections::BTreeSet, path::PathBuf};
use x3_chain_runtime::{
    x3_kernel_default_assets, AccountId, AtlasKernelConfig, AuraConfig, BalancesConfig,
    CouncilConfig, GrandpaConfig, RuntimeGenesisConfig, Signature, TreasuryConfig, X3CoinConfig,
    WASM_BINARY,
};

/// Chain specification specialized to this runtime's genesis configuration.
pub type ChainSpec = GenericChainSpec;

const DEFAULT_PROTOCOL_ID: &str = "x3";
const X3: u128 = 1_000_000_000_000;
const ENDOWMENT: u128 = 1_000_000 * X3;

type AccountPublic = <Signature as Verify>::Signer;

// EVM callers funded in dev/local chains through their mapped Substrate accounts.
#[cfg(feature = "frontier")]
const DEV_EVM_CALLERS: [[u8; 20]; 2] = [
    [
        0xd4, 0x35, 0x93, 0xc7, 0x15, 0xfd, 0xd3, 0x1c, 0x61, 0x14, 0x1a, 0xbd, 0x04, 0xa9, 0x9f,
        0xd6, 0x82, 0x2c, 0x85, 0x58,
    ],
    [
        0xf3, 0x9f, 0xd6, 0xe5, 0x1a, 0xad, 0x88, 0xf6, 0xf4, 0xce, 0x6a, 0xb8, 0x82, 0x72, 0x79,
        0xcf, 0xff, 0xb9, 0x22, 0x66,
    ],
];

#[cfg(feature = "frontier")]
fn dev_evm_endowed_accounts() -> Vec<AccountId> {
    DEV_EVM_CALLERS
        .iter()
        .map(|bytes| {
            let evm_addr = H160::from(*bytes);
            <pallet_evm::HashedAddressMapping<BlakeTwo256> as pallet_evm::AddressMapping<
                AccountId,
            >>::into_account_id(evm_addr)
        })
        .collect()
}

#[cfg(not(feature = "frontier"))]
fn dev_evm_endowed_accounts() -> Vec<AccountId> {
    Vec::new()
}

#[derive(Debug, Deserialize)]
struct ExternalAuthority {
    aura: String,
    grandpa: String,
}

fn parse_authorities_from_env(var: &str) -> Result<Vec<(AuraId, GrandpaId)>, String> {
    let raw = std::env::var(var).map_err(|_| {
        format!(
            "Missing {}. Expected JSON array of {{aura,grandpa}} SS58 keys",
            var
        )
    })?;

    let decoded: Vec<ExternalAuthority> =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid {} JSON: {}", var, e))?;
    if decoded.is_empty() {
        return Err(format!("{} cannot be empty", var));
    }

    decoded
        .into_iter()
        .map(|entry| {
            let aura = AuraId::from_ss58check(&entry.aura)
                .map_err(|e| format!("Invalid Aura SS58 in {}: {}", var, e))?;
            let grandpa = GrandpaId::from_ss58check(&entry.grandpa)
                .map_err(|e| format!("Invalid Grandpa SS58 in {}: {}", var, e))?;
            Ok((aura, grandpa))
        })
        .collect()
}

fn parse_endowed_accounts_from_env(var: &str) -> Result<Vec<AccountId>, String> {
    let raw = std::env::var(var)
        .map_err(|_| format!("Missing {}. Expected JSON array of SS58 account IDs", var))?;

    let decoded: Vec<String> =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid {} JSON: {}", var, e))?;
    if decoded.is_empty() {
        return Err(format!("{} cannot be empty", var));
    }

    decoded
        .into_iter()
        .map(|ss58| {
            AccountId::from_ss58check(&ss58)
                .map_err(|e| format!("Invalid SS58 account in {}: {}", var, e))
        })
        .collect()
}

fn load_bootnodes() -> Vec<sc_network::config::MultiaddrWithPeerId> {
    if let Ok(raw) = std::env::var("TESTNET_BOOTNODES") {
        return raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<sc_network::config::MultiaddrWithPeerId>().ok())
            .collect();
    }

    let info_path = PathBuf::from("deployment/keys/bootnode-info.txt");
    let Ok(contents) = std::fs::read_to_string(info_path) else {
        return Vec::new();
    };

    contents
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("/ip4/") || line.starts_with("/dns"))
        .filter_map(|line| line.parse::<sc_network::config::MultiaddrWithPeerId>().ok())
        .collect()
}

fn assert_no_forbidden_live_seed() -> Result<(), String> {
    const FORBIDDEN: &[&str] = &[
        "Alice",
        "Bob",
        "TestnetAlpha",
        "TestnetBeta",
        "TestnetGamma",
        "TestnetDelta",
        "ValidatorAlpha",
        "ValidatorBeta",
        "ValidatorGamma",
        "ValidatorDelta",
        "ValidatorEpsilon",
    ];

    if let Ok(seed_hint) = std::env::var("X3_DEV_SEED") {
        if FORBIDDEN.iter().any(|s| seed_hint.contains(s)) {
            return Err("Refusing Live chain config with known development seed".to_string());
        }
    }
    Ok(())
}

fn assert_no_seed_accounts(endowed_accounts: &[AccountId]) -> Result<(), String> {
    const FORBIDDEN_SEEDS: &[&str] = &[
        "Alice",
        "Bob",
        "Charlie",
        "Dave",
        "Eve",
        "Ferdie",
        "AtlasFoundation",
        "AtlasEcosystem",
        "AtlasCommunity",
        "TestnetFaucet",
        "TestnetAlice",
        "TestnetBob",
        "TestnetCharlie",
        "TestnetDave",
        "TreasuryFoundation",
        "CommunityFund",
        "DevelopmentAllocation",
    ];

    for account in endowed_accounts {
        for seed in FORBIDDEN_SEEDS {
            if let Ok(seed_account) = get_account_id_from_seed::<sr25519::Public>(seed) {
                if account == &seed_account {
                    return Err(format!(
                        "Endowed account {} matches forbidden seed '{}'. Live networks must use pre-generated accounts from config, not seeds.",
                        account.to_ss58check(),
                        seed
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Load the named `ChainSpec` via the supplied identifier string.
///
/// Chain specs can be loaded by:
/// - Name: "dev", "local", "staging", "production"
/// - Environment: $CHAIN_SPEC env var (if no other ID specified)
/// - File path: Any JSON chainspec file path
///
/// Environment variables:
/// - `CHAIN_SPEC`: Override default chain spec (e.g., "dev", "staging", "production")
/// - `X3_ENVIRONMENT`: Set environment tier for auto-config selection
/// - `X3_STAGING_AUTHORITIES`: JSON array of staging validator keys (format: [{"aura":"SS58","grandpa":"SS58"},...])
/// - `X3_STAGING_ENDOWED_ACCOUNTS`: JSON array of staging endowed account SS58 IDs
/// - `X3_STAGING_COUNCIL_MEMBERS`: JSON array of staging council member SS58 IDs
/// - `X3_STAGING_TREASURY_SIGNERS`: JSON array of staging treasury signer SS58 IDs
/// - `X3_TESTNET_AUTHORITIES`: JSON array of testnet validator keys
/// - `X3_TESTNET_ENDOWED_ACCOUNTS`: JSON array of testnet endowed account SS58 IDs
/// - `X3_TESTNET_COUNCIL_MEMBERS`: JSON array of testnet council member SS58 IDs
/// - `X3_TESTNET_TREASURY_SIGNERS`: JSON array of testnet treasury signer SS58 IDs
/// - `X3_PRODUCTION_AUTHORITIES`: JSON array of production validator keys
/// - `X3_PRODUCTION_ENDOWED_ACCOUNTS`: JSON array of production endowed account SS58 IDs
/// - `X3_PRODUCTION_COUNCIL_MEMBERS`: JSON array of production council member SS58 IDs
/// - `X3_PRODUCTION_TREASURY_SIGNERS`: JSON array of production treasury signer SS58 IDs
/// - `TESTNET_BOOTNODES`: Comma-separated list of bootnode multiaddrs
/// - `X3_DEV_SEED`: Development seed hint (must not contain forbidden seeds like "Alice", "Bob")
pub fn load_spec(id: &str) -> Result<Box<dyn ServiceChainSpec>, String> {
    let effective_id = if id.is_empty() {
        // Check environment for chain spec override
        std::env::var("CHAIN_SPEC").unwrap_or_else(|_| "dev".to_string())
    } else {
        id.to_string()
    };

    match effective_id.as_str() {
        "" | "dev" => Ok(Box::new(development_config()?)),
        "local" => Ok(Box::new(local_testnet_config()?)),
        "local3" | "local-3" => Ok(Box::new(local_three_validator_config()?)),
        "staging" => Ok(Box::new(staging_config()?)),
        "testnet" => Ok(Box::new(testnet_config()?)),
        "production" => Ok(Box::new(production_config()?)),
        path => Ok(Box::new(ChainSpec::from_json_file(PathBuf::from(path))?)),
    }
}

fn require_embedded_wasm(chain_name: &str) -> Result<&'static [u8], String> {
    WASM_BINARY.ok_or_else(|| {
        format!(
            "Embedded runtime WASM is missing for '{chain_name}'. \
This node requires an embedded runtime blob at startup. \
Ensure SKIP_WASM_BUILD is unset and rebuild (for example: SKIP_WASM_BUILD= cargo run -p x3-chain-node -- --chain={chain_name})."
        )
    })
}

/// Build the `ChainSpec` powering the development network (local node).
pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary = require_embedded_wasm("dev")?;
    let initial_authorities = vec![authority_keys_from_seed("Alice")?];
    let mut endowed_accounts = vec![
        get_account_id_from_seed::<sr25519::Public>("Alice")?,
        get_account_id_from_seed::<sr25519::Public>("Bob")?,
        get_account_id_from_seed::<sr25519::Public>("Charlie")?,
        get_account_id_from_seed::<sr25519::Public>("Dave")?,
        get_account_id_from_seed::<sr25519::Public>("Eve")?,
        get_account_id_from_seed::<sr25519::Public>("Ferdie")?,
    ];
    endowed_accounts.extend(dev_evm_endowed_accounts());

    // Single-member dev council so EnsureRootOrHalfCouncil-gated calls can be
    // executed via Council::propose(threshold=1, ...) without a Sudo pallet.
    let council_members = vec![get_account_id_from_seed::<sr25519::Public>("Alice")?];

    let genesis_config = x3_chain_genesis(
        initial_authorities,
        endowed_accounts,
        council_members,
        Vec::new(),
        sp_core::H160::zero(),
        [0u8; 32],
    );
    Ok(ChainSpec::builder(wasm_binary, Default::default())
        .with_name("X3 Chain Development")
        .with_id("x3_chain_dev")
        .with_chain_type(ChainType::Development)
        .with_protocol_id(DEFAULT_PROTOCOL_ID)
        .with_genesis_config(
            serde_json::to_value(genesis_config)
                .map_err(|e| format!("Genesis serialization failed: {e}"))?,
        )
        .build())
}

/// Build the local testnet `ChainSpec` used during development.
pub fn local_testnet_config() -> Result<ChainSpec, String> {
    let wasm_binary = require_embedded_wasm("local")?;
    let initial_authorities = vec![
        authority_keys_from_seed("Alice")?,
        authority_keys_from_seed("Bob")?,
    ];
    let mut endowed_accounts = vec![
        get_account_id_from_seed::<sr25519::Public>("Alice")?,
        get_account_id_from_seed::<sr25519::Public>("Bob")?,
        get_account_id_from_seed::<sr25519::Public>("Charlie")?,
        get_account_id_from_seed::<sr25519::Public>("Dave")?,
        get_account_id_from_seed::<sr25519::Public>("Eve")?,
        get_account_id_from_seed::<sr25519::Public>("Ferdie")?,
    ];
    endowed_accounts.extend(dev_evm_endowed_accounts());

    let council_members = vec![
        get_account_id_from_seed::<sr25519::Public>("Alice")?,
        get_account_id_from_seed::<sr25519::Public>("Bob")?,
    ];

    let genesis_config = x3_chain_genesis(
        initial_authorities,
        endowed_accounts,
        council_members,
        Vec::new(),
        sp_core::H160::zero(),
        [0u8; 32],
    );
    Ok(ChainSpec::builder(wasm_binary, Default::default())
        .with_name("X3 Chain Local Testnet")
        .with_id("x3_chain_local")
        .with_chain_type(ChainType::Local)
        .with_protocol_id(DEFAULT_PROTOCOL_ID)
        .with_genesis_config(
            serde_json::to_value(genesis_config)
                .map_err(|e| format!("Genesis serialization failed: {e}"))?,
        )
        .build())
}

/// Build a local 3-validator `ChainSpec` (Alice/Bob/Charlie) for MVP consensus validation.
pub fn local_three_validator_config() -> Result<ChainSpec, String> {
    let wasm_binary = require_embedded_wasm("local3")?;
    let initial_authorities = vec![
        authority_keys_from_seed("Alice")?,
        authority_keys_from_seed("Bob")?,
        authority_keys_from_seed("Charlie")?,
    ];
    let mut endowed_accounts = vec![
        get_account_id_from_seed::<sr25519::Public>("Alice")?,
        get_account_id_from_seed::<sr25519::Public>("Bob")?,
        get_account_id_from_seed::<sr25519::Public>("Charlie")?,
        get_account_id_from_seed::<sr25519::Public>("Dave")?,
        get_account_id_from_seed::<sr25519::Public>("Eve")?,
        get_account_id_from_seed::<sr25519::Public>("Ferdie")?,
    ];
    endowed_accounts.extend(dev_evm_endowed_accounts());

    let council_members = vec![
        get_account_id_from_seed::<sr25519::Public>("Alice")?,
        get_account_id_from_seed::<sr25519::Public>("Bob")?,
        get_account_id_from_seed::<sr25519::Public>("Charlie")?,
    ];

    let genesis_config = x3_chain_genesis(
        initial_authorities,
        endowed_accounts,
        council_members,
        Vec::new(),
        sp_core::H160::zero(),
        [0u8; 32],
    );
    Ok(ChainSpec::builder(wasm_binary, Default::default())
        .with_name("X3 Chain Local 3-Validator Testnet")
        .with_id("x3_chain_local3")
        .with_chain_type(ChainType::Local)
        .with_protocol_id(DEFAULT_PROTOCOL_ID)
        .with_genesis_config(
            serde_json::to_value(genesis_config)
                .map_err(|e| format!("Genesis serialization failed: {e}"))?,
        )
        .build())
}

/// Build the staging `ChainSpec` matching the release network parameters.
pub fn staging_config() -> Result<ChainSpec, String> {
    assert_no_forbidden_live_seed()?;
    // Staging networks are expected to have a proper WASM runtime embedded.
    // Keep the strict check here so that any missing or invalid blob fails
    // fast during chain spec construction.
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| "X3 Chain WASM binary not available".to_string())?;
    let initial_authorities = parse_authorities_from_env("X3_STAGING_AUTHORITIES")?;
    let bootnodes = load_bootnodes();
    let endowed_accounts = parse_endowed_accounts_from_env("X3_STAGING_ENDOWED_ACCOUNTS")?;
    assert_no_seed_accounts(&endowed_accounts)?;
    let council_members = parse_endowed_accounts_from_env("X3_STAGING_COUNCIL_MEMBERS")?;
    let treasury_signers = parse_endowed_accounts_from_env("X3_STAGING_TREASURY_SIGNERS")?;
    if council_members.is_empty() {
        return Err("Staging network requires at least one council member".to_string());
    }
    if treasury_signers.is_empty() {
        return Err("Staging network requires at least one treasury signer".to_string());
    }
    let evm_escrow = parse_escrow_addr_from_env("X3_EVM_ESCROW_ADDR");
    let svm_escrow = parse_svm_escrow_from_env("X3_SVM_ESCROW_ADDR");
    if evm_escrow.is_zero() {
        return Err("Staging network requires non-zero EVM escrow address".to_string());
    }
    if svm_escrow == [0u8; 32] {
        return Err("Staging network requires non-zero SVM escrow address".to_string());
    }

    let genesis_config = x3_chain_genesis(
        initial_authorities,
        endowed_accounts,
        council_members,
        treasury_signers,
        evm_escrow,
        svm_escrow,
    );
    Ok(ChainSpec::builder(wasm_binary, Default::default())
        .with_name("X3 Chain Staging")
        .with_id("x3_chain_staging")
        .with_chain_type(ChainType::Live)
        .with_protocol_id(DEFAULT_PROTOCOL_ID)
        .with_boot_nodes(bootnodes)
        .with_genesis_config(
            serde_json::to_value(genesis_config)
                .map_err(|e| format!("Genesis serialization failed: {e}"))?,
        )
        .build())
}

/// Build a multi-validator testnet `ChainSpec` for external testing.
///
/// This config is designed for public testnet deployments with:
/// - 3+ validators (TestnetAlpha, TestnetBeta, TestnetGamma, TestnetDelta)
/// - Faucet accounts for distributing test tokens
/// - ChainType::Live for realistic peer-to-peer networking
///
/// Usage:
/// ```bash
/// x3-chain-node --chain=testnet --validator --name=TestnetAlpha
/// x3-chain-node --chain=testnet --validator --name=TestnetBeta
/// x3-chain-node --chain=testnet --validator --name=TestnetGamma
/// x3-chain-node --chain=testnet --validator --name=TestnetDelta
/// ```
pub fn testnet_config() -> Result<ChainSpec, String> {
    assert_no_forbidden_live_seed()?;
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| "X3 Chain WASM binary not available for testnet".to_string())?;

    let initial_authorities = parse_authorities_from_env("X3_TESTNET_AUTHORITIES")?;
    let bootnodes = load_bootnodes();

    let endowed_accounts = parse_endowed_accounts_from_env("X3_TESTNET_ENDOWED_ACCOUNTS")?;
    assert_no_seed_accounts(&endowed_accounts)?;
    let council_members = parse_endowed_accounts_from_env("X3_TESTNET_COUNCIL_MEMBERS")?;
    let treasury_signers = parse_endowed_accounts_from_env("X3_TESTNET_TREASURY_SIGNERS")?;
    if council_members.is_empty() {
        return Err("Testnet network requires at least one council member".to_string());
    }
    if treasury_signers.is_empty() {
        return Err("Testnet network requires at least one treasury signer".to_string());
    }
    let evm_escrow = parse_escrow_addr_from_env("X3_EVM_ESCROW_ADDR");
    let svm_escrow = parse_svm_escrow_from_env("X3_SVM_ESCROW_ADDR");
    if evm_escrow.is_zero() {
        return Err("Testnet network requires non-zero EVM escrow address".to_string());
    }
    if svm_escrow == [0u8; 32] {
        return Err("Testnet network requires non-zero SVM escrow address".to_string());
    }

    let genesis_config = x3_chain_genesis(
        initial_authorities,
        endowed_accounts,
        council_members,
        treasury_signers,
        evm_escrow,
        svm_escrow,
    );
    Ok(ChainSpec::builder(wasm_binary, Default::default())
        .with_name("X3 Chain Testnet")
        .with_id("x3_chain_testnet")
        .with_chain_type(ChainType::Live)
        .with_protocol_id(DEFAULT_PROTOCOL_ID)
        .with_boot_nodes(bootnodes)
        .with_genesis_config(
            serde_json::to_value(genesis_config)
                .map_err(|e| format!("Genesis serialization failed: {e}"))?,
        )
        .build())
}

/// Build the production `ChainSpec` for mainnet deployment.
///
/// Production network requires:
/// - Valid WASM runtime (no fallback to native-only)
/// - Multiple validators for security
/// - Production-grade authorities and bootstrap nodes
/// - Persistent network configuration
///
/// # Safety
/// This function will fail if WASM binary is not available, ensuring
/// only properly built binaries can boot the network.
pub fn production_config() -> Result<ChainSpec, String> {
    assert_no_forbidden_live_seed()?;
    let wasm_binary = WASM_BINARY
        .ok_or_else(|| "X3 Chain WASM binary not available for production".to_string())?;

    let initial_authorities = parse_authorities_from_env("X3_PRODUCTION_AUTHORITIES")?;
    let bootnodes = load_bootnodes();

    let endowed_accounts = parse_endowed_accounts_from_env("X3_PRODUCTION_ENDOWED_ACCOUNTS")?;
    assert_no_seed_accounts(&endowed_accounts)?;
    let council_members = parse_endowed_accounts_from_env("X3_PRODUCTION_COUNCIL_MEMBERS")?;
    let treasury_signers = parse_endowed_accounts_from_env("X3_PRODUCTION_TREASURY_SIGNERS")?;
    if council_members.is_empty() {
        return Err("Production network requires at least one council member".to_string());
    }
    if treasury_signers.is_empty() {
        return Err("Production network requires at least one treasury signer".to_string());
    }
    let evm_escrow = parse_escrow_addr_from_env("X3_EVM_ESCROW_ADDR");
    let svm_escrow = parse_svm_escrow_from_env("X3_SVM_ESCROW_ADDR");
    if evm_escrow.is_zero() {
        return Err("Production network requires non-zero EVM escrow address".to_string());
    }
    if svm_escrow == [0u8; 32] {
        return Err("Production network requires non-zero SVM escrow address".to_string());
    }

    let genesis_config = x3_chain_genesis(
        initial_authorities,
        endowed_accounts,
        council_members,
        treasury_signers,
        evm_escrow,
        svm_escrow,
    );
    Ok(ChainSpec::builder(wasm_binary, Default::default())
        .with_name("X3 Chain Production")
        .with_id("x3_chain_production")
        .with_chain_type(ChainType::Live)
        .with_protocol_id(DEFAULT_PROTOCOL_ID)
        .with_boot_nodes(bootnodes)
        .with_genesis_config(
            serde_json::to_value(genesis_config)
                .map_err(|e| format!("Genesis serialization failed: {e}"))?,
        )
        .build())
}

/// Parse a 20-byte EVM address from an env var (hex with or without 0x prefix).
/// Returns `H160::zero()` if the var is absent or unparseable.
fn parse_escrow_addr_from_env(var: &str) -> sp_core::H160 {
    std::env::var(var)
        .ok()
        .and_then(|s| {
            let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
            let bytes = hex::decode(s).ok()?;
            if bytes.len() == 20 {
                let mut arr = [0u8; 20];
                arr.copy_from_slice(&bytes);
                Some(sp_core::H160(arr))
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Parse a 32-byte SVM address from an env var (hex with or without 0x prefix, or base58).
/// Returns `[0u8; 32]` if the var is absent or unparseable.
fn parse_svm_escrow_from_env(var: &str) -> [u8; 32] {
    std::env::var(var)
        .ok()
        .and_then(|s| {
            let s = s.trim();
            // Try hex first
            let hex_s = s.trim_start_matches("0x").trim_start_matches("0X");
            if let Ok(bytes) = hex::decode(hex_s) {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    return Some(arr);
                }
            }
            // Try base58
            if let Ok(bytes) = bs58::decode(s).into_vec() {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    return Some(arr);
                }
            }
            None
        })
        .unwrap_or([0u8; 32])
}

fn x3_chain_genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    endowed_accounts: Vec<AccountId>,
    council_members: Vec<AccountId>,
    treasury_signers: Vec<AccountId>,
    evm_escrow_addr: sp_core::H160,
    svm_escrow_addr: [u8; 32],
) -> RuntimeGenesisConfig {
    let mut endowed: BTreeSet<AccountId> = endowed_accounts.into_iter().collect();

    // Add authority accounts to endowed set
    for (aura, _) in initial_authorities.iter() {
        // Derive account from Aura public key
        let mut account_bytes = [0u8; 32];
        account_bytes.copy_from_slice(&aura.encode()[..32]);
        let account_id = AccountId::from(account_bytes);
        endowed.insert(account_id);
    }

    let balances = endowed
        .iter()
        .cloned()
        .map(|account| (account, ENDOWMENT))
        .collect::<Vec<_>>();

    let grandpa_authorities: Vec<(GrandpaId, u64)> = initial_authorities
        .iter()
        .map(|(_, grandpa)| (grandpa.clone(), 1))
        .collect();

    let aura_authorities: Vec<AuraId> = initial_authorities
        .iter()
        .map(|(aura, _)| aura.clone())
        .collect();

    RuntimeGenesisConfig {
        system: Default::default(),
        balances: BalancesConfig {
            balances,
            dev_accounts: None,
        },
        aura: AuraConfig {
            authorities: aura_authorities,
        },
        grandpa: GrandpaConfig {
            authorities: grandpa_authorities,
            _config: Default::default(),
        },
        atlas_kernel: AtlasKernelConfig {
            assets: x3_kernel_default_assets(),
            evm_escrow: evm_escrow_addr,
            svm_escrow: svm_escrow_addr,
        },
        transaction_payment: Default::default(),
        council: CouncilConfig {
            members: council_members,
            phantom: Default::default(),
        },
        #[cfg(feature = "frontier")]
        evm: Default::default(),
        governance: Default::default(),
        treasury: TreasuryConfig {
            initial_signers: treasury_signers,
        },
        x3_coin: X3CoinConfig {
            team_allocations: Vec::new(),
            ecosystem_allocations: Vec::new(),
            liquidity_allocations: Vec::new(),
        },
        session: Default::default(),
        #[cfg(feature = "frontier")]
        ethereum: Default::default(),
        evolution_core: Default::default(),
        x3_verifier: Default::default(),
        depin_marketplace: Default::default(),
        private_execution: Default::default(),
        x3_invariants: Default::default(),
        x3_dapp_hub: Default::default(),
        x3_flash_loan: Default::default(),
    }
}

fn authority_keys_from_seed(seed: &str) -> Result<(AuraId, GrandpaId), String> {
    Ok((
        get_from_seed::<AuraId>(seed)?,
        get_from_seed::<GrandpaId>(seed)?,
    ))
}

fn get_account_id_from_seed<TPublic>(seed: &str) -> Result<AccountId, String>
where
    AccountPublic: From<TPublic>,
    TPublic: Public,
    TPublic::Pair: Pair,
    TPublic: From<<TPublic::Pair as Pair>::Public>,
{
    Ok(AccountPublic::from(get_from_seed::<TPublic>(seed)?).into_account())
}

fn get_from_seed<TPublic>(seed: &str) -> Result<TPublic, String>
where
    TPublic: Public,
    TPublic::Pair: Pair,
    TPublic: From<<TPublic::Pair as Pair>::Public>,
{
    let public: TPublic = TPublic::Pair::from_string(&format!("//{}", seed), None)
        .map_err(|e| format!("invalid seed '{}': {}", seed, e))?
        .public()
        .into();

    Ok(public)
}
