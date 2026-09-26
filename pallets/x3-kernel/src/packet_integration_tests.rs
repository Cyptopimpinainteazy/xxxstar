//! Integration tests for packet deserialization and routing (Phase 1.3 / 1.4)
//!
//! These tests verify the full flow from raw Vec<u8> payloads through
//! packet deserialization and domain routing.
//!
//! Phase 1.4 tests verify that malformed and wrong-domain packets are
//! rejected at the extrinsic level (submit_comit / submit_comit_v2).

#[cfg(test)]
mod integration_tests {
    use frame_support::assert_ok;
    use parity_scale_codec::Encode;
    use x3_packet_schema::{EvmCall, EvmPacket, Packet, SvmAccount, SvmPacket, X3VmPacket, U256};

    use crate::{
        mock::new_test_ext,
        packet_adapters::{deserialize_packet, route_packet, validate_packet, DomainRoute},
    };

    /// Test 1: Deserialize and route EVM Call packet
    #[test]
    fn test_evm_call_packet_deserialization() {
        new_test_ext().execute_with(|| {
            // Create an EVM Call packet
            let packet = Packet::Evm(EvmPacket::Call {
                contract: [0x42; 20],
                function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
                args: vec![1, 2, 3, 4, 5],
                value: U256::from(1000u64),
            });

            // Serialize to bytes
            let payload = packet.encode();
            assert!(!payload.is_empty(), "Packet should serialize");

            // Deserialize back
            let deserialized = deserialize_packet(&payload);
            assert!(deserialized.is_ok(), "Deserialization should succeed");

            let deserialized_packet = deserialized.unwrap();
            assert_eq!(
                deserialized_packet.domain_mask(),
                0b0001,
                "Should be EVM domain"
            );

            // Validate packet
            let validation = validate_packet(&deserialized_packet);
            assert!(validation.is_ok(), "Packet should pass validation");

            // Route packet
            let route = route_packet(&deserialized_packet);
            assert!(route.is_ok(), "Routing should succeed");
            assert_eq!(route.unwrap(), DomainRoute::EvmOnly, "Should route to EVM");

            println!("✅ Phase 1.3: EVM Call packet deserialized and routed correctly");
        });
    }

    /// Test 2: Deserialize and route EVM Deploy packet
    #[test]
    fn test_evm_deploy_packet_deserialization() {
        new_test_ext().execute_with(|| {
            // Create an EVM Deploy packet
            let bytecode = vec![0x60, 0x60, 0x60, 0x40]; // Simple bytecode
            let packet = Packet::Evm(EvmPacket::Deploy {
                bytecode: bytecode.clone(),
                args: vec![10, 20],
                value: U256::from(5000u64),
            });

            // Serialize and deserialize
            let payload = packet.encode();
            let deserialized = deserialize_packet(&payload);
            assert!(deserialized.is_ok(), "Deserialization should succeed");

            let deserialized_packet = deserialized.unwrap();

            // Validate and route
            assert_ok!(validate_packet(&deserialized_packet));
            let route = route_packet(&deserialized_packet);
            assert_eq!(route.unwrap(), DomainRoute::EvmOnly, "Should route to EVM");

            println!("✅ Phase 1.3: EVM Deploy packet deserialized and routed correctly");
        });
    }

    /// Test 3: Deserialize and route SVM Invoke packet
    #[test]
    fn test_svm_invoke_packet_deserialization() {
        new_test_ext().execute_with(|| {
            // Create an SVM Invoke packet
            let accounts = vec![
                SvmAccount {
                    pubkey: [0x01; 32],
                    is_signer: true,
                    is_writable: true,
                    is_executable: false,
                    lamports: 0,
                    owner: [0u8; 32],
                },
                SvmAccount {
                    pubkey: [0x02; 32],
                    is_signer: false,
                    is_writable: false,
                    is_executable: false,
                    lamports: 0,
                    owner: [0u8; 32],
                },
            ];

            let packet = Packet::Svm(SvmPacket::Invoke {
                program_id: [0x99; 32],
                accounts: accounts.clone(),
                data: vec![0xaa, 0xbb, 0xcc],
            });

            // Serialize and deserialize
            let payload = packet.encode();
            let deserialized = deserialize_packet(&payload);
            assert!(deserialized.is_ok(), "Deserialization should succeed");

            let deserialized_packet = deserialized.unwrap();
            assert_eq!(
                deserialized_packet.domain_mask(),
                0b0010,
                "Should be SVM domain"
            );

            // Validate and route
            assert_ok!(validate_packet(&deserialized_packet));
            let route = route_packet(&deserialized_packet);
            assert_eq!(route.unwrap(), DomainRoute::SvmOnly, "Should route to SVM");

            println!("✅ Phase 1.3: SVM Invoke packet deserialized and routed correctly");
        });
    }

