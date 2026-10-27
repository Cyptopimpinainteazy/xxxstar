// Tests for pallet-x3-verifier

use crate as pallet_x3_verifier;
use crate::*;
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types, traits::Hooks, BoundedVec,
};
use frame_system::EnsureRoot;
use sp_core::{ecdsa, Pair, H256};
use sp_io::hashing::{blake2_256, keccak_256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, BuildStorage, DispatchError,
};

type AccountId = AccountId32;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Verifier: pallet_x3_verifier,
    }
);

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
    pub const MinExecutorStake: u128 = 1_000;
    pub const MaxOutputSize: u32 = 64;
    pub const MaxKeySize: u32 = 32;
    pub const MaxValueSize: u32 = 64;
    pub const MaxStateChanges: u32 = 8;
    pub const MaxProofDepth: u32 = 8;
    pub const ExecutorRewardShare: u32 = 70;
    pub const ProtocolFeeShare: u32 = 15;
    pub const SlashAmount: u128 = 400;
    pub const JobTimeout: u64 = 10;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type AccountData = pallet_balances::AccountData<u128>;
}

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Balance = u128;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type MaxFreezes = ();
    type DoneSlashHandler = ();
}

impl pallet_x3_verifier::Config for Test {
    type Currency = Balances;
    type ExecutorRegistrar = EnsureRoot<AccountId>;
    type MinExecutorStake = MinExecutorStake;
    type MaxOutputSize = MaxOutputSize;
    type MaxKeySize = MaxKeySize;
    type MaxValueSize = MaxValueSize;
    type MaxStateChanges = MaxStateChanges;
    type MaxProofDepth = MaxProofDepth;
    type ExecutorRewardShare = ExecutorRewardShare;
    type ProtocolFeeShare = ProtocolFeeShare;
    type SlashAmount = SlashAmount;
    type JobTimeout = JobTimeout;
    type WeightInfo = ();
}

const ENDOWED: u128 = 1_000_000;
const REWARD: u128 = 10_000;
const GAS_LIMIT: u128 = 1_000;

fn submitter() -> AccountId {
    AccountId32::new([1u8; 32])
}

fn stranger() -> AccountId {
    AccountId32::new([2u8; 32])
}

fn executor_key() -> ecdsa::Pair {
    ecdsa::Pair::from_seed(&[7u8; 32])
}

fn other_key() -> ecdsa::Pair {
    ecdsa::Pair::from_seed(&[8u8; 32])
}

