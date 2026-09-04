//! The six articles of the X3 Constitution.

use crate::types::ConstitutionHash;
use serde::{Deserialize, Serialize};

/// One of the six constitutional articles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Article {
    /// Article I: Authority derives from mathematical correctness, not actors, operators,
    /// or voters.
    Sovereignty,
    /// Article II: No computation may execute unless it is deterministic, bounded, and
    /// provably compliant with system invariants.
    Execution,
    /// Article III: Core invariants (supply, treasury, agent limits, governance bounds) are
    /// immutable except via formally proven refinement.
    SafetyInvariants,
    /// Article IV: Governance may propose changes but may not violate invariants, expand
    /// powers beyond constitutional limits, or bypass proof requirements.
    Governance,
    /// Article V: Amendments must prove refinement of prior spec, preservation of
    /// meta-invariants, termination, and safety. Unprovable amendments are invalid
    /// regardless of vote outcome.
    Amendments,
    /// Article VI: Invariant violations result in automatic slashing, execution halt, and
    /// forensic replay.
    Enforcement,
}

impl Article {
    /// Returns the canonical text of this article as defined in vΩ-1.0.
    pub fn canonical_text(&self) -> &'static str {
        match self {
            Article::Sovereignty => {
                "The system derives authority from mathematical correctness, \
                 not actors, operators, or voters."
            }
            Article::Execution => {
                "No computation may execute unless it is deterministic, \
                 bounded, and provably compliant with system invariants."
            }
            Article::SafetyInvariants => {
                "Core invariants (supply, treasury, agent limits, governance bounds) \
                 are immutable except via formally proven refinement."
            }
            Article::Governance => {
                "Governance may propose changes but may not violate invariants, \
                 expand powers beyond constitutional limits, or bypass proof requirements."
            }
            Article::Amendments => {
                "Amendments must prove: refinement of prior spec, preservation of \
                 meta-invariants, termination and safety. Unprovable amendments are \
                 invalid regardless of vote outcome."
            }
            Article::Enforcement => {
                "Invariant violations result in automatic slashing, execution halt, \
                 and forensic replay."
            }
        }
    }

    /// All articles in canonical order.
    pub fn all() -> [Article; 6] {
        [
            Article::Sovereignty,
            Article::Execution,
            Article::SafetyInvariants,
            Article::Governance,
            Article::Amendments,
            Article::Enforcement,
        ]
    }
}

/// The full canonical manifest of the X3 Constitution vΩ-1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionManifest {
    /// Specification version.
    pub version: &'static str,
    /// Specification status.
    pub status: &'static str,
    /// Final assertion.
    pub final_assertion: &'static str,
    /// The six articles.
    pub articles: Vec<Article>,
}

impl Default for ConstitutionManifest {
    fn default() -> Self {
        Self {
            version: "vΩ-1.0",
            status: "FINAL FORM",
            final_assertion: "X3 is not governed by trust, incentives, or social norms. \
                              It is governed by proof, determinism, and constitutional constraint.",
            articles: Article::all().to_vec(),
        }
    }
}

impl ConstitutionManifest {
    /// Compute the canonical SHA-256 hash of this constitution manifest.
    /// This hash must match what is checkpointed on-chain.
    pub fn constitution_hash(&self) -> ConstitutionHash {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(self.version.as_bytes());
        hasher.update(self.status.as_bytes());
        hasher.update(self.final_assertion.as_bytes());
        for article in &self.articles {
            hasher.update(article.canonical_text().as_bytes());
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        ConstitutionHash(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constitution_hash_is_deterministic() {
        let m1 = ConstitutionManifest::default();
        let m2 = ConstitutionManifest::default();
        assert_eq!(m1.constitution_hash(), m2.constitution_hash());
    }

    #[test]
    fn all_six_articles_present() {
        let manifest = ConstitutionManifest::default();
        assert_eq!(manifest.articles.len(), 6);
    }

    #[test]
    fn article_texts_are_non_empty() {
        for article in Article::all() {
            assert!(!article.canonical_text().is_empty());
        }
    }
}
