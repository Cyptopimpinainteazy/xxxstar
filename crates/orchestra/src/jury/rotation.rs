//! Jury rotation — randomized selection of on-chain agents for off-chain jury duty.
//!
//! Enforces Commandment IX: Rotation is randomized; no agent may choose its own jury session.

use crate::agent::identity::{AgentId, AlignmentScore, OrchestraSection};
use crate::agent::on_chain::OnChainAgent;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

/// Configuration for jury rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Fraction of on-chain agents to rotate per session (0.0 - 1.0).
    pub rotation_fraction: f64,
    /// Minimum alignment score to be eligible for rotation.
    pub min_alignment: AlignmentScore,
    /// Maximum proportion from any one section.
    pub max_section_proportion: f64,
    /// Seed entropy source — in production, this comes from the blockchain
    /// (block hash + timestamp) to ensure verifiable randomness.
    pub use_chain_entropy: bool,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            rotation_fraction: 0.2, // 20% of eligible agents rotate per session
            min_alignment: AlignmentScore::JURY_ELIGIBLE_THRESHOLD,
            max_section_proportion: OrchestraSection::MAX_JURY_PROPORTION,
            use_chain_entropy: false,
        }
    }
}

/// Result of a rotation selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    /// Agent IDs selected for jury duty.
    pub selected: Vec<AgentId>,
    /// Section distribution of selected agents.
    pub section_distribution: Vec<(OrchestraSection, u32)>,
    /// Total eligible agents considered.
    pub total_eligible: usize,
    /// Seed used for randomization (for auditability).
    pub seed_hex: String,
}

/// Jury rotation engine.
pub struct JuryRotation {
    config: RotationConfig,
}

impl JuryRotation {
    pub fn new(config: RotationConfig) -> Self {
        Self { config }
    }

    /// Select agents for jury duty from the pool of on-chain agents.
    ///
    /// Selection criteria:
    /// 1. Agent must be Active (not already on jury duty, suspended, etc.)
    /// 2. Agent must meet minimum alignment score
    /// 3. Section proportions must be balanced
    /// 4. Selection is randomized using the provided seed
    pub fn select(
        &self,
        agents: &[OnChainAgent],
        target_size: u32,
        seed: &[u8; 32],
    ) -> RotationResult {
        // Filter eligible agents
        let eligible: Vec<&OnChainAgent> = agents
            .iter()
            .filter(|a| a.is_jury_eligible() && a.identity.alignment >= self.config.min_alignment)
            .collect();

        let total_eligible = eligible.len();

        // An explicit target size is the authority: a jury that comes back a
        // juror short cannot reach quorum, and that failure would otherwise be
        // silent. `rotation_fraction` sizes the jury only when the caller does
        // not ask for a specific size (target_size == 0).
        let desired = if target_size == 0 {
            (eligible.len() as f64 * self.config.rotation_fraction).ceil() as usize
        } else {
            target_size as usize
        }
        .min(eligible.len());

        if desired == 0 {
            return RotationResult {
                selected: Vec::new(),
                section_distribution: Vec::new(),
                total_eligible,
                seed_hex: hex::encode(seed),
            };
        }

        // Shuffle using seeded RNG (deterministic, verifiable)
        let mut rng = rand::rngs::StdRng::from_seed(*seed);
        let mut pool: Vec<&OnChainAgent> = eligible;
        pool.shuffle(&mut rng);

        // Select while respecting section proportions
        let mut selected: Vec<AgentId> = Vec::new();
        let mut section_counts: std::collections::HashMap<OrchestraSection, u32> =
            std::collections::HashMap::new();

        for agent in &pool {
            if selected.len() >= desired {
                break;
            }

            let section = agent.identity.section;
            let current_count = *section_counts.get(&section).unwrap_or(&0);

            // Section quota: a section may hold at most `max_section_proportion`
            // of the jury as it currently stands, but never fewer than one
            // seat — a quota rounded down to zero would reject the first
            // candidate of every section and return an empty jury.
            let jury_size_after = selected.len() as f64 + 1.0;
            let quota = (self.config.max_section_proportion * jury_size_after)
                .floor()
                .max(1.0) as u32;

            if current_count + 1 > quota {
                continue; // skip, try next
            }

            selected.push(agent.identity.id);
            *section_counts.entry(section).or_insert(0) += 1;
        }

        let section_distribution: Vec<(OrchestraSection, u32)> =
            section_counts.into_iter().collect();

        RotationResult {
            selected,
            section_distribution,
            total_eligible,
            seed_hex: hex::encode(seed),
        }
    }

