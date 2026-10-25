//! Account-backed factory tokens must not enter supply-only cross-VM routing.
use codec::Encode;
use frame_support::{assert_noop, assert_ok, traits::Currency};
use sp_runtime::DispatchError;
use x3_asset_kernel_types::{
    traits::{RouteInspect, SupplyLedgerWrite},
    AccountBytes, AssetId, DomainId, SupplyPolicy, TokenClass,
};
use x3_chain_runtime::{AccountId, Balances, Runtime, RuntimeOrigin, TreasuryAccountId};

type Factory = pallet_x3_token_factory::Pallet<Runtime>;
type Router = pallet_x3_cross_vm_router::Pallet<Runtime>;
type Kernel = pallet_x3_kernel::Pallet<Runtime>;
type Supply = pallet_x3_supply_ledger::Pallet<Runtime>;
type System = frame_system::Pallet<Runtime>;

fn issue_factory_token(creator: &AccountId) -> (AssetId, u32) {
    let (asset, ledger_asset) = Factory::do_create_token(
        creator.clone(),
        pallet_x3_token_factory::TokenFactoryConfig {
            symbol: b"OMNI".to_vec().try_into().unwrap(),
            name: b"Account backed OmniToken".to_vec().try_into().unwrap(),
            canonical_decimals: 6,
            initial_supply: 1_000,
            max_supply: None,
            class: TokenClass::FixedSupply,
            enabled_domains: vec![DomainId::X3Native, DomainId::X3Evm, DomainId::X3Svm]
                .try_into()
                .unwrap(),
        },
    )
    .unwrap();
    (asset, ledger_asset.unwrap())
}

#[test]
fn factory_tokens_cannot_leave_spendable_balances_behind_a_pending_route() {
    sp_io::TestExternalities::default().execute_with(|| {
        System::set_block_number(1);
        let creator = AccountId::new([31; 32]);
        let (asset, ledger_asset) = issue_factory_token(&creator);
        assert_eq!(Kernel::ledger_balance_of(ledger_asset, &creator), 1_000);
        let account_total = || {
            pallet_x3_kernel::CanonicalLedger::<Runtime>::iter()
                .filter(|(_, id, _)| *id == ledger_asset)
                .map(|(_, _, balance)| balance)
                .sum::<u128>()
        };
        assert_eq!(account_total(), 1_000);
        let before = Supply::ledger(&asset).unwrap();
        assert_eq!(before.native_supply, 1_000);
        assert_eq!(before.evm_supply, 0);
        assert_eq!(before.svm_supply, 0);
        assert_eq!(before.external_locked_supply, 0);
        assert_eq!(before.pending_supply, 0);
        assert_eq!(before.canonical_supply, 1_000);
        assert!(pallet_x3_asset_registry::Pallet::<Runtime>::is_route_open(
            &asset,
            DomainId::X3Native,
            DomainId::X3Evm,
        ));
        assert_ok!(pallet_x3_custody::Pallet::<Runtime>::authorize_gateway(
            RuntimeOrigin::root(),
            pallet_x3_custody::GatewayRole::X3Lang,
            creator.clone(),
        ));
        let treasury = TreasuryAccountId::get();
        let _ = Balances::make_free_balance_be(&creator, 1_000_000_000_000);
        let _ = Balances::make_free_balance_be(&treasury, 1_000_000_000_000);
        let native_before = (
            Balances::free_balance(&creator),
            Balances::free_balance(&treasury),
        );
        assert_noop!(
            Router::xvm_transfer(
                RuntimeOrigin::signed(creator.clone()),
                asset,
                DomainId::X3Evm,
                AccountBytes::Evm([32; 20]),
                1_000,
                10,
            ),
            DispatchError::Other("FactoryTokenCrossVmAccountingUnavailable")
        );
        assert_eq!(
            (
                Balances::free_balance(&creator),
                Balances::free_balance(&treasury)
            ),
            native_before,
        );
        assert_noop!(
            Router::xvm_transfer_from_vm(
                RuntimeOrigin::signed(creator.clone()),
                asset,
                DomainId::X3Evm,
                AccountBytes::Evm([31; 20]),
                DomainId::X3Native,
                AccountBytes::X3Native([32; 32]),
                100,
                10,
            ),
            DispatchError::Other("FactoryTokenCrossVmAccountingUnavailable")
        );
        // Call the shared initiation boundary, used by both gateway and VM entrypoints.
        // assert_noop also checks the full storage root (fees/nonces/records/events).
        for _ in 0..2 {
            assert_noop!(
                Router::do_initiate_transfer(
                    asset,
                    DomainId::X3Native,
                    DomainId::X3Evm,
                    AccountBytes::X3Native([31; 32]),
                    AccountBytes::Evm([32; 20]),
                    100,
                    10,
                    None,
                ),
                DispatchError::Other("FactoryTokenCrossVmAccountingUnavailable")
            );
            assert_eq!(Supply::ledger(&asset).unwrap(), before);
            assert_eq!(account_total(), before.canonical_supply);
            assert_eq!(Kernel::ledger_balance_of(ledger_asset, &creator), 1_000);
        }
        System::set_block_number(11);
        assert_eq!(
            pallet_x3_cross_vm_router::Transfers::<Runtime>::iter().count(),
            0
        );
        assert_eq!(Supply::ledger(&asset).unwrap(), before);
        // Rejected routing never locks funds: the entire initial balance remains usable.
        let recipient = AccountId::new([32; 32]);
        assert_ok!(Kernel::ledger_transfer(
            ledger_asset,
            &creator,
            &recipient,
            1_000
        ));
        assert_eq!(Kernel::ledger_balance_of(ledger_asset, &creator), 0);
        assert_eq!(Kernel::ledger_balance_of(ledger_asset, &recipient), 1_000);
        assert_eq!(account_total(), before.canonical_supply);
    });
}

