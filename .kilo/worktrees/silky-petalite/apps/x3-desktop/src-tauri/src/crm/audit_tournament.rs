// TIER 9: Adversarial Audit Tournament Framework
// Public challenge structure, bounty model, and scoring system

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use std::collections::HashMap;

// ================================================
// TOURNAMENT STRUCTURE
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTournament {
    pub id: String,
    pub name: String,
    pub version: String,  // "X3 Adversarial Audit Championship v1.0"
    pub description: String,
    
    // Timeline
    pub launch_date: DateTime<Utc>,
    pub submission_deadline: DateTime<Utc>,
    pub analysis_period_end: DateTime<Utc>,
    pub leaderboard_finalized_date: DateTime<Utc>,
    
    // Participation
    pub max_participants: Option<i32>,
    pub current_participants: i32,
    pub registration_open: bool,
    
    // Prize Pool
    pub total_prize_pool_usd: u64,
    pub remaining_prize_pool_usd: u64,
    
    // Challenge Targets
    pub attack_categories: Vec<AttackCategory>,
    pub systems_under_review: Vec<String>,
    pub difficulty_rating: String,  // 'extreme', 'hard', 'medium'
    
    // Rules & Governance
    pub rules: String,
    pub code_of_conduct: String,
    pub publications_policy: String,  // Can findings be published?
    pub attribution_model: String,  // How are findings credited?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackCategory {
    pub id: String,
    pub category_name: String,
    pub description: String,
    pub examples: Vec<String>,
    pub estimated_difficulty: String,
    pub base_bounty_usd: u64,
    pub max_bounty_usd: u64,
    pub current_submissions: i32,
}

// ================================================
// BOUNTY MODEL
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyTier {
    pub tier_name: String,  // 'Informational', 'Low', 'Medium', 'High', 'Critical'
    pub severity_score: i32,  // 0-100
    pub base_bounty_usd: u64,
    pub max_bounty_usd: u64,
    pub impact_description: String,
    pub example_vulnerabilities: Vec<String>,
}

pub fn get_bounty_tiers() -> Vec<BountyTier> {
    vec![
        BountyTier {
            tier_name: "Informational".to_string(),
            severity_score: 0,
            base_bounty_usd: 0,
            max_bounty_usd: 100,
            impact_description: "No direct security impact. Suggestions for improvement, documentation clarity, or best practices.".to_string(),
            example_vulnerabilities: vec![
                "Typo in documentation".to_string(),
                "Suboptimal code style".to_string(),
                "Missing inline comments".to_string(),
            ],
        },
        BountyTier {
            tier_name: "Low".to_string(),
            severity_score: 10,
            base_bounty_usd: 500,
            max_bounty_usd: 2000,
            impact_description: "Minor security concern with minimal exploit potential. Affects edge cases or requires very specific conditions.".to_string(),
            example_vulnerabilities: vec![
                "Integer overflow in non-critical path".to_string(),
                "Inefficient algorithm causing minor gas waste".to_string(),
                "Validation logic missing for optional parameters".to_string(),
            ],
        },
        BountyTier {
            tier_name: "Medium".to_string(),
            severity_score: 40,
            base_bounty_usd: 5000,
            max_bounty_usd: 25000,
            impact_description: "Moderate security impact. Could lead to loss of funds under certain conditions or affect network quality.".to_string(),
            example_vulnerabilities: vec![
                "Validator equity miscalculation under load".to_string(),
                "Cross-chain relay timeout edge case".to_string(),
                "Insufficient signature validation in specific ordering".to_string(),
            ],
        },
        BountyTier {
            tier_name: "High".to_string(),
            severity_score: 70,
            base_bounty_usd: 25000,
            max_bounty_usd: 100000,
            impact_description: "Significant security vulnerability. Direct loss of funds, network disruption, or critical component failure possible.".to_string(),
            example_vulnerabilities: vec![
                "Validator equivocation allowing double-spend".to_string(),
                "MEV extraction bypassing ordering guarantees".to_string(),
                "Consensus break exploitable by small number of validators".to_string(),
            ],
        },
        BountyTier {
            tier_name: "Critical".to_string(),
            severity_score: 95,
            base_bounty_usd: 100000,
            max_bounty_usd: 500000,
            impact_description: "Catastrophic vulnerability. Complete network failure, massive fund loss, or fundamental protocol break.".to_string(),
            example_vulnerabilities: vec![
                "Consensus break exploitable by single validator".to_string(),
                "State corruption allowing arbitrary fund theft".to_string(),
                "Cryptographic break defeating signature verification".to_string(),
            ],
        },
    ]
}

