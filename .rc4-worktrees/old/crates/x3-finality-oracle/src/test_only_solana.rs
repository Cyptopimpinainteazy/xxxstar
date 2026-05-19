use crate::{
    Chain, FinalityOracle, FinalityRule, FinalityVerdict, InMemoryFinalityOracle, ObservedBlock,
};

pub fn solana_oracle() -> InMemoryFinalityOracle {
    InMemoryFinalityOracle::with_rules([(
        Chain::Solana,
        FinalityRule {
            min_confirmations: 32,
            max_allowed_reorg_depth: 0,
        },
    )])
}

pub fn evaluate_solana_slot(confirmations: u64, reorg_depth: u64) -> FinalityVerdict {
    solana_oracle().evaluate(ObservedBlock {
        chain: Chain::Solana,
        height: 0,
        confirmations,
        observed_reorg_depth: reorg_depth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FinalityStatus;

    #[test]
    fn solana_slot_finalizes_when_threshold_met() {
        let verdict = evaluate_solana_slot(40, 0);
        assert_eq!(verdict.status, FinalityStatus::Finalized);
    }

    #[test]
    fn solana_slot_rejects_reorg_risk() {
        let verdict = evaluate_solana_slot(40, 1);
        assert_eq!(verdict.status, FinalityStatus::ReorgRiskExceeded);
    }
}