#[test]
fn legacy_factory_pending_supply_can_only_be_refunded_not_completed() {
    sp_io::TestExternalities::default().execute_with(|| {
        System::set_block_number(1);
        let creator = AccountId::new([41; 32]);
        let (asset, ledger_asset) = issue_factory_token(&creator);
        let before = Supply::ledger(&asset).unwrap();
        // Reproduce a pending transfer persisted by the previous supply-only adapter.
        // Its source account was never debited. Completion must not create a second
        // representation; refund must remain available for recovery.
        assert_ok!(Supply::debit_source_to_pending(
            &asset,
            DomainId::X3Native,
            100
        ));
        type RouterLedger = <Runtime as pallet_x3_cross_vm_router::Config>::Ledger;
        assert_noop!(
            RouterLedger::credit_destination_from_pending(&asset, DomainId::X3Evm, 100),
            DispatchError::Other("FactoryTokenCrossVmAccountingUnavailable")
        );
        assert_eq!(Kernel::ledger_balance_of(ledger_asset, &creator), 1_000);
        assert_eq!(RouterLedger::ledger(&asset).unwrap().pending_supply, 100);
        assert_ok!(RouterLedger::refund_pending_to_source(
            &asset,
            DomainId::X3Native,
            100
        ));
        assert_eq!(RouterLedger::ledger(&asset).unwrap(), before);
        assert_noop!(
            RouterLedger::refund_pending_to_source(&asset, DomainId::X3Native, 100),
            pallet_x3_supply_ledger::Error::<Runtime>::Underflow
        );
        assert_eq!(Kernel::ledger_balance_of(ledger_asset, &creator), 1_000);
    });
}

#[test]
fn non_factory_supply_routes_keep_their_existing_transactional_behavior() {
    sp_io::TestExternalities::default().execute_with(|| {
        System::set_block_number(1);
        type Registry = pallet_x3_asset_registry::Pallet<Runtime>;
        type RouterLedger = <Runtime as pallet_x3_cross_vm_router::Config>::Ledger;
        let asset = Registry::do_register_asset(
            b"LEGACY".to_vec(),
            b"Existing supply asset".to_vec(),
            6,
            DomainId::X3Native,
            0,
            vec![51; 32],
            SupplyPolicy::NativeMintBurn,
        )
        .unwrap();
        assert_ok!(Registry::do_activate_asset(&asset));
        assert_ok!(Supply::do_mint_canonical(&asset, DomainId::X3Native, 1_000));
        assert_ok!(RouterLedger::debit_source_to_pending(
            &asset,
            DomainId::X3Native,
            100
        ));
        assert_ok!(RouterLedger::credit_destination_from_pending(
            &asset,
            DomainId::X3Evm,
            100
        ));
        let settled = RouterLedger::ledger(&asset).unwrap();
        assert_eq!(settled.native_supply, 900);
        assert_eq!(settled.evm_supply, 100);
        assert_eq!(settled.pending_supply, 0);
        assert_eq!(settled.canonical_supply, 1_000);
        assert_ok!(RouterLedger::debit_source_to_pending(
            &asset,
            DomainId::X3Evm,
            100
        ));
        assert_ok!(RouterLedger::refund_pending_to_source(
            &asset,
            DomainId::X3Evm,
            100
        ));
        assert_eq!(RouterLedger::ledger(&asset).unwrap(), settled);
    });
}

