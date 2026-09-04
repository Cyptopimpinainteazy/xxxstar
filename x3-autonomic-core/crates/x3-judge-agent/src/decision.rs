// Decision types for judge agent

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub enum UpgradeDecision {
    Approved,
    ConditionalApproval,
    Rejected,
    Blocked,
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub struct JudgeReport {
    pub invariant_pass: bool,
    pub shadow_replay_pass: bool,
    pub benchmark_pass: bool,
    pub regression_tests_pass: bool,
    pub canary_pass: bool,
    pub governance_approval: bool,
    pub multisig_threshold: u32,
    pub multisig_received: u32,
    pub timestamp: u64,
}

impl JudgeReport {
    pub fn new() -> Self {
        Self {
            invariant_pass: false,
            shadow_replay_pass: false,
            benchmark_pass: false,
            regression_tests_pass: false,
            canary_pass: false,
            governance_approval: false,
            multisig_threshold: 0,
            multisig_received: 0,
            timestamp: 0,
        }
    }
}

impl Default for JudgeReport {
    fn default() -> Self {
        Self::new()
    }
}