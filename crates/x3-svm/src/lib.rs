//! X3 SVM (Solana Virtual Machine) Compatibility Layer
//!
//! Solana program implementations, CPI routing, and fork simulation for devnet testing.

pub mod solana_programs;
pub mod anchor_idl_parser;
pub mod spl_token_bridge;
pub mod solana_devnet_fork;

pub use solana_programs::{SystemProgram, TokenProgram, TokenAccount, AssociatedTokenAccount, MemoProgram, SolanaPrograms};
pub use anchor_idl_parser::{AnchorIDL, AnchorIDLParser, InstructionDef, AccountDef, TypeDef, EventDef, ErrorDef, GeneratedCode};
pub use spl_token_bridge::{SPLTokenMint, SPLTokenBridge, BridgeVault, TokenBridgeRequest, WrappedToken, BridgedBalance};
pub use solana_devnet_fork::{SolanaDevnetFork, DevnetForkConfig, ForkState, ForkedAccount, ForkSnapshot, TransactionLog, ComputeMetrics};
