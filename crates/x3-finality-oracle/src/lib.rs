use std::collections::HashMap;

#[cfg(all(feature = "test-verifier", feature = "production"))]
compile_error!("feature `test-verifier` must never be enabled with `production`");

#[cfg(any(test, feature = "test-verifier"))]
pub mod test_only_evm;
#[cfg(any(test, feature = "test-verifier"))]
pub mod test_only_solana;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chain {
    Ethereum,
    Solana,
    Bitcoin,
    Other(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalityRule {
    pub min_confirmations: u64,
    pub max_allowed_reorg_depth: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedBlock {
    pub chain: Chain,
    pub height: u64,
    pub confirmations: u64,
    pub observed_reorg_depth: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalityStatus {
    Finalized,
    NotEnoughConfirmations,
    ReorgRiskExceeded,
    UnknownChainRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalityVerdict {
    pub status: FinalityStatus,
    pub required_confirmations: u64,
    pub observed_confirmations: u64,
    pub observed_reorg_depth: u64,
}

pub trait FinalityOracle {
    fn evaluate(&self, observed: ObservedBlock) -> FinalityVerdict;
    fn set_rule(&mut self, chain: Chain, rule: FinalityRule);
    fn get_rule(&self, chain: Chain) -> Option<FinalityRule>;
}

#[derive(Debug, Default)]
pub struct InMemoryFinalityOracle {
    rules: HashMap<Chain, FinalityRule>,
}

impl InMemoryFinalityOracle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rules(rules: impl IntoIterator<Item = (Chain, FinalityRule)>) -> Self {
        let mut out = Self::new();
        for (chain, rule) in rules {
            out.set_rule(chain, rule);
        }
        out
    }
}

impl FinalityOracle for InMemoryFinalityOracle {
    fn evaluate(&self, observed: ObservedBlock) -> FinalityVerdict {
        let Some(rule) = self.rules.get(&observed.chain).copied() else {
            return FinalityVerdict {
                status: FinalityStatus::UnknownChainRule,
                required_confirmations: 0,
                observed_confirmations: observed.confirmations,
                observed_reorg_depth: observed.observed_reorg_depth,
            };
        };

        if observed.observed_reorg_depth > rule.max_allowed_reorg_depth {
            return FinalityVerdict {
                status: FinalityStatus::ReorgRiskExceeded,
                required_confirmations: rule.min_confirmations,
                observed_confirmations: observed.confirmations,
                observed_reorg_depth: observed.observed_reorg_depth,
            };
        }

        if observed.confirmations < rule.min_confirmations {
            return FinalityVerdict {
                status: FinalityStatus::NotEnoughConfirmations,
                required_confirmations: rule.min_confirmations,
                observed_confirmations: observed.confirmations,
                observed_reorg_depth: observed.observed_reorg_depth,
            };
        }

        FinalityVerdict {
            status: FinalityStatus::Finalized,
            required_confirmations: rule.min_confirmations,
            observed_confirmations: observed.confirmations,
            observed_reorg_depth: observed.observed_reorg_depth,
        }
    }

    fn set_rule(&mut self, chain: Chain, rule: FinalityRule) {
        self.rules.insert(chain, rule);
    }

    fn get_rule(&self, chain: Chain) -> Option<FinalityRule> {
        self.rules.get(&chain).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_unknown_rule_for_unconfigured_chain() {
        let oracle = InMemoryFinalityOracle::new();
        let verdict = oracle.evaluate(ObservedBlock {
            chain: Chain::Ethereum,
            height: 100,
            confirmations: 12,
            observed_reorg_depth: 0,
        });

        assert_eq!(verdict.status, FinalityStatus::UnknownChainRule);
    }

    #[test]
    fn returns_not_enough_confirmations_when_below_threshold() {
        let oracle = InMemoryFinalityOracle::with_rules([(
            Chain::Ethereum,
            FinalityRule {
                min_confirmations: 12,
                max_allowed_reorg_depth: 2,
            },
        )]);

        let verdict = oracle.evaluate(ObservedBlock {
            chain: Chain::Ethereum,
            height: 120,
            confirmations: 6,
            observed_reorg_depth: 0,
        });

        assert_eq!(verdict.status, FinalityStatus::NotEnoughConfirmations);
        assert_eq!(verdict.required_confirmations, 12);
    }

    #[test]
    fn returns_reorg_risk_exceeded_when_reorg_depth_too_high() {
        let oracle = InMemoryFinalityOracle::with_rules([(
            Chain::Ethereum,
            FinalityRule {
                min_confirmations: 12,
                max_allowed_reorg_depth: 1,
            },
        )]);

        let verdict = oracle.evaluate(ObservedBlock {
            chain: Chain::Ethereum,
            height: 130,
            confirmations: 20,
            observed_reorg_depth: 2,
        });

        assert_eq!(verdict.status, FinalityStatus::ReorgRiskExceeded);
    }

    #[test]
    fn returns_finalized_when_thresholds_are_met() {
        let oracle = InMemoryFinalityOracle::with_rules([(
            Chain::Solana,
            FinalityRule {
                min_confirmations: 32,
                max_allowed_reorg_depth: 0,
            },
        )]);

        let verdict = oracle.evaluate(ObservedBlock {
            chain: Chain::Solana,
            height: 250,
            confirmations: 40,
            observed_reorg_depth: 0,
        });

        assert_eq!(verdict.status, FinalityStatus::Finalized);
    }
}
