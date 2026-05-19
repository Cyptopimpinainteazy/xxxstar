use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// Policy violation types
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum ViolationType {
    CapabilityNotPermitted,
    ReputationBelowMinimum,
    MaxTasksPerBlockExceeded,
    CollusionAttempted,
    RateLimitExceeded,
}

/// Enforcement actions
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum EnforcementAction<AccountId> {
    LogOnly,
    Slash(u64),
    RevokeCapability,
    Blacklist(u32), // duration in blocks
    Jail(AccountId),
}

/// Policy rules governing agent behavior
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
pub enum PolicyRule<AccountId: Encode + Decode + MaxEncodedLen + DecodeWithMemTracking> {
    /// Agent can only execute these capabilities (list of capability names as byte strings)
    CapabilityAllowed(#[codec(skip)] Vec<Vec<u8>>),
    /// Agent must maintain minimum reputation score
    ReputationMinimum(u64),
    /// Hard cap on tasks scheduled per block
    MaxTasksPerBlock(u32),
    /// Agent cannot coordinate with these accounts
    NoCollusionWith(#[codec(skip)] Vec<AccountId>),
    /// Rate limit: max extrinsics per epoch
    RateLimit(u32),
}

/// Slashing reasons
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum SlashingReason {
    InvalidProof,
    TaskGriefing,
    CollusionDetected,
    PolicyViolation,
    RepeatOffender,
}

/// Capability revocation reasons
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum RevocationReason {
    ReputationDropped,
    PolicyViolation,
    ManualRevocation,
}

/// Policy evaluation result
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyResult {
    Pass,
    Fail(ViolationType),
}

impl PolicyResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, PolicyResult::Pass)
    }

    pub fn is_fail(&self) -> bool {
        !self.is_pass()
    }
}
