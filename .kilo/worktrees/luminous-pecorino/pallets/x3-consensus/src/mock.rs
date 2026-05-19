//! Mock runtime for consensus pallet tests

use crate::*;
use frame_support::{derive_impl, parameter_types, traits::ConstBool};
use frame_system as system;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::H256;
use sp_runtime::{
    impl_opaque_keys,
    testing::UintAuthorityId,
    traits::{BlakeTwo256, ConvertInto, IdentityLookup, OpaqueKeys},
    BuildStorage, RuntimeAppPublic,
};

type Block = frame_system::mocking::MockBlock<Test>;

impl_opaque_keys! {
    pub struct MockSessionKeys {
        pub dummy: UintAuthorityId,
    }
}

pub struct MockSessionHandler;
impl pallet_session::SessionHandler<u64> for MockSessionHandler {
    const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[UintAuthorityId::ID];
    fn on_genesis_session<Ks: OpaqueKeys>(_keys: &[(u64, Ks)]) {}
    fn on_new_session<Ks: OpaqueKeys>(_changed: bool, _validators: &[(u64, Ks)], _queued_validators: &[(u64, Ks)]) {}
    fn on_before_session_ending() {}
    fn on_disabled(_validator_index: u32) {}
}

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        Session: pallet_session,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Consensus: crate,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaxValidators: u32 = 100;
    pub const MaxAuthorities: u32 = 100;
    pub const ExistentialDeposit: u64 = 1;
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
    pub const MinimumPeriod: u64 = 1;
    /// 10 % slash per offence report.
    pub const SlashFraction: u32 = 1_000;
    /// Minimum stake that may remain after any single slash.
    pub const MinStakeAfterSlash: u128 = 1_000_000;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

impl pallet_session::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = u64;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = ();
    type SessionHandler = MockSessionHandler;
    type Keys = MockSessionKeys;
    type WeightInfo = ();
    type DisablingStrategy = pallet_session::disabling::UpToLimitDisablingStrategy;
    type Currency = Balances;
    type KeyDeposit = ();
}

impl pallet_aura::Config for Test {
    type AuthorityId = AuraId;
    type MaxAuthorities = MaxAuthorities;
    type DisabledValidators = ();
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
    type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Test>;
}

impl pallet_grandpa::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxSetIdSessionEntries = frame_support::traits::ConstU64<0>;
    type MaxNominators = frame_support::traits::ConstU32<0>;
}

pub struct MockWeightInfo;
impl crate::weights::WeightInfo for MockWeightInfo {
    fn set_validators() -> frame_support::weights::Weight {
        frame_support::weights::Weight::zero()
    }
    fn report_misbehavior() -> frame_support::weights::Weight {
        frame_support::weights::Weight::zero()
    }
    fn on_initialize() -> frame_support::weights::Weight {
        frame_support::weights::Weight::zero()
    }
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MaxValidators = MaxValidators;
    type SlashFraction = SlashFraction;
    type MinStakeAfterSlash = MinStakeAfterSlash;
    type WeightInfo = MockWeightInfo;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}
