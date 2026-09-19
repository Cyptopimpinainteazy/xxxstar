//! X3 Wallet CLI - Command-line interface for wallet operations.
//!
//! Supports hardware wallets, multisig, recovery, biometric, and DEX swaps.

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::error::Error as StdError;

#[derive(Parser, Debug)]
#[command(name = "x3-wallet")]
#[command(about = "X3 Chain Wallet CLI - Manage hardware wallets, multisig accounts, and execute swaps", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true, default_value = "http://127.0.0.1:9944")]
    rpc_endpoint: String,

    #[arg(long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Hardware wallet operations
    Hardware(HardwareCmd),

    /// Multisig wallet operations
    Multisig(MultisigCmd),

    /// Recovery & guardian operations
    Recovery(RecoveryCmd),

    /// Account & balance management
    Account(AccountCmd),

    /// Sign & submit transactions
    Transaction(TransactionCmd),

    /// DEX swap operations
    Swap(SwapCmd),

    /// Biometric enrollment & verification
    Biometric(BiometricCmd),

    /// View wallet status
    Status(StatusCmd),
}

#[derive(Parser, Debug)]
struct HardwareCmd {
    #[command(subcommand)]
    action: HardwareAction,
}

#[derive(Subcommand, Debug)]
enum HardwareAction {
    /// Register a new hardware wallet
    Register {
        #[arg(long)]
        device_type: String,
        #[arg(long)]
        device_model: String,
        #[arg(long)]
        public_key: String,
    },
    /// List connected hardware wallets
    List,
    /// Verify hardware connection
    Verify {
        #[arg(long)]
        device_id: String,
    },
}

#[derive(Parser, Debug)]
struct MultisigCmd {
    #[command(subcommand)]
    action: MultisigAction,
}

#[derive(Subcommand, Debug)]
enum MultisigAction {
    /// Create a new multisig wallet
    Create {
        #[arg(long)]
        signers: String,
        #[arg(long)]
        threshold: u32,
        #[arg(long)]
        delay: Option<u32>,
    },
    /// Get multisig wallet info
    Info {
        #[arg(long)]
        wallet_id: String,
    },
    /// Propose a multisig transaction
    Propose {
        #[arg(long)]
        wallet_id: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u128,
    },
    /// Approve pending transaction
    Approve {
        #[arg(long)]
        tx_id: String,
    },
    /// Execute approved transaction
    Execute {
        #[arg(long)]
        tx_id: String,
    },
}

#[derive(Parser, Debug)]
struct RecoveryCmd {
    #[command(subcommand)]
    action: RecoveryAction,
}

#[derive(Subcommand, Debug)]
enum RecoveryAction {
    /// Add a recovery guardian
    AddGuardian {
        #[arg(long)]
        guardian_address: String,
        #[arg(long)]
        guardian_type: String, // family, friend, service
    },
    /// Initiate account recovery
    Initiate {
        #[arg(long)]
        new_owner: String,
    },
    /// Approve recovery as guardian
    Approve {
        #[arg(long)]
        recovery_id: String,
    },
    /// List guardians
    ListGuardians,
}

#[derive(Parser, Debug)]
struct AccountCmd {
    #[command(subcommand)]
    action: AccountAction,
}

#[derive(Subcommand, Debug)]
enum AccountAction {
    /// Get account balance
    Balance {
        #[arg(long)]
        account: Option<String>,
        #[arg(long)]
        token_id: Option<String>,
    },
    /// Add address to contact book
    AddContact {
        #[arg(long)]
        name: String,
        #[arg(long)]
        address: String,
        #[arg(long)]
        network: Option<String>,
    },
    /// List saved contacts
    ListContacts,
    /// Import account from seed
    Import {
        #[arg(long)]
        mnemonic: String,
    },
    /// Export account (encrypted)
    Export {
        #[arg(long)]
        password: String,
    },
}

#[derive(Parser, Debug)]
struct TransactionCmd {
    #[command(subcommand)]
    action: TransactionAction,
}

#[derive(Subcommand, Debug)]
enum TransactionAction {
    /// Sign a transaction with hardware wallet
    Sign {
        #[arg(long)]
        tx_data: String,
        #[arg(long)]
        wallet_id: String,
    },
    /// Submit a signed transaction
    Submit {
        #[arg(long)]
        signed_tx: String,
    },
    /// Get transaction status
    Status {
        #[arg(long)]
        tx_hash: String,
    },
    /// Estimate transaction fees
    EstimateFee {
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u128,
    },
}