/// The account the pallet binds to a secp256k1 key:
/// `blake2_256(keccak_256(uncompressed_pubkey)[12..])`.
fn account_of(pair: &ecdsa::Pair) -> AccountId {
    // Recover the 64-byte uncompressed key from a throwaway signature, since
    // `ecdsa::Public` only exposes the 33-byte compressed form.
    let digest = [0u8; 32];
    let sig = pair.sign_prehashed(&digest);
    let pk = sp_io::crypto::secp256k1_ecdsa_recover(&sig.0, &digest)
        .ok()
        .expect("own signature recovers");
    let eth_addr = &keccak_256(&pk)[12..];
    AccountId32::new(blake2_256(eth_addr))
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (submitter(), ENDOWED),
            (stranger(), ENDOWED),
            (account_of(&executor_key()), ENDOWED),
            (account_of(&other_key()), ENDOWED),
        ],
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();
    pallet_x3_verifier::GenesisConfig::<Test> {
        verification_enabled: true,
        initial_executors: vec![],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn free(who: &AccountId) -> u128 {
    Balances::free_balance(who)
}

fn reserved(who: &AccountId) -> u128 {
    Balances::reserved_balance(who)
}

fn register(pair: &ecdsa::Pair) -> AccountId {
    let who = account_of(pair);
    assert_ok!(Verifier::register_executor(
        RuntimeOrigin::signed(who.clone()),
        MinExecutorStake::get()
    ));
    who
}

fn input_hash() -> H256 {
    H256::repeat_byte(0xA1)
}

/// Submit a job from `submitter()` and return its id.
fn submit_job() -> JobId {
    assert_ok!(Verifier::submit_job(
        RuntimeOrigin::signed(submitter()),
        H256::repeat_byte(0xB1),
        input_hash(),
        GAS_LIMIT,
        REWARD
    ));
    match System::events().last().map(|r| r.event.clone()) {
        Some(RuntimeEvent::Verifier(Event::JobSubmitted { job_id, .. })) => job_id,
        other => panic!("expected JobSubmitted, got {other:?}"),
    }
}

type StateChanges =
    BoundedVec<(BoundedVec<u8, MaxKeySize>, BoundedVec<u8, MaxValueSize>), MaxStateChanges>;

fn changes(pairs: &[(&[u8], &[u8])]) -> StateChanges {
    pairs
        .iter()
        .map(|(k, v)| {
            (
                k.to_vec().try_into().unwrap(),
                v.to_vec().try_into().unwrap(),
            )
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

/// The root the pallet expects: fold each change, then each proof element, into
/// `state_root_before` with blake2_256.
fn expected_root(before: H256, changes: &StateChanges, proof: &[H256]) -> H256 {
    let mut root = before;
    for (k, v) in changes.iter() {
        let mut buf = root.as_bytes().to_vec();
        buf.extend(blake2_256(k));
        buf.extend(blake2_256(v));
        root = H256::from(blake2_256(&buf));
    }
    for p in proof {
        let mut buf = root.as_bytes().to_vec();
        buf.extend(p.as_bytes());
        root = H256::from(blake2_256(&buf));
    }
    root
}

/// A receipt for `job_id` with no state changes, signed by `signer` and naming
/// `executor`. Callers adjust fields and call `sign` again to re-sign.
fn receipt(job_id: JobId, signer: &ecdsa::Pair, executor: AccountId) -> ExecutionReceipt<Test> {
    let mut r = ExecutionReceipt::<Test> {
        job_id,
        executor,
        input_hash: input_hash(),
        output_hash: H256::repeat_byte(0xC1),
        state_root_before: H256::repeat_byte(0xD1),
        state_root_after: H256::repeat_byte(0xD1),
        gas_used: 500,
        timestamp: 1_700_000_000,
        output_data: Default::default(),
        state_changes: Default::default(),
        merkle_proof: Default::default(),
        signature: Default::default(),
    };
    sign(&mut r, signer);
    r
}

fn sign(r: &mut ExecutionReceipt<Test>, signer: &ecdsa::Pair) {
    let mut msg = Vec::new();
    msg.extend(r.job_id.as_bytes());
    msg.extend(r.input_hash.as_bytes());
    msg.extend(r.output_hash.as_bytes());
    msg.extend(r.state_root_before.as_bytes());
    msg.extend(r.state_root_after.as_bytes());
    msg.extend(&r.gas_used.to_le_bytes());
    msg.extend(&r.timestamp.to_le_bytes());
    let sig = signer.sign_prehashed(&keccak_256(&msg));
    r.signature = sig.0.to_vec().try_into().unwrap();
}

// ── Executor registration ───────────────────────────────────────────────────

#[test]
fn registering_an_executor_reserves_its_stake() {
    new_test_ext().execute_with(|| {
        let who = register(&executor_key());

        let record = Verifier::executors(&who).unwrap();
        assert!(record.active);
        assert_eq!(record.stake, MinExecutorStake::get());
        assert_eq!(reserved(&who), MinExecutorStake::get());
    });
}

#[test]
fn an_executor_cannot_register_twice() {
    new_test_ext().execute_with(|| {
        let who = register(&executor_key());
        assert_noop!(
            Verifier::register_executor(RuntimeOrigin::signed(who), MinExecutorStake::get()),
            Error::<Test>::AlreadyRegistered
        );
    });
}

#[test]
fn a_stake_below_the_minimum_is_refused() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Verifier::register_executor(
                RuntimeOrigin::signed(stranger()),
                MinExecutorStake::get() - 1
            ),
            Error::<Test>::InsufficientStake
        );
        assert_eq!(reserved(&stranger()), 0);
    });
}

