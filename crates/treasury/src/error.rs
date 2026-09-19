use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

#[derive(Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum TreasuryError {
    InsufficientFunds,
    InvalidProposal,
    Unauthorized,
}
