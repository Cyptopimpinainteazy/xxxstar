//! X3 Evolution Core - autonomous improvement loop for X3 Autoprove.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionSignal {
    pub source: String,
    pub severity: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvolutionDecision {
    ProposeTask(String),
    RequireApproval(String),
    GenerateReport(String),
    Defer(String),
}

pub fn propose_evolution_action(signal: EvolutionSignal) -> EvolutionDecision {
    if signal.severity >= 8 {
        EvolutionDecision::RequireApproval(signal.message)
    } else if signal.severity >= 5 {
        EvolutionDecision::ProposeTask(signal.message)
    } else {
        EvolutionDecision::Defer(signal.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn propose_high_severity_requires_approval() {
        let signal = EvolutionSignal {
            source: "test".into(),
            severity: 9,
            message: "runtime issue".into(),
        };
        matches!(
            propose_evolution_action(signal),
            EvolutionDecision::RequireApproval(_)
        );
    }
}
