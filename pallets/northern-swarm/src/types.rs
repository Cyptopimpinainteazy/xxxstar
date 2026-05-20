use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::ConstU32, BoundedVec};
use scale_info::TypeInfo;

/// Hardware profile advertised by an executor on registration.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct HardwareProfile {
    pub cpu_cores: u32,
    /// GPU VRAM in MiB; 0 if no GPU.
    pub gpu_vram_mib: u32,
    /// Available RAM in MiB.
    pub ram_mib: u64,
    /// Advertised bandwidth in Mbps.
    pub bandwidth_mbps: u32,
}

/// Executor registration record stored in the Executors map.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutorRecord<Balance, BlockNumber> {
    pub stake: Balance,
    pub hardware: HardwareProfile,
    pub reputation: u64,
    pub status: ExecutorStatus,
    pub last_heartbeat: BlockNumber,
    pub deregistering_at: Option<BlockNumber>,
}

/// Executor lifecycle status.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum ExecutorStatus {
    Active,
    Deregistering,
    Suspended,
}

/// Task record stored in the Tasks map.
/// Hash is the runtime T::Hash type (typically sp_core::H256).
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TaskRecord<AccountId, Balance, BlockNumber, Hash> {
    pub submitter: AccountId,
    pub payload_uri: BoundedVec<u8, ConstU32<512>>,
    pub reward: Balance,
    pub status: TaskStatus,
    pub claimed_by: Option<AccountId>,
    pub submitted_at: BlockNumber,
    pub result_hash: Option<Hash>,
}

/// Task lifecycle state.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum TaskStatus {
    Pending,
    Claimed,
    ResultCommitted,
    Finalised,
    Disputed,
    Expired,
}

/// Reason for a slash event.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum SlashReason {
    QuorumMismatch,
    LivenessFault,
    FraudProof,
    Governance,
}