#[derive(Parser, Debug)]
struct SwapCmd {
    #[command(subcommand)]
    action: SwapAction,
}

#[derive(Subcommand, Debug)]
enum SwapAction {
    /// Estimate swap output
    Estimate {
        #[arg(long)]
        token_in: String,
        #[arg(long)]
        token_out: String,
        #[arg(long)]
        amount: u128,
    },
    /// Execute a DEX swap
    Execute {
        #[arg(long)]
        token_in: String,
        #[arg(long)]
        token_out: String,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        min_output: u128,
        #[arg(long)]
        wallet_id: Option<String>,
    },
    /// Approve token for swapping
    Approve {
        #[arg(long)]
        token: String,
        #[arg(long)]
        amount: u128,
    },
    /// Get swap history
    History {
        #[arg(long)]
        limit: Option<u32>,
    },
}

#[derive(Parser, Debug)]
struct BiometricCmd {
    #[command(subcommand)]
    action: BiometricAction,
}

#[derive(Subcommand, Debug)]
enum BiometricAction {
    /// Enroll biometric
    Enroll {
        #[arg(long)]
        biometric_type: String, // fingerprint, face, iris
    },
    /// Verify biometric
    Verify {
        #[arg(long)]
        biometric_type: String,
    },
    /// Require biometric for approvals
    RequireForApproval {
        #[arg(long)]
        enabled: bool,
    },
}

#[derive(Parser, Debug)]
struct StatusCmd {
    /// Show full wallet status
    #[arg(long)]
    full: bool,

    /// Pretty-print JSON
    #[arg(long)]
    json: bool,
}

/// `main` used to be `async fn main` with no executor attribute — the compiler
/// rejects that outright, so the binary had never built. Nothing awaits any
/// more, so it is a plain `fn`.
fn main() -> Result<(), Box<dyn StdError>> {
    let args = Args::parse();

    if args.verbose {
        println!("{}", "X3 Wallet CLI - Verbose Mode".yellow().bold());
        println!("RPC Endpoint: {}", args.rpc_endpoint.cyan());
    }

    let command = match args.command {
        Commands::Hardware(cmd) => format!("hardware {}", action_name(&cmd.action)),
        Commands::Multisig(cmd) => format!("multisig {}", action_name(&cmd.action)),
        Commands::Recovery(cmd) => format!("recovery {}", action_name(&cmd.action)),
        Commands::Account(cmd) => format!("account {}", action_name(&cmd.action)),
        Commands::Transaction(cmd) => format!("transaction {}", action_name(&cmd.action)),
        Commands::Swap(cmd) => format!("swap {}", action_name(&cmd.action)),
        Commands::Biometric(cmd) => format!("biometric {}", action_name(&cmd.action)),
        Commands::Status(cmd) => format!("status (full: {}, json: {})", cmd.full, cmd.json),
    };

    not_implemented(&command)
}

/// Every command in this CLI is a stub.
///
/// The handlers used to print what they *would* do — "Creating 3-of-5 multisig
/// wallet", "Adding hardware guardian: …", with "// Implementation: …" in the
/// source — and then return `Ok(())`. A user therefore saw a confirmation and a
/// zero exit status for an operation that never happened, and `rpc_endpoint` was
/// accepted and never used. Until the RPC plumbing exists, the CLI reports the
/// truth and exits non-zero.
fn not_implemented(command: &str) -> Result<(), Box<dyn StdError>> {
    Err(format!("`{command}` is not implemented: this CLI does not talk to a node yet").into())
}

/// Name of the subcommand variant, for the message above.
fn action_name<T: std::fmt::Debug>(action: &T) -> String {
    format!("{action:?}")
        .split(['(', ' ', '{'])
        .next()
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The CLI must never report success for work it did not do. This is the
    /// contract the print-only handlers broke.
    #[test]
    fn commands_fail_loudly_instead_of_printing_confirmations() {
        let err = not_implemented("multisig Create").expect_err("stubs must fail");
        let message = err.to_string();
        assert!(message.contains("not implemented"), "{message}");
        assert!(
            message.contains("multisig Create"),
            "the message names the command: {message}"
        );
    }

    #[test]
    fn action_names_are_the_variant_names() {
        assert_eq!(action_name(&HardwareAction::List), "List");
        assert_eq!(
            action_name(&MultisigAction::Approve {
                tx_id: "0x01".to_string()
            }),
            "Approve"
        );
    }
}