    /// Generate a seed from chain entropy (block hash + timestamp).
    /// In production, this uses actual blockchain data.
    pub fn generate_seed(block_hash: &[u8], timestamp: u64) -> [u8; 32] {
        let mut data = Vec::new();
        data.extend_from_slice(block_hash);
        data.extend_from_slice(&timestamp.to_le_bytes());
        *blake3::hash(&data).as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::identity::AlignmentScore;
    use crate::agent::on_chain::OnChainAgent;

    fn make_eligible_agents(count: usize) -> Vec<OnChainAgent> {
        let sections = [
            OrchestraSection::Strings,
            OrchestraSection::Brass,
            OrchestraSection::Percussion,
            OrchestraSection::Woodwinds,
        ];

        (0..count)
            .map(|i| {
                let mut agent =
                    OnChainAgent::new(i as u32, format!("agent-{}", i), sections[i % 4]);
                agent.identity.alignment = AlignmentScore::new(150); // eligible
                agent
            })
            .collect()
    }

    #[test]
    fn rotation_selects_correct_count() {
        let rotation = JuryRotation::new(RotationConfig {
            rotation_fraction: 0.5,
            ..Default::default()
        });

        let agents = make_eligible_agents(20);
        let seed = [42u8; 32];

        let result = rotation.select(&agents, 5, &seed);
        assert_eq!(result.selected.len(), 5);
        assert_eq!(result.total_eligible, 20);
    }

    #[test]
    fn rotation_is_deterministic() {
        let rotation = JuryRotation::new(Default::default());
        let agents = make_eligible_agents(10);
        let seed = [99u8; 32];

        let r1 = rotation.select(&agents, 3, &seed);
        let r2 = rotation.select(&agents, 3, &seed);

        assert_eq!(r1.selected, r2.selected); // same seed → same result
    }

    #[test]
    fn rotation_respects_section_proportions() {
        let rotation = JuryRotation::new(RotationConfig {
            rotation_fraction: 1.0,
            max_section_proportion: 0.4,
            ..Default::default()
        });

        // All agents from same section
        let agents: Vec<OnChainAgent> = (0..10)
            .map(|i| {
                let mut a = OnChainAgent::new(i, format!("agent-{}", i), OrchestraSection::Strings);
                a.identity.alignment = AlignmentScore::new(150);
                a
            })
            .collect();

        let seed = [1u8; 32];
        let result = rotation.select(&agents, 5, &seed);

        // Every eligible agent is from Strings, so the first seat is the only
        // one the 40% section quota can grant: a second Strings juror would be
        // 2/2 = 100% of the jury. A pool that cannot fill a balanced jury
        // yields a short jury, not an unbalanced one.
        assert_eq!(result.selected.len(), 1);
        assert_eq!(result.total_eligible, 10);
    }

    #[test]
    fn a_zero_target_size_sizes_the_jury_from_the_rotation_fraction() {
        let rotation = JuryRotation::new(RotationConfig {
            rotation_fraction: 0.5,
            ..Default::default()
        });

        let agents = make_eligible_agents(20);
        let result = rotation.select(&agents, 0, &[7u8; 32]);

        // 50% of the 20 eligible agents, spread over four sections.
        assert_eq!(result.selected.len(), 10);
    }

    #[test]
    fn ineligible_agents_excluded() {
        let rotation = JuryRotation::new(Default::default());

        let mut agents = make_eligible_agents(5);
        // Make some ineligible
        agents[0].identity.alignment = AlignmentScore::new(50); // below threshold
        agents[1].identity.alignment = AlignmentScore::new(10); // way below

        let seed = [7u8; 32];
        let result = rotation.select(&agents, 5, &seed);

        assert_eq!(result.total_eligible, 3);
        assert!(!result.selected.contains(&0));
        assert!(!result.selected.contains(&1));
    }

    #[test]
    fn seed_from_chain_entropy() {
        let block_hash = b"0xdeadbeef";
        let timestamp = 1707307200u64;

        let seed = JuryRotation::generate_seed(block_hash, timestamp);
        assert_ne!(seed, [0u8; 32]);

        // Same inputs → same seed
        let seed2 = JuryRotation::generate_seed(block_hash, timestamp);
        assert_eq!(seed, seed2);
    }
}