// ================================================
// SUBMISSION & SCORING SYSTEM
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSubmission {
    pub id: String,
    pub tournament_id: String,
    
    // Researcher Info
    pub researcher_handle: String,
    pub researcher_email: String,
    pub researcher_affiliation: Option<String>,
    pub researcher_github: Option<String>,
    
    // Finding Details
    pub attack_category: String,
    pub severity_tier: String,
    pub vulnerability_title: String,
    pub description: String,
    pub impact_assessment: String,
    pub proof_of_concept: String,
    pub remediation_suggestion: String,
    
    // Submission Metadata
    pub submission_date: DateTime<Utc>,
    pub first_found_date: DateTime<Utc>,  // When discovered (affects scoring)
    pub reproducible: bool,
    pub verified: bool,
    pub severity_confirmed_score: i32,  // 0-100, set by organizers
    
    // Scoring
    pub discovery_speed_bonus: f32,  // Earlier = higher bonus
    pub uniqueness_score: f32,  // 0-100 (is this novel?)
    pub quality_score: f32,  // 0-100 (writeup quality, clarity)
    pub impact_multiplier: f32,  // 1.0-3.0x based on severity
    
    pub final_score: f32,
    pub bounty_awarded_usd: u64,
    
    // Status
    pub status: String,  // 'submitted', 'reviewing', 'approved', 'revalidating', 'rejected'
    pub reviewer_notes: Option<String>,
    pub publication_approved: bool,
    pub publication_link: Option<String>,
    
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringAlgorithm {
    pub discovery_speed_weight: f32,  // 20%
    pub uniqueness_weight: f32,  // 20%
    pub quality_weight: f32,  // 20%
    pub severity_weight: f32,  // 40%
    
    pub max_multiplier_for_critical: f32,  // 3.0x
    pub max_multiplier_for_high: f32,  // 2.0x
    pub max_multiplier_for_medium: f32,  // 1.5x
}

impl ScoringAlgorithm {
    pub fn calculate_finding_score(
        discovery_speed_score: f32,  // 0-100
        uniqueness_score: f32,  // 0-100
        quality_score: f32,  // 0-100
        severity_score: i32,  // 0-100
        days_before_deadline: i32,
    ) -> f32 {
        let base_score =
            (discovery_speed_score * 0.2) +
            (uniqueness_score * 0.2) +
            (quality_score * 0.2) +
            (severity_score as f32 * 0.4);
        
        // Early discovery bonus
        let early_bonus = if days_before_deadline > 30 {
            1.5  // 50% bonus if submitted 30+ days early
        } else if days_before_deadline > 14 {
            1.25  // 25% bonus if submitted 14+ days early
        } else {
            1.0
        };
        
        (base_score * early_bonus).min(100.0)
    }
}

// ================================================
// LEADERBOARD & RANKINGS
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: i32,
    pub researcher_handle: String,
    pub affiliation: Option<String>,
    pub findings_count: i32,
    pub total_score: f32,
    pub total_bounty_awarded: u64,
    pub critical_findings: i32,
    pub high_findings: i32,
    pub medium_findings: i32,
    pub average_quality_score: f32,
    pub average_uniqueness_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentLeaderboard {
    pub tournament_id: String,
    pub finalized_date: DateTime<Utc>,
    pub total_participants: i32,
    pub total_unique_findings: i32,
    pub total_bounty_distributed: u64,
    pub entries: Vec<LeaderboardEntry>,
}

// ================================================
// HONOR & FAME MULTIPLIERS
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearcherHonor {
    pub researcher_handle: String,
    
    // Recognition
    pub hall_of_adversaries_status: String,  // 'bronze', 'silver', 'gold', 'platinum'
    pub conference_talk_offer: bool,
    pub whitepaper_publication_eligible: bool,
    pub co_authorship_offer: bool,
    
    // Perks
    pub exclusive_research_access: bool,
    pub advanced_testing_environment: bool,
    pub direct_protocol_team_contact: bool,
    pub next_round_early_access: bool,
    pub annual_researcher_summit_invite: bool,
    
    // Portfolio Building
    pub public_profile_link: String,
    pub verifiable_achievement_badge: String,
    pub recommendation_letter_available: bool,
    pub speaking_opportunity_count: i32,
}

// ================================================
// SYSTEMS UNDER REVIEW (Documentation)
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemUnderReview {
    pub system_name: String,
    pub version: String,
    pub lines_of_code: i32,
    pub languages: Vec<String>,
    pub architecture_overview: String,
    pub threat_model: String,
    pub critical_invariants: Vec<String>,
    pub known_limitations: Vec<String>,
    pub security_assumptions: Vec<String>,
    
    // Access
    pub source_code_repo: String,
    pub architecture_diagram_url: String,
    pub test_environment_instructions: String,
    pub deterministic_build_instructions: String,
    pub reproduction_toolkit_url: String,
}

pub fn get_x3_validator_system() -> SystemUnderReview {
    SystemUnderReview {
        system_name: "X3 Validator & GPU Consensus Core".to_string(),
        version: "v1.0-alpha".to_string(),
        lines_of_code: 125000,
        languages: vec!["Rust".to_string(), "Golang".to_string()],
        architecture_overview: "GPU-coordinated validator network with deterministic execution and cross-chain fast relay".to_string(),
        threat_model: "Assumes up to 1/3 Byzantine validators. GPU determinism runtime assumed correct. Cryptographic primitives (EdDSA, Keccak) assumed secure.".to_string(),
        critical_invariants: vec![
            "Validator consensus finality cannot be reorged after 300ms".to_string(),
            "Funds cannot be double-spent on any chain".to_string(),
            "No single validator can unilaterally drain treasury".to_string(),
            "GPU determinism: Same input → always same output".to_string(),
        ],
        known_limitations: vec![
            "Maximum 1,000 validators before consensus overhead".to_string(),
            "Cross-chain relay requires 2-of-3 validator signatures".to_string(),
            "Post-quantum cryptography integration planned (not current)".to_string(),
        ],
        security_assumptions: vec![
            "Validator hardware is non-compromised".to_string(),
            "GPU firmware is unmodified".to_string(),
            "Network is not partitioned > 30s (triggers safety halt)".to_string(),
            "At least 2/3 validators are honest".to_string(),
        ],
        source_code_repo: "github.com/x3-infrastructure/validator-core".to_string(),
        architecture_diagram_url: "docs.x3.dev/architecture/validator-consensus".to_string(),
        test_environment_instructions: "docker-compose -f x3-testnet.yml up".to_string(),
        deterministic_build_instructions: "cargo build --release --target x86_64-unknown-linux-musl".to_string(),
        reproduction_toolkit_url: "github.com/x3-infrastructure/audit-toolkit".to_string(),
    }
}

// ================================================
// TOURNAMENT TIMELINE (Suggested Structure)
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentPhase {
    pub phase_name: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub description: String,
    pub key_milestones: Vec<String>,
}

pub fn get_tournament_timeline() -> Vec<TournamentPhase> {
    vec![
        TournamentPhase {
            phase_name: "Phase 1: Announcement & Participant Onboarding".to_string(),
            start_date: Utc::now(),
            end_date: Utc::now() + Duration::days(14),
            description: "Public announcement, documentation release, Q&A session, environment setup".to_string(),
            key_milestones: vec![
                "Day 1: Public tournament announcement + media coverage".to_string(),
                "Day 3: AMA session with X3 team".to_string(),
                "Day 7: Participant registration deadline".to_string(),
                "Day 14: All testing environments live".to_string(),
            ],
        },
        TournamentPhase {
            phase_name: "Phase 2: Active Hunt Period".to_string(),
            start_date: Utc::now() + Duration::days(14),
            end_date: Utc::now() + Duration::days(60),
            description: "Researchers actively hunt for vulnerabilities and submit findings".to_string(),
            key_milestones: vec![
                "Weekly leaderboard updates".to_string(),
                "Day 30: Halfway leaderboard announcement".to_string(),
                "Day 45: Final submissions window open (15 days remaining)".to_string(),
                "Day 60: Submission deadline (no late submissions)".to_string(),
            ],
        },
        TournamentPhase {
            phase_name: "Phase 3: Rigorous Review & Verification".to_string(),
            start_date: Utc::now() + Duration::days(60),
            end_date: Utc::now() + Duration::days(90),
            description: "X3 team + independent reviewers verify all submissions".to_string(),
            key_milestones: vec![
                "Day 61-70: Initial triage and reproducibility check".to_string(),
                "Day 71-85: Severity confirmation and impact assessment".to_string(),
                "Day 86-90: All scoring finalized, researcher notifications".to_string(),
            ],
        },
        TournamentPhase {
            phase_name: "Phase 4: Remediation & Public Results".to_string(),
            start_date: Utc::now() + Duration::days(90),
            end_date: Utc::now() + Duration::days(120),
            description: "X3 fixes issues, publishes leaderboard, awards prizes".to_string(),
            key_milestones: vec![
                "Day 91: Leaderboard finalized and published".to_string(),
                "Day 100: All prizes distributed".to_string(),
                "Day 105: Publication-approved findings released to public".to_string(),
                "Day 120: Academic papers and conference talks planned".to_string(),
            ],
        },
    ]
}

// ================================================
// PRIZE DISTRIBUTION MODEL
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrizeDistributionModel {
    pub total_pool_usd: u64,
    pub distribution: HashMap<String, u64>,  // category -> amount
    pub top_researcher_bonus: u64,  // leaderboard winner
    pub group_bonus_threshold: i32,  // if N findings of 1 type, bonus
}

pub fn get_prize_distribution(pool_usd: u64) -> PrizeDistributionModel {
    let mut distribution = HashMap::new();
    
    distribution.insert("critical".to_string(), (pool_usd as f64 * 0.40) as u64);
    distribution.insert("high".to_string(), (pool_usd as f64 * 0.30) as u64);
    distribution.insert("medium".to_string(), (pool_usd as f64 * 0.20) as u64);
    distribution.insert("low".to_string(), (pool_usd as f64 * 0.08) as u64);
    distribution.insert("informational".to_string(), (pool_usd as f64 * 0.02) as u64);
    
    PrizeDistributionModel {
        total_pool_usd: pool_usd,
        distribution,
        top_researcher_bonus: (pool_usd as f64 * 0.10) as u64,  // 10% bonus to #1
        group_bonus_threshold: 3,  // Bonus if 3+ of same type found
    }
}
