//! Evidence bundles and the seed database.
//!
//! A campaign that reports a number nobody can reproduce is worse than no
//! campaign: it creates confidence without evidence. Everything written here
//! exists so a reported failure can be re-run, and so a *pass* can be audited
//! for what it actually covered rather than how many runs it performed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::campaign::Campaign;
use crate::engine::CampaignReport;

/// Artifacts written one file at a time, so a partial write cannot look
/// complete.
#[derive(Debug, Clone)]
pub struct EvidenceBundle {
    pub root: PathBuf,
}

/// What this bundle does *not* establish. Written into every summary so a
/// reader cannot mistake a green campaign for a proof.
const NOT_COVERED: [&str; 6] = [
    "proof timing and freshness",
    "per-VM execution latency and timeouts",
    "RPC failure as distinct from message loss",
    "consensus across validators",
    "storage durability beyond a lost persisted write",
    "economic parameters",
];

const INTERPRETATION: &str = "The observed rate is a property of the sampled \
distributions, not of the system. The interval bounds the rate for these \
distributions; it is not proof that the true rate is zero, and it says nothing \
about a distribution this campaign did not sample.";

impl EvidenceBundle {
    pub fn create(root: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(root)
            .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    fn write(&self, name: &str, body: &str) -> Result<(), String> {
        let path = self.root.join(name);
        std::fs::write(&path, body)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    fn pretty<T: serde::Serialize>(value: &T) -> String {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    }

    /// The campaign as it was actually executed, including the resolved seed.
    pub fn write_campaign(&self, campaign: &Campaign, resolved_seed: u64) -> Result<(), String> {
        let mut value = serde_json::to_value(campaign)
            .map_err(|error| format!("campaign did not serialize: {error}"))?;
        if let Some(object) = value.get_mut("campaign").and_then(|meta| meta.as_object_mut()) {
            object.insert("resolved_seed".to_string(), resolved_seed.into());
        }
        self.write("campaign.json", &Self::pretty(&value))
    }

    /// What this machine and this build were.
    pub fn write_environment(&self, extra: &BTreeMap<String, String>) -> Result<(), String> {
        let mut map = extra.clone();
        map.insert("tool".to_string(), "x3-mc".to_string());
        map.insert(
            "tool_version".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
        map.insert(
            "target".to_string(),
            format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
        );
        self.write("environment.json", &Self::pretty(&map))
    }

    /// Counts, coverage, the honest interval, and what was not tested.
    pub fn write_summary(&self, report: &CampaignReport) -> Result<(), String> {
        let (observed, low, high) = report.failure_rate();
        let mut summary = serde_json::Map::new();
        summary.insert("campaign".into(), report.campaign.clone().into());
        summary.insert("scenario".into(), report.scenario.clone().into());
        summary.insert("master_seed".into(), report.master_seed.into());
        summary.insert("runs_requested".into(), report.runs_requested.into());
        summary.insert("runs_completed".into(), report.runs_completed.into());
        summary.insert("failing_runs".into(), report.failing_runs.into());
        summary.insert(
            "distinct_parameter_points".into(),
            report.distinct_parameter_points.into(),
        );
        summary.insert("unique_root_causes".into(), report.unique_root_causes.into());
        summary.insert(
            "stop_reason".into(),
            serde_json::to_value(&report.stop_reason).unwrap_or(serde_json::Value::Null),
        );
        summary.insert("observed_failure_rate".into(), observed.into());
        summary.insert(
            "failure_rate_95_interval".into(),
            serde_json::json!([low, high]),
        );
        summary.insert(
            "violations_by_code".into(),
            serde_json::to_value(&report.coverage.violations).unwrap_or(serde_json::Value::Null),
        );
        summary.insert(
            "observations_mean".into(),
            serde_json::to_value(&report.observations).unwrap_or(serde_json::Value::Null),
        );
        summary.insert(
            "coverage".into(),
            serde_json::to_value(&report.coverage.variables).unwrap_or(serde_json::Value::Null),
        );
        summary.insert(
            "interpretation".into(),
            serde_json::json!({
                "observed_is_not_probability": INTERPRETATION,
                "not_covered_here": NOT_COVERED,
            }),
        );
        summary.insert(
            "failures".into(),
            serde_json::Value::Array(
                report
                    .failures
                    .iter()
                    .map(|failure| {
                        serde_json::json!({
                            "run_index": failure.run_index,
                            "seed": failure.seed,
                            "violations": failure.violations,
                            "minimized": failure.minimized.as_ref().map(|result| &result.params),
                        })
                    })
                    .collect(),
            ),
        );
        self.write("summary.json", &Self::pretty(&summary))
    }

    /// Every failing run, minimized copy included.
    pub fn write_failures(&self, report: &CampaignReport) -> Result<(), String> {
        let directory = self.root.join("failures");
        std::fs::create_dir_all(&directory)
            .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
        for (index, failure) in report.failures.iter().enumerate() {
            let name = format!(
                "{:05}-run{}-seed{}.json",
                index, failure.run_index, failure.seed
            );
            let path = directory.join(name);
            std::fs::write(&path, Self::pretty(failure))
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        }
        Ok(())
    }

    /// The seeds worth keeping, in a form a later campaign can replay.
    pub fn write_seeds(&self, report: &CampaignReport) -> Result<(), String> {
        let mut lines = vec![
            "# x3-mc seed database".to_string(),
            format!(
                "# campaign: {}  master_seed: {}",
                report.campaign, report.master_seed
            ),
            "# replay one with: x3-mc replay <campaign.json> --index <run> --master-seed <master>"
                .to_string(),
        ];
        for failure in &report.failures {
            let code = failure
                .violations
                .first()
                .map(|violation| violation.code.clone())
                .unwrap_or_else(|| "UNKNOWN".to_string());
            lines.push(format!(
                "run={} seed={} violation={} params={}",
                failure.run_index,
                failure.seed,
                code,
                serde_json::to_string(&failure.params).unwrap_or_default()
            ));
        }
        self.write("seeds.txt", &(lines.join("\n") + "\n"))
    }

    /// Everything at once.
    pub fn write_all(
        &self,
        campaign: &Campaign,
        report: &CampaignReport,
        resolved_seed: u64,
        environment: &BTreeMap<String, String>,
    ) -> Result<(), String> {
        self.write_campaign(campaign, resolved_seed)?;
        self.write_environment(environment)?;
        self.write_summary(report)?;
        self.write_failures(report)?;
        self.write_seeds(report)
    }
}
