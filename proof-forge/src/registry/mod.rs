#![allow(dead_code)] // intentional registry scaffold; tracked in readiness backlog

// Registry module for claim management and verification tracking

use crate::proof::{Claim, ProofResult, ProofStatus};
use std::collections::HashMap;

pub mod claims;
/// Claim registry with verification tracking
#[derive(Debug, Clone)]
pub struct Registry {
    claims: HashMap<String, Claim>,
    results: HashMap<String, ProofResult>,
    version: String,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            claims: HashMap::new(),
            results: HashMap::new(),
            version: "1.0".to_string(),
        }
    }

    /// Register a new claim
    pub fn register_claim(&mut self, claim: Claim) {
        self.claims.insert(claim.id.clone(), claim);
    }

    /// Record proof result for a claim
    pub fn record_result(&mut self, result: ProofResult) {
        self.results.insert(result.claim_id.clone(), result);
    }

    /// Get all claims in registry
    pub fn all_claims(&self) -> Vec<&Claim> {
        self.claims.values().collect()
    }

    /// Get all results
    pub fn all_results(&self) -> Vec<&ProofResult> {
        self.results.values().collect()
    }

    /// Get claims by area
    pub fn claims_by_area(&self, area: &str) -> Vec<&Claim> {
        self.claims.values().filter(|c| c.area == area).collect()
    }

    /// Get unproven claims
    pub fn unproven_claims(&self) -> Vec<&Claim> {
        self.claims
            .values()
            .filter(|c| !self.results.contains_key(&c.id))
            .collect()
    }

    /// Get claims with blocking status
    pub fn blocking_claims(&self) -> Vec<(&Claim, &ProofResult)> {
        self.claims
            .iter()
            .filter_map(|(id, claim)| {
                self.results.get(id).map(|result| {
                    if result.status.is_blocking() {
                        Some((claim, result))
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .collect()
    }

    /// Calculate overall verification status
    pub fn overall_status(&self) -> ProofStatus {
        let total = self.claims.len();
        if total == 0 {
            return ProofStatus::Unverified;
        }

        let verified = self
            .results
            .values()
            .filter(|r| r.status == ProofStatus::Verified)
            .count();

        if verified == total {
            ProofStatus::Verified
        } else if verified > 0 {
            ProofStatus::Partial
        } else {
            ProofStatus::Unverified
        }
    }

    /// Export registry as JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&serde_json::json!({
            "version": self.version,
            "claims_count": self.claims.len(),
            "results_count": self.results.len(),
            "claims": self.claims.values().collect::<Vec<_>>(),
            "results": self.results.values().collect::<Vec<_>>(),
            "overall_status": format!("{:?}", self.overall_status()),
        }))
    }
}

/// High-severity claim markers
pub mod markers {
    pub const CRITICAL: &str = "CRITICAL";
    pub const SECURITY: &str = "SECURITY";
    pub const MAINNET: &str = "MAINNET";
    pub const TESTNET: &str = "TESTNET";
    pub const CONSENSUS: &str = "CONSENSUS";
    pub const ECONOMIC: &str = "ECONOMIC";
}
