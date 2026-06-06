use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::prelude::vec::Vec;
use scale_info::TypeInfo;

/// U256 type for Ethereum
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
#[repr(transparent)]
pub struct U256(pub [u8; 32]);

impl U256 {
    pub fn from(value: u64) -> Self {
        let mut bytes = [0u8; 32];
        bytes[24..32].copy_from_slice(&value.to_be_bytes());
        U256(bytes)
    }

    pub fn zero() -> Self {
        U256([0u8; 32])
    }
}

impl Default for U256 {
    fn default() -> Self {
        Self::zero()
    }
}

/// EVM contract call target
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EvmCall {
    /// Contract address (20 bytes)
    pub contract: [u8; 20],

    /// Function selector (4 bytes)
    pub function_selector: [u8; 4],

    /// ABI-encoded arguments
    pub args: Vec<u8>,
}

/// EVM packet variants
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum EvmPacket {
    /// Call smart contract
    Call {
        contract: [u8; 20],
        function_selector: [u8; 4],
        args: Vec<u8>,
        value: U256,
    },

    /// Deploy contract
    Deploy {
        bytecode: Vec<u8>,
        args: Vec<u8>,
        value: U256,
    },

    /// Batch multiple calls
    Batch {
        calls: Vec<(EvmCall, Option<U256>)>,
        continue_on_revert: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_from_u64() {
        let val = U256::from(1000);
        assert_eq!(val.0[31], 232); // 1000 in last byte of big-endian
    }

    #[test]
    fn test_evm_call_packet_round_trip() {
        let packet = EvmPacket::Call {
            contract: [0x42; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![1, 2, 3],
            value: U256::from(1000),
        };

        let encoded = packet.encode();
        let decoded: EvmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_evm_deploy_packet_round_trip() {
        let packet = EvmPacket::Deploy {
            bytecode: vec![0xBF; 100],
            args: vec![],
            value: U256::from(0),
        };

        let encoded = packet.encode();
        let decoded: EvmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_evm_batch_packet_round_trip() {
        let packet = EvmPacket::Batch {
            calls: vec![(
                EvmCall {
                    contract: [0x11; 20],
                    function_selector: [0x12, 0x34, 0x56, 0x78],
                    args: vec![],
                },
                None,
            )],
            continue_on_revert: true,
        };

        let encoded = packet.encode();
        let decoded: EvmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }
}
