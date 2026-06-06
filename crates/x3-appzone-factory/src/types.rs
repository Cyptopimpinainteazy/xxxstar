//! Types for X3 AppZone Factory

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct AppZoneConfig {
    pub name: Vec<u8>,
    pub pallets: Vec<PalletConfig>,
    pub features: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct PalletConfig {
    pub name: Vec<u8>,
    pub path: Vec<u8>,
    pub config: Vec<u8>,
}

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct DeploymentConfig {
    pub network: Vec<u8>,
    pub endpoints: Vec<Vec<u8>>,
    pub gas_limit: u64,
    pub confirmations: u32,
}

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct DeploymentReceipt {
    pub app_name: Vec<u8>,
    pub network: Vec<u8>,
    pub tx_hash: [u8; 32],
    pub block_number: u64,
    pub timestamp: u64,
}