    /// Test 4: Deserialize and route X3VM AtomicCross packet
    #[test]
    fn test_x3vm_atomic_cross_packet_deserialization() {
        new_test_ext().execute_with(|| {
            // Create an EVM packet and serialize it
            let evm_packet_obj = EvmPacket::Call {
                contract: [0x42; 20],
                function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
                args: vec![1, 2, 3],
                value: U256::from(100u64),
            };

            // Create an SVM packet and serialize it
            let svm_packet_obj = SvmPacket::Invoke {
                program_id: [0x99; 32],
                accounts: vec![],
                data: vec![0xff, 0xee],
            };

            // Create an X3VM AtomicCross packet with packet objects (Box-wrapped)
            let packet = Packet::X3Vm(X3VmPacket::AtomicCross {
                evm: Some(Box::new(evm_packet_obj)),
                svm: Some(Box::new(svm_packet_obj)),
                atomic: true,
            });

            // Serialize and deserialize
            let payload = packet.encode();
            let deserialized = deserialize_packet(&payload);
            assert!(deserialized.is_ok(), "Deserialization should succeed");

            let deserialized_packet = deserialized.unwrap();
            assert_eq!(
                deserialized_packet.domain_mask(),
                0b0100,
                "Should be X3VM domain"
            );

            // Validate and route
            assert_ok!(validate_packet(&deserialized_packet));
            let route = route_packet(&deserialized_packet);
            assert_eq!(
                route.unwrap(),
                DomainRoute::EvmAndSvm,
                "Should route to both EVM and SVM"
            );

            println!("✅ Phase 1.3: X3VM AtomicCross packet deserialized and routed correctly");
        });
    }

    /// Test 5: Empty payload should fail deserialization
    #[test]
    fn test_empty_payload_deserialization_fails() {
        new_test_ext().execute_with(|| {
            let empty_payload: Vec<u8> = vec![];
            let result = deserialize_packet(&empty_payload);
            assert!(result.is_err(), "Empty payload should fail deserialization");

            println!("✅ Phase 1.3: Empty payload correctly rejected");
        });
    }

    /// Test 6: Oversized payload should fail
    #[test]
    fn test_oversized_payload_deserialization_fails() {
        new_test_ext().execute_with(|| {
            // Create a payload that exceeds 65535 bytes
            let oversized_payload = vec![0u8; 65536];
            let result = deserialize_packet(&oversized_payload);
            assert!(
                result.is_err(),
                "Oversized payload should fail deserialization"
            );

            println!("✅ Phase 1.3: Oversized payload correctly rejected");
        });
    }

    /// Test 7: Corrupted payload should fail deserialization
    #[test]
    fn test_corrupted_payload_deserialization_fails() {
        new_test_ext().execute_with(|| {
            // Create a payload that's just too short (less than minimum packet size)
            let corrupted_payload = vec![0u8; 10]; // Too short to be valid
            let result = deserialize_packet(&corrupted_payload);
            assert!(
                result.is_err(),
                "Corrupted payload should fail deserialization"
            );

            println!("✅ Phase 1.3: Corrupted payload correctly rejected");
        });
    }

    /// Test 8: Round-trip serialization/deserialization should be idempotent
    #[test]
    fn test_packet_round_trip_idempotence() {
        new_test_ext().execute_with(|| {
            // Create an original packet
            let original = Packet::Evm(EvmPacket::Call {
                contract: [0x42; 20],
                function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
                args: vec![1, 2, 3, 4, 5],
                value: U256::from(1234u64),
            });

            // Serialize
            let bytes1 = original.encode();

            // Deserialize
            let deserialized = deserialize_packet(&bytes1).expect("First round should succeed");

            // Re-serialize
            let bytes2 = deserialized.encode();

            // Both serializations should be identical
            assert_eq!(
                bytes1, bytes2,
                "Round-trip serialization should be idempotent"
            );

            println!("✅ Phase 1.3: Packet round-trip serialization is idempotent");
        });
    }

