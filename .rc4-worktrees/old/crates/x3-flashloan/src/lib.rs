//! # X3 Cross-Chain Flashloan Engine
//!
//! Flashloans are **transient execution capital** — never custodied, never bridged.
//! Capital is borrowed, used, and repaid within a single atomic execution context.
//!
//! ## Design Principles
//!
//! 1. **No bridged custody**: Capital never crosses a bridge in custody terms.
//!    Each chain provides its own flashloan liquidity; the *intent* unifies them.
//! 2. **Atomic settlement**: Failure at any leg reverts the entire transaction.
//!    No partial fills. No orphaned capital.
//! 3. **Proof-based settlement**: Every flashloan generates an `ExecutionProof`
//!    binding borrow → use → repay into a single verifiable chain.
//! 4. **Fee-integrated**: Flashloan premium is calculated via the X3 fee curve
//!    engine, not a flat rate. Reputable agents pay less.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    X3 FLASHLOAN ENGINE                       │
//! │                                                              │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────────────────┐  │
//! │  │ Liquidity  │  │  Flashloan │  │  Atomic Execution      │  │
//! │  │ Registry   │──│  Planner   │──│  Context               │  │
//! │  │ (per-chain)│  │            │  │  (borrow→use→repay)    │  │
//! │  └────────────┘  └────────────┘  └────────────────────────┘  │
//! │                         │                    │               │
//! │                         ▼                    ▼               │
//! │  ┌──────────────────────────────────────────────────────┐    │
//! │  │              Settlement Layer                         │    │
//! │  │  • Proof generation per borrow/repay                  │    │
//! │  │  • Fee calculation via x3-fees                        │    │
//! │  │  • Slash on failure if agent-initiated                │    │
//! │  └──────────────────────────────────────────────────────┘    │
//! └──────────────────────────────────────────────────────────────┘
//! ```

pub mod error;
pub mod executor;
pub mod planner;
pub mod pool;
pub mod settlement;
pub mod types;

pub use error::FlashloanError;
pub use executor::AtomicExecutor;
pub use planner::FlashloanPlanner;
pub use pool::FlashloanPool;
pub use settlement::SettlementEngine;
pub use types::*;

#[cfg(test)]
#[path = "tests/attack_oracle_manipulation.rs"]
mod attack_oracle_manipulation;

#[cfg(test)]
#[path = "tests/attack_reentrancy.rs"]
mod attack_reentrancy;

#[cfg(test)]
#[path = "tests/attack_repayment_bypass.rs"]
mod attack_repayment_bypass;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flashloan_lifecycle() {
        // Create pool
        let mut pool = FlashloanPool::new();
        pool.deposit(
            ChainKind::Evm(1),
            AssetId::new("USDC"),
            1_000_000_000_000, // 1M USDC (6 decimals)
        );

        // Create borrow request
        let borrow = BorrowRequest {
            id: FlashloanId::new(),
            chain: ChainKind::Evm(1),
            asset: AssetId::new("USDC"),
            amount: 100_000_000_000, // 100K USDC
            purpose: BorrowPurpose::ArbExecution,
        };

        // Verify pool has enough
        assert!(pool.available(borrow.chain, &borrow.asset) >= borrow.amount);

        // Borrow
        let receipt = pool.borrow(&borrow).unwrap();
        assert_eq!(receipt.principal, borrow.amount);
        assert!(receipt.premium > 0);

        // Pool liquidity should be reduced
        assert_eq!(
            pool.available(borrow.chain, &borrow.asset),
            1_000_000_000_000 - 100_000_000_000
        );

        // Repay (principal + premium)
        let repay_amount = receipt.principal + receipt.premium;
        let settled = pool.repay(&receipt.id, repay_amount).unwrap();
        assert!(settled);

        // Pool should be restored + premium
        assert!(pool.available(borrow.chain, &borrow.asset) > 1_000_000_000_000);
    }

    #[test]
    fn test_atomic_executor_revert() {
        let mut executor = AtomicExecutor::new();

        // Start atomic context
        let ctx_id = executor.begin();

        // Simulate leg 1 success
        executor.record_leg(
            ctx_id,
            LegOutcome::Success {
                chain: ChainKind::Evm(1),
                gas_used: 150_000,
                output_amount: 50_000_000_000,
            },
        );

        // Simulate leg 2 failure
        executor.record_leg(
            ctx_id,
            LegOutcome::Failure {
                chain: ChainKind::Evm(137),
                reason: "insufficient liquidity".to_string(),
            },
        );

        // Context should be reverted — no partial fill
        let result = executor.finalize(ctx_id);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FlashloanError::AtomicRevert { .. }
        ));
    }

    #[test]
    fn test_cross_chain_plan() {
        let planner = FlashloanPlanner::new();

        let plan = planner.plan(FlashloanPlan {
            intent_id: "intent-001".to_string(),
            borrows: vec![
                BorrowRequest {
                    id: FlashloanId::new(),
                    chain: ChainKind::Evm(1),
                    asset: AssetId::new("WETH"),
                    amount: 10_000_000_000_000_000_000, // 10 ETH
                    purpose: BorrowPurpose::ArbExecution,
                },
                BorrowRequest {
                    id: FlashloanId::new(),
                    chain: ChainKind::Svm,
                    asset: AssetId::new("SOL"),
                    amount: 500_000_000_000, // 500 SOL (9 decimals)
                    purpose: BorrowPurpose::ArbExecution,
                },
            ],
            legs: vec![
                ExecutionLeg {
                    chain: ChainKind::Evm(1),
                    action: LegAction::Swap {
                        from: AssetId::new("WETH"),
                        to: AssetId::new("USDC"),
                        amount: 10_000_000_000_000_000_000,
                    },
                    gas_limit: 300_000,
                },
                ExecutionLeg {
                    chain: ChainKind::Svm,
                    action: LegAction::Swap {
                        from: AssetId::new("SOL"),
                        to: AssetId::new("USDC"),
                        amount: 500_000_000_000,
                    },
                    gas_limit: 200_000,
                },
            ],
            deadline_ms: 30_000,
        });

        assert!(plan.is_ok());
        let validated = plan.unwrap();
        assert_eq!(validated.borrows.len(), 2);
        assert_eq!(validated.legs.len(), 2);
        assert!(validated.total_premium > 0);
    }

    #[test]
    fn test_settlement_proof_generation() {
        use x3_proof::{AgentIdentity, ProofEngine, ProofEngineConfig};

        let engine = SettlementEngine::new();
        let proof_engine = ProofEngine::new(
            ProofEngineConfig::default(),
            100,
            AgentIdentity {
                pubkey: [0xBB; 32],
                ephemeral: true,
            },
            [0xCC; 32],
        );

        let receipt = BorrowReceipt {
            id: FlashloanId::new(),
            chain: ChainKind::Evm(1),
            asset: AssetId::new("USDC"),
            principal: 100_000_000_000,
            premium: 50_000_000, // 0.05% premium
            borrowed_at: 1700000000000,
            must_repay_by: 1700000030000,
        };

        let settlement = engine.settle(
            &receipt,
            100_050_000_000, // principal + premium
            &proof_engine,
        );

        assert!(settlement.is_ok());
        let record = settlement.unwrap();
        assert!(record.proof_hash.len() == 64); // hex-encoded SHA-256
        assert_eq!(record.status, SettlementStatus::Settled);
    }
}
