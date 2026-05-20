//! Intent lifecycle management — the state machine for ArbIntents.
//!
//! Lifecycle: submit → bind_route → execute → finalize
//!
//! Each transition is validated and produces side effects
//! (bonding, fee locking, proof generation, slashing).

use crate::error::IntentError;
use crate::intent::ArbIntent;
use crate::types::*;
use sha2::{Digest, Sha256};
use x3_fees::types::FeeVector;
use x3_proof::types::{AgentIdentity, BlockHeight, Hash256, IntentId};
use x3_slash::types::BondId;

/// Intent lifecycle manager — enforces the state machine.
pub struct IntentLifecycle;

impl IntentLifecycle {
    /// Submit a new intent. Creates the intent in Submitted state.
    ///
    /// Pre-conditions:
    /// - Agent must have sufficient balance for bond
    /// - Program hash must be valid
    /// - Slashable flag must be true for production
    pub fn submit_intent(
        id: IntentId,
        agent_id: AgentIdentity,
        program_hash: Hash256,
        flags: IntentFlags,
        bond_amount: u128,
        fee_cap: u128,
        current_block: BlockHeight,
        finality_window: u64,
    ) -> Result<ArbIntent, IntentError> {
        if bond_amount == 0 && flags.slashable {
            return Err(IntentError::ZeroBond);
        }

        if fee_cap == 0 {
            return Err(IntentError::ZeroFeeCap);
        }

        Ok(ArbIntent::new(
            id,
            agent_id,
            program_hash,
            flags,
            bond_amount,
            fee_cap,
            current_block,
            finality_window,
        ))
    }

    /// Bind a route to the intent. Seals the execution path.
    ///
    /// After this, the route cannot be changed. This prevents front-running.
    ///
    /// Pre-conditions:
    /// - Intent must be in Submitted state
    /// - Route must have at least one leg
    /// - Fee must not exceed fee cap
    pub fn bind_route(
        intent: &mut ArbIntent,
        legs: Vec<RouteLeg>,
        fee: FeeVector,
        bond_id: BondId,
        current_block: BlockHeight,
    ) -> Result<(), IntentError> {
        if intent.state != IntentState::Submitted {
            return Err(IntentError::InvalidTransition {
                from: intent.state,
                to: IntentState::RouteBound,
            });
        }

        if intent.is_expired(current_block) {
            return Err(IntentError::Expired(intent.id));
        }

        if legs.is_empty() {
            return Err(IntentError::EmptyRoute);
        }

        if fee.total > intent.fee_cap {
            return Err(IntentError::FeeCapExceeded {
                fee: fee.total,
                cap: intent.fee_cap,
            });
        }

        // Compute total capital
        let total_capital = legs.iter().map(|l| l.amount_in).sum::<u128>();

        // Compute route hash
        let route_hash = Self::hash_route(&legs);

        intent.route = Some(SealedRoute {
            legs,
            route_hash,
            sealed_at: current_block,
            fee,
            total_capital,
        });
        intent.bond_id = Some(bond_id);
        intent.state = IntentState::RouteBound;

        Ok(())
    }

    /// Begin execution of a bound intent.
    ///
    /// Pre-conditions:
    /// - Intent must be in RouteBound state
    /// - Must not be expired
    pub fn begin_execute(
        intent: &mut ArbIntent,
        current_block: BlockHeight,
    ) -> Result<(), IntentError> {
        if intent.state != IntentState::RouteBound {
            return Err(IntentError::InvalidTransition {
                from: intent.state,
                to: IntentState::Executing,
            });
        }

        if intent.is_expired(current_block) {
            return Err(IntentError::Expired(intent.id));
        }

        intent.state = IntentState::Executing;
        Ok(())
    }

    /// Complete execution with a result.
    ///
    /// Pre-conditions:
    /// - Intent must be in Executing state
    pub fn complete_execute(
        intent: &mut ArbIntent,
        result: ExecutionResult,
    ) -> Result<(), IntentError> {
        if intent.state != IntentState::Executing {
            return Err(IntentError::InvalidTransition {
                from: intent.state,
                to: IntentState::Executed,
            });
        }

        intent.result = Some(result);
        intent.state = IntentState::Executed;
        Ok(())
    }

    /// Finalize a completed intent — settle or slash.
    ///
    /// Pre-conditions:
    /// - Intent must be in Executed state
    /// - If execution failed and slashable, triggers slashing
    pub fn finalize(
        intent: &mut ArbIntent,
        current_block: BlockHeight,
    ) -> Result<Settlement, IntentError> {
        if intent.state != IntentState::Executed {
            return Err(IntentError::InvalidTransition {
                from: intent.state,
                to: IntentState::Finalized,
            });
        }

        let result = intent
            .result
            .as_ref()
            .ok_or(IntentError::NoExecutionResult)?;

        if !result.success && intent.flags.slashable {
            intent.state = IntentState::Slashed;
            return Err(IntentError::ExecutionFailed {
                intent_id: intent.id,
                slashable: true,
            });
        }

        let fee_paid = intent.route.as_ref().map(|r| r.fee.total).unwrap_or(0);

        let bond_returned = if result.success {
            intent.bond_amount
        } else {
            0
        };

        let settlement = Settlement {
            intent_id: intent.id,
            agent_id: intent.agent_id.clone(),
            result: result.clone(),
            fee_paid,
            bond_returned,
            settled_at: current_block,
        };

        intent.state = IntentState::Finalized;

        Ok(settlement)
    }

    /// Cancel an intent before route binding.
    ///
    /// Pre-conditions:
    /// - Intent must be in Submitted state
    pub fn cancel(intent: &mut ArbIntent) -> Result<(), IntentError> {
        if intent.state != IntentState::Submitted {
            return Err(IntentError::InvalidTransition {
                from: intent.state,
                to: IntentState::Cancelled,
            });
        }

        intent.state = IntentState::Cancelled;
        Ok(())
    }

