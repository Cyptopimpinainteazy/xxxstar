//! Failure minimization.
//!
//! A failing run reports the parameters that produced it; those parameters are
//! almost never the smallest version of the failure. The original report might
//! say *four partitions, 27% loss, 11 stale writes, 42 sessions*, when the same
//! defect reproduces at *one partition, 27% loss, no stale writes, 2 sessions*.
//!
//! The search is greedy delta debugging: for each variable in a fixed order,
//! try progressively simpler values and keep the first that still fails. It is
//! not guaranteed to find the global minimum — it is a search, and saying
//! otherwise would be a claim the code cannot support — but it reliably removes
//! the axes a failure does not depend on, and every accepted step is verified by
//! re-execution rather than assumed.

use std::collections::BTreeMap;

use crate::engine::{MonteCarloScenario, ParamSet, ViolationReport};

/// What minimization produced.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MinimizeResult {
    /// The smallest parameter set that still reproduces the violation.
    pub params: ParamSet,
    /// The violation codes that survive at the minimized point.
    pub violations: Vec<ViolationReport>,
    /// Executions spent, including the ones that did not help.
    pub executions: u64,
    /// How many variables were reduced.
    pub reduced_variables: usize,
}

/// Candidate simplifications for one value, cheapest and most aggressive first.
///
/// Integral values keep their integrality: halving a validator count to 2.5
/// would produce a parameter the scenario never samples and a reproducer an
/// engineer cannot type.
fn candidates(value: f64) -> Vec<f64> {
    let mut out = Vec::new();
    if value == 0.0 {
        return out;
    }
    let integral = value.fract() == 0.0;
    let push = |out: &mut Vec<f64>, candidate: f64| {
        let candidate = if integral { candidate.round() } else { candidate };
        if candidate.is_finite() && candidate < value && !out.contains(&candidate) {
            out.push(candidate);
        }
    };

    push(&mut out, 0.0);
    push(&mut out, value / 2.0);
    push(&mut out, value - 1.0);
    push(&mut out, value / 4.0);
    push(&mut out, value - value.abs().max(1.0) / 10.0);
    out
}

/// Shrink `params` while the violation survives.
///
/// `seed` is held fixed: the point is to find a smaller *input* that fails, not
/// a different run.
pub fn minimize(
    scenario: &dyn MonteCarloScenario,
    params: &ParamSet,
    seed: u64,
    max_executions: u64,
) -> MinimizeResult {
    let mut current = params.clone();
    let baseline = scenario.execute(&current, seed);
    let mut violations = scenario.invariants(&current, &baseline);
    let target: Vec<String> = violations.iter().map(|v| v.code.clone()).collect();
    let mut executions = 1u64;
    let mut reduced = 0usize;

    if violations.is_empty() {
        return MinimizeResult {
            params: current,
            violations,
            executions,
            reduced_variables: 0,
        };
    }

    // Keep passing until a full pass reduces nothing, or the budget runs out.
    // A fixed number of passes leaves obviously removable variables behind
    // whenever an earlier shrink changes which later ones matter, and it stops
    // well short of the boundary the failure actually sits on.
    loop {
        let before = reduced;
        for key in params.keys() {
            let start = current.get(key).copied().unwrap_or(0.0);
            for candidate in candidates(start) {
                if executions >= max_executions {
                    break;
                }
                let mut trial = current.clone();
                trial.insert(key.clone(), candidate);
                let outcome = scenario.execute(&trial, seed);
                executions += 1;
                let found = scenario.invariants(&trial, &outcome);
                // Only accept a shrink that still shows the *same* violation.
                // Accepting any violation would drift onto a different defect
                // and report it as a minimized version of this one.
                let still_fails = found.iter().any(|v| target.contains(&v.code));
                if still_fails {
                    current = trial;
                    violations = found;
                    reduced += 1;
                    break;
                }
            }
        }
        if reduced == before || executions >= max_executions {
            break;
        }
    }

    MinimizeResult {
        params: current,
        violations,
        executions,
        reduced_variables: reduced,
    }
}

/// Group failures that share a cause so a campaign reports root causes rather
/// than a pile of duplicate rows.
///
/// The signature is the set of violated properties. That is deliberately
/// coarse: two runs that break the same property are reported as one cause even
/// when they arrived by different parameters, because the parameter values are
/// what minimization *removes* — keying on them, as an earlier version did,
/// meant a thousand failures of one defect clustered into a thousand "root
/// causes". A stack trace or code path would be a sharper signature and is the
/// natural upgrade once the harnesses produce one.
pub fn cluster(failures: &[crate::engine::Failure]) -> BTreeMap<String, usize> {
    let mut clusters: BTreeMap<String, usize> = BTreeMap::new();
    for failure in failures {
        let source = failure
            .minimized
            .as_ref()
            .map(|result| result.violations.as_slice())
            .unwrap_or(failure.violations.as_slice());
        let mut codes: Vec<&str> = source
            .iter()
            .map(|violation| violation.code.as_str())
            .collect();
        codes.sort_unstable();
        codes.dedup();
        let signature = if codes.is_empty() {
            "UNCLASSIFIED".to_string()
        } else {
            codes.join("+")
        };
        *clusters.entry(signature).or_insert(0) += 1;
    }
    clusters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_keep_integers_integral() {
        assert!(candidates(8.0).iter().all(|value| value.fract() == 0.0));
        assert!(candidates(0.0).is_empty(), "zero is already minimal");
    }

    #[test]
    fn candidates_only_move_downward() {
        for value in [0.5, 1.0, 7.0, 1_500.0] {
            assert!(candidates(value).iter().all(|candidate| *candidate < value));
        }
    }
}
