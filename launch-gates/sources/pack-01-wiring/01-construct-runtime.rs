# Runtime Construct Macro
construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Session: pallet_session,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Scheduler: pallet_scheduler,
        Preimage: pallet_preimage,
        EVM: pallet_evm,
        AtlasKernel: pallet_x3_kernel,
        X3Coin: pallet_x3_coin,
        AtomicTradeEngine: pallet_atomic_trade_engine,
        Council: pallet_collective::<Instance1>,
        Sudo: pallet_sudo,
        Governance: pallet_governance,
        Treasury: pallet_treasury,
        AgentAccounts: pallet_agent_accounts,
        AgentMemory: pallet_agent_memory,
        EvolutionCore: pallet_evolution_core,
        X3Verifier: pallet_x3_verifier,
        X3DomainRegistry: pallet_x3_domain_registry,
        X3JuryAnchor: pallet_x3_jury_anchor,
        X3SettlementEngine: pallet_x3_settlement_engine,
        Swarm: pallet_swarm,
        DepinMarketplace: pallet_depin_marketplace,
        PrivateExecution: pallet_private_execution,
        X3Sequencer: pallet_x3_sequencer,
        FraudProofs: crate::fraud_proofs::pallet::pallet,
        X3Da: pallet_x3_da,
        X3AtomicKernel: pallet_x3_atomic_kernel,
        X3AssetRegistry: pallet_x3_asset_registry,
        X3SupplyLedger: pallet_x3_supply_ledger,
        X3CrossVmRouter: pallet_x3_cross_vm_router,
        X3TokenFactory: pallet_x3_token_factory,
        CrossChainValidator: pallet_cross_chain_validator,
    }
);

#[cfg(not(feature = "dev"))]
construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Session: pallet_session,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Scheduler: pallet_scheduler,
        Preimage: pallet_preimage,
        EVM: pallet_evm,
        AtlasKernel: pallet_x3_kernel,
        X3Coin: pallet_x3_coin,
        AtomicTradeEngine: pallet_atomic_trade_engine,
        Council: pallet_collective::<Instance1>,
        Governance: pallet_governance,
        Treasury: pallet_treasury,
        AgentAccounts: pallet_agent_accounts,
        AgentMemory: pallet_agent_memory,
        EvolutionCore: pallet_evolution_core,
        X3Verifier: pallet_x3_verifier,
        X3DomainRegistry: pallet_x3_domain_registry,
        X3JuryAnchor: pallet_x3_jury_anchor,
        X3SettlementEngine: pallet_x3_settlement_engine,
        Swarm: pallet_swarm,
        DepinMarketplace: pallet_depin_marketplace,
        PrivateExecution: pallet_private_execution,
        X3Sequencer: pallet_x3_sequencer,
        X3Da: pallet_x3_da,
        // ISSUE #3 FIX: FraudProofs moved AFTER X3Da to avoid forward reference
        // FraudProofs now reads X3Da state after block execution completes
        FraudProofs: crate::fraud_proofs::pallet::pallet,
        X3AtomicKernel: pallet_x3_atomic_kernel,
        X3AssetRegistry: pallet_x3_asset_registry,
        X3SupplyLedger: pallet_x3_supply_ledger,
        X3CrossVmRouter: pallet_x3_cross_vm_router,
        X3TokenFactory: pallet_x3_token_factory,
        CrossChainValidator: pallet_cross_chain_validator,
    }
);

pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
// Runtime storage migrations tuple. Add migration structs for pallets that need upgrades.
// Note: Only x3-kernel has migrations currently implemented
pub type Migrations = (pallet_x3_kernel::migrations::Migration<Runtime>,);

// Use the migrations tuple in the executive so migrations run on runtime upgrades
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
    }
}

pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

// ===== Config Impls (after construct_runtime!) =====

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl frame_support::traits::OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanced(amount: NegativeImbalance) {
        drop(amount);
    }
}

pub struct FixedGasPrice;
impl pallet_evm::FeeCalculator for FixedGasPrice {
    fn min_gas_price() -> (U256, Weight) {
        (U256::from(NATIVE_GAS_PRICE), Weight::zero())
    }
}

impl frame_system::Config for Runtime {
    type BaseCallFilter = Everything;
    type Block = Block;
    type BlockWeights = BlockWeights;
    type BlockLength = BlockLength;
    type DbWeight = RocksDbWeight;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = RuntimeVersion;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type Nonce = Index;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = sp_consensus_aura::sr25519::AuthorityId;
    type MaxAuthorities = MaxAuthorities;
    type DisabledValidators = ();
    type AllowMultipleBlocksPerSlot = ConstBool<true>; // Enable multiple blocks per slot for higher TPS
}

parameter_types! {
    pub const ReportLongevity: u64 = 1000;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = pallet_session::PeriodicSessions<ConstU32<1800>, ConstU32<0>>;
    type NextSessionRotation = pallet_session::PeriodicSessions<ConstU32<1800>, ConstU32<0>>;
    type SessionManager = ();
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Self>;
}

pub type Historical = pallet_session::historical::Pallet<Runtime>;

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = AccountId;
    type FullIdentificationOf = ConvertInto;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type RuntimeHoldReason = ();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightToFee = IdentityFee<Balance>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = ();
}

#[cfg(feature = "dev")]
impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

pub type EnsureRootOrHalfCouncil = frame_support::traits::EitherOfDiverse<
    frame_system::EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
>;