#[test]
fn deactivating_releases_the_stake_exactly_once() {
    new_test_ext().execute_with(|| {
        let who = register(&executor_key());
        // The executor also has a job of its own in flight, whose reward is reserved.
        assert_ok!(Verifier::submit_job(
            RuntimeOrigin::signed(who.clone()),
            H256::repeat_byte(0xB2),
            input_hash(),
            GAS_LIMIT,
            REWARD
        ));
        assert_eq!(reserved(&who), MinExecutorStake::get() + REWARD);

        assert_ok!(Verifier::deactivate_executor(RuntimeOrigin::signed(
            who.clone()
        )));
        assert_eq!(reserved(&who), REWARD);
        assert!(!Verifier::executors(&who).unwrap().active);

        // A second call must not unreserve the job's reward as if it were stake.
        assert_ok!(Verifier::deactivate_executor(RuntimeOrigin::signed(
            who.clone()
        )));
        assert_eq!(reserved(&who), REWARD);
    });
}

#[test]
fn deactivating_an_unknown_executor_is_refused() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Verifier::deactivate_executor(RuntimeOrigin::signed(stranger())),
            Error::<Test>::ExecutorNotRegistered
        );
    });
}

// ── Jobs ────────────────────────────────────────────────────────────────────

#[test]
fn submitting_a_job_reserves_the_reward() {
    new_test_ext().execute_with(|| {
        let job_id = submit_job();

        let job = Verifier::jobs(job_id).unwrap();
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.reward, REWARD);
        assert_eq!(reserved(&submitter()), REWARD);
        assert_eq!(TotalJobsSubmitted::<Test>::get(), 1);
    });
}

#[test]
fn the_same_job_twice_in_one_block_is_refused_instead_of_overwritten() {
    new_test_ext().execute_with(|| {
        submit_job();
        assert_noop!(
            Verifier::submit_job(
                RuntimeOrigin::signed(submitter()),
                H256::repeat_byte(0xB1),
                input_hash(),
                GAS_LIMIT,
                REWARD
            ),
            Error::<Test>::DuplicateJob
        );
        assert_eq!(reserved(&submitter()), REWARD, "only one reward is held");
    });
}

#[test]
fn the_same_job_in_a_later_block_is_a_new_job() {
    new_test_ext().execute_with(|| {
        let first = submit_job();
        System::set_block_number(2);
        let second = submit_job();
        assert_ne!(first, second);
        assert_eq!(reserved(&submitter()), 2 * REWARD);
    });
}

#[test]
fn jobs_are_refused_while_verification_is_disabled() {
    new_test_ext().execute_with(|| {
        assert_ok!(Verifier::toggle_verification(RuntimeOrigin::root(), false));
        assert_noop!(
            Verifier::submit_job(
                RuntimeOrigin::signed(submitter()),
                H256::zero(),
                input_hash(),
                GAS_LIMIT,
                REWARD
            ),
            Error::<Test>::VerificationDisabled
        );
    });
}

#[test]
fn only_the_registrar_origin_can_toggle_verification() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Verifier::toggle_verification(RuntimeOrigin::signed(submitter()), false),
            DispatchError::BadOrigin
        );
        assert!(Verifier::verification_enabled());
    });
}

#[test]
fn a_pending_job_times_out_and_returns_the_reward() {
    new_test_ext().execute_with(|| {
        let job_id = submit_job();

        Verifier::on_finalize(1 + JobTimeout::get() - 1);
        assert_eq!(Verifier::jobs(job_id).unwrap().status, JobStatus::Pending);
        assert_eq!(reserved(&submitter()), REWARD);

        Verifier::on_finalize(1 + JobTimeout::get());
        assert_eq!(Verifier::jobs(job_id).unwrap().status, JobStatus::Failed);
        assert_eq!(reserved(&submitter()), 0);
        assert_eq!(free(&submitter()), ENDOWED);
    });
}

// ── Receipts: the happy path ────────────────────────────────────────────────

