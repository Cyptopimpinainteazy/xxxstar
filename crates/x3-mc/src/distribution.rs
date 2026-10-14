//! Parameter distributions.
//!
//! A campaign says what a variable may be, not what it is. Everything here
//! draws from the run's [`McRng`], so a campaign is reproducible from its seed
//! alone, and every distribution is bounded: an unbounded normal could propose
//! a negative validator count, and a scenario that has to defend against its
//! own sampler is testing the sampler.

use serde::{Deserialize, Deserializer, Serialize};

use crate::rng::McRng;

/// The distributions a campaign may name.
///
/// Deserialization is hand-written rather than derived, and the reason is worth
/// recording. The obvious form is `#[serde(tag = "distribution")]`, which makes
/// serde buffer the whole object before it knows which variant it is. That
/// buffered path represents a JSON number as a special single-entry map when
/// `serde_json`'s `arbitrary_precision` feature is on — and it *is* on here,
/// because the Substrate crates that `x3-cross-vm-bridge` pulls in enable it.
/// The result was that every `f64` field failed to load with the delightfully
/// unhelpful `invalid type: map, expected f64`, while `i64` fields loaded fine.
/// Going through [`serde_json::Value`] avoids the buffered representation, and
/// `Value::as_f64` handles the arbitrary-precision form correctly.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Distribution {
    /// A single value. Useful to pin one axis while searching the others.
    Constant { value: f64 },
    /// Uniform over `[min, max]`.
    Uniform { min: f64, max: f64 },
    /// Normal, truncated to `[min, max]` when given.
    Normal {
        mean: f64,
        std_dev: f64,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// `exp(Normal(mean, std_dev))`, truncated when bounds are given.
    LogNormal {
        mean: f64,
        std_dev: f64,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// Uniform in log space over `[min, max]`. Spans decades without
    /// concentrating the sample at the top of the range.
    LogUniform { min: f64, max: f64 },
    /// True with probability `p`, as `1.0` or `0.0`.
    Bernoulli { p: f64 },
    /// Uniform over the listed values.
    Categorical { values: Vec<f64> },
    /// A value drawn in proportion to its weight.
    WeightedCategorical {
        values: Vec<f64>,
        weights: Vec<f64>,
    },
    /// Poisson, by Knuth's method. Fine for the `lambda` a campaign uses.
    Poisson { lambda: f64 },
    /// An inclusive integer range, returned as a float.
    Integer { min: i64, max: i64 },
    /// Resample one of the given observations. How a measured distribution
    /// (a real load trace, a historical incident) enters a search.
    Empirical { samples: Vec<f64> },
}

impl Distribution {
    /// Draw one value.
    pub fn sample(&self, rng: &mut McRng) -> f64 {
        match self {
            Distribution::Constant { value } => *value,
            Distribution::Uniform { min, max } => rng.range(*min, *max),
            Distribution::Normal {
                mean,
                std_dev,
                min,
                max,
            } => clamp(mean + std_dev * rng.standard_normal(), *min, *max),
            Distribution::LogNormal {
                mean,
                std_dev,
                min,
                max,
            } => {
                let value = (mean + std_dev * rng.standard_normal()).exp();
                clamp(value, *min, *max)
            }
            Distribution::LogUniform { min, max } => {
                let (low, high) = (*min, *max);
                // A non-positive bound has no logarithm; fall back to the
                // linear range rather than producing a NaN that would silently
                // poison every parameter derived from it.
                if low <= 0.0 || high <= 0.0 {
                    return rng.range(low, high);
                }
                (low.ln() + rng.unit() * (high.ln() - low.ln())).exp()
            }
            Distribution::Bernoulli { p } => {
                if rng.unit() < p.clamp(0.0, 1.0) {
                    1.0
                } else {
                    0.0
                }
            }
            Distribution::Categorical { values } => {
                if values.is_empty() {
                    return 0.0;
                }
                values[rng.below(values.len())]
            }
            Distribution::WeightedCategorical { values, weights } => {
                if values.is_empty() {
                    return 0.0;
                }
                let total: f64 = weights.iter().filter(|w| **w > 0.0).sum();
                if !(total > 0.0) {
                    return values[rng.below(values.len())];
                }
                let mut roll = rng.unit() * total;
                for (index, weight) in weights.iter().enumerate() {
                    if *weight <= 0.0 || index >= values.len() {
                        continue;
                    }
                    roll -= weight;
                    if roll <= 0.0 {
                        return values[index];
                    }
                }
                values[values.len() - 1]
            }
            Distribution::Poisson { lambda } => {
                if *lambda <= 0.0 {
                    return 0.0;
                }
                // Knuth: multiply uniforms until the product drops below
                // e^-lambda. The guard caps the loop for a large lambda rather
                // than letting a bad config spin.
                let limit = (-lambda).exp();
                let mut product = 1.0;
                let mut count = 0.0;
                for _ in 0..1_000_000 {
                    product *= rng.unit();
                    if product <= limit {
                        break;
                    }
                    count += 1.0;
                }
                count
            }
            Distribution::Integer { min, max } => {
                if max < min {
                    return *min as f64;
                }
                let span = (*max as i128 - *min as i128 + 1).max(1) as u128;
                let offset = (rng.next_u64() as u128 % span) as i64;
                (*min + offset) as f64
            }
            Distribution::Empirical { samples } => {
                if samples.is_empty() {
                    return 0.0;
                }
                samples[rng.below(samples.len())]
            }
        }
    }

    /// A one-line description for an evidence bundle.
    pub fn describe(&self) -> String {
        match self {
            Distribution::Constant { value } => format!("constant({value})"),
            Distribution::Uniform { min, max } => format!("uniform({min}..{max})"),
            Distribution::Normal { mean, std_dev, .. } => format!("normal({mean},{std_dev})"),
            Distribution::LogNormal { mean, std_dev, .. } => format!("log_normal({mean},{std_dev})"),
            Distribution::LogUniform { min, max } => format!("log_uniform({min}..{max})"),
            Distribution::Bernoulli { p } => format!("bernoulli(p={p})"),
            Distribution::Categorical { values } => format!("categorical({} values)", values.len()),
            Distribution::WeightedCategorical { values, .. } => {
                format!("weighted_categorical({} values)", values.len())
            }
            Distribution::Poisson { lambda } => format!("poisson(lambda={lambda})"),
            Distribution::Integer { min, max } => format!("integer({min}..={max})"),
            Distribution::Empirical { samples } => format!("empirical({} samples)", samples.len()),
        }
    }
}

fn clamp(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let mut out = value;
    if let Some(low) = min {
        out = out.max(low);
    }
    if let Some(high) = max {
        out = out.min(high);
    }
    out
}

impl Distribution {
    /// Build from a campaign entry.
    ///
    /// Every failure names the distribution and the field, because the
    /// alternative is an operator staring at `invalid type: map, expected f64`.
    pub fn from_value(value: &serde_json::Value) -> Result<Self, String> {
        let kind = value
            .get("distribution")
            .and_then(|tag| tag.as_str())
            .ok_or_else(|| "a variable needs a `distribution` field".to_string())?;

        let number = |key: &str| -> Result<f64, String> {
            value
                .get(key)
                .and_then(|item| item.as_f64())
                .ok_or_else(|| format!("`{kind}` needs a numeric `{key}`"))
        };
        let optional = |key: &str| -> Result<Option<f64>, String> {
            match value.get(key) {
                None | Some(serde_json::Value::Null) => Ok(None),
                Some(item) => item
                    .as_f64()
                    .map(Some)
                    .ok_or_else(|| format!("`{kind}` `{key}` must be a number")),
            }
        };
        let array = |key: &str| -> Result<Vec<f64>, String> {
            let items = value
                .get(key)
                .and_then(|item| item.as_array())
                .ok_or_else(|| format!("`{kind}` needs an array `{key}`"))?;
            items
                .iter()
                .map(|item| {
                    item.as_f64()
                        .ok_or_else(|| format!("`{kind}` `{key}` must contain only numbers"))
                })
                .collect()
        };

        Ok(match kind {
            "constant" => Distribution::Constant {
                value: number("value")?,
            },
            "uniform" => Distribution::Uniform {
                min: number("min")?,
                max: number("max")?,
            },
            "normal" => Distribution::Normal {
                mean: number("mean")?,
                std_dev: number("std_dev")?,
                min: optional("min")?,
                max: optional("max")?,
            },
            "log_normal" => Distribution::LogNormal {
                mean: number("mean")?,
                std_dev: number("std_dev")?,
                min: optional("min")?,
                max: optional("max")?,
            },
            "log_uniform" => Distribution::LogUniform {
                min: number("min")?,
                max: number("max")?,
            },
            "bernoulli" => Distribution::Bernoulli { p: number("p")? },
            "categorical" | "discrete" => Distribution::Categorical {
                values: array("values")?,
            },
            "weighted_categorical" | "weighted" => Distribution::WeightedCategorical {
                values: array("values")?,
                weights: array("weights")?,
            },
            "poisson" => Distribution::Poisson {
                lambda: number("lambda")?,
            },
            "integer" | "bounded_integer" => Distribution::Integer {
                min: number("min")?.round() as i64,
                max: number("max")?.round() as i64,
            },
            "empirical" => Distribution::Empirical {
                samples: array("samples")?,
            },
            other => {
                return Err(format!(
                    "unknown distribution `{other}`; expected one of constant, uniform, normal, \
                     log_normal, log_uniform, bernoulli, categorical/discrete, \
                     weighted_categorical, poisson, integer, empirical"
                ))
            }
        })
    }
}

impl<'de> Deserialize<'de> for Distribution {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Distribution::from_value(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(distribution: &Distribution, n: usize, seed: u64) -> Vec<f64> {
        let mut rng = McRng::from_seed(seed);
        (0..n).map(|_| distribution.sample(&mut rng)).collect()
    }

    #[test]
    fn every_distribution_stays_inside_its_support() {
        let cases = vec![
            (Distribution::Uniform { min: 2.0, max: 4.0 }, 2.0, 4.0),
            (
                Distribution::Normal { mean: 10.0, std_dev: 5.0, min: Some(4.0), max: Some(16.0) },
                4.0,
                16.0,
            ),
            (
                Distribution::LogNormal { mean: 1.0, std_dev: 1.0, min: Some(0.5), max: Some(20.0) },
                0.5,
                20.0,
            ),
            (Distribution::LogUniform { min: 10.0, max: 50_000.0 }, 10.0, 50_000.0),
            (Distribution::Bernoulli { p: 0.3 }, 0.0, 1.0),
            (Distribution::Integer { min: 4, max: 21 }, 4.0, 21.0),
            (Distribution::Poisson { lambda: 3.0 }, 0.0, 10_000.0),
            (Distribution::Categorical { values: vec![4.0, 7.0, 10.0] }, 4.0, 10.0),
            (
                Distribution::WeightedCategorical {
                    values: vec![1.0, 2.0, 3.0],
                    weights: vec![1.0, 0.0, 2.0],
                },
                1.0,
                3.0,
            ),
            (Distribution::Empirical { samples: vec![5.0, 6.0, 7.0] }, 5.0, 7.0),
        ];

        for (distribution, low, high) in cases {
            for value in draw(&distribution, 5_000, 1) {
                assert!(
                    value >= low && value <= high,
                    "{} produced {value}, outside {low}..={high}",
                    distribution.describe()
                );
            }
        }
    }

    #[test]
    fn a_zero_weight_category_is_never_drawn() {
        let distribution = Distribution::WeightedCategorical {
            values: vec![1.0, 2.0],
            weights: vec![1.0, 0.0],
        };
        assert!(draw(&distribution, 1_000, 3).iter().all(|value| *value == 1.0));
    }

    #[test]
    fn a_discrete_set_is_covered_evenly_enough() {
        let distribution = Distribution::Categorical { values: vec![4.0, 7.0, 10.0, 14.0, 21.0] };
        let draws = draw(&distribution, 50_000, 8);
        for expected in [4.0, 7.0, 10.0, 14.0, 21.0] {
            let count = draws.iter().filter(|value| **value == expected).count();
            let share = count as f64 / draws.len() as f64;
            assert!((share - 0.2).abs() < 0.02, "value {expected} appeared {share:.3} of the time");
        }
    }

    #[test]
    fn log_uniform_spans_the_decades_rather_than_the_top() {
        let draws = draw(&Distribution::LogUniform { min: 10.0, max: 50_000.0 }, 20_000, 9);
        let low_decade = draws.iter().filter(|value| **value < 500.0).count();
        // 10..500 is about 1.7 of the 3.7 decades, so a log-uniform sample puts
        // roughly 46% of its mass there. A *linear* uniform over the same range
        // would put under 1% there, which is the bias this distribution exists
        // to avoid.
        let share = low_decade as f64 / draws.len() as f64;
        assert!((0.40..0.52).contains(&share), "share below 500 was {share:.3}");
    }

    #[test]
    fn sampling_is_reproducible_from_the_seed() {
        let distribution = Distribution::LogNormal {
            mean: 0.0,
            std_dev: 2.0,
            min: None,
            max: None,
        };
        assert_eq!(draw(&distribution, 500, 77), draw(&distribution, 500, 77));
    }

    #[test]
    fn degenerate_inputs_do_not_panic() {
        let mut rng = McRng::from_seed(1);
        let cases = vec![
            Distribution::Uniform { min: 1.0, max: 1.0 },
            Distribution::LogUniform { min: 0.0, max: 5.0 },
            Distribution::Categorical { values: vec![] },
            Distribution::Empirical { samples: vec![] },
            Distribution::Poisson { lambda: 0.0 },
            Distribution::Integer { min: 5, max: 1 },
            Distribution::WeightedCategorical { values: vec![1.0], weights: vec![0.0] },
        ];
        for distribution in cases {
            let value = distribution.sample(&mut rng);
            assert!(value.is_finite(), "{} produced {value}", distribution.describe());
        }
    }

    #[test]
    fn a_campaign_deserializes_from_the_documented_json_shape() {
        let text = r#"{"distribution": "weighted_categorical", "values": [1, 2], "weights": [3, 1]}"#;
        let parsed: Distribution = serde_json::from_str(text).expect("schema must load");
        assert_eq!(
            parsed,
            Distribution::WeightedCategorical { values: vec![1.0, 2.0], weights: vec![3.0, 1.0] }
        );
    }

    #[test]
    fn each_variant_loads_from_its_documented_json_shape() {
        let cases = [
            r#"{"distribution":"constant","value":1}"#,
            r#"{"distribution":"uniform","min":0.0,"max":0.3}"#,
            r#"{"distribution":"normal","mean":1.0,"std_dev":2.0}"#,
            r#"{"distribution":"log_normal","mean":1.0,"std_dev":2.0,"min":0.1}"#,
            r#"{"distribution":"log_uniform","min":10,"max":50000}"#,
            r#"{"distribution":"bernoulli","p":0.05}"#,
            r#"{"distribution":"discrete","values":[4,7,10]}"#,
            r#"{"distribution":"categorical","values":[4,7,10]}"#,
            r#"{"distribution":"weighted_categorical","values":[1,2],"weights":[3,1]}"#,
            r#"{"distribution":"poisson","lambda":3}"#,
            r#"{"distribution":"integer","min":1,"max":5}"#,
            r#"{"distribution":"empirical","samples":[1,2,3]}"#,
        ];
        for text in cases {
            let parsed: Distribution = serde_json::from_str(text)
                .unwrap_or_else(|error| panic!("{text} did not load: {error}"));
            assert!(!parsed.describe().is_empty());
        }
    }
}
