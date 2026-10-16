/// Integration bridge with x3_operator package patterns.
///
/// This module mirrors the operator lifecycle defined in Python's
/// `x3_operator` package, providing Rust-native equivalents that the
/// Tauri backend can use for operator registration, health checks,
/// and configuration management.
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Operator role — mirrors x3_operator.config.OperatorRole
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OperatorRole {
    Validator,
    Gpu,
    Storage,
    Relayer,
}

// ---------------------------------------------------------------------------
// Network phase — mirrors x3_operator.config.NetworkPhase
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkPhase {
    Devnet,
    Testnet,
    Mainnet,
}

// ---------------------------------------------------------------------------
// Bond status — mirrors x3_operator.bonding.BondStatus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BondStatus {
    Unbonded,
    Pending,
    Bonded,
    Unbonding,
    Slashed,
}

// ---------------------------------------------------------------------------
// Operator config — mirrors x3_operator.config.X3Config subset
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X3OperatorConfig {
    pub rpc_url: String,
    pub chain_id: String,
    pub network_phase: NetworkPhase,
    pub operator_role: OperatorRole,
    pub min_bond: u64,
    pub heartbeat_interval_seconds: u32,
    pub metrics_port: u16,
    pub data_dir: String,
}

impl Default for X3OperatorConfig {
    fn default() -> Self {
        Self {
            rpc_url: "ws://127.0.0.1:9944".into(),
            chain_id: "atlas-sphere-devnet".into(),
            network_phase: NetworkPhase::Devnet,
            operator_role: OperatorRole::Validator,
            min_bond: 10_000,
            heartbeat_interval_seconds: 30,
            metrics_port: 9615,
            data_dir: "~/.x3_operator".into(),
        }
    }
}

impl X3OperatorConfig {
    /// Minimum bond for a given role (mirrors x3_operator.config.min_bond_for_role)
    pub fn min_bond_for_role(role: &OperatorRole) -> u64 {
        match role {
            OperatorRole::Validator => 10_000,
            OperatorRole::Gpu => 1_000,
            OperatorRole::Storage => 2_000,
            OperatorRole::Relayer => 5_000,
        }
    }
}

// ---------------------------------------------------------------------------
// Fault type — mirrors x3_operator.slashing.FaultType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FaultType {
    Downtime,
    Equivocation,
    InvalidProof,
    MissedHeartbeat,
    SlaViolation,
    DataCorruption,
    GovernanceAbuse,
    AgentViolation,
}

// ---------------------------------------------------------------------------
// Severity table — mirrors x3_operator.config.SlashingConfig.severity_table
// ---------------------------------------------------------------------------

/// Returns the base severity fraction for a fault type.
pub fn fault_severity(fault: &FaultType) -> f64 {
    match fault {
        FaultType::Downtime => 0.01,
        FaultType::Equivocation => 0.5,
        FaultType::InvalidProof => 0.1,
        FaultType::MissedHeartbeat => 0.005,
        FaultType::SlaViolation => 0.05,
        FaultType::DataCorruption => 1.0,
        FaultType::GovernanceAbuse => 0.25,
        FaultType::AgentViolation => 0.1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_bond_values() {
        assert_eq!(X3OperatorConfig::min_bond_for_role(&OperatorRole::Validator), 10_000);
        assert_eq!(X3OperatorConfig::min_bond_for_role(&OperatorRole::Gpu), 1_000);
        assert_eq!(X3OperatorConfig::min_bond_for_role(&OperatorRole::Storage), 2_000);
        assert_eq!(X3OperatorConfig::min_bond_for_role(&OperatorRole::Relayer), 5_000);
    }

    #[test]
    fn severity_range() {
        let faults = [
            FaultType::Downtime, FaultType::Equivocation, FaultType::InvalidProof,
            FaultType::MissedHeartbeat, FaultType::SlaViolation, FaultType::DataCorruption,
            FaultType::GovernanceAbuse, FaultType::AgentViolation,
        ];
        for f in &faults {
            let s = fault_severity(f);
            assert!(s > 0.0 && s <= 1.0, "severity out of range for {:?}: {}", f, s);
        }
    }
}
