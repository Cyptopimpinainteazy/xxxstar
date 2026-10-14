//! The campaign loop.
//!
//! ```text
//! distributions -> per-run seed -> sample parameters -> execute
//!   -> check invariants -> failure or next run
//! ```
//!
//! The engine owns three things and no more: drawing a parameter set for a run,
//! calling the scenario, and judging the result with the scenario's invariants.
//! It does not know what a validator is, and it never decides on its own that a
//! run passed — a run passes because the scenario found no violation.
//!
//! A run is pinned by `(campaign seed, run index)`. Neither the number of runs
//! executed before it nor the order they ran in can change its result, which is
//! what makes `x3-mc replay` exact and what will let this scale across workers
//! without renumbering anything.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::campaign::Campaign;
use crate::minimize::{cluster, minimize, MinimizeResult};
use crate::rng::{derive_seed, McRng};

/// The sampled inputs of one run.
pub type ParamSet = BTreeMap<String, f64>;

/// What a scenario produced, in two parts: numbers the engine compares across
/// runs, and the raw evidence a human reads after a failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOutcome {
    pub observations: BTreeMap<String, f64>,
    pub evidence: serde_json::Value,
}

impl RunOutcome {
    pub fn empty() -> Self {
        Self {
            observations: BTreeMap::new(),
            evidence: serde_json::Value::Null,
        }
    }

    pub fn observe(mut self, key: &str, value: f64) -> Self {
        self.observations.insert(key.to_string(), value);
        self
    }
}

/// One broken property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViolationReport {
    pub code: String,
    pub detail: String,
}

impl ViolationReport {
    pub fn new(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            detail: detail.into(),
        }
    }
}

/// A scenario the engine can search.
///
/// `execute` must be a pure function of `(params, seed)`; anything else breaks
/// replay, which is the only reason a failure found here is worth anything.
pub trait MonteCarloScenario {
    /// Stable name, used in evidence bundles and seed databases.
    fn name(&self) -> &str;

    /// Run once at the given point.
    fn execute(&self, params: &ParamSet, seed: u64) -> RunOutcome;

    /// Judge a run. An empty result means the run held.
    fn invariants(&self, params: &ParamSet, outcome: &RunOutcome) -> Vec<ViolationReport>;
}

/// One run that broke an invariant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub run_index: u64,
    pub seed: u64,
    pub params: ParamSet,
    pub violations: Vec<ViolationReport>,
    pub observations: BTreeMap<String, f64>,
    /// Present when minimization ran and changed something.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimized: Option<MinimizeResult>,
}

/// What happened to a variable across the campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableCoverage {
    pub observed_min: f64,
    pub observed_max: f64,
    pub mean: f64,
    /// Occupied buckets out of `BUCKETS`. Twenty distinct buckets over the
    /// observed range is not the same as "explored everything", but it does
    /// distinguish a million runs over one region from a million over twenty.
    pub buckets_hit: usize,
    pub buckets: usize,
}

pub const COVERAGE_BUCKETS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coverage {
    pub variables: BTreeMap<String, VariableCoverage>,
    /// Runs per violation code.
    pub violations: BTreeMap<String, u64>,
}

/// Why the loop stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// Every planned run completed.
    Completed,
    /// A configured `abort_on` code was observed.
    AbortedOnViolation(String),
    /// The failure budget was reached.
    FailureBudget,
}

/// Knobs for one campaign execution.
#[derive(Debug, Clone)]
pub struct CampaignConfig {
    pub runs: u64,
    pub seed: u64,
    /// Minimize each distinct failing parameter set. Costs extra executions.
    pub minimize: bool,
    pub minimize_budget: u64,
    /// Stop after this many failing runs. `0` means no limit.
    pub max_failures: usize,
    /// Invariant code that ends the campaign immediately.
    pub abort_on: Option<String>,
}

impl Default for CampaignConfig {
    fn default() -> Self {
        Self {
            runs: 100,
            seed: 0,
            minimize: true,
            minimize_budget: 200,
            max_failures: 0,
            abort_on: None,
        }
    }
}

/// The result of a campaign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignReport {
    pub campaign: String,
    pub scenario: String,
    pub master_seed: u64,
    pub runs_requested: u64,
    pub runs_completed: u64,
    pub failures: Vec<Failure>,
    pub failing_runs: u64,
    /// Distinct sampled parameter points. A million runs that visited 40 states
    /// is a weaker campaign than the run count suggests.
    pub distinct_parameter_points: u64,
    pub unique_root_causes: usize,
    pub coverage: Coverage,
    pub stop_reason: StopReason,
    /// Mean of each observation across all runs, for cliff hunting.
    pub observations: BTreeMap<String, f64>,
}

impl CampaignReport {
    pub fn passed(&self) -> bool {
        self.failing_runs == 0
    }