#[test]
fn a_valid_receipt_is_applied_and_pays_the_executor() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        let exec_before = free(&exec);

        let r = receipt(job_id, &executor_key(), exec.clone());
        assert_ok!(Verifier::submit_receipt(
            RuntimeOrigin::signed(exec.clone()),
            r.clone()
        ));

        let job = Verifier::jobs(job_id).unwrap();
        assert_eq!(job.status, JobStatus::Applied);
        assert_eq!(job.executor, Some(exec.clone()));
        assert_eq!(
            VerifiedStateRoots::<Test>::get(job_id),
            Some(r.state_root_after)
        );

        let executor_amount = REWARD * u128::from(ExecutorRewardShare::get()) / 100;
        assert_eq!(free(&exec), exec_before + executor_amount);
        assert_eq!(reserved(&submitter()), 0);

        let record = Verifier::executors(&exec).unwrap();
        assert_eq!(record.jobs_completed, 1);
        assert_eq!(record.total_rewards, executor_amount);
        assert_eq!(record.reputation, 51);
        assert_eq!(TotalJobsVerified::<Test>::get(), 1);
    });
}

#[test]
fn a_receipt_with_state_changes_and_a_matching_root_is_applied() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();

        let mut r = receipt(job_id, &executor_key(), exec.clone());
        r.state_changes = changes(&[(b"balance/alice", b"100"), (b"balance/bob", b"7")]);
        let proof = vec![H256::repeat_byte(0xE1)];
        r.merkle_proof = proof.clone().try_into().unwrap();
        r.state_root_after = expected_root(r.state_root_before, &r.state_changes, &proof);
        sign(&mut r, &executor_key());

        assert_ok!(Verifier::submit_receipt(RuntimeOrigin::signed(exec), r));
        assert_eq!(Verifier::jobs(job_id).unwrap().status, JobStatus::Applied);
    });
}

#[test]
fn an_applied_job_cannot_be_paid_twice() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        let r = receipt(job_id, &executor_key(), exec.clone());
        assert_ok!(Verifier::submit_receipt(
            RuntimeOrigin::signed(exec.clone()),
            r.clone()
        ));

        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::JobAlreadyCompleted
        );
    });
}

// ── Receipts: refusals ──────────────────────────────────────────────────────

#[test]
fn a_receipt_from_an_unregistered_account_is_refused() {
    new_test_ext().execute_with(|| {
        let job_id = submit_job();
        let exec = account_of(&executor_key());
        let r = receipt(job_id, &executor_key(), exec.clone());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::ExecutorNotRegistered
        );
    });
}

#[test]
fn a_receipt_from_a_deactivated_executor_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        assert_ok!(Verifier::deactivate_executor(RuntimeOrigin::signed(
            exec.clone()
        )));
        let job_id = submit_job();
        let r = receipt(job_id, &executor_key(), exec.clone());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::ExecutorNotActive
        );
    });
}

#[test]
fn an_executor_cannot_submit_a_receipt_naming_another_executor() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let other = register(&other_key());
        let job_id = submit_job();
        let r = receipt(job_id, &other_key(), other);
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::NotAuthorized
        );
    });
}

#[test]
fn a_receipt_signed_by_a_different_key_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        register(&other_key());
        let job_id = submit_job();
        // Names `exec`, but signed with someone else's key.
        let r = receipt(job_id, &other_key(), exec.clone());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::InvalidSignature
        );
    });
}

#[test]
fn a_receipt_altered_after_signing_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        let mut r = receipt(job_id, &executor_key(), exec.clone());
        r.output_hash = H256::repeat_byte(0xC2);
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::InvalidSignature
        );
    });
}

#[test]
fn a_receipt_with_a_truncated_signature_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        let mut r = receipt(job_id, &executor_key(), exec.clone());
        r.signature = r.signature[..64].to_vec().try_into().unwrap();
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::InvalidSignature
        );
    });
}

#[test]
fn a_receipt_for_a_different_input_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        let mut r = receipt(job_id, &executor_key(), exec.clone());
        r.input_hash = H256::repeat_byte(0xA2);
        sign(&mut r, &executor_key());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::InvalidReceiptFormat
        );
        assert_eq!(reserved(&submitter()), REWARD, "the reward is still held");
    });
}

