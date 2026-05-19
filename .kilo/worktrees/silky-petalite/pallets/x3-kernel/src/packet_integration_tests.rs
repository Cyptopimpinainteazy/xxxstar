//! Integration tests for packet deserialization and routing (Phase 1.3)
//!
//! These tests verify the full flow from raw Vec<u8> payloads through
//! packet deserialization and domain routing.

#[cfg(test)]
mod integration_tests {
    use frame_support::assert_ok;
    use parity_scale_codec::Encode;
    use x3_packet_schema::{
        EvmCall, EvmPacket, Packet, SvmAccount, SvmDeployMetadata, SvmPacket, X3VmPacket, U256,
    };

    use crate::{
        mock::new_test_ext,
        packet_adapters::{deserialize_packet, route_packet, validate_packet, DomainRoute},
    };

    type Test = crate::mock::Test;

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
            let evm_bytes = evm_packet_obj.encode();

            // Create an SVM packet and serialize it
            let svm_packet_obj = SvmPacket::Invoke {
                program_id: [0x99; 32],
                accounts: vec![],
                data: vec![0xff, 0xee],
            };
            let svm_bytes = svm_packet_obj.encode();

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
}
