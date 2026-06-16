#![cfg(test)]

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
pub type BlockNumber = u64;

pub type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
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
    }
}

pub fn deposit_proof(proof_id: ProofId, amount: Balance) -> DepositProof {
    DepositProof {
        version: 1,
        proof_id,
        source_chain: ExternalChainId::BaseSepolia,
        source_block: 100,
        source_tx_hash: [7u8; 32],
        event_index: 0,
        external_asset: asset(),
        sender: bounded("0xSENDER"),
        recipient: bounded("0xRECIPIENT"),
        amount,
        nonce: 1,
        observed_at_block: 110,
        finalized_at_block: 120,
        proof_payload: bounded_payload("valid_payload"),
    }
}
