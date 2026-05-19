pub struct RuntimeVersion;
impl frame_support::traits::Get<sp_version::RuntimeVersion> for RuntimeVersion {
    fn get() -> sp_version::RuntimeVersion {
        VERSION
    }
}
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

pub const NANO_ATLAS: Balance = 1;
pub const MICRO_ATLAS: Balance = 1_000 * NANO_ATLAS;
pub const MILLI_ATLAS: Balance = 1_000 * MICRO_ATLAS;
pub const X3: Balance = 1_000 * MILLI_ATLAS;
pub const NATIVE_GAS_PRICE: u64 = 1_000_000_000;

#[sp_version::runtime_version]
pub const VERSION: sp_version::RuntimeVersion = sp_version::RuntimeVersion {
    spec_name: create_runtime_str!("x3-chain"),
    impl_name: create_runtime_str!("x3-chain"),
    authoring_version: 1,
    // v5: 200ms slot duration migration. Nodes MUST check spec_version to select
    // the correct slot duration for pre/post-upgrade blocks to prevent Aura
    // slot monotonicity failures. See node/src/service.rs slot_duration_for_spec().
    spec_version: 5,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2_400;
    pub const SS58Prefix: u16 = 42;
    pub const MinimumPeriod: Moment = (MILLISECS_PER_BLOCK / 2) as Moment;
    pub const ExistentialDeposit: Balance = 100 * MICRO_ATLAS;
    pub const TransactionByteFee: Balance = 10 * MICRO_ATLAS;
    pub const MaxAssetsPerAccount: u32 = 32;
    pub const MaxAssetSymbolLength: u32 = 16;
    pub const MaxPayloadLength: u32 = 128 * 1024;
    pub const MaxEvmPayloadLength: u32 = 64 * 1024;  // 64 KB for EVM payloads
    pub const MaxSvmPayloadLength: u32 = 64 * 1024;  // 64 KB for SVM payloads
    pub const MaxX3PayloadLength: u32 = 64 * 1024;  // 64 KB for X3 payloads
    pub const MaxCombinedPayloadLength: u32 = 128 * 1024;  // 128 KB combined limit
    pub const MaxCombinedPayloadLengthV2: u32 = 192 * 1024;  // 192 KB combined (EVM+SVM+X3)
    pub const MaxAuthorities: u32 = 100;  // Maximum 100 authorities
    pub const MinAuthorities: u32 = 1;  // Minimum 1 authority required
    pub const DefaultEvmGasLimit: u64 = 12_000_000;  // tuned for 200ms slots on commodity validators
    pub const DefaultSvmComputeLimit: u64 = 200_000;  // 200k compute units for SVM
    pub const DefaultX3GasLimit: u64 = 6_000_000;  // tuned for 200ms slots on commodity validators
    pub const CrossVmPrepareTtl: BlockNumber = 50; // 50 blocks (~10s at 200ms)
    pub const MaxPreparedCrossVmOps: u32 = 1024;
    pub const MaxPreparedOpsPerBlock: u32 = 64;
    /// Maximum replay-store entries pruned per block. Bounds
    /// `on_initialize` work by the cross-VM replay-store pruner.
    pub const MaxReplayPruneItemsPerBlock: u32 = 256;
    pub const RequireCrossVmProof: bool = true;
    /// EVM bridge escrow contract address for atomic cross-VM swaps.
    pub BridgeEvmEscrow: H160 = H160([
        0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90,
        0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x56, 0x78, 0x90,
    ]);
    /// SVM bridge escrow program address for atomic cross-VM swaps.
    pub BridgeSvmEscrow: [u8; 32] = [
        0x58, 0x33, 0x42, 0x72, 0x69, 0x64, 0x67, 0x65, 0x45, 0x73, 0x63, 0x72, 0x6f, 0x77,
        0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31, 0x31,
        0x31, 0x31, 0x31, 0x31,
    ];
    pub BlockWeights: limits::BlockWeights = limits::BlockWeights::with_sensible_defaults(
        // Keep max execution budget below slot time (200ms) to avoid author/import divergence.
        Weight::from_parts((WEIGHT_REF_TIME_PER_SECOND / 1000) * 150, 5 * 1024 * 1024),
        Perbill::from_percent(90),
    );
    pub BlockLength: limits::BlockLength = limits::BlockLength::max_with_normal_ratio(
        5 * 1024 * 1024, // 5MB hard cap to reduce import pressure
        Perbill::from_percent(90),
    );
}

parameter_types! {
    pub const ChainId: u64 = 650_000;
    pub const GasLimitPovSizeRatio: u64 = 40;
    pub WeightPerGas: Weight = Weight::from_parts(20_000, 0);
}

pub struct BlockGasLimit;
impl Get<U256> for BlockGasLimit {
    fn get() -> U256 {
        U256::from(15_000_000u64)
    }
}

pub struct PrecompilesValue;
impl Get<FrontierPrecompiles<Runtime>> for PrecompilesValue {
    fn get() -> FrontierPrecompiles<Runtime> {
        FrontierPrecompiles::new()
    }
}

#[cfg(feature = "std")]
pub fn native_version() -> sp_version::NativeVersion {
    sp_version::NativeVersion {
        runtime_version: VERSION,
