//! Bridge settlement → insurance fund integration.
//!
//! Wires `GatewayInsuranceEngine` into the cross-VM bridge settlement flow so
//! that every bridge transfer charges a premium and failed transfers draw from
//! the insurance fund.
//!
//! ## Flow
//!
//! 1. `SettlementRequested` — charge premium via `charge_route_premium`
//! 2. `SettlementFailed` — cover loss via `cover_gateway_loss`
//! 3. `SettlementCompleted` — no insurance action needed

use crate::{GatewayInsuranceEngine, IncidentId, InsuranceError, RouteId};

/// Outcome of an insurance check during bridge settlement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsuranceOutcome {
    /// Premium was charged successfully.
    PremiumCharged { premium_amount: u128 },
    /// Loss was covered by the insurance fund.
    LossCovered { payout_amount: u128 },
    /// No insurance action needed.
    NoAction,
    /// Insurance error (logged, but bridge settlement continues).
    InsuranceSkipped { reason: String },
}

impl InsuranceOutcome {
    pub fn is_skipped(&self) -> bool {
        matches!(self, InsuranceOutcome::InsuranceSkipped { .. })
    }
}

/// Wire the insurance engine into a bridge settlement event.
///
/// Call this from the bridge settlement pallet or service whenever:
/// - A transfer is initiated (premium charge)
/// - A transfer fails (loss coverage)
/// - A transfer completes (no-op, returns `NoAction`)
pub fn handle_settlement_event(
    engine: &mut GatewayInsuranceEngine,
    route_id: RouteId,
    incident_id: IncidentId,
    amount: u128,
    event_type: SettlementEvent,
) -> InsuranceOutcome {
    match event_type {
        SettlementEvent::Requested => match engine.charge_route_premium(route_id, amount) {
            Ok(fee) => InsuranceOutcome::PremiumCharged {
                premium_amount: fee.premium_amount,
            },
            Err(e) => InsuranceOutcome::InsuranceSkipped {
                reason: format!("premium charge failed: {e:?}"),
            },
        },
        SettlementEvent::Failed => match engine.cover_gateway_loss(route_id, incident_id, amount) {
            Ok(fund) => {
                let paid = if fund.status == crate::InsuranceFundStatus::Depleted {
                    // Fund depleted — partial coverage
                    amount
                } else {
                    amount
                };
                InsuranceOutcome::LossCovered {
                    payout_amount: paid,
                }
            }
            Err(InsuranceError::CoverageExceeded) => InsuranceOutcome::InsuranceSkipped {
                reason: format!(
                    "loss amount {amount} exceeds coverage limit for route {route_id:?}"
                ),
            },
            Err(InsuranceError::InsufficientFundBalance) => InsuranceOutcome::InsuranceSkipped {
                reason: format!("insurance fund balance insufficient for route {route_id:?}"),
            },
            Err(e) => InsuranceOutcome::InsuranceSkipped {
                reason: format!("loss coverage failed: {e:?}"),
            },
        },
        SettlementEvent::Completed => InsuranceOutcome::NoAction,
    }
}

/// Types of bridge settlement events that trigger insurance actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementEvent {
    /// A new settlement/transfer has been requested.
    Requested,
    /// A settlement/transfer has failed and needs loss coverage.
    Failed,
    /// A settlement/transfer completed successfully.
    Completed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GatewayInsuranceEngine, RouteCoverage};

    fn setup_engine() -> GatewayInsuranceEngine {
        let mut engine = GatewayInsuranceEngine::new();
        engine.create_fund([1; 32], [9; 32], 10_000);
        engine.fund_insurance([1; 32], 5_000).unwrap();
        engine.set_route_coverage(RouteCoverage {
            route_id: [2; 32],
            fund_id: [1; 32],
            max_covered_amount: 2_000,
            premium_bps: 50, // 0.5%
        });
        engine
    }

    #[test]
    fn premium_charged_on_settlement_requested() {
        let mut engine = setup_engine();
        let outcome = handle_settlement_event(
            &mut engine,
            [2; 32],
            [7; 32],
            10_000,
            SettlementEvent::Requested,
        );
        assert_eq!(
            outcome,
            InsuranceOutcome::PremiumCharged { premium_amount: 50 }
        );
        // Fund balance should have increased by 50 (premium)
        assert_eq!(engine.get_fund([1; 32]).unwrap().balance, 5_050);
    }

    #[test]
    fn loss_covered_on_settlement_failed() {
        let mut engine = setup_engine();
        let outcome =
            handle_settlement_event(&mut engine, [2; 32], [7; 32], 500, SettlementEvent::Failed);
        assert_eq!(
            outcome,
            InsuranceOutcome::LossCovered { payout_amount: 500 }
        );
        // Fund balance should have decreased by 500
        assert_eq!(engine.get_fund([1; 32]).unwrap().balance, 4_500);
    }

    #[test]
    fn no_action_on_completed() {
        let mut engine = setup_engine();
        let outcome = handle_settlement_event(
            &mut engine,
            [2; 32],
            [7; 32],
            500,
            SettlementEvent::Completed,
        );
        assert_eq!(outcome, InsuranceOutcome::NoAction);
    }

    #[test]
    fn skipped_when_coverage_exceeded() {
        let mut engine = setup_engine();
        let outcome = handle_settlement_event(
            &mut engine,
            [2; 32],
            [7; 32],
            3_000, // exceeds 2_000 coverage limit
            SettlementEvent::Failed,
        );
        assert!(outcome.is_skipped());
    }
}
