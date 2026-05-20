use crate::{
    Chain, FinalityOracle, FinalityRule, FinalityVerdict, InMemoryFinalityOracle, ObservedBlock,
};

pub fn evm_oracle() -> InMemoryFinalityOracle {
    InMemoryFinalityOracle::with_rules([(
        Chain::Ethereum,
        FinalityRule {
            min_confirmations: 12,
            max_allowed_reorg_depth: 2,
        },
    )])
}

pub fn evaluate_eth_block(confirmations: u64, reorg_depth: u64) -> FinalityVerdict {
    evm_oracle().evaluate(ObservedBlock {
        chain: Chain::Ethereum,
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
    fn eth_block_is_finalized_with_enough_confirmations() {
        let verdict = evaluate_eth_block(15, 1);
        assert_eq!(verdict.status, FinalityStatus::Finalized);
    }

    #[test]
    fn eth_block_is_not_finalized_with_low_confirmations() {
        let verdict = evaluate_eth_block(5, 0);
        assert_eq!(verdict.status, FinalityStatus::NotEnoughConfirmations);
    }
}
