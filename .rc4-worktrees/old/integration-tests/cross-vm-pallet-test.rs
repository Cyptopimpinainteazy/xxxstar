//! Companion integration tests for cross-VM atomic kernel runtime wiring.
//!
//! This file complements `integration-tests/cross-vm-atomic-test.rs` by using
//! real pallet `TestExternalities` and a node-level WebSocket JSON-RPC check.

use frame_support::{assert_ok, BoundedVec};
use pallet_x3_atomic_kernel::{self as atomic_kernel, BundleStatus};
use pallet_x3_atomic_kernel::mock::{
    new_test_ext, run_to_block, test_leg, ALICE, BOB, CHARLIE, AtomicKernel, RuntimeEvent,
    RuntimeOrigin, System, Test,
};
use pallet_x3_atomic_kernel::proof::{BundleLeg, VmType};
use sp_core::H256;

#[test]
fn submit_assign_finalize_transitions_bundle_status_and_emits_events() {
    new_test_ext().execute_with(|| {
        let legs_vec = vec![test_leg(VmType::Evm), test_leg(VmType::Svm)];
        let legs: BoundedVec<BundleLeg, <Test as atomic_kernel::Config>::MaxLegsPerBundle> =
            legs_vec.try_into().expect("legs should fit into MaxLegsPerBundle");

        assert_ok!(AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(ALICE),
            legs,
            10,
        ));

        run_to_block(2);

        let bundle_id = System::events()
            .iter()
            .find_map(|record| match &record.event {
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleSubmitted { bundle_id, .. }) => {
                    Some(*bundle_id)
                }
                _ => None,
            })
            .expect("BundleSubmitted event must exist");

        let pending = atomic_kernel::Bundles::<Test>::get(bundle_id).expect("bundle must exist");
        assert_eq!(pending.status, BundleStatus::Pending);

        assert_ok!(AtomicKernel::assign_bundle_executor(
            RuntimeOrigin::signed(BOB),
            bundle_id,
        ));

        run_to_block(3);

        let executing = atomic_kernel::Bundles::<Test>::get(bundle_id).expect("bundle must exist");
        assert_eq!(executing.status, BundleStatus::Executing);
        assert!(System::events().iter().any(|record| {
            matches!(
                &record.event,
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleAssigned { bundle_id: id, .. }) if *id == bundle_id
            )
        }));

        assert_ok!(AtomicKernel::finalize_atomic_bundle(
            RuntimeOrigin::signed(CHARLIE),
            bundle_id,
            H256::repeat_byte(0xAA),
            H256::zero(),
            3,
        ));

        run_to_block(4);

        let finalized = atomic_kernel::Bundles::<Test>::get(bundle_id).expect("bundle must exist");
        assert_eq!(finalized.status, BundleStatus::Finalized);
        assert!(System::events().iter().any(|record| {
            matches!(
                &record.event,
                RuntimeEvent::AtomicKernel(atomic_kernel::Event::BundleFinalized { bundle_id: id, .. }) if *id == bundle_id
            )
        }));
    });
}

#[tokio::test]
#[ignore = "requires a running dev node at ws://127.0.0.1:9944"]
async fn node_rpc_submit_cross_vm_tx_hash_observed_after_finalization() {
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::rpc_params;
    use jsonrpsee::ws_client::WsClientBuilder;
    use serde_json::json;

    let client = WsClientBuilder::default()
        .build("ws://127.0.0.1:9944")
        .await
        .expect("ws client must connect");

    let payload = json!({
        "evm_payload": "0x6001600101",
        "svm_payload": "0x",
        "atomic": true
    });

    let tx_hash: String = client
        .request("x3_submitCrossVmTransaction", rpc_params![payload])
        .await
        .expect("x3_submitCrossVmTransaction should return tx hash");

    assert!(tx_hash.starts_with("0x"));
    assert!(tx_hash.len() >= 10);

    let mut matched = false;
    for _ in 0..10u8 {
        let finalized_head: String = client
            .request("chain_getFinalizedHead", rpc_params![])
            .await
            .expect("chain_getFinalizedHead should succeed");

        let block: serde_json::Value = client
            .request("chain_getBlock", rpc_params![finalized_head])
            .await
            .expect("chain_getBlock should succeed");

        let extrinsics = block
            .get("block")
            .and_then(|b| b.get("extrinsics"))
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();

        if extrinsics.iter().any(|ext| ext.as_str().map(|s| s.contains(&tx_hash[2..])).unwrap_or(false)) {
            matched = true;
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    assert!(
        matched,
        "submitted hash should be observable in finalized chain data"
    );
}
