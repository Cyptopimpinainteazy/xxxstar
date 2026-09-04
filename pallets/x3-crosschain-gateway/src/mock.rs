#![allow(deprecated)]
use crate as pallet_x3_crosschain_gateway;
use crate::*;

use frame_support::{construct_runtime, parameter_types, traits::ConstU32, BoundedVec};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

pub type AccountId = u64;
pub type Balance = u128;
pub type _BlockNumber = u64;

pub type _UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        X3CrosschainGateway: pallet_x3_crosschain_gateway,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const DailyWindow: u64 = 100;
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type SS58Prefix = ();
    type OnSetCode = ();
    type SystemWeightInfo = ();
    type ExtensionsWeightInfo = ();
    type RuntimeTask = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_x3_crosschain_gateway::Config for Test {
    type GovernanceOrigin = frame_system::EnsureRoot<AccountId>;
    type RelayerOrigin = frame_system::EnsureSigned<AccountId>;
    type OperationalOrigin = frame_system::EnsureSigned<AccountId>;
    type DailyLimitWindowBlocks = DailyWindow;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub fn bounded(s: &str) -> BoundedVec<u8, ConstU32<128>> {
    BoundedVec::try_from(s.as_bytes().to_vec()).unwrap()
}

pub fn bounded_payload(s: &str) -> BoundedVec<u8, ConstU32<4096>> {
    BoundedVec::try_from(s.as_bytes().to_vec()).unwrap()
}

pub fn asset() -> ExternalAssetRef {
    ExternalAssetRef {
        chain_id: ExternalChainId::BaseSepolia,
        token_address_or_mint: bounded("0xTOKEN"),
    }
}

pub fn route() -> RouteConfig {
    RouteConfig {
        route_id: [1u8; 32],
        external_chain_id: ExternalChainId::BaseSepolia,
        external_asset: asset(),
        x3_asset_id: [9u8; 32],
        destination_domain: X3Domain::Native,
        enabled: false,
        min_amount: 1,
        max_amount: 100_000,
        daily_limit: 10_000,
        daily_deposited: 0,
        daily_reset_at_block: 0,
        pending_limit: 10,
        finality_requirement: 12,
        verification_level: RouteVerificationLevel::X3Internal,
        fee_bps: 10,
        mode: GatewayMode::TestnetLive,
        require_dispute_window: false,
        dispute_window_blocks: 0,
        contract_address: bounded("0xGATEWAY"),
    }
}

pub fn deposit_proof(proof_id: ProofId, amount: Balance) -> DepositProof {
    deposit_proof_with(
        proof_id,
        amount,
        ExternalChainId::BaseSepolia,
        asset(),
        1,
        bounded_payload("valid_payload"),
    )
}

pub fn deposit_proof_with(
    proof_id: ProofId,
    amount: Balance,
    chain: ExternalChainId,
    external_asset: ExternalAssetRef,
    nonce: u64,
    proof_payload: BoundedVec<u8, ConstU32<4096>>,
) -> DepositProof {
    DepositProof {
        version: 1,
        proof_id,
        source_chain: chain,
        source_block: 100,
        source_tx_hash: [7u8; 32],
        event_index: 0,
        external_asset,
        sender: bounded("0xSENDER"),
        recipient: bounded("0xRECIPIENT"),
        amount,
        nonce,
        observed_at_block: 110,
        finalized_at_block: 120,
        proof_payload,
    }
}

pub fn bounded20(data: [u8; 20]) -> BoundedVec<u8, ConstU32<128>> {
    BoundedVec::try_from(data.to_vec()).unwrap()
}

// ── Asset helpers ─────────────────────────────────────────────────────────

pub fn solana_asset() -> ExternalAssetRef {
    ExternalAssetRef {
        chain_id: ExternalChainId::SolanaDevnet,
        token_address_or_mint: bounded("0xSOLTOKEN"),
    }
}

pub fn bitcoin_asset() -> ExternalAssetRef {
    ExternalAssetRef {
        chain_id: ExternalChainId::BitcoinTestnet,
        token_address_or_mint: bounded("0xBTCTOKEN"),
    }
}

// ── Route helpers ─────────────────────────────────────────────────────────

pub fn validator_route() -> RouteConfig {
    RouteConfig {
        route_id: [2u8; 32],
        x3_asset_id: [9u8; 32],
        verification_level: RouteVerificationLevel::ValidatorQuorum {
            threshold: 2,
            total: 3,
        },
        ..route()
    }
}

pub fn solana_route() -> RouteConfig {
    RouteConfig {
        route_id: [3u8; 32],
        external_chain_id: ExternalChainId::SolanaDevnet,
        external_asset: solana_asset(),
        x3_asset_id: [10u8; 32],
        verification_level: RouteVerificationLevel::SolanaFinalizedProof,
        ..route()
    }
}

pub fn _bitcoin_route() -> RouteConfig {
    RouteConfig {
        route_id: [4u8; 32],
        external_chain_id: ExternalChainId::BitcoinTestnet,
        external_asset: bitcoin_asset(),
        x3_asset_id: [11u8; 32],
        verification_level: RouteVerificationLevel::BitcoinSpvProof,
        ..route()
    }
}

pub fn evm_route() -> RouteConfig {
    RouteConfig {
        route_id: [5u8; 32],
        x3_asset_id: [9u8; 32],
        verification_level: RouteVerificationLevel::EvmReceiptProof,
        contract_address: bounded20([0xaa; 20]),
        ..route()
    }
}

// ── Proof payload helpers ─────────────────────────────────────────────────

pub fn valid_proof_payload() -> BoundedVec<u8, ConstU32<4096>> {
    bounded_payload("some_valid_payload_data_1234567890")
}