    /// Expire an intent that has exceeded its finality window.
    pub fn expire(intent: &mut ArbIntent, current_block: BlockHeight) -> Result<(), IntentError> {
        if intent.is_terminal() {
            return Err(IntentError::AlreadyTerminal(intent.id));
        }

        if !intent.is_expired(current_block) {
            return Err(IntentError::NotExpired(intent.id));
        }

        intent.state = IntentState::Expired;
        Ok(())
    }

    /// Compute the hash of a route for sealing.
    fn hash_route(legs: &[RouteLeg]) -> Hash256 {
        let mut hasher = Sha256::new();
        hasher.update(&(legs.len() as u64).to_le_bytes());
        for leg in legs {
            hasher.update(&leg.source_chain.to_le_bytes());
            hasher.update(&leg.dest_chain.to_le_bytes());
            hasher.update(&(leg.source_asset.len() as u64).to_le_bytes());
            hasher.update(&leg.source_asset);
            hasher.update(&(leg.dest_asset.len() as u64).to_le_bytes());
            hasher.update(&leg.dest_asset);
            hasher.update(&leg.amount_in.to_le_bytes());
            hasher.update(&leg.min_amount_out.to_le_bytes());
            hasher.update(&(leg.venue.len() as u64).to_le_bytes());
            hasher.update(&leg.venue);
            hasher.update(&leg.state_touches.to_le_bytes());
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_agent() -> AgentIdentity {
        AgentIdentity {
            pubkey: [1u8; 32],
            ephemeral: false,
        }
    }

    fn test_leg() -> RouteLeg {
        RouteLeg {
            source_chain: 0,
            dest_chain: 0,
            source_asset: vec![1],
            dest_asset: vec![2],
            amount_in: 1000,
            min_amount_out: 900,
            venue: vec![0xDE, 0xAD],
            state_touches: 3,
        }
    }

    fn test_fee() -> FeeVector {
        FeeVector {
            base_fee: 100,
            complexity_fee: 50,
            capital_fee: 30,
            reputation_discount: 10,
            total: 170,
        }
    }

    #[test]
    fn test_full_lifecycle() {
        // 1. Submit
        let mut intent = IntentLifecycle::submit_intent(
            IntentId(1),
            test_agent(),
            [0xAA; 32],
            IntentFlags {
                slashable: true,
                ..Default::default()
            },
            1_000_000,
            1_000_000,
            100,
            50,
        )
        .unwrap();
        assert_eq!(intent.state, IntentState::Submitted);

        // 2. Bind route
        IntentLifecycle::bind_route(&mut intent, vec![test_leg()], test_fee(), BondId(0), 101)
            .unwrap();
        assert_eq!(intent.state, IntentState::RouteBound);

        // 3. Execute
        IntentLifecycle::begin_execute(&mut intent, 102).unwrap();
        assert_eq!(intent.state, IntentState::Executing);

        IntentLifecycle::complete_execute(
            &mut intent,
            ExecutionResult {
                success: true,
                actual_outputs: vec![950],
                gas_consumed: 5000,
                pnl: 50,
                proof_chain_hash: [0xBB; 32],
                state_diffs_hash: [0xCC; 32],
            },
        )
        .unwrap();
        assert_eq!(intent.state, IntentState::Executed);

        // 4. Finalize
        let settlement = IntentLifecycle::finalize(&mut intent, 103).unwrap();
        assert_eq!(intent.state, IntentState::Finalized);
        assert_eq!(settlement.bond_returned, 1_000_000);
    }

    #[test]
    fn test_failed_execution_slashing() {
        let mut intent = IntentLifecycle::submit_intent(
            IntentId(2),
            test_agent(),
            [0xAA; 32],
            IntentFlags {
                slashable: true,
                ..Default::default()
            },
            1_000_000,
            1_000_000,
            100,
            50,
        )
        .unwrap();

        IntentLifecycle::bind_route(&mut intent, vec![test_leg()], test_fee(), BondId(0), 101)
            .unwrap();

        IntentLifecycle::begin_execute(&mut intent, 102).unwrap();

        IntentLifecycle::complete_execute(
            &mut intent,
            ExecutionResult {
                success: false,
                actual_outputs: vec![0],
                gas_consumed: 3000,
                pnl: -1000,
                proof_chain_hash: [0xBB; 32],
                state_diffs_hash: [0xCC; 32],
            },
        )
        .unwrap();

        let result = IntentLifecycle::finalize(&mut intent, 103);
        assert!(result.is_err());
        assert_eq!(intent.state, IntentState::Slashed);
    }

    #[test]
    fn test_fee_cap_enforcement() {
        let mut intent = IntentLifecycle::submit_intent(
            IntentId(3),
            test_agent(),
            [0xAA; 32],
            IntentFlags {
                slashable: true,
                ..Default::default()
            },
            1_000_000,
            100, // Very low fee cap
            100,
            50,
        )
        .unwrap();

        let result = IntentLifecycle::bind_route(
            &mut intent,
            vec![test_leg()],
            test_fee(), // fee.total = 170, exceeds cap of 100
            BondId(0),
            101,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cancellation() {
        let mut intent = IntentLifecycle::submit_intent(
            IntentId(4),
            test_agent(),
            [0xAA; 32],
            IntentFlags::default(),
            0,
            1000,
            100,
            50,
        )
        .unwrap();

        IntentLifecycle::cancel(&mut intent).unwrap();
        assert_eq!(intent.state, IntentState::Cancelled);
    }
}