pub type EnsureCouncilMember = pallet_collective::EnsureMember<AccountId, CouncilCollective>;

pub type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxProposalWeight = MaxProposalWeight;
}

// ── Fraud-proof inline pallet config ─────────────────────────────────────────
impl crate::fraud_proofs::pallet::pallet::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxTxCount = FraudProofMaxTxCount;
    type DisputeWindowBlocks = FraudProofDisputeWindowBlocks;
    type ReporterRewardAmount = FraudProofReporterReward;
    type GovernanceOrigin = EnsureRootOrHalfCouncil;
}

impl pallet_evm::Config for Runtime {
    type FeeCalculator = FixedGasPrice;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_evm::SubstrateBlockHashMapping<Self>;
    type CallOrigin = pallet_evm::EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = pallet_evm::EnsureAddressTruncated;
    type AddressMapping = pallet_evm::HashedAddressMapping<BlakeTwo256>;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type PrecompilesType = FrontierPrecompiles<Self>;
    type PrecompilesValue = PrecompilesValue;
    type ChainId = ChainId;
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = pallet_evm::EVMCurrencyAdapter<Balances, ()>;
    type OnCreate = ();
    type FindAuthor = ();
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type Timestamp = Timestamp;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<Self>;
}

/// Production cross-chain proof verifier.
///
/// Validates `LockProof` and `MerkleReceipt` payloads with structural
/// sanity checks and enforces a byzantine threshold of validator signatures
/// from the currently configured X3 kernel authorities.
pub struct SubstrateProofVerifier;

impl pallet_x3_kernel::CrossChainProofVerifier<AccountId> for SubstrateProofVerifier {
    fn verify_proof(
        _origin: &AccountId,
        _operation: &x3_cross_vm_bridge::CrossVmOperation,
        proof: &pallet_x3_kernel::CrossChainProof,
    ) -> Result<(), frame_support::sp_runtime::DispatchError> {
        use codec::Encode;
        use pallet_x3_kernel::CrossChainProof;

        fn threshold(authority_count: usize) -> usize {
            // 2/3 + 1, but always at least 1.
            let needed = (authority_count.saturating_mul(2) / 3).saturating_add(1);
            core::cmp::max(1, needed)
        }

        fn account_to_key_bytes(
            account: &AccountId,
        ) -> Result<[u8; 32], frame_support::sp_runtime::DispatchError> {
            let encoded = account.encode();
            if encoded.len() != 32 {
                return Err(frame_support::sp_runtime::DispatchError::Other(
                    "Authority key must SCALE-encode to 32 bytes",
                ));
            }
            let mut out = [0u8; 32];
            out.copy_from_slice(&encoded);
            Ok(out)
        }

        fn verify_signature_any(pubkey_bytes: [u8; 32], message: &[u8], signature: &[u8]) -> bool {
            if signature.len() != 64 {
                return false;
            }

            // sr25519
            {
                let pubkey = sp_core::sr25519::Public::from_raw(pubkey_bytes);
                let sig = sp_core::sr25519::Signature::from_raw({
                    let mut buf = [0u8; 64];
                    buf.copy_from_slice(signature);
                    buf
                });
                if sp_io::crypto::sr25519_verify(&sig, message, &pubkey) {
                    return true;
                }
            }

            // ed25519
            {
                let pubkey = sp_core::ed25519::Public::from_raw(pubkey_bytes);
                let sig = sp_core::ed25519::Signature::from_raw({
                    let mut buf = [0u8; 64];
                    buf.copy_from_slice(signature);
                    buf
                });
                sp_io::crypto::ed25519_verify(&sig, message, &pubkey)
            }
        }

        fn require_len(
            actual: usize,
            expected: usize,
            label: &'static str,
        ) -> Result<(), frame_support::sp_runtime::DispatchError> {
            if actual != expected {
                return Err(frame_support::sp_runtime::DispatchError::Other(label));
            }
            Ok(())
        }

        let authorities = pallet_x3_kernel::Authorities::<Runtime>::get();
        let authority_keys: sp_std::collections::btree_set::BTreeSet<[u8; 32]> = authorities
            .into_iter()
            .map(|a| account_to_key_bytes(&a))
            .collect::<Result<_, _>>()?;
        let needed = threshold(authority_keys.len());

        match proof {
            CrossChainProof::None => Ok(()),
            CrossChainProof::LockProof(bytes) => {
                if bytes.is_empty() {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "LockProof: empty bytes",
                    ));
                }
                // Format:
                // [0..32)  event_hash
                // [32]     sig_count (u8)
                // repeat sig_count times:
                //   [validator_id:32][signature:64]
                if bytes.len() < 33 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "LockProof: payload too short (< 33 bytes)",
                    ));
                }

                let event_hash = &bytes[0..32];
                let sig_count = bytes[32] as usize;
                if sig_count == 0 {
                    return Err(frame_support::sp_runtime::DispatchError::Other(
                        "LockProof: signature count must be > 0",
                    ));
                }

                let expected_len = 33usize.saturating_add(sig_count.saturating_mul(96));
                require_len(
                    bytes.len(),
                    expected_len,
                    "LockProof: malformed payload length",
                )?;

                let mut valid = 0usize;
                let mut seen: sp_std::collections::btree_set::BTreeSet<[u8; 32]> =
                    sp_std::collections::btree_set::BTreeSet::new();

                for idx in 0..sig_count {
                    let offset = 33 + idx * 96;
                    let mut validator_id = [0u8; 32];
