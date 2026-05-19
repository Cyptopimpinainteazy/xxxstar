use super::pallet::{Error, X3DnsRecord, X3RecordData};
use crate::mock::{new_test_ext, RuntimeOrigin, X3DomainRegistry};
use frame_support::{assert_noop, assert_ok, BoundedVec};

#[test]
fn register_non_x3_domain_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            X3DomainRegistry::register_domain(RuntimeOrigin::signed(1), b"example.com".to_vec()),
            Error::<crate::mock::Test>::NotX3Domain
        );
    });
}

#[test]
fn register_x3_domain_succeeds() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3DomainRegistry::register_domain(
            RuntimeOrigin::signed(1),
            b"rpc.testnet.x3".to_vec()
        ));
    });
}

#[test]
fn set_records_requires_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3DomainRegistry::register_domain(
            RuntimeOrigin::signed(1),
            b"rpc.testnet.x3".to_vec()
        ));

        let a = X3DnsRecord::<crate::mock::Test> {
            ttl: 300,
            data: X3RecordData::A([127, 0, 0, 1]),
        };

        assert_noop!(
            X3DomainRegistry::set_records(
                RuntimeOrigin::signed(2),
                b"rpc.testnet.x3".to_vec(),
                vec![a.clone()]
            ),
            Error::<crate::mock::Test>::NotDomainOwner
        );

        assert_ok!(X3DomainRegistry::set_records(
            RuntimeOrigin::signed(1),
            b"rpc.testnet.x3".to_vec(),
            vec![a]
        ));
    });
}

#[test]
fn runtime_api_records_empty_for_invalid_domain() {
    new_test_ext().execute_with(|| {
        let records = X3DomainRegistry::runtime_get_records(b"example.com".to_vec());
        assert!(records.is_empty());
    });
}

#[test]
fn runtime_api_records_roundtrip() {
    new_test_ext().execute_with(|| {
        assert_ok!(X3DomainRegistry::register_domain(
            RuntimeOrigin::signed(1),
            b"rpc.testnet.x3".to_vec()
        ));

        let txt: BoundedVec<u8, crate::mock::MaxTxtLen> = b"hello".to_vec().try_into().unwrap();
        let rec = X3DnsRecord::<crate::mock::Test> {
            ttl: 60,
            data: X3RecordData::Txt(txt),
        };

        assert_ok!(X3DomainRegistry::set_records(
            RuntimeOrigin::signed(1),
            b"rpc.testnet.x3".to_vec(),
            vec![rec]
        ));

        let records = X3DomainRegistry::runtime_get_records(b"rpc.testnet.x3".to_vec());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].ttl, 60);
        assert_eq!(records[0].rr_type, 16);
        assert_eq!(records[0].data, b"hello".to_vec());
    });
}
