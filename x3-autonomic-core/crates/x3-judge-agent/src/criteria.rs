// Judgement criteria for upgrade decisions

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub struct JudgementCriteria {
    pub min_invariant_coverage: f64,
    pub min_shadow_replay_blocks: u32,
    pub min_benchmark_score: u8,
    pub min_regression_tests: u32,
    pub canary_duration_blocks: u32,
    pub governance_vote_threshold: f64,
    pub multisig_required: bool,
}

impl Default for JudgementCriteria {
    fn default() -> Self {
        Self {
            min_invariant_coverage: 1.0,
            min_shadow_replay_blocks: 1000,
            min_benchmark_score: 80,
            min_regression_tests: 10,
            canary_duration_blocks: 100,
            governance_vote_threshold: 0.67,
            multisig_required: true,
        }
    }
}

impl JudgementCriteria {
    pub fn mainnet() -> Self {
        Self {
            min_invariant_coverage: 1.0,
            min_shadow_replay_blocks: 10000,
            min_benchmark_score: 90,
            min_regression_tests: 50,
            canary_duration_blocks: 500,
            governance_vote_threshold: 0.8,
            multisig_required: true,
        }
    }

    pub fn testnet() -> Self {
        Self {
            min_invariant_coverage: 0.95,
            min_shadow_replay_blocks: 100,
            min_benchmark_score: 70,
            min_regression_tests: 5,
            canary_duration_blocks: 20,
            governance_vote_threshold: 0.5,
            multisig_required: false,
        }
    }
}