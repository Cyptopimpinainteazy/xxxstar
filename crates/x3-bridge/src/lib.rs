#![allow(unused, dead_code, deprecated)]

//! X3 Bridge Infrastructure
//!
//! Cross-chain bridges: Ethereum, Solana, Cosmos, IBC, L2, Bitcoin, governance, and fee abstraction.

pub mod bitcoin_htlc;
/// Bitcoin SPV: one implementation, `x3-btc-spv`, shared with the gateway's verifier and the
/// settlement engine. This crate carried its own until it was found unsound and unused: it
/// compared hashes against targets in the wrong byte order, accepted any `nBits` (so a header at
/// a trivial target passed), applied Merkle paths without regard to position and refused
/// one-transaction blocks, and counted confirmations from a height the caller supplied.
pub use x3_btc_spv as btc_spv;
pub mod cross_chain_account;
pub mod cross_chain_proofs;
pub mod ethereum_bridge;
pub mod gas_relayer;
pub mod ibc_light_client;
pub mod l2_bridge;
pub mod security_council;
pub mod wormhole_adapter;

pub use bitcoin_htlc::{BitcoinAddress, BitcoinHTLC, HTLCContract, Preimage};
pub use cross_chain_account::{CrossChainAccount, CrossChainAccountManager, DerivedAddress};
pub use cross_chain_proofs::*;
pub use ethereum_bridge::{BridgeDeposit, ERC20Token, EthereumBridge};
pub use gas_relayer::{FeeRequest, GasRelayer, RelayerConfig, SponsorPool};
pub use ibc_light_client::{ConsensusState, CosmosChainInfo, IBCLightClient, IBCPacket};
pub use l2_bridge::{L2Bridge, L2BridgeDeposit, L2Withdrawal, OutputRoot};
pub use security_council::{BridgeSecurityCouncil, Proposal, ProposalType};
pub use wormhole_adapter::{WormholeBridge, WrappedSPLToken, VAA};
pub use x3_btc_spv::{BlockHeader as BitcoinBlockHeader, LightClient as BtcBlockchain};