    /// Test 9: Domain mask routing is consistent
    #[test]
    fn test_domain_mask_routing_consistency() {
        new_test_ext().execute_with(|| {
            // Test EVM routing
            let evm_packet = Packet::Evm(EvmPacket::Call {
                contract: [0u8; 20],
                function_selector: [0u8; 4],
                args: Vec::new(),
                value: U256::zero(),
            });
            assert_eq!(evm_packet.domain_mask(), 0b0001);
            assert_eq!(route_packet(&evm_packet).unwrap(), DomainRoute::EvmOnly);

            // Test SVM routing
            let svm_packet = Packet::Svm(SvmPacket::Invoke {
                program_id: [0u8; 32],
                accounts: Vec::new(),
                data: Vec::new(),
            });
            assert_eq!(svm_packet.domain_mask(), 0b0010);
            assert_eq!(route_packet(&svm_packet).unwrap(), DomainRoute::SvmOnly);

            // Test X3VM routing
            let x3vm_packet = Packet::X3Vm(X3VmPacket::AtomicCross {
                evm: None,
                svm: None,
                atomic: false,
            });
            assert_eq!(x3vm_packet.domain_mask(), 0b0100);

            println!("✅ Phase 1.3: Domain mask routing is consistent across all packet types");
        });
    }

    /// Test 10: Large valid payload should deserialize successfully
    #[test]
    fn test_large_valid_payload() {
        new_test_ext().execute_with(|| {
            // Create an EVM Batch with many calls
            let mut calls = vec![];
            for i in 0..100 {
                calls.push((
                    EvmCall {
                        contract: [i as u8; 20],
                        function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
                        args: vec![i as u8; 50],
                    },
                    Some(U256::from(i as u64)),
                ));
            }

            let packet = Packet::Evm(EvmPacket::Batch {
                calls,
                continue_on_revert: true,
            });

            let payload = packet.encode();
            assert!(payload.len() <= 65535, "Payload should be within limits");

            let deserialized = deserialize_packet(&payload);
            assert!(
                deserialized.is_ok(),
                "Large valid payload should deserialize"
            );

            println!(
                "✅ Phase 1.3: Large valid payload ({} bytes) deserialized successfully",
                payload.len()
            );
        });
    }

    // ── Phase 1.4: Extrinsic-level packet validation regression tests ──

    /// X3-LANG-004: a payload is the artifact the adapter in that slot executes, so these pin the
    /// boundary at the extrinsic. The module was `phase14_extrinsic_validation` and asserted the
    /// opposite — that every non-empty payload had to SCALE-decode as a `Packet` carrying the
    /// slot's domain bit. That rule accepted exactly the payloads no adapter can execute (each
    /// adapter runs its input as code, and `Packet`'s first byte is the discriminant `0x00`, i.e.
    /// EVM `STOP`) and rejected the ones they can, so the cases now read the other way round.
    mod payload_convention {
        use frame_support::{assert_err, assert_ok};
        use parity_scale_codec::Encode;
        use sp_core::H256;
        use x3_packet_schema::{EvmPacket, Packet, SvmPacket, U256};

        use crate::{
            mock::{new_test_ext, AtlasKernel, RuntimeOrigin, Test, ALICE},
            test_helpers::{wrap_evm_payload, wrap_svm_payload},
            Error,
        };

        /// A packet is a semantic operation, not bytecode: the EVM slot refuses one by name.
        #[test]
        fn a_packet_is_refused_on_the_evm_slot() {
            new_test_ext().execute_with(|| {
                let svm_packet = Packet::Svm(SvmPacket::Invoke {
                    program_id: [0u8; 32],
                    accounts: Vec::new(),
                    data: vec![1, 2, 3],
                });
                let payload = svm_packet.encode();
                assert!(payload.len() >= 30, "the fixture has to be a packet");

                assert_err!(
                    AtlasKernel::submit_comit(
                        RuntimeOrigin::signed(ALICE),
                        H256::repeat_byte(0x01), // comit_id
                        payload,                 // evm_payload (a packet)
                        Vec::new(),              // svm_payload
                        0,                       // nonce
                        0,                       // fee
                        H256::zero(),            // prepare_root
                    ),
                    Error::<Test>::InvalidEvmPacket
                );
            });
        }

