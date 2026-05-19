#![cfg_attr(not(feature = "std"), no_std)]

use crc32fast::Hasher;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::prelude::vec::Vec;
use scale_info::TypeInfo;

mod evm;
mod header;
mod svm;
mod x3vm;

pub use evm::{EvmCall, EvmPacket, U256};
pub use header::PacketHeader;
pub use svm::{SvmAccount, SvmDeployMetadata, SvmPacket};
pub use x3vm::{X3Condition, X3VmPacket};

/// Top-level packet wrapper for router dispatch
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum Packet {
    Evm(EvmPacket),
    Svm(SvmPacket),
    X3Vm(X3VmPacket),
}

/// Packet type identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketType {
    Evm = 0,
    Svm = 1,
    X3Vm = 2,
}

impl Packet {
    /// Get the domain mask for routing
    pub fn domain_mask(&self) -> u8 {
        match self {
            Packet::Evm(_) => 0b0001,
            Packet::Svm(_) => 0b0010,
            Packet::X3Vm(_) => 0b0100,
        }
    }

    /// Get packet type identifier
    pub fn packet_type(&self) -> u8 {
        match self {
            Packet::Evm(_) => 0,
            Packet::Svm(_) => 1,
            Packet::X3Vm(_) => 2,
        }
    }

    /// Create a new packet with automatic header generation
    pub fn new(
        packet_type: PacketType,
        payload: Vec<u8>,
        domain_mask: u8,
    ) -> Result<Self, &'static str> {
        if domain_mask == 0 {
            return Err("Must target at least one domain");
        }
        if domain_mask & !0b0111 != 0 {
            return Err("Invalid domain mask bits set");
        }

        let packet = match packet_type {
            PacketType::Evm => {
                let evm_packet = EvmPacket::decode(&mut &payload[..])
                    .map_err(|_| "Failed to decode EVM packet payload")?;
                Packet::Evm(evm_packet)
            }
            PacketType::Svm => {
                let svm_packet = SvmPacket::decode(&mut &payload[..])
                    .map_err(|_| "Failed to decode SVM packet payload")?;
                Packet::Svm(svm_packet)
            }
            PacketType::X3Vm => {
                let x3vm_packet = X3VmPacket::decode(&mut &payload[..])
                    .map_err(|_| "Failed to decode X3VM packet payload")?;
                Packet::X3Vm(x3vm_packet)
            }
        };

        Ok(packet)
    }

    /// Serialize to Vec<u8> (SCALE codec only, no header/CRC)
    pub fn to_bytes(&self) -> Vec<u8> {
        self.encode()
    }

    /// Deserialize from raw SCALE bytes (no header/CRC)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, parity_scale_codec::Error> {
        Self::decode(&mut &bytes[..])
    }

    /// Serialize to full wire format: [Header: 32][Type: 1][Payload: n][CRC32: 4]
    pub fn to_wire_format(&self) -> Result<Vec<u8>, &'static str> {
        let payload = self.encode();
        if payload.len() > u16::MAX as usize {
            return Err("Payload exceeds maximum size");
        }
        let payload_size = payload.len() as u16;

        let domain_mask = self.domain_mask();
        let packet_type = self.packet_type();

        // Build header
        let header = PacketHeader::new(1, domain_mask, payload_size);
        let mut header_bytes = header.encode();

        // Serialize packet type (SCALE: single byte)
        let type_byte = packet_type.encode();

        // Calculate CRC32 over header + type + payload
        let mut hasher = Hasher::new();
        hasher.update(&header_bytes);
        hasher.update(&type_byte);
        hasher.update(&payload);
        let crc32 = hasher.finalize();

        // Combine: header + type + payload + crc32
        let mut result = Vec::with_capacity(header_bytes.len() + 1 + payload.len() + 4);
        result.append(&mut header_bytes);
        result.push(type_byte[0]);
        result.extend_from_slice(&payload);
        result.extend_from_slice(&crc32.to_le_bytes());

        Ok(result)
    }

    /// Deserialize from full wire format
    pub fn from_wire_format(bytes: &[u8]) -> Result<Self, &'static str> {
        // Streaming decode: header first
        let mut cursor = bytes;
        let header =
            PacketHeader::decode(&mut cursor).map_err(|_| "Failed to decode packet header")?;

        // Compute header bytes for CRC (portion already consumed)
        let header_bytes = &bytes[..bytes.len() - cursor.len()];

        // Validate header
        header.validate()?;

        // Need at least: type(1) + payload(payload_size) + CRC(4)
        let needed = 1usize + header.payload_size as usize + 4;
        if cursor.len() < needed {
            return Err("Packet too short for payload + CRC");
        }

        // Decode packet type (u8, 1 byte)
        let packet_type = u8::decode(&mut cursor).map_err(|_| "Failed to decode packet type")?;

        // Extract payload (exact size from header)
        let payload = &cursor[..header.payload_size as usize];
        let after_payload = &cursor[payload.len()..];

        // CRC is next 4 bytes
        let (crc_bytes, _rest) = after_payload.split_at(4);

        // Verify CRC32 over (header_bytes || type_byte || payload)
        let mut hasher = Hasher::new();
        hasher.update(header_bytes);
        hasher.update(&[packet_type]);
        hasher.update(payload);
        let computed_crc = hasher.finalize();

        let expected_crc =
            u32::from_le_bytes(crc_bytes.try_into().map_err(|_| "Invalid CRC length")?);

        if computed_crc != expected_crc {
            return Err("CRC32 mismatch");
        }

        // Deserialize packet based on type
        let packet = match packet_type {
            0 => {
                let evm_packet = EvmPacket::decode(&mut &payload[..])
                    .map_err(|_| "Failed to decode EVM packet")?;
                Packet::Evm(evm_packet)
            }
            1 => {
                let svm_packet = SvmPacket::decode(&mut &payload[..])
                    .map_err(|_| "Failed to decode SVM packet")?;
                Packet::Svm(svm_packet)
            }
            2 => {
                let x3vm_packet = X3VmPacket::decode(&mut &payload[..])
                    .map_err(|_| "Failed to decode X3VM packet")?;
                Packet::X3Vm(x3vm_packet)
            }
            _ => return Err("Unknown packet type"),
        };

        Ok(packet)
    }
}

