use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

#[derive(Clone, Eq, PartialEq, Default, RuntimeDebug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct TreasuryProposal {
    pub proposer: sp_core::H256,
    pub value: u128,
    pub beneficiary: sp_core::H256,
    pub bond: u128,
}