#[test]
fn a_receipt_over_the_gas_limit_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        let mut r = receipt(job_id, &executor_key(), exec.clone());
        r.gas_used = GAS_LIMIT + 1;
        sign(&mut r, &executor_key());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::InvalidReceiptFormat
        );
    });
}

#[test]
fn a_receipt_whose_root_does_not_follow_from_its_changes_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        let mut r = receipt(job_id, &executor_key(), exec.clone());
        r.state_changes = changes(&[(b"balance/alice", b"100")]);
        let proof = vec![H256::repeat_byte(0xE1)];
        r.merkle_proof = proof.clone().try_into().unwrap();
        // Root computed for a different value than the one claimed.
        r.state_root_after = expected_root(
            r.state_root_before,
            &changes(&[(b"balance/alice", b"999")]),
            &proof,
        );
        sign(&mut r, &executor_key());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::StateRootMismatch
        );
    });
}

#[test]
fn state_changes_without_a_proof_are_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        let mut r = receipt(job_id, &executor_key(), exec.clone());
        r.state_changes = changes(&[(b"balance/alice", b"100")]);
        sign(&mut r, &executor_key());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::InvalidMerkleProof
        );
    });
}

#[test]
fn a_receipt_for_an_unknown_job_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let r = receipt(H256::repeat_byte(0xFF), &executor_key(), exec.clone());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::JobNotFound
        );
    });
}

#[test]
fn a_receipt_for_a_timed_out_job_is_refused() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        Verifier::on_finalize(1 + JobTimeout::get());
        let r = receipt(job_id, &executor_key(), exec.clone());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::JobAlreadyCompleted
        );
    });
}

#[test]
fn receipts_are_refused_while_verification_is_disabled() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let job_id = submit_job();
        assert_ok!(Verifier::toggle_verification(RuntimeOrigin::root(), false));
        let r = receipt(job_id, &executor_key(), exec.clone());
        assert_noop!(
            Verifier::submit_receipt(RuntimeOrigin::signed(exec), r),
            Error::<Test>::VerificationDisabled
        );
    });
}

// ── Disputes and slashing ───────────────────────────────────────────────────

#[test]
fn a_pending_job_cannot_be_disputed() {
    new_test_ext().execute_with(|| {
        let job_id = submit_job();
        assert_noop!(
            Verifier::dispute_receipt(
                RuntimeOrigin::signed(stranger()),
                job_id,
                b"bad".to_vec().try_into().unwrap()
            ),
            Error::<Test>::JobAlreadyCompleted
        );
    });
}

#[test]
fn slashing_burns_reserved_stake_and_deactivates_below_the_minimum() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        let issuance_before = Balances::total_issuance();

        assert_ok!(Verifier::slash_executor(&exec));

        let record = Verifier::executors(&exec).unwrap();
        assert_eq!(record.stake, MinExecutorStake::get() - SlashAmount::get());
        assert_eq!(
            reserved(&exec),
            MinExecutorStake::get() - SlashAmount::get()
        );
        assert_eq!(record.jobs_failed, 1);
        assert_eq!(record.reputation, 40);
        assert!(!record.active, "stake fell below the minimum");
        assert_eq!(
            Balances::total_issuance(),
            issuance_before - SlashAmount::get(),
            "slashed stake is burned"
        );
    });
}

#[test]
fn slashing_never_takes_more_than_the_stake() {
    new_test_ext().execute_with(|| {
        let exec = register(&executor_key());
        for _ in 0..5 {
            assert_ok!(Verifier::slash_executor(&exec));
        }
        assert_eq!(Verifier::executors(&exec).unwrap().stake, 0);
        assert_eq!(reserved(&exec), 0);
        assert_eq!(free(&exec), ENDOWED - MinExecutorStake::get());
    });
}

#[test]
fn slashing_an_unknown_executor_is_refused() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Verifier::slash_executor(&stranger()),
            Error::<Test>::ExecutorNotRegistered
        );
    });
}