#[cfg(test)]
mod integration_tests {
    use super::header::PacketHeader;
    use super::*;

    #[test]
    fn test_packet_wrapper_round_trip() {
        let evm_packet = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![],
            value: U256::from(0),
        };

        let packet = Packet::Evm(evm_packet.clone());
        let bytes = packet.to_bytes();
        let decoded = Packet::from_bytes(&bytes).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_domain_masks() {
        let evm_pkt = Packet::Evm(EvmPacket::Call {
            contract: [0; 20],
            function_selector: [0; 4],
            args: vec![],
            value: U256::zero(),
        });
        assert_eq!(evm_pkt.domain_mask(), 0b0001);

        let svm_pkt = Packet::Svm(SvmPacket::Invoke {
            program_id: [0; 32],
            accounts: vec![],
            data: vec![],
        });
        assert_eq!(svm_pkt.domain_mask(), 0b0010);

        let x3vm_pkt = Packet::X3Vm(X3VmPacket::Transfer {
            from_domain: 0,
            to_domain: 1,
            asset_id: 0,
            amount: 0,
            recipient: vec![],
        });
        assert_eq!(x3vm_pkt.domain_mask(), 0b0100);
    }

    #[test]
    fn test_packet_wire_format_round_trip() {
        let evm_packet = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![1, 2, 3, 4],
            value: U256::from(1000),
        };
        let packet = Packet::Evm(evm_packet);

        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_wire_format_svm() {
        let svm_packet = SvmPacket::Invoke {
            program_id: [0x11; 32],
            accounts: vec![SvmAccount {
                pubkey: [0x22; 32],
                is_writable: true,
                is_signer: false,
                is_executable: false,
                lamports: 1000000,
                owner: [0x33; 32],
            }],
            data: vec![0x01, 0x02, 0x03],
        };
        let packet = Packet::Svm(svm_packet);

        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_wire_format_x3vm_transfer() {
        let x3vm_packet = X3VmPacket::Transfer {
            from_domain: 0,
            to_domain: 1,
            asset_id: 5,
            amount: 1000000000000,
            recipient: vec![0xAA; 32],
        };
        let packet = Packet::X3Vm(x3vm_packet);

        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_wire_format_atomic_cross() {
        let evm = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![],
            value: U256::from(0),
        };
        let svm = SvmPacket::Invoke {
            program_id: [0x11; 32],
            accounts: vec![],
            data: vec![],
        };
        let x3vm_packet = X3VmPacket::AtomicCross {
            evm: Some(Box::new(evm)),
            svm: Some(Box::new(svm)),
            atomic: true,
        };
        let packet = Packet::X3Vm(x3vm_packet);

        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_wire_format_conditional() {
        let x3vm_packet = X3VmPacket::Conditional {
            condition: X3Condition::BlockHeightAbove { min_height: 1000 },
            if_true: Box::new(X3VmPacket::Transfer {
                from_domain: 0,
                to_domain: 1,
                asset_id: 0,
                amount: 100,
                recipient: vec![],
            }),
            if_false: None,
        };
        let packet = Packet::X3Vm(x3vm_packet);

        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_wire_format_conditional_complex() {
        let condition = X3Condition::And(vec![
            X3Condition::BlockHeightAbove { min_height: 100 },
            X3Condition::BalanceAbove {
                account: vec![0xAA; 20],
                threshold: 1000000,
            },
        ]);
        let x3vm_packet = X3VmPacket::Conditional {
            condition: condition.clone(),
            if_true: Box::new(X3VmPacket::Transfer {
                from_domain: 0,
                to_domain: 1,
                asset_id: 0,
                amount: 100,
                recipient: vec![],
            }),
            if_false: Some(Box::new(X3VmPacket::Transfer {
                from_domain: 1,
                to_domain: 0,
                asset_id: 0,
                amount: 50,
                recipient: vec![],
            })),
        };
        let packet = Packet::X3Vm(x3vm_packet);

        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_new_builder_evm() {
        let evm_call = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![1, 2, 3],
            value: U256::from(1000),
        };
        let payload = evm_call.encode();

        let packet = Packet::new(PacketType::Evm, payload, 0b0001).unwrap();

        assert!(matches!(packet, Packet::Evm(_)));
    }

    #[test]
    fn test_packet_new_builder_svm() {
        let svm_invoke = SvmPacket::Invoke {
            program_id: [0x11; 32],
            accounts: vec![],
            data: vec![0x01],
        };
        let payload = svm_invoke.encode();

        let packet = Packet::new(PacketType::Svm, payload, 0b0010).unwrap();

        assert!(matches!(packet, Packet::Svm(_)));
    }

    #[test]
    fn test_packet_new_builder_x3vm() {
        let x3vm_transfer = X3VmPacket::Transfer {
            from_domain: 0,
            to_domain: 1,
            asset_id: 1,
            amount: 1000,
            recipient: vec![0xAA; 32],
        };
        let payload = x3vm_transfer.encode();

        let packet = Packet::new(PacketType::X3Vm, payload, 0b0100).unwrap();

        assert!(matches!(packet, Packet::X3Vm(_)));
    }

    #[test]
    fn test_packet_new_builder_invalid_domain() {
        let evm_call = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![],
            value: U256::from(0),
        };
        let payload = evm_call.encode();

        let result = Packet::new(PacketType::Evm, payload, 0).unwrap_err();
        assert_eq!(result, "Must target at least one domain");
    }

    #[test]
    fn test_packet_new_builder_invalid_domain_mask_bits() {
        let evm_call = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![],
            value: U256::from(0),
        };
        let payload = evm_call.encode();

        let result = Packet::new(PacketType::Evm, payload, 0b1000).unwrap_err();
        assert_eq!(result, "Invalid domain mask bits set");
    }

    #[test]
    fn test_packet_new_builder_invalid_payload() {
        let invalid_payload = vec![0u8; 10]; // Not a valid SCALE encoding

        let result = Packet::new(PacketType::Evm, invalid_payload, 0b0001).unwrap_err();
        assert!(result.contains("decode"));
    }

    #[test]
    fn test_packet_wire_format_crc_validation() {
        let packet = Packet::Evm(EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![],
            value: U256::from(0),
        });

        let mut wire = packet.to_wire_format().unwrap();

        // Corrupt CRC bytes
        let len = wire.len();
        wire[len - 1] = 0xFF;
        wire[len - 2] = 0xFF;

        let result = Packet::from_wire_format(&wire);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CRC32"));
    }

