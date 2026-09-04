use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::prelude::{boxed::Box, vec::Vec};
use scale_info::TypeInfo;

use crate::EvmPacket;
use crate::SvmPacket;

/// X3VM Condition for conditional execution
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum X3Condition {
    /// Check account balance threshold
    BalanceAbove {
        /// Account address (VM-specific encoding)
        account: Vec<u8>,
        /// Minimum balance required
        threshold: u128,
    },

    /// Check contract state value
    StateEquals {
        /// Contract address
        contract: Vec<u8>,
        /// State key
        key: Vec<u8>,
        /// Expected value
        expected: Vec<u8>,
    },

    /// Check block height
    BlockHeightAbove {
        /// Minimum block height
        min_height: u32,
    },

    /// Logical AND of conditions
    And(Vec<X3Condition>),

    /// Logical OR of conditions
    Or(Vec<X3Condition>),
}

/// X3VM packet variants
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum X3VmPacket {
    /// Atomic cross-VM transaction
    AtomicCross {
        /// EVM portion (optional)
        evm: Option<Box<EvmPacket>>,

        /// SVM portion (optional)
        svm: Option<Box<SvmPacket>>,

        /// Rollback on failure flag
        atomic: bool,
    },

    /// Conditional execution
    Conditional {
        /// Condition to evaluate
        condition: X3Condition,

        /// Execute if condition true
        if_true: Box<X3VmPacket>,

        /// Execute if condition false (optional)
        if_false: Option<Box<X3VmPacket>>,
    },

    /// Value transfer across domains
    Transfer {
        /// Source domain (EVM=0, SVM=1, X3VM=2)
        from_domain: u8,

        /// Destination domain
        to_domain: u8,

        /// Asset ID (0 = native, >0 = token)
        asset_id: u32,

        /// Amount in base units
        amount: u128,

        /// Recipient address (encoded for destination domain)
        recipient: Vec<u8>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvmPacket, SvmPacket, U256};

    #[test]
    fn test_x3vm_transfer_packet_round_trip() {
        let packet = X3VmPacket::Transfer {
            from_domain: 0,
            to_domain: 1,
            asset_id: 0,
            amount: 1000000,
            recipient: vec![0xAA; 32],
        };

        let encoded = packet.encode();
        let decoded: X3VmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_x3vm_atomic_cross_packet_round_trip() {
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

        let packet = X3VmPacket::AtomicCross {
            evm: Some(Box::new(evm)),
            svm: Some(Box::new(svm)),
            atomic: true,
        };

        let encoded = packet.encode();
        let decoded: X3VmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_x3vm_conditional_packet_round_trip() {
        let packet = X3VmPacket::Conditional {
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

        let encoded = packet.encode();
        let decoded: X3VmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_x3vm_condition_balance_above_round_trip() {
        let condition = X3Condition::BalanceAbove {
            account: vec![0xAA; 20],
            threshold: 1_000_000,
        };
        let encoded = condition.encode();
        let decoded: X3Condition = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(condition, decoded);
    }

    #[test]
    fn test_x3vm_condition_state_equals_round_trip() {
        let condition = X3Condition::StateEquals {
            contract: vec![0xBB; 20],
            key: b"balance".to_vec(),
            expected: b"1000000".to_vec(),
        };
        let encoded = condition.encode();
        let decoded: X3Condition = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(condition, decoded);
    }

    #[test]
    fn test_x3vm_condition_and_round_trip() {
        let condition = X3Condition::And(vec![
            X3Condition::BlockHeightAbove { min_height: 100 },
            X3Condition::BalanceAbove {
                account: vec![0xAA; 20],
                threshold: 1000,
            },
        ]);
        let encoded = condition.encode();
        let decoded: X3Condition = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(condition, decoded);
    }

    #[test]
    fn test_x3vm_condition_or_round_trip() {
        let condition = X3Condition::Or(vec![
            X3Condition::BlockHeightAbove { min_height: 100 },
            X3Condition::BlockHeightAbove { min_height: 200 },
        ]);
        let encoded = condition.encode();
        let decoded: X3Condition = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(condition, decoded);
    }
}
