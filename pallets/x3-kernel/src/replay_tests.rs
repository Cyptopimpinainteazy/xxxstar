// Patch 4a pallet-level replay tests.
//
// Exercise the pallet's `X3vmReplayStore` invariants end-to-end
// against the mock runtime:
//
//   * `admit_x3vm_call` is idempotent-negative — second admission of
//     the same (vm, hash) returns `DispatchError`.
//   * `abort_x3vm_admission` releases an admitted hash so it can be
//     re-admitted.
//   * `admit_x3vm_call_for` derives the key through
//     `x3vm_replay_key`, i.e. binds to the pallet's
//     `source_finalized_hash` (parent hash).
//   * `prune_x3vm_replay_store` is bounded by
//     `T::MaxReplayPruneItemsPerBlock` and only removes entries
//     strictly at or below `now - REPLAY_PRUNE_HORIZON_BLOCKS`.
//   * `on_initialize` invokes the pruner at block boundaries.

use frame_support::traits::Hooks;
use sp_core::H256;

use crate::X3vmReplayStore;
use crate::mock::{new_test_ext, AtlasKernel, System, Test};

use x3_cross_vm_bridge::canonical::{
    CrossVmCall, VmId, REPLAY_PRUNE_HORIZON_BLOCKS,
};

/// Minimal `CrossVmCall` helper for the pallet tests. Varies only
/// `nonce` so distinct calls produce distinct `call_hash`es under the
/// same source-finalized hash.
fn make_call(nonce: u64) -> CrossVmCall {
    CrossVmCall::new(
        VmId::X3Vm,
        VmId::X3Vm,
        [0u8; 4],
        b"payload".to_vec(),
        100_000,
        nonce,
        u64::MAX, // deadline in the far future — irrelevant to replay tests
    )
    .expect("payload fits in MAX_CROSS_VM_PAYLOAD")
}

#[test]
fn admit_x3vm_call_inserts_and_rejects_duplicate() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        let call = make_call(1);
        let key = AtlasKernel::x3vm_replay_key(&call);

        assert!(!AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &key));
        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_ok());
        assert!(AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &key));

        // Second admission MUST be rejected — the core replay
        // protection invariant.
        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_err());
    });
}

#[test]
fn abort_x3vm_admission_releases_and_allows_requeue() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        let call = make_call(2);
        let key = AtlasKernel::x3vm_replay_key(&call);

        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_ok());
        assert!(AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &key));

        // Abort releases — second abort returns false (nothing to
        // remove).
        assert!(AtlasKernel::abort_x3vm_admission(VmId::X3Vm, &key));
        assert!(!AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &key));
        assert!(!AtlasKernel::abort_x3vm_admission(VmId::X3Vm, &key));

        // After abort the caller may re-admit exactly once.
        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_ok());
        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_err());
    });
}

#[test]
fn admit_x3vm_call_for_binds_to_source_finalized_hash() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        let call = make_call(3);

        // The key computed via the pallet helper MUST match the one
        // the replay map is keyed on.
        let key = AtlasKernel::admit_x3vm_call_for(&call).expect("first admit");
        assert_eq!(key, AtlasKernel::x3vm_replay_key(&call));
        assert!(AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &key));

        // Second admission via the same helper rejects.
        assert!(AtlasKernel::admit_x3vm_call_for(&call).is_err());
    });
}

#[test]
fn prune_noop_before_horizon_reached() {
    new_test_ext().execute_with(|| {
        // Block number well below the horizon — pruning must be a
        // no-op, never underflow, never touch storage.
        System::set_block_number(5);
        let call = make_call(4);
        let key = AtlasKernel::x3vm_replay_key(&call);
        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_ok());

        let removed = AtlasKernel::prune_x3vm_replay_store(System::block_number());
        assert_eq!(removed, 0);
        assert!(AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &key));
    });
}

#[test]
fn prune_removes_entries_past_horizon() {
    new_test_ext().execute_with(|| {
        // Seed admission at block 1.
        System::set_block_number(1);
        let call = make_call(5);
        let key = AtlasKernel::x3vm_replay_key(&call);
        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_ok());

        // Jump forward past the horizon. `now - horizon` is now
        // strictly > 1, so the block-1 entry satisfies the
        // `admitted_at <= threshold` predicate.
        let past = (REPLAY_PRUNE_HORIZON_BLOCKS as u64) + 2;
        System::set_block_number(past);

        let removed = AtlasKernel::prune_x3vm_replay_store(System::block_number());
        assert_eq!(removed, 1);
        assert!(!AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &key));
    });
}

#[test]
fn prune_respects_per_block_budget() {
    // Mock sets `MaxReplayPruneItemsPerBlock = 64`. Seed 80 stale
    // entries and confirm the pruner removes exactly 64 in one call,
    // leaving 16 for the next block to sweep.
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let mut keys = Vec::with_capacity(80);
        for nonce in 0..80u64 {
            let call = make_call(100 + nonce);
            let key = AtlasKernel::x3vm_replay_key(&call);
            assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_ok());
            keys.push(key);
        }

        // Advance past horizon.
        let past = (REPLAY_PRUNE_HORIZON_BLOCKS as u64) + 10;
        System::set_block_number(past);

        let first = AtlasKernel::prune_x3vm_replay_store(System::block_number());
        assert_eq!(first, 64);
        // Exactly 16 entries must remain.
        let remaining = X3vmReplayStore::<Test>::iter().count();
        assert_eq!(remaining, 16);

        // Next call sweeps the tail.
        let second = AtlasKernel::prune_x3vm_replay_store(System::block_number());
        assert_eq!(second, 16);
        assert_eq!(X3vmReplayStore::<Test>::iter().count(), 0);
    });
}

#[test]
fn on_initialize_runs_pruner() {
    // End-to-end: admit at block 1, advance past horizon, trigger
    // `on_initialize`, confirm the entry disappears. This proves the
    // hook is actually wired up, not just the free helper.
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let call = make_call(999);
        let key = AtlasKernel::x3vm_replay_key(&call);
        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, key).is_ok());

        let past = (REPLAY_PRUNE_HORIZON_BLOCKS as u64) + 5;
        System::set_block_number(past);
        let _ = <AtlasKernel as Hooks<u64>>::on_initialize(past);

        assert!(!AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &key));
    });
}

#[test]
fn admit_distinguishes_between_target_vms() {
    // Same `call_hash` under a different `VmId` must be treated as a
    // distinct replay slot. `VmId` is the outer key — this is load-
    // bearing if we ever route non-X3Vm calls through the same store.
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        let h = H256::repeat_byte(0xAB);

        assert!(AtlasKernel::admit_x3vm_call(VmId::X3Vm, h).is_ok());
        assert!(AtlasKernel::is_x3vm_call_replayed(VmId::X3Vm, &h));
        assert!(!AtlasKernel::is_x3vm_call_replayed(VmId::Evm, &h));

        // Admitting under a different VmId must succeed — the
        // `(vm, hash)` pair is the replay key.
        assert!(AtlasKernel::admit_x3vm_call(VmId::Evm, h).is_ok());
        assert!(AtlasKernel::is_x3vm_call_replayed(VmId::Evm, &h));
    });
}