    #[test]
    fn test_packet_wire_format_rejects_invalid_type() {
        // Build minimal packet wire format with invalid type
        let evm_packet = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![],
            value: U256::from(0),
        };
        let payload = evm_packet.encode();
        let header = PacketHeader::new(1, 0b0001, payload.len() as u16);
        let header_bytes = header.encode();
        let type_byte = 0u8.encode();
        let mut hasher = Hasher::new();
        hasher.update(&header_bytes);
        hasher.update(&type_byte);
        hasher.update(&payload);
        let crc32 = hasher.finalize();

        let mut wire = Vec::new();
        wire.extend_from_slice(&header_bytes);
        wire.push(type_byte[0]);
        wire.extend_from_slice(&payload);
        wire.extend_from_slice(&crc32.to_le_bytes());

        // Corrupt type byte (should be 0-2)
        wire[30] = 99;

        let result = Packet::from_wire_format(&wire);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown packet type"));
    }

    #[test]
    fn test_packet_max_payload_size() {
        // Create payload at the boundary (65535 is max)
        let large_payload = vec![0u8; 65535];
        let domain_mask = 0b0001;

        let header = PacketHeader::new(1, domain_mask, 65535);
        let header_bytes = header.encode();
        let type_byte = 0u8.encode();

        let mut hasher = Hasher::new();
        hasher.update(&header_bytes);
        hasher.update(&type_byte);
        hasher.update(&large_payload);
        let crc32 = hasher.finalize();

        let mut wire = Vec::new();
        wire.extend_from_slice(&header_bytes);
        wire.push(type_byte[0]);
        wire.extend_from_slice(&large_payload);
        wire.extend_from_slice(&crc32.to_le_bytes());

        // Should succeed at boundary
        let result = Packet::from_wire_format(&wire);
        assert!(result.is_ok());

        // Now test with header claiming 65536 (impossible due to u16) but we simulate by lying in header bytes
        // Actually u16 can't represent 65536, so the boundary test is sufficient
    }

    #[test]
    fn test_packet_empty_payload() {
        let evm_packet = EvmPacket::Call {
            contract: [0; 20],
            function_selector: [0; 4],
            args: vec![],
            value: U256::zero(),
        };
        let packet = Packet::Evm(evm_packet);
        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_wire_format_size() {
        let evm_packet = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![1, 2, 3],
            value: U256::from(1000),
        };
        let packet = Packet::Evm(evm_packet);

        let wire = packet.to_wire_format().unwrap();

        // Should be at least header (30 encoded) + type (1) + payload + crc (4)
        assert!(wire.len() >= 35);

        // Verify structure: CRC at the end
        let (_header_type, crc) = wire.split_at(wire.len() - 4);
        assert_eq!(crc.len(), 4);
    }

    #[test]
    fn test_packet_domain_mask_all() {
        let packet = Packet::Evm(EvmPacket::Call {
            contract: [0; 20],
            function_selector: [0; 4],
            args: vec![],
            value: U256::zero(),
        });
        // EVM packet should have EVM bit set
        assert_eq!(packet.domain_mask(), 0b0001);
    }

    #[test]
    fn test_packet_batch_round_trip() {
        let packet = EvmPacket::Batch {
            calls: vec![
                (
                    EvmCall {
                        contract: [0x11; 20],
                        function_selector: [0x12, 0x34, 0x56, 0x78],
                        args: vec![],
                    },
                    None,
                ),
                (
                    EvmCall {
                        contract: [0x22; 20],
                        function_selector: [0x98, 0x76, 0x54, 0x32],
                        args: vec![1, 2, 3],
                    },
                    Some(U256::from(1000)),
                ),
            ],
            continue_on_revert: false,
        };
        let packet = Packet::Evm(packet);

        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_x3vm_condition_or() {
        let condition = X3Condition::Or(vec![
            X3Condition::BlockHeightAbove { min_height: 100 },
            X3Condition::BlockHeightAbove { min_height: 200 },
        ]);
        let payload = condition.encode();
        let packet = Packet::new(PacketType::X3Vm, payload, 0b0100).unwrap();

        let wire = packet.to_wire_format().unwrap();
        let decoded = Packet::from_wire_format(&wire).unwrap();

        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_packet_header_checksum_calculation() {
        let payload = b"test payload data";
        let checksum = PacketHeader::calculate_checksum(payload);
        assert_ne!(checksum, 0);
    }

    #[test]
    fn test_packet_header_checksum_verification() {
        let payload = b"test data";
        let header = PacketHeader {
            checksum: PacketHeader::calculate_checksum(payload),
            ..Default::default()
        };
        assert!(header.verify_checksum(payload));
    }

    #[test]
    fn test_packet_header_checksum_fails_on_corruption() {
        let payload = b"test data";
        let header = PacketHeader {
            checksum: PacketHeader::calculate_checksum(payload),
            ..Default::default()
        };
        let corrupted = &payload[0..5]; // Different data
        assert!(!header.verify_checksum(corrupted));
    }

    #[test]
    fn test_packet_header_validation_all_domains() {
        let mut header = PacketHeader::default();
        header.domain_mask = 0b0111;
        assert!(header.targets_evm());
        assert!(header.targets_svm());
        assert!(header.targets_x3vm());
    }

    #[test]
    fn test_packet_header_validation_single_domains() {
        let mut h_evm = PacketHeader::default();
        h_evm.domain_mask = 0b0001;
        let mut h_svm = PacketHeader::default();
        h_svm.domain_mask = 0b0010;
        let mut h_x3 = PacketHeader::default();
        h_x3.domain_mask = 0b0100;

        assert!(h_evm.targets_evm() && !h_evm.targets_svm() && !h_evm.targets_x3vm());
        assert!(h_svm.targets_svm() && !h_svm.targets_evm() && !h_svm.targets_x3vm());
        assert!(h_x3.targets_x3vm() && !h_x3.targets_evm() && !h_x3.targets_svm());
    }
}

pub mod prelude {
    pub use crate::{EvmPacket, Packet, PacketHeader, PacketType, SvmPacket, X3VmPacket};
}