        /// A payload the EVM validator refuses never reaches execution.
        ///
        /// `mini_evm::validate_evm` rejects bytecode leading with `0xEF` (EIP-3541 reserves that
        /// prefix) — the one rejection that validator actually performs. The case it replaces fed
        /// 40 bytes of `0xDE 0xAD 0xBE 0xEF` and expected a refusal for not being a packet; under
        /// the convention that refusal is gone, because those bytes *are* accepted as bytecode.
        #[test]
        fn an_evm_payload_the_validator_refuses_is_rejected() {
            new_test_ext().execute_with(|| {
                let mut refused = vec![0u8; 40];
                refused[0] = 0xEF;

                let result = AtlasKernel::submit_comit(
                    RuntimeOrigin::signed(ALICE),
                    H256::repeat_byte(0x02),
                    refused,    // evm_payload (EIP-3541 reserved prefix)
                    Vec::new(), // svm_payload
                    0,          // nonce
                    0,          // fee
                    H256::zero(),
                );

                assert_err!(result, Error::<Test>::InvalidEvmPacket);
            });
        }

        /// A short payload is bytecode too, and is accepted: length is not what makes a payload
        /// valid, the adapter's own validator is. Phase 1.4 rejected anything under 30 bytes
        /// because it had to be a packet — the rule X3-LANG-004 retired.
        ///
        /// The fee is what the test adapters report (21_000 EVM gas + 5_000 SVM compute, priced at
        /// 1000 per unit), so this reaches execution rather than failing on price.
        #[test]
        fn a_short_payload_is_bytecode_and_is_accepted() {
            new_test_ext().execute_with(|| {
                let short_payload = wrap_evm_payload(&[0x01; 5]);
                assert!(
                    short_payload.len() < 30,
                    "the case is only meaningful below the old 30-byte threshold"
                );

                let comit_id = H256::repeat_byte(0x03);
                let prepare_root =
                    AtlasKernel::compute_prepare_root(comit_id, &short_payload, &[], 0, 21);
                assert_ok!(AtlasKernel::submit_comit(
                    RuntimeOrigin::signed(ALICE),
                    comit_id,
                    short_payload,
                    Vec::new(),
                    0,
                    21,
                    prepare_root,
                ));
            });
        }

        /// The point of the convention: EVM bytecode and an SVM program in one comit both pass
        /// validation and execute.
        ///
        /// The old body asserted only that two *packets* were not refused with the two payload
        /// errors — and did so with `if let Err(e)`, so an `Ok` or any third error passed too. This
        /// requires the submission to be accepted outright.
        #[test]
        fn evm_bytecode_and_svm_program_payloads_are_accepted() {
            new_test_ext().execute_with(|| {
                let evm_payload = wrap_evm_payload(&[1, 2, 3]);
                let svm_payload = wrap_svm_payload(&[4, 5, 6]);
                assert!(!crate::packet_adapters::payload_is_packet(&evm_payload));
                assert!(!crate::packet_adapters::payload_is_packet(&svm_payload));

                // 21_000 EVM gas + 5_000 SVM compute over the pallet's 1000-per-unit divisor.
                let comit_id = H256::repeat_byte(0x04);
                let prepare_root =
                    AtlasKernel::compute_prepare_root(comit_id, &evm_payload, &svm_payload, 0, 26);
                assert_ok!(AtlasKernel::submit_comit(
                    RuntimeOrigin::signed(ALICE),
                    comit_id,
                    evm_payload,
                    svm_payload,
                    0,
                    26,
                    prepare_root,
                ));
            });
        }

        /// A packet is not a program: the SVM slot refuses one by name.
        #[test]
        fn a_packet_is_refused_on_the_svm_slot() {
            new_test_ext().execute_with(|| {
                let evm_packet = Packet::Evm(EvmPacket::Call {
                    contract: [0x42; 20],
                    function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
                    args: vec![1, 2, 3],
                    value: U256::from(100u64),
                });
                let wrong_domain_payload = evm_packet.encode();
                // An EVM-shaped packet sent as the SVM payload: the shape is what is refused.

                let result = AtlasKernel::submit_comit(
                    RuntimeOrigin::signed(ALICE),
                    H256::repeat_byte(0x05),
                    Vec::new(),           // evm_payload empty
                    wrong_domain_payload, // svm_payload (actually EVM)
                    0,
                    0,
                    H256::zero(),
                );

                assert_err!(result, Error::<Test>::InvalidSvmPacket);
            });
        }
    }
}
