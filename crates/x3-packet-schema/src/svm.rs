use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::prelude::{string::String, vec::Vec};
use scale_info::TypeInfo;

/// SVM account metadata
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct SvmAccount {
    pub pubkey: [u8; 32],
    pub is_writable: bool,
    pub is_signer: bool,
    pub is_executable: bool,
    pub lamports: u64,
    pub owner: [u8; 32],
}

/// SVM deployment metadata
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct SvmDeployMetadata {
    pub name: String,
    pub version: String,
    pub upgrade_authority: Option<[u8; 32]>,
}

/// SVM packet variants
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum SvmPacket {
    /// Invoke program
    Invoke {
        program_id: [u8; 32],
        accounts: Vec<SvmAccount>,
        data: Vec<u8>,
    },

    /// Deploy program
    Deploy {
        bytecode: Vec<u8>,
        metadata: SvmDeployMetadata,
    },

    /// Initialize state account
    InitializeState { account: [u8; 32], state: Vec<u8> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svm_invoke_packet_round_trip() {
        let packet = SvmPacket::Invoke {
            program_id: [0x11; 32],
            accounts: vec![],
            data: vec![1, 2, 3],
        };

        let encoded = packet.encode();
        let decoded: SvmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_svm_deploy_packet_round_trip() {
        let packet = SvmPacket::Deploy {
            bytecode: vec![0xBF; 100],
            metadata: SvmDeployMetadata {
                name: "test_program".to_string(),
                version: "1.0.0".to_string(),
                upgrade_authority: Some([0xAA; 32]),
            },
        };

        let encoded = packet.encode();
        let decoded: SvmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_svm_initialize_state_packet_round_trip() {
        let packet = SvmPacket::InitializeState {
            account: [0x22; 32],
            state: vec![42, 43, 44],
        };

        let encoded = packet.encode();
        let decoded: SvmPacket = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_svm_account_round_trip() {
        let account = SvmAccount {
            pubkey: [0x33; 32],
            is_writable: true,
            is_signer: false,
            is_executable: true,
            lamports: 1000000,
            owner: [0x44; 32],
        };

        let encoded = account.encode();
        let decoded: SvmAccount = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(account, decoded);
    }

    #[test]
    fn test_svm_deploy_metadata_round_trip() {
        let metadata = SvmDeployMetadata {
            name: "my_program".to_string(),
            version: "2.1.3".to_string(),
            upgrade_authority: None,
        };

        let encoded = metadata.encode();
        let decoded: SvmDeployMetadata = Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(metadata, decoded);
    }
}
