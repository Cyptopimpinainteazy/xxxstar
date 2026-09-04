//! Gas/fees/slippage simulator for IXL planner output.
//!
//! This module provides a simulator that consumes an ExecutionPlan
//! produced by the x3-ixl planner and estimates gas consumption,
//! fee costs, and slippage for cross-chain operations.

use crate::ixl::ExecutionPlan;
use crate::ixl::Instruction;
use crate::ixl::AssetKind;
use crate::ixl::AssetId;

#[derive(Debug, Clone, PartialEq)]
pub enum SimulatorError {
    NotConfigured,
    MissingGasSchedule,
    UnsupportedInstruction(&'static str),
}

impl core::fmt::Display for SimulatorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SimulatorError::NotConfigured => write!(f, "simulator not configured"),
            SimulatorError::MissingGasSchedule => write!(f, "no gas schedule loaded"),
            SimulatorError::UnsupportedInstruction(name) => {
                write!(f, "unsupported instruction: {name}")
            }
        }
    }
}

/// Result of a simulation run.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationResult {
    /// Estimated total gas units.
    pub gas: u64,
    /// Estimated fee in the native token (e.g., wei, lamports).
    pub fee: u128,
    /// Estimated slippage as a percentage (0.0 – 100.0).
    pub slippage_percent: f64,
    /// Number of instructions processed.
    pub instructions_processed: usize,
}

impl Default for SimulationResult {
    fn default() -> Self {
        SimulationResult {
            gas: 0,
            fee: 0,
            slippage_percent: 0.0,
            instructions_processed: 0,
        }
    }
}

/// Simple simulator that walks an execution plan and produces a
/// `SimulationResult`. Provides deterministic placeholder values
/// that can be expanded with real estimation logic.
pub struct Simulator;

impl Simulator {
    /// Create a new simulator instance.
    pub fn new() -> Self {
        Simulator
    }

    /// Run the simulation on a given execution plan.
    pub fn simulate(&self, plan: &ExecutionPlan) -> Result<SimulationResult, SimulatorError> {
        let mut gas = 0u64;
        let mut fee = 0u128;
        let mut slippage = 0.0f64;
        let mut instructions_processed = 0;

        for instruction in &plan.instructions {
            instructions_processed += 1;

            match instruction {
                Instruction::Lock { amount, .. } => {
                    gas += 21_000 + (amount / 1_000_000) as u64;
                    fee += 1_000_000_000;
                }
                Instruction::Mint { amount: _, .. } => {
                    gas += 15_000;
                    fee += 800_000_000;
                }
                Instruction::Swap { amount_in, min_out, asset_in: _, asset_out: _, .. } => {
                    let swap_gas = 50_000;
                    gas += swap_gas * 2 + (amount_in / 1_000_000) as u64;
                    fee += 2_000_000_000;
                    if *min_out < *amount_in {
                        slippage += (amount_in - *min_out) as f64 / *amount_in as f64 * 100.0;
                    }
                }
                Instruction::Settle { .. } => {
                    gas += 30_000;
                    fee += 1_500_000_000;
                }
                Instruction::Burn { .. } => {
                    gas += 10_000;
                    fee += 500_000_000;
                }
                Instruction::Refund { .. } => {
                    gas += 25_000;
                    fee += 1_200_000_000;
                }
                Instruction::EmitProof { .. } => {
                    gas += 40_000;
                    fee += 2_000_000_000;
                }
                Instruction::Abort => {
                    gas += 5_000;
                }
            }
        }

        Ok(SimulationResult {
            gas,
            fee,
            slippage_percent: slippage,
            instructions_processed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ixl::{ExecutionPlan, Instruction, AssetKind, AssetId};
    use sp_core::H256;

    // Helper to create test asset ID
    fn asset(b: u8) -> AssetId {
        let mut a = [0u8; 32];
        a[0] = b;
        a
    }

    // Helper to create test address
    fn addr(b: u8) -> [u8; 32] {
        let mut a = [0u8; 32];
        a[31] = b;
        a
    }

    #[test]
    fn basic_simulation_returns_defaults() {
        let simulator = Simulator::new();
        let empty_plan = ExecutionPlan {
            instructions: vec![],
        };
        let result = simulator.simulate(&empty_plan).unwrap();
        assert_eq!(result.gas, 0);
        assert_eq!(result.fee, 0);
        assert_eq!(result.slippage_percent, 0.0);
        assert_eq!(result.instructions_processed, 0);
    }

    #[test]
    fn lock_and_settle_simulation() {
        let simulator = Simulator::new();
        let plan = ExecutionPlan {
            instructions: vec![
                Instruction::Lock {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset: asset(1),
                    payer: addr(1),
                    amount: 100,
                },
                Instruction::Settle {
                    slot_id: 0,
                    kind: AssetKind::X3Evm,
                    receiver: addr(2),
                },
            ],
        };
        let result = simulator.simulate(&plan).unwrap();
        assert!(result.gas > 0);
        assert!(result.fee > 0);
        assert_eq!(result.instructions_processed, 2);
    }

    #[test]
    fn swap_with_slippage() {
        let simulator = Simulator::new();
        let plan = ExecutionPlan {
            instructions: vec![
                Instruction::Lock {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset: asset(1),
                    payer: addr(1),
                    amount: 1_000_000,
                },
                Instruction::Swap {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset_in: asset(1),
                    asset_out: asset(2),
                    amount_in: 1_000_000,
                    min_out: 990_000,
                },
                Instruction::Settle {
                    slot_id: 0,
                    kind: AssetKind::X3Evm,
                    receiver: addr(2),
                },
            ],
        };
        let result = simulator.simulate(&plan).unwrap();
        // Slippage should be (1,000,000 - 990,000) / 1,000,000 * 100 = 1%
        assert_eq!(result.slippage_percent, 1.0);
        assert_eq!(result.instructions_processed, 3);
    }
}
