//! The campaign file format.
//!
//! A campaign names the variables a search may move and the distribution each
//! one is drawn from. It is deliberately data, not code: the same scenario can
//! be run at 100 samples for a pull request and 1,000,000 for a release gate
//! without editing the scenario.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};

use crate::distribution::Distribution;

/// `seed: auto` chooses one; `seed: 1234` pins one.
///
/// Written by hand for the same reason [`crate::distribution::Distribution`]
/// is: an untagged enum makes serde buffer the value, and the buffered
/// representation of a JSON number under `serde_json`'s `arbitrary_precision`
/// feature — enabled transitively by the Substrate crates — is a map rather
/// than a scalar, so a plain integer seed failed to load.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SeedSpec {
    /// The literal string `auto`.
    Auto(String),
    Fixed(u64),
}

impl<'de> Deserialize<'de> for SeedSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match &value {
            serde_json::Value::String(text) => Ok(SeedSpec::Auto(text.clone())),
            other => other
                .as_u64()
                .map(SeedSpec::Fixed)
                .ok_or_else(|| serde::de::Error::custom("seed must be `auto` or an integer")),
        }
    }
}

impl SeedSpec {
    /// Resolve to a concrete seed.
    ///
    /// `auto` is resolved from the caller's entropy rather than from a clock
    /// read inside the engine, so the library stays testable and the chosen
    /// value is always recorded in the evidence bundle. A campaign that
    /// reports a failure is reproducible no matter which path chose its seed.
    pub fn resolve(&self, entropy: u64) -> Result<u64, String> {
        match self {
            SeedSpec::Fixed(value) => Ok(*value),
            SeedSpec::Auto(text) if text.eq_ignore_ascii_case("auto") => Ok(entropy),
            SeedSpec::Auto(text) => Err(format!("seed must be `auto` or an integer, found `{text}`")),
        }
    }
}

/// The `campaign:` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignMeta {
    pub name: String,
    /// Planned number of runs. `--runs` may lower or raise it.
    pub runs: u64,
    #[serde(default = "default_seed")]
    pub seed: SeedSpec,
    #[serde(default)]
    pub description: Option<String>,
    /// The invariant code whose first sighting ends the campaign early.
    #[serde(default)]
    pub abort_on: Option<String>,
}

fn default_seed() -> SeedSpec {
    SeedSpec::Auto("auto".to_string())
}

/// A whole campaign file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Campaign {
    pub campaign: CampaignMeta,
    /// Variable name to distribution.
    ///
    /// A `BTreeMap`, so the draw order is the sorted order and not the order
    /// the keys happen to appear in the file: editing the file's layout must
    /// not silently change what a seed produces.
    #[serde(default)]
    pub variables: BTreeMap<String, Distribution>,
}

impl Campaign {
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| format!("campaign did not parse: {error}"))
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Self::parse(&text)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
    {
      "campaign": { "name": "x3-atomic-chaos-001", "runs": 1000, "seed": "auto" },
      "variables": {
        "validator_count": { "distribution": "categorical", "values": [4, 7, 10, 14, 21] },
        "packet_loss":      { "distribution": "uniform", "min": 0.0, "max": 0.30 },
        "tx_rate":          { "distribution": "log_uniform", "min": 10, "max": 50000 },
        "restart":          { "distribution": "bernoulli", "p": 0.05 }
      }
    }"#;

    #[test]
    fn the_documented_campaign_shape_loads() {
        let campaign = Campaign::parse(EXAMPLE).expect("example must load");
        assert_eq!(campaign.campaign.name, "x3-atomic-chaos-001");
        assert_eq!(campaign.campaign.runs, 1000);
        assert_eq!(campaign.variables.len(), 4);
        assert_eq!(campaign.seed(7).unwrap(), 7, "an auto seed takes the caller's entropy");
    }

    #[test]
    fn an_explicit_seed_wins_over_entropy() {
        let campaign = Campaign::parse(
            r#"{"campaign": {"name": "pinned", "runs": 1, "seed": 441837226}}"#,
        )
        .expect("must load");
        assert_eq!(campaign.seed(1).unwrap(), 441_837_226);
    }

    #[test]
    fn a_malformed_seed_is_an_error_rather_than_a_silent_default() {
        let campaign = Campaign::parse(
            r#"{"campaign": {"name": "bad", "runs": 1, "seed": "whenever"}}"#,
        )
        .expect("structure is fine");
        assert!(campaign.seed(1).is_err());
    }

    #[test]
    fn variable_order_in_the_file_does_not_change_the_draw_order() {
        let first = Campaign::parse(
            r#"{"campaign":{"name":"o","runs":1},"variables":{
                 "a":{"distribution":"constant","value":1},"b":{"distribution":"constant","value":2}}}"#,
        )
        .unwrap();
        let second = Campaign::parse(
            r#"{"campaign":{"name":"o","runs":1},"variables":{
                 "b":{"distribution":"constant","value":2},"a":{"distribution":"constant","value":1}}}"#,
        )
        .unwrap();
        assert_eq!(
            first.variables.keys().collect::<Vec<_>>(),
            second.variables.keys().collect::<Vec<_>>()
        );
    }
}

impl Campaign {
    /// The campaign seed, resolved.
    pub fn seed(&self, entropy: u64) -> Result<u64, String> {
        self.campaign.seed.resolve(entropy)
    }
}
