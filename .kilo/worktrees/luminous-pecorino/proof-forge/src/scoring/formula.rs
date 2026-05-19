// Proof scoring formula implementation

use super::ScoringContext;

/// Calculate proof score using weighted formula
#[allow(dead_code)]
pub fn calculate_proof_score(context: &ScoringContext) -> f64 {
    context.calculate_score()
}

/// Determine if a score meets mainnet readiness threshold
#[allow(dead_code)]
pub fn is_mainnet_ready(score: f64) -> bool {
    score >= 0.95
}

/// Determine if a score meets testnet readiness threshold
#[allow(dead_code)]
pub fn is_testnet_ready(score: f64) -> bool {
    score >= 0.85
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_score() {
        let ctx = ScoringContext {
            compile_score: 1.0,
            unit_tests_score: 1.0,
            integration_score: 1.0,
            invariant_score: 1.0,
            adversarial_score: 1.0,
            benchmark_score: 1.0,
            wiring_score: 1.0,
            drift_score: 1.0,
        };
        assert_eq!(ctx.calculate_score(), 1.0);
    }

    #[test]
    fn test_zero_score() {
        let ctx = ScoringContext {
            compile_score: 0.0,
            unit_tests_score: 0.0,
            integration_score: 0.0,
            invariant_score: 0.0,
            adversarial_score: 0.0,
            benchmark_score: 0.0,
            wiring_score: 0.0,
            drift_score: 0.0,
        };
        assert_eq!(ctx.calculate_score(), 0.0);
    }

    #[test]
    fn test_mainnet_threshold() {
        assert!(is_mainnet_ready(0.95));
        assert!(is_mainnet_ready(0.99));
        assert!(!is_mainnet_ready(0.94));
    }

    #[test]
    fn test_testnet_threshold() {
        assert!(is_testnet_ready(0.85));
        assert!(is_testnet_ready(0.95));
        assert!(!is_testnet_ready(0.84));
    }
}
