// Funding & Investor Tracking Module for X3 CRM
// Specialized models for grants, investors, and fundraising campaigns

use serde::{Deserialize, Serialize};

// ============================================
// INVESTOR PROFILES
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Investor {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub investor_type: String, // venture_capital, angel, family_office, corporate, accelerator
    pub firm_name: String,
    pub email: String,
    pub phone: String,
    pub website: String,
    pub location: String,
    pub focus_sectors: Vec<String>, // e.g., ["AI", "Climate", "FinTech"]
    pub stage_preference: String,   // seed, series_a, series_b, growth, late_stage
    pub ticket_size_min: u64,       // in USD
    pub ticket_size_max: u64,       // in USD
    pub portfolio_companies: Vec<String>,
    pub past_exits: u32,
    pub years_investing: u32,
    pub website_url: String,
    pub crunchbase_url: String,
    pub linkedin_url: String,
    pub twitter_handle: String,
    pub notes: String,
    pub rating: String, // hot, warm, cold
    pub last_contacted: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvestorInput {
    pub name: String,
    pub investor_type: String,
    pub firm_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub location: Option<String>,
    pub focus_sectors: Option<Vec<String>>,
    pub stage_preference: Option<String>,
    pub ticket_size_min: Option<u64>,
    pub ticket_size_max: Option<u64>,
    pub website_url: Option<String>,
    pub crunchbase_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
}

// ============================================
// GRANT OPPORTUNITIES
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Grant {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub provider: String,          // government, foundation, corporate, accelerator
    pub amount: u64,               // in USD
    pub currency: String,
    pub deadline: String,          // ISO date
    pub eligibility_criteria: String,
    pub focus_areas: Vec<String>,  // ["AI", "Green Energy", "Education"]
    pub location_focus: String,    // geographic restrictions
    pub url: String,
    pub status: String,            // open, closed, applied, awarded, rejected
    pub notes: String,
    pub match_score: f32,          // 0-100 how well company fits
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGrantInput {
    pub name: String,
    pub provider: String,
    pub amount: String,
    pub currency: Option<String>,
    pub deadline: String,
    pub eligibility_criteria: Option<String>,
    pub focus_areas: Option<Vec<String>>,
    pub location_focus: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
}

// ============================================
// FUNDING ROUNDS
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingRound {
    pub id: String,
    pub user_id: String,
    pub company_name: String,
    pub round_type: String,        // seed, series_a, series_b, series_c, growth, ipo
    pub target_amount: u64,
    pub raised_amount: u64,
    pub currency: String,
    pub status: String,            // planning, in_progress, closed, failed
    pub start_date: String,
    pub target_close_date: String,
    pub actual_close_date: Option<String>,
    pub lead_investor: Option<String>,
    pub follow_investors: Vec<String>,
    pub use_of_funds: String,
    pub pitch_deck_url: String,
    pub financial_model_url: String,
    pub notes: String,
    pub investors_contacted: u32,
    pub meetings_scheduled: u32,
    pub meetings_completed: u32,
    pub term_sheets_received: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFundingRoundInput {
    pub company_name: String,
    pub round_type: String,
    pub target_amount: String,
    pub currency: Option<String>,
    pub target_close_date: String,
    pub use_of_funds: Option<String>,
    pub pitch_deck_url: Option<String>,
    pub financial_model_url: Option<String>,
}

// ============================================
// INVESTOR MEETINGS & INTERACTIONS
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvestorMeeting {
    pub id: String,
    pub user_id: String,
    pub investor_id: String,
    pub investor_name: String,
    pub meeting_type: String,      // initial_call, pitch, due_diligence, follow_up
    pub meeting_date: String,
    pub duration_minutes: u32,
    pub outcome: String,           // interested, maybe, not_interested, follow_up_scheduled
    pub next_step: String,
    pub notes: String,
    pub materials_shared: Vec<String>,
    pub follow_up_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvestorMeetingInput {
    pub investor_id: String,
    pub meeting_type: String,
    pub meeting_date: String,
    pub duration_minutes: Option<u32>,
    pub outcome: Option<String>,
    pub next_step: Option<String>,
    pub notes: Option<String>,
    pub follow_up_date: Option<String>,
}

// ============================================
// FUNDING PIPELINE ANALYTICS
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingPipelineAnalytics {
    pub total_target: u64,
    pub total_raised: u64,
    pub funding_gap: i64,
    pub sources: FundingSourceBreakdown,
    pub timeline: FundingTimeline,
    pub investor_stats: InvestorStatistics,
    pub grant_stats: GrantStatistics,
    pub success_probability: f32,
    pub months_to_close: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingSourceBreakdown {
    pub venture_capital: u64,
    pub angel: u64,
    pub grants: u64,
    pub corporate: u64,
    pub accelerator: u64,
    pub other: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingTimeline {
    pub earliest_possible: String,
    pub most_likely: String,
    pub latest_possible: String,
    pub milestones: Vec<Milestone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    pub name: String,
    pub target_date: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvestorStatistics {
    pub total_investors: u32,
    pub active_conversations: u32,
    pub meetings_scheduled: u32,
    pub term_sheets: u32,
    pub avg_response_time_days: f32,
    pub conversion_rate: f32, // % of contacts that responded
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantStatistics {
    pub total_opportunities: u32,
    pub matching_criteria: u32,
    pub applied: u32,
    pub awarded: u32,
    pub total_potential: u64,
    pub application_rate: f32, // % of matches applied to
}

// ============================================
// INVESTOR MATCHING PROFILE
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvestorMatchProfile {
    pub investor_id: String,
    pub company_name: String,
    pub match_score: f32,        // 0-100
    pub reason: String,
    pub sector_alignment: f32,
    pub stage_alignment: f32,
    pub ticket_size_alignment: f32,
    pub location_alignment: f32,
    pub contact_probability: f32, // likelihood of positive response
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvestorMatchRequest {
    pub company_sectors: Vec<String>,
    pub current_stage: String,
    pub seeking_amount: u64,
    pub company_location: String,
    pub investor_type_filter: Option<Vec<String>>, // limit to certain types
}

// ============================================
// GRANT APPLICATION TRACKING
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantApplication {
    pub id: String,
    pub user_id: String,
    pub grant_id: String,
    pub grant_name: String,
    pub amount_requested: u64,
    pub status: String,           // draft, submitted, reviewing, approved, rejected, awarded
    pub submitted_date: Option<String>,
    pub decision_date: Option<String>,
    pub decision: Option<String>, // approved, rejected, pending
    pub feedback: String,
    pub next_deadline: Option<String>,
    pub application_url: String,
    pub files_attached: Vec<String>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGrantApplicationInput {
    pub grant_id: String,
    pub amount_requested: String,
    pub application_url: Option<String>,
    pub notes: Option<String>,
    pub files: Option<Vec<String>>,
}

// ============================================
// FUNDING STRATEGY & TARGETS
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingStrategy {
    pub id: String,
    pub user_id: String,
    pub target_amount: u64,
    pub currency: String,
    pub primary_sources: Vec<FundingSource>,
    pub timeline_months: u32,
    pub key_milestones: Vec<String>,
    pub success_criteria: Vec<String>,
    pub risk_factors: Vec<String>,
    pub mitigation_strategies: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingSource {
    pub source_type: String,       // venture_capital, angel, grants, corporate, etc.
    pub target_amount: u64,
    pub allocation_percentage: f32,
    pub priority: u32,             // 1 = highest
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFundingStrategyInput {
    pub target_amount: String,
    pub timeline_months: u32,
    pub primary_sources: Vec<FundingSource>,
    pub key_milestones: Option<Vec<String>>,
}

// ============================================
// INVESTOR RESEARCH & INTELLIGENCE
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvestorIntelligence {
    pub investor_id: String,
    pub recent_investments_count: u32,
    pub avg_check_size: u64,
    pub sector_focus: Vec<String>,
    pub geographic_focus: Vec<String>,
    pub stage_preferences: Vec<String>,
    pub recent_portfolio_companies: Vec<String>,
    pub known_co_investors: Vec<String>,
    pub decision_timeline_days: u32,
    pub due_diligence_duration_days: u32,
    pub founder_background: Vec<String>,
    pub media_mentions: u32,
    pub last_updated: String,
}

// ============================================
// FUNDING SUMMARY FOR DASHBOARD
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FundingSummary {
    pub total_raised: u64,
    pub current_round: Option<String>,
    pub target_remaining: u64,
    pub runway_months: f32,
    pub investor_count: u32,
    pub active_conversations: u32,
    pub next_milestone: Option<String>,
    pub success_probability: f32,
}