#[test]
fn legacy_factory_router_messages_refund_after_expiry_exactly_once() {
    use x3_asset_kernel_types::{
        derive_message_id, TransferStatus, X3TransferMessage, MESSAGE_FORMAT_VERSION,
    };
    use x3_packet_standard::packet::{Packet, PacketCommitment};

    sp_io::TestExternalities::default().execute_with(|| {
        System::set_block_number(1);
        let creator = AccountId::new([61; 32]);
        let (asset, ledger_asset) = issue_factory_token(&creator);
        let before = Supply::ledger(&asset).unwrap();
        assert_ok!(pallet_x3_custody::Pallet::<Runtime>::authorize_gateway(
            RuntimeOrigin::root(),
            pallet_x3_custody::GatewayRole::X3Lang,
            creator.clone(),
        ));
        // Restore the actual storage shape of a pre-upgrade supply-only transfer.
        let message = X3TransferMessage {
            version: MESSAGE_FORMAT_VERSION,
            asset_id: asset,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender: AccountBytes::X3Native([61; 32]),
            recipient: AccountBytes::Evm([62; 20]),
            amount: 100,
            nonce: 0,
            created_at: 1u32,
            expires_at: 10u32,
        };
        let message_id = derive_message_id(&message);
        let id = |bytes: &[u8]| {
            let mut out = [0; 32];
            out[..bytes.len()].copy_from_slice(bytes);
            out
        };
        let packet = Packet::try_new(
            id(b"x3-native"),
            id(b"x3-cross-vm-router"),
            id(b"x3-evm"),
            id(b"x3-cross-vm-router"),
            0,
            10,
            0,
            message.encode(),
        )
        .unwrap();
        assert_ok!(Supply::debit_source_to_pending(
            &asset,
            DomainId::X3Native,
            100
        ));
        pallet_x3_cross_vm_router::Transfers::<Runtime>::insert(
            message_id,
            pallet_x3_cross_vm_router::TransferRecord::<Runtime> {
                message,
                status: TransferStatus::SourceDebited,
            },
        );
        pallet_x3_cross_vm_router::UsedMessages::<Runtime>::insert(message_id, ());
        pallet_x3_cross_vm_router::PendingCount::<Runtime>::insert(
            asset,
            (DomainId::X3Native, DomainId::X3Evm),
            1,
        );
        pallet_x3_cross_vm_router::PacketCommitments::<Runtime>::insert(
            message_id,
            PacketCommitment::of(&packet).0,
        );
        assert_noop!(
            Router::complete_xvm_transfer(RuntimeOrigin::signed(creator.clone()), message_id),
            DispatchError::Other("FactoryTokenCrossVmAccountingUnavailable")
        );
        assert_noop!(
            Router::cancel_expired_xvm_transfer(RuntimeOrigin::signed(creator.clone()), message_id),
            pallet_x3_cross_vm_router::Error::<Runtime>::NotYetExpired
        );
        assert_eq!(Kernel::ledger_balance_of(ledger_asset, &creator), 1_000);
        assert_eq!(Supply::ledger(&asset).unwrap().pending_supply, 100);
        System::set_block_number(11);
        assert_ok!(Router::cancel_expired_xvm_transfer(
            RuntimeOrigin::signed(creator.clone()),
            message_id
        ));
        assert_eq!(Supply::ledger(&asset).unwrap(), before);
        assert_eq!(Kernel::ledger_balance_of(ledger_asset, &creator), 1_000);
        assert_eq!(
            pallet_x3_cross_vm_router::PendingCount::<Runtime>::get(
                asset,
                (DomainId::X3Native, DomainId::X3Evm)
            ),
            0
        );
        assert_eq!(
            Router::transfers(message_id).unwrap().status,
            TransferStatus::Refunded
        );
        assert_noop!(
            Router::cancel_expired_xvm_transfer(RuntimeOrigin::signed(creator.clone()), message_id),
            pallet_x3_cross_vm_router::Error::<Runtime>::IllegalStateTransition
        );
        assert_noop!(
            Router::complete_xvm_transfer(RuntimeOrigin::signed(creator), message_id),
            pallet_x3_cross_vm_router::Error::<Runtime>::IllegalStateTransition
        );
    });
}
