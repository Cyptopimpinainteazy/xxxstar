//! X3 SVM (Solana Virtual Machine) Compatibility Layer
//!
//! Solana program implementations, CPI routing, and fork simulation for devnet testing.

pub mod anchor_idl_parser;
pub mod solana_devnet_fork;
pub mod solana_programs;
pub mod spl_token_bridge;

pub use anchor_idl_parser::{
    AccountDef, AnchorIDL, AnchorIDLParser, ErrorDef, EventDef, GeneratedCode, InstructionDef,
    TypeDef,
};
pub use solana_devnet_fork::{
    ComputeMetrics, DevnetForkConfig, ForkSnapshot, ForkState, ForkedAccount, SolanaDevnetFork,
    TransactionLog,
};
pub use solana_programs::{
    AssociatedTokenAccount, MemoProgram, SolanaPrograms, SystemProgram, TokenAccount, TokenProgram,
};
pub use spl_token_bridge::{
    BridgeVault, BridgedBalance, SPLTokenBridge, SPLTokenMint, TokenBridgeRequest, WrappedToken,
};