    /// The observed failure rate and its 95% Wilson interval.
    ///
    /// This is a statement about the sampled scenarios, not about the system.
    /// A campaign that observes zero failures in a million runs has bounded the
    /// rate for *these* distributions; it has not shown the rate is zero.
    pub fn failure_rate(&self) -> (f64, f64, f64) {
        wilson(self.failing_runs, self.runs_completed, 1.96)
    }

    /// Seeds to add to the seed database.
    pub fn interesting_seeds(&self) -> Vec<u64> {
        let mut seeds: Vec<u64> = self.failures.iter().map(|failure| failure.seed).collect();
        seeds.sort_unstable();
        seeds.dedup();
        seeds
    }
}

/// Wilson score interval, which stays sane at zero successes and small `n`
/// where the normal approximation does not.
pub fn wilson(successes: u64, trials: u64, z: f64) -> (f64, f64, f64) {
    if trials == 0 {
        return (0.0, 0.0, 1.0);
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let denominator = 1.0 + z * z / n;
    let centre = p + z * z / (2.0 * n);
    let spread = z * ((p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt());
    (
        p,
        ((centre - spread) / denominator).max(0.0),
        ((centre + spread) / denominator).min(1.0),
    )
}

/// Sample every campaign variable for one run.
pub fn sample_params(campaign: &Campaign, rng: &mut McRng) -> ParamSet {
    campaign
        .variables
        .iter()
        .map(|(name, distribution)| (name.clone(), distribution.sample(rng)))
        .collect()
}

/// Execute one run at a known point. Used by `replay`.
pub fn execute_one(
    scenario: &dyn MonteCarloScenario,
    campaign: &Campaign,
    index: u64,
    master_seed: u64,
) -> (ParamSet, u64, RunOutcome, Vec<ViolationReport>) {
    let seed = derive_seed(master_seed, index);
    let mut rng = McRng::from_seed(seed);
    let params = sample_params(campaign, &mut rng);
    let outcome = scenario.execute(&params, seed);
    let violations = scenario.invariants(&params, &outcome);
    (params, seed, outcome, violations)
}

/// Run a campaign.
pub fn run_campaign(
    scenario: &dyn MonteCarloScenario,
    campaign: &Campaign,
    config: &CampaignConfig,
) -> CampaignReport {
    let mut failures: Vec<Failure> = Vec::new();
    let mut violation_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut observation_sums: BTreeMap<String, f64> = BTreeMap::new();
    let mut variable_min: BTreeMap<String, f64> = BTreeMap::new();
    let mut variable_max: BTreeMap<String, f64> = BTreeMap::new();
    let mut variable_sum: BTreeMap<String, f64> = BTreeMap::new();
    let mut samples: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut distinct_points: Vec<u64> = Vec::new();
    let mut stop_reason = StopReason::Completed;
    let mut runs_completed = 0u64;
    let mut failing_runs = 0u64;
    // Minimizing the same parameter set twice is wasted work: a campaign that
    // hits one defect ten thousand times would re-shrink it ten thousand times.
    let mut minimized_cache: BTreeMap<String, MinimizeResult> = BTreeMap::new();

    for index in 0..config.runs {
        let (params, seed, outcome, violations) = execute_one(scenario, campaign, index, config.seed);
        runs_completed += 1;

        for (name, value) in &params {
            variable_min
                .entry(name.clone())
                .and_modify(|current| *current = current.min(*value))
                .or_insert(*value);
            variable_max
                .entry(name.clone())
                .and_modify(|current| *current = current.max(*value))
                .or_insert(*value);
            *variable_sum.entry(name.clone()).or_insert(0.0) += value;
            samples.entry(name.clone()).or_default().push(*value);
        }
        for (name, value) in &outcome.observations {
            *observation_sums.entry(name.clone()).or_insert(0.0) += value;
        }

        // A fingerprint of the sampled point, so the report can distinguish a
        // million runs over a million states from a million over one.
        let mut fingerprint = blake3::Hasher::new();
        for value in params.values() {
            fingerprint.update(&value.to_bits().to_le_bytes());
        }
        let digest = fingerprint.finalize();
        let short = u64::from_le_bytes(digest.as_bytes()[..8].try_into().unwrap_or([0; 8]));
        if !distinct_points.contains(&short) {
            distinct_points.push(short);
        }

        for violation in &violations {
            *violation_counts.entry(violation.code.clone()).or_insert(0) += 1;
        }

        if !violations.is_empty() {
            failing_runs += 1;
            let minimized = if config.minimize {
                let signature: String = params
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(",");
                Some(
                    minimized_cache
                        .entry(signature)
                        .or_insert_with(|| minimize(scenario, &params, seed, config.minimize_budget))
                        .clone(),
                )
            } else {
                None
            };
            if let Some(result) = &minimized {
                for (name, value) in &result.params {
                    if let Some(current) = variable_min.get_mut(name) {
                        *current = current.min(*value);
                    }
                    if let Some(current) = variable_max.get_mut(name) {
                        *current = current.max(*value);
                    }
                }
            }
            failures.push(Failure {
                run_index: index,
                seed,
                params,
                violations: violations.clone(),
                observations: outcome.observations.clone(),
                minimized,
            });

            if let Some(code) = &config.abort_on {
                if violations.iter().any(|violation| &violation.code == code) {
                    stop_reason = StopReason::AbortedOnViolation(code.clone());
                    break;
                }
            }
            if config.max_failures > 0 && failures.len() >= config.max_failures {
                stop_reason = StopReason::FailureBudget;
                break;
            }
        }
    }

    // Bucket coverage, computed after the loop so one grid is shared by every
    // variable instead of allocating one per run.
    let buckets: BTreeMap<String, Vec<u64>> = samples
        .iter()
        .map(|(name, values)| {
            let low = variable_min.get(name).copied().unwrap_or(0.0);
            let high = variable_max.get(name).copied().unwrap_or(low);
            let span = high - low;
            let mut grid = vec![0u64; COVERAGE_BUCKETS];
            for value in values {
                let bucket = if span <= 0.0 {
                    0
                } else {
                    (((value - low) / span) * (COVERAGE_BUCKETS as f64 - 1.0)).round() as usize
                };
                grid[bucket.min(COVERAGE_BUCKETS - 1)] += 1;
            }
            (name.clone(), grid)
        })
        .collect();

    let coverage = Coverage {
        variables: variable_min
            .iter()
            .map(|(name, low)| {
                let high = variable_max.get(name).copied().unwrap_or(*low);
                let grid = buckets.get(name).cloned().unwrap_or_default();
                (
                    name.clone(),
                    VariableCoverage {
                        observed_min: *low,
                        observed_max: high,
                        mean: variable_sum.get(name).copied().unwrap_or(0.0)
                            / runs_completed.max(1) as f64,
                        buckets_hit: grid.iter().filter(|count| **count > 0).count(),
                        buckets: COVERAGE_BUCKETS,
                    },
                )
            })
            .collect(),
        violations: violation_counts,
    };

    let root_causes = cluster(&failures).len();
    CampaignReport {
        campaign: campaign.campaign.name.clone(),
        scenario: scenario.name().to_string(),
        master_seed: config.seed,
        runs_requested: config.runs,
        runs_completed,
        failures,
        failing_runs,
        distinct_parameter_points: distinct_points.len() as u64,
        unique_root_causes: root_causes,
        coverage,
        stop_reason,
        observations: observation_sums
            .into_iter()
            .map(|(key, sum)| (key, sum / runs_completed.max(1) as f64))
            .collect(),
    }
}

/// Typed reads out of a sampled parameter set.
///
/// Every accessor has a default, so a campaign that omits a variable still
/// runs: the scenario's baseline is what the variable was pinned to, not a
/// crash.
pub fn as_f64(params: &ParamSet, key: &str, default: f64) -> f64 {
    params.get(key).copied().unwrap_or(default)
}

pub fn as_usize(params: &ParamSet, key: &str, default: usize) -> usize {
    as_f64(params, key, default as f64).round().max(0.0) as usize
}

pub fn as_u32(params: &ParamSet, key: &str, default: u32) -> u32 {
    as_f64(params, key, default as f64).round().clamp(0.0, u32::MAX as f64) as u32
}

pub fn as_u64(params: &ParamSet, key: &str, default: u64) -> u64 {
    as_f64(params, key, default as f64).round().max(0.0) as u64
}

pub fn as_bool(params: &ParamSet, key: &str, default: bool) -> bool {
    as_f64(params, key, if default { 1.0 } else { 0.0 }) >= 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wilson_interval_is_sane_at_zero_failures() {
        let (rate, low, high) = wilson(0, 1_000_000, 1.96);
        assert_eq!(rate, 0.0);
        assert_eq!(low, 0.0);
        // Zero observed failures in a million runs bounds the rate at roughly
        // 3.7e-6. Reporting "0" would be reporting the observation as a
        // property.
        assert!(high > 0.0 && high < 1e-5, "upper bound was {high}");
    }

    #[test]
    fn a_wilson_interval_brackets_the_observed_rate() {
        let (rate, low, high) = wilson(3, 1_000_000, 1.96);
        assert!((rate - 3e-6).abs() < 1e-12);
        assert!(low < rate && rate < high);
    }

    #[test]
    fn an_empty_campaign_says_it_learned_nothing() {
        let (rate, low, high) = wilson(0, 0, 1.96);
        assert_eq!((rate, low, high), (0.0, 0.0, 1.0));
    }

    #[test]
    fn parameter_accessors_have_defaults_rather_than_panicking() {
        let params = ParamSet::new();
        assert_eq!(as_usize(&params, "missing", 7), 7);
        assert_eq!(as_f64(&params, "missing", 1.5), 1.5);
        assert!(as_bool(&params, "missing", true));
        assert_eq!(as_u64(&params, "missing", 9), 9);
    }

    #[test]
    fn a_negative_sample_does_not_become_a_huge_count() {
        let mut params = ParamSet::new();
        params.insert("n".to_string(), -3.0);
        assert_eq!(as_usize(&params, "n", 1), 0);
    }
}
