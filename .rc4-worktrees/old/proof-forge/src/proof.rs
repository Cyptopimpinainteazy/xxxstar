#![allow(dead_code)] // intentional proof model scaffold; tracked in readiness backlog

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Proof levels from P0 to P7
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProofLevel {
    P0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
}

/// Edge case proof levels E0 to E10
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EdgeCaseLevel {
    E0,
    E1,
    E2,
    E3,
    E4,
    E5,
    E6,
    E7,
    E8,
    E9,
    E10,
}

/// Hack resistance levels H0 to H10
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HackLevel {
    H0,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    H7,
    H8,
    H9,
    H10,
}

/// Operator safety levels I0 to I10
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OperatorLevel {
    I0,
    I1,
    I2,
    I3,
    I4,
    I5,
    I6,
    I7,
    I8,
    I9,
    I10,
}

/// Degraded operation levels D0 to D10
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DegradedLevel {
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    D10,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    #[serde(rename = "verified")]
    Verified,
    #[serde(rename = "partial")]
    Partial,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "unverified")]
    Unverified,
    #[serde(rename = "blocked")]
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResult {
    pub claim_id: String,
    pub claim: String,
    pub status: ProofStatus,
    pub proof_level: Option<ProofLevel>,
    pub edge_case_level: Option<EdgeCaseLevel>,
    pub hack_level: Option<HackLevel>,
    pub operator_level: Option<OperatorLevel>,
    pub degraded_level: Option<DegradedLevel>,
    pub files_inspected: Vec<String>,
    pub commands_run: Vec<String>,
    pub passed_checks: Vec<String>,
    pub failed_checks: Vec<String>,
    pub missing_proofs: Vec<String>,
    pub blockers: Vec<String>,
    pub score: f64,
    pub evidence: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofReceipt {
    pub receipt_id: String,
    pub timestamp: DateTime<Utc>,
    pub receipt_type: String,
    pub areas: Vec<String>,
    pub results: Vec<ProofResult>,
    pub overall_status: ProofStatus,
    pub overall_score: f64,
    pub signatures: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRegistry {
    pub claims: Vec<Claim>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub area: String,
    pub criticality: String,
    pub required_proof_level: ProofLevel,
    pub required_edge_level: Option<EdgeCaseLevel>,
    pub required_hack_level: Option<HackLevel>,
    pub required_operator_level: Option<OperatorLevel>,
    pub edge_cases: Vec<EdgeCase>,
    pub attack_vectors: Vec<AttackVector>,
    pub safe_degradation_modes: Vec<DegradationMode>,
    pub operator_controls: Vec<OperatorControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCase {
    pub name: String,
    pub description: String,
    pub expected_result: String,
    pub test_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackVector {
    pub name: String,
    pub threat_level: String,
    pub attack_scenario: String,
    pub defense_mechanism: String,
    pub test_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationMode {
    pub mode_name: String,
    pub trigger_condition: String,
    pub safe_behavior: String,
    pub recovery_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorControl {
    pub control_name: String,
    pub prevents: String,
    pub safe_default: String,
    pub confirmation_required: bool,
}

impl ProofStatus {
    #[allow(dead_code)]
    pub fn is_passing(&self) -> bool {
        matches!(self, ProofStatus::Verified)
    }

    pub fn is_blocking(&self) -> bool {
        matches!(self, ProofStatus::Blocked | ProofStatus::Failed)
    }
}

impl ProofLevel {
    #[allow(dead_code)]
    pub fn as_number(&self) -> u32 {
        match self {
            ProofLevel::P0 => 0,
            ProofLevel::P1 => 1,
            ProofLevel::P2 => 2,
            ProofLevel::P3 => 3,
            ProofLevel::P4 => 4,
            ProofLevel::P5 => 5,
            ProofLevel::P6 => 6,
            ProofLevel::P7 => 7,
        }
    }
}

impl EdgeCaseLevel {
    #[allow(dead_code)]
    pub fn as_number(&self) -> u32 {
        match self {
            EdgeCaseLevel::E0 => 0,
            EdgeCaseLevel::E1 => 1,
            EdgeCaseLevel::E2 => 2,
            EdgeCaseLevel::E3 => 3,
            EdgeCaseLevel::E4 => 4,
            EdgeCaseLevel::E5 => 5,
            EdgeCaseLevel::E6 => 6,
            EdgeCaseLevel::E7 => 7,
            EdgeCaseLevel::E8 => 8,
            EdgeCaseLevel::E9 => 9,
            EdgeCaseLevel::E10 => 10,
        }
    }
}

impl HackLevel {
    #[allow(dead_code)]
    pub fn as_number(&self) -> u32 {
        match self {
            HackLevel::H0 => 0,
            HackLevel::H1 => 1,
            HackLevel::H2 => 2,
            HackLevel::H3 => 3,
            HackLevel::H4 => 4,
            HackLevel::H5 => 5,
            HackLevel::H6 => 6,
            HackLevel::H7 => 7,
            HackLevel::H8 => 8,
            HackLevel::H9 => 9,
            HackLevel::H10 => 10,
        }
    }
}

impl OperatorLevel {
    #[allow(dead_code)]
    pub fn as_number(&self) -> u32 {
        match self {
            OperatorLevel::I0 => 0,
            OperatorLevel::I1 => 1,
            OperatorLevel::I2 => 2,
            OperatorLevel::I3 => 3,
            OperatorLevel::I4 => 4,
            OperatorLevel::I5 => 5,
            OperatorLevel::I6 => 6,
            OperatorLevel::I7 => 7,
            OperatorLevel::I8 => 8,
            OperatorLevel::I9 => 9,
            OperatorLevel::I10 => 10,
        }
    }
}

impl DegradedLevel {
    #[allow(dead_code)]
    pub fn as_number(&self) -> u32 {
        match self {
            DegradedLevel::D0 => 0,
            DegradedLevel::D1 => 1,
            DegradedLevel::D2 => 2,
            DegradedLevel::D3 => 3,
            DegradedLevel::D4 => 4,
            DegradedLevel::D5 => 5,
            DegradedLevel::D6 => 6,
            DegradedLevel::D7 => 7,
            DegradedLevel::D8 => 8,
            DegradedLevel::D9 => 9,
            DegradedLevel::D10 => 10,
        }
    }
}
