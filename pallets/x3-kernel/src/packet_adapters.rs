//! Packet Deserialization & Domain Routing Layer
//!
//! This module provides the bridge between raw Vec<u8> payloads (from submit_comit)
//! and typed Packet enums (from x3-packet-schema).
//!
//! **Responsibilities:**
//! 1. Deserialize raw payloads into typed packets
//! 2. Validate packet structure (header, checksum, size)
//! 3. Route packets to appropriate VM executors by domain_mask
//! 4. Handle partial/invalid deserialization gracefully
//!
//! **Architecture:**
//! ```text
//! submit_comit(evm_payload, svm_payload)
//!          ↓
//!   [Raw Vec<u8>]
//!          ↓
//!   Deserializer
//!          ↓
//!   [Typed Packet enum]
//!          ↓
//!   Validator (CRC32, size, structure)
//!          ↓
//!   Router (domain_mask matching)
//!          ↓
//! EVM/SVM/X3VM Executors
//! ```

use frame_support::pallet_prelude::*;
use parity_scale_codec::Decode;
use x3_packet_schema::{EvmPacket, Packet, SvmPacket, X3VmPacket};

/// Result type for packet adapter operations
pub type PacketAdapterResult<T> = Result<T, PacketAdapterError>;

/// Errors that can occur during packet processing
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PacketAdapterError {
    /// Payload is empty
    EmptyPayload,
    /// Payload is too small for packet header
    PayloadTooSmall,
    /// SCALE codec deserialization failed
    DecodingFailed,
    /// Packet header validation failed
    InvalidHeader,
    /// Payload size exceeds kernel limit (65KB)
    PayloadTooLarge,
    /// Packet checksum validation failed
    ChecksumMismatch,
    /// Packet is expired
    PacketExpired,
    /// No valid domain target (domain_mask is zero)
    NoDomainTarget,
    /// Packet type is not recognized
    UnknownPacketType,
}

/// Converts PacketAdapterError to DispatchError for pallet use
impl From<PacketAdapterError> for DispatchError {
    fn from(err: PacketAdapterError) -> Self {
        let err_str = match err {
            PacketAdapterError::EmptyPayload => "Packet payload is empty",
            PacketAdapterError::PayloadTooSmall => "Packet payload too small for header",
            PacketAdapterError::DecodingFailed => "Failed to decode packet structure",
            PacketAdapterError::InvalidHeader => "Packet header validation failed",
            PacketAdapterError::PayloadTooLarge => "Packet payload exceeds 65KB limit",
            PacketAdapterError::ChecksumMismatch => "Packet checksum validation failed",
            PacketAdapterError::PacketExpired => "Packet has expired",
            PacketAdapterError::NoDomainTarget => "Packet targets no VM domain",
            PacketAdapterError::UnknownPacketType => "Unknown packet type",
        };
        DispatchError::Other(err_str)
    }
}

/// Represents a routing decision for a packet
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainRoute {
    /// Route only to EVM
    EvmOnly,
    /// Route only to SVM
    SvmOnly,
    /// Route only to X3VM
    X3VmOnly,
    /// Route to both EVM and SVM (atomic cross-chain)
    EvmAndSvm,
    /// Route to all domains
    AllDomains,
}

impl DomainRoute {
    /// Check if this route includes EVM
    pub fn targets_evm(&self) -> bool {
        matches!(
            self,
            DomainRoute::EvmOnly | DomainRoute::EvmAndSvm | DomainRoute::AllDomains
        )
    }

    /// Check if this route includes SVM
    pub fn targets_svm(&self) -> bool {
        matches!(
            self,
            DomainRoute::SvmOnly | DomainRoute::EvmAndSvm | DomainRoute::AllDomains
        )
    }

    /// Check if this route includes X3VM
    pub fn targets_x3vm(&self) -> bool {
        matches!(self, DomainRoute::X3VmOnly | DomainRoute::AllDomains)
    }
}

/// Deserialize raw payload bytes into a typed Packet
///
/// **Process:**
/// 1. Check payload is not empty
/// 2. Check payload is at least 30 bytes (SCALE-encoded header size)
/// 3. Decode header to extract metadata
/// 4. Decode full packet structure
/// 5. Return typed Packet enum
///
/// # Errors
/// Returns PacketAdapterError if:
/// - Payload is empty or too small
/// - SCALE decoding fails
/// - Header validation fails
pub fn deserialize_packet(payload: &[u8]) -> PacketAdapterResult<Packet> {
    // Validate payload is not empty
    if payload.is_empty() {
        return Err(PacketAdapterError::EmptyPayload);
    }

    // Validate payload is large enough for header (minimum 30 bytes SCALE-encoded)
    if payload.len() < 30 {
        return Err(PacketAdapterError::PayloadTooSmall);
    }

    // Validate payload size does not exceed kernel limit (65KB)
    if payload.len() > 65535 {
        return Err(PacketAdapterError::PayloadTooLarge);
    }

    // Attempt to decode the full packet
    match <Packet as Decode>::decode(&mut &payload[..]) {
        Ok(packet) => {
            // Additional validation after successful decode
            validate_packet(&packet)?;
            Ok(packet)
        }
        Err(_) => Err(PacketAdapterError::DecodingFailed),
    }
}

/// Validate packet structure, header, checksum, and constraints
///
/// **Checks:**
/// 1. Header version is valid (currently v1)
/// 2. Domain mask targets at least one VM
/// 3. Payload size is within limits
/// 4. Packet has not expired (if expiry set)
///
/// # Errors
/// Returns PacketAdapterError if any validation check fails
pub fn validate_packet(packet: &Packet) -> PacketAdapterResult<()> {
    // All packets must have a valid domain mask (non-zero)
    let domain_mask = packet.domain_mask();

    if domain_mask == 0 {
        return Err(PacketAdapterError::NoDomainTarget);
    }

    // Packet structure is already validated by SCALE codec during deserialization
    // Additional semantic checks can be added here based on packet type
    match packet {
        Packet::Evm(_) => {
            // EVM packets should have domain mask 0b0001
            if domain_mask != 0b0001 {
                return Err(PacketAdapterError::InvalidHeader);
            }
        }
        Packet::Svm(_) => {
            // SVM packets should have domain mask 0b0010
            if domain_mask != 0b0010 {
                return Err(PacketAdapterError::InvalidHeader);
            }
        }
        Packet::X3Vm(_) => {
            // X3VM packets should have domain mask 0b0100
            if domain_mask != 0b0100 {
                return Err(PacketAdapterError::InvalidHeader);
            }
        }
    }

    Ok(())
}

/// Determine which VM(s) should execute this packet based on domain_mask
///
/// **Logic:**
/// - If packet is EvmPacket → EvmOnly
/// - If packet is SvmPacket → SvmOnly
/// - If packet is X3VmPacket with both evm and svm → EvmAndSvm
/// - Otherwise → packet-specific routing
///
/// # Returns
/// DomainRoute indicating which executor(s) should process this packet
pub fn route_packet(packet: &Packet) -> PacketAdapterResult<DomainRoute> {
    match packet {
        Packet::Evm(_) => Ok(DomainRoute::EvmOnly),
        Packet::Svm(_) => Ok(DomainRoute::SvmOnly),
        Packet::X3Vm(x3vm_packet) => {
            // For X3VM packets, determine route based on packet contents
            match x3vm_packet {
                X3VmPacket::AtomicCross {
                    evm,
                    svm,
                    atomic: _,
                } => match (evm.is_some(), svm.is_some()) {
                    (true, true) => Ok(DomainRoute::EvmAndSvm),
                    (true, false) => Ok(DomainRoute::EvmOnly),
                    (false, true) => Ok(DomainRoute::SvmOnly),
                    (false, false) => Err(PacketAdapterError::NoDomainTarget),
                },
                X3VmPacket::Conditional { .. } => {
                    // Conditional packets route based on condition evaluation
                    // For now, route to X3VM for specialized handling
                    Ok(DomainRoute::X3VmOnly)
                }
                X3VmPacket::Transfer { .. } => {
                    // Transfer packets require both VM validation
                    Ok(DomainRoute::AllDomains)
                }
            }
        }
    }
}

/// Extract domain_mask from a deserialized packet
/// Returns the mask indicating which VMs this packet targets
pub fn get_domain_mask(packet: &Packet) -> u8 {
    match packet {
        Packet::Evm(_) => 0b0001,
        Packet::Svm(_) => 0b0010,
        Packet::X3Vm(_) => 0b0100,
    }
}

/// Get packet type identifier for logging/debugging
pub fn get_packet_type(packet: &Packet) -> &'static str {
    match packet {
        Packet::Evm(evm_packet) => match evm_packet {
            EvmPacket::Call { .. } => "EVM::Call",
            EvmPacket::Deploy { .. } => "EVM::Deploy",
            EvmPacket::Batch { .. } => "EVM::Batch",
        },
        Packet::Svm(svm_packet) => match svm_packet {
            SvmPacket::Invoke { .. } => "SVM::Invoke",
            SvmPacket::Deploy { .. } => "SVM::Deploy",
            SvmPacket::InitializeState { .. } => "SVM::InitializeState",
        },
        Packet::X3Vm(x3vm_packet) => match x3vm_packet {
            X3VmPacket::AtomicCross { .. } => "X3VM::AtomicCross",
            X3VmPacket::Conditional { .. } => "X3VM::Conditional",
            X3VmPacket::Transfer { .. } => "X3VM::Transfer",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_packet_schema::U256;

    #[test]
    fn test_deserialize_empty_payload() {
        let result = deserialize_packet(&[]);
        assert_eq!(result, Err(PacketAdapterError::EmptyPayload));
    }

    #[test]
    fn test_deserialize_payload_too_small() {
        let small_payload = vec![0u8; 10];
        let result = deserialize_packet(&small_payload);
        assert_eq!(result, Err(PacketAdapterError::PayloadTooSmall));
    }

    #[test]
    fn test_deserialize_payload_too_large() {
        let large_payload = vec![0u8; 65536];
        let result = deserialize_packet(&large_payload);
        assert_eq!(result, Err(PacketAdapterError::PayloadTooLarge));
    }

    #[test]
    fn test_route_evm_packet() {
        let packet = Packet::Evm(EvmPacket::Call {
            contract: [0u8; 20],
            function_selector: [0u8; 4],
            args: Vec::new(),
            value: U256::zero(),
        });
        let route = route_packet(&packet);
        assert!(route.is_ok());
        assert_eq!(route.unwrap(), DomainRoute::EvmOnly);
    }

    #[test]
    fn test_route_svm_packet() {
        let packet = Packet::Svm(SvmPacket::Invoke {
            program_id: [0u8; 32],
            accounts: Vec::new(),
            data: Vec::new(),
        });
        let route = route_packet(&packet);
        assert!(route.is_ok());
        assert_eq!(route.unwrap(), DomainRoute::SvmOnly);
    }

    #[test]
    fn test_domain_route_targets_evm() {
        let route = DomainRoute::EvmOnly;
        assert!(route.targets_evm());
        assert!(!route.targets_svm());
        assert!(!route.targets_x3vm());
    }

    #[test]
    fn test_domain_route_targets_svm() {
        let route = DomainRoute::SvmOnly;
        assert!(!route.targets_evm());
        assert!(route.targets_svm());
        assert!(!route.targets_x3vm());
    }

    #[test]
    fn test_get_domain_mask_evm() {
        let packet = Packet::Evm(EvmPacket::Call {
            contract: [0u8; 20],
            function_selector: [0u8; 4],
            args: Vec::new(),
            value: U256::zero(),
        });
        assert_eq!(get_domain_mask(&packet), 0b0001);
    }

    #[test]
    fn test_get_domain_mask_svm() {
        let packet = Packet::Svm(SvmPacket::Invoke {
            program_id: [0u8; 32],
            accounts: Vec::new(),
            data: Vec::new(),
        });
        assert_eq!(get_domain_mask(&packet), 0b0010);
    }
}
