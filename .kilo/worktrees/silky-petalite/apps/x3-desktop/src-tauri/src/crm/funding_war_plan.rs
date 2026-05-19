// TIER 8: Comprehensive Funding War Plan v1.0
// Battle-ready fundraising strategy with 12-month projections,
// capital stack simulation, and industry-specific outreach

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use std::collections::HashMap;

// ================================================
// FUNDING WAR PLAN DOCUMENT
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingWarPlan {
    pub id: String,
    pub version: String,  // "v1.0"
    pub company_name: String,
    pub creation_date: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    
    // Strategic Overview
    pub executive_summary: WarPlanExecutiveSummary,
    pub market_context: MarketContext,
    pub product_differentiation: Vec<String>,
    pub competitive_advantages: Vec<CompetitiveAdvantage>,
    
    // Fundraising Strategy
    pub target_raise_usd: u64,
    pub current_raised_usd: u64,
    pub target_valuation_usd: u64,
    pub desired_equity_given: f32,  // percentage 0-100
    pub pre_money_valuation_usd: Option<u64>,
    
    // Timeline
    pub fundraising_timeline: FundraisingTimeline,
    pub key_milestones: Vec<Milestone>,
    pub critical_path: Vec<CriticalPathItem>,
    
    // Financial Models
    pub financial_projection_12m: FinancialProjection12m,
    pub cap_table_simulation: CapTableSimulation,
    pub treasury_runway_analysis: TreasuryRunwayAnalysis,
    
    // Investor Strategy
    pub investor_strategy: InvestorStrategy,
    pub outreach_campaigns: Vec<OutreachCampaign>,
    
    // Risk Analysis
    pub key_risks: Vec<RiskItem>,
    pub mitigation_strategies: Vec<MitigationStrategy>,
    
    // Document Status
    pub status: WarPlanStatus,  // "draft", "final", "approved", "archived"
    pub confidentiality_level: String,  // "confidential", "restricted", "public"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarPlanExecutiveSummary {
    pub headline: String,  // "Closing Series A in Q2 2026"
    pub business_focus: String,
    pub market_size: String,  // "TAM: $15B"
    pub growth_stage: String,  // "seed", "series_a", "series_b", etc.
    pub key_metrics: HashMap<String, String>,  // e.g., {"MRR": "$50K", "CAC": "$2.5K"}
    pub narrative: String,  // 2-3 paragraph business narrative
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketContext {
    pub total_addressable_market: String,
    pub serviceable_addressable_market: String,
    pub market_growth_rate: f32,  // percentage
    pub market_trends: Vec<String>,
    pub regulatory_environment: String,
    pub competitive_landscape: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitiveAdvantage {
    pub advantage: String,
    pub defensibility: String,  // "patent", "moat", "speed", "network_effect", "cost"
    pub timeline_to_copy: String,  // "months", "years", "nearly_impossible"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundraisingTimeline {
    pub phase_1_intro_date: DateTime<Utc>,
    pub phase_1_deadline: DateTime<Utc>,  // Introduce to top investors
    pub phase_2_meetings_start: DateTime<Utc>,
    pub phase_2_deadline: DateTime<Utc>,  // Key meetings & data room
    pub phase_3_final_close_target: DateTime<Utc>,  // Final commitments
    pub total_expected_duration_weeks: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,
    pub target_date: DateTime<Utc>,
    pub description: String,
    pub importance: String,  // "critical", "high", "medium"
    pub owner: String,  // Person responsible
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPathItem {
    pub sequence: i32,
    pub task: String,
    pub duration_days: i32,
    pub dependencies: Vec<String>,
    pub owner: String,
    pub completion_status: f32,  // 0-100 percentage
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorStrategy {
    pub target_investor_types: Vec<InvestorTypeProfile>,
    pub lead_investor_target: Option<String>,  // Desired lead VC
    pub co_investor_strategy: String,  // "broad_syndicate" or "tight_round"
    pub geographic_focus: Vec<String>,  // ["San Francisco", "New York", "Europe"]
    pub total_investor_meetings_target: i32,
    pub expected_conversion_rate: f32,  // 0-1.0
    pub average_check_size_usd: u64,
    pub minimum_check_size_usd: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorTypeProfile {
    pub investor_type: String,  // "tier_1_vc", "tier_2_vc", "angels", "corporate", "family_offices"
    pub target_count: i32,
    pub average_check_size_usd: u64,
    pub allocation_percentage: f32,  // of total raise
    pub key_decision_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachCampaign {
    pub name: String,
    pub objective: String,  // "find_leads", "warm_intros", "meetings", "closing"
    pub target_investor_type: String,
    pub vertical_focus: Option<String>,  // "cloud", "ai", "quantum", "enterprise"
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub target_contacts: i32,
    pub success_metrics: Vec<String>,  // ["10 meetings", "$2M soft commitments"]
    pub campaign_resources: Vec<String>,  // ["pitch_deck", "one_pager", "data_room"]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskItem {
    pub risk: String,
    pub impact: String,  // "critical", "high", "medium", "low"
    pub probability: f32,  // 0-1.0
    pub detection_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationStrategy {
    pub risk: String,
    pub mitigation: String,
    pub owner: String,
    pub timeline: String,
}

pub type WarPlanStatus = String;

// ================================================
// 12-MONTH FINANCIAL PROJECTION
// ================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialProjection12m {
    pub projection_start_date: DateTime<Utc>,
    pub base_case: FinancialScenario,
    pub bull_case: FinancialScenario,
    pub bear_case: FinancialScenario,
    pub monthly_projections: Vec<MonthlyProjection>,
    pub key_assumptions: Vec<FinancialAssumption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialScenario {
    pub name: String,  // "Base", "Bull", "Bear"
    pub probability: f32,  // 0-1.0
    pub year_1_revenue: u64,
    pub year_1_gross_margin: f32,
    pub year_1_burn_rate: i64,  // Can be negative (profit)
    pub runway_months: Option<i32>,
    pub break_even_month: Option<i32>,
    pub key_drivers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyProjection {
    pub month_num: i32,  // 1-12
    pub month_date: DateTime<Utc>,
    pub revenue_usd: u64,
    pub expenses_usd: u64,
    pub gross_margin: f32,
    pub headcount: i32,
    pub cash_position_usd: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialAssumption {
    pub assumption: String,  // "10% MoM growth", "80% CAC payback in 12m"
    pub impact: String,  // How it affects projections
    pub confidence: f32,  // 0-1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapTableSimulation {
    pub current_shares_outstanding: u64,
    pub founder_equity: FounderEquityAllocation,
    pub employee_pool_percentage: f32,
    pub existing_investors: Vec<ExistingInvestor>,
    pub new_round_structure: NewRoundSimulation,
    pub pro_forma_cap_table: Vec<CapTableRow>,
    pub dilution_scenarios: Vec<DilutionScenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FounderEquityAllocation {
    pub founder_1: (String, f32),  // (Name, percentage)
    pub founder_2: Option<(String, f32)>,
    pub founder_3: Option<(String, f32)>,
    pub advisor_pool_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistingInvestor {
    pub investor_name: String,
    pub shares_owned: u64,
    pub percentage_ownership: f32,
    pub preferred_shares_type: String,  // "pref_a", "pref_b", "common"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRoundSimulation {
    pub round_name: String,  // "Series A"
    pub raise_amount_usd: u64,
    pub valuation_usd: u64,  // Post-money
    pub shares_issued: u64,
    pub price_per_share: f32,
    pub investor_ownership_percentage: f32,
    pub pro_rata_investor_list: Vec<ProRataInvestor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProRataInvestor {
    pub investor_name: String,
    pub pro_rata_shares: u64,
    pub pro_rata_percentage: f32,
    pub check_size_usd: u64,
    pub allocation_status: String,  // "committed", "pending", "declined"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapTableRow {
    pub holder_name: String,
    pub share_type: String,
    pub share_count: u64,
    pub percentage_post_new_round: f32,
    pub liquidation_preference: Option<String>,  // "1x", "2x non-participating", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DilutionScenario {
    pub scenario_name: String,
    pub additional_rounds: i32,  // How many future rounds before exit
    pub final_founder_ownership_at_exit: f32,
    pub total_dilution_percentage: f32,
    pub exit_value_estimate: u64,
    pub founder_net_proceeds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryRunwayAnalysis {
    pub current_cash_position_usd: u64,
    pub monthly_burn_rate_usd: i64,  // Can be negative if profitable
    pub monthly_revenue_usd: u64,
    pub contribution_margin_usd: i64,  // Revenue minus variable costs
    
    // Runway Calculations
    pub months_of_runway: f32,  // Until cash runs out (current cash / burn rate)
    pub cash_flow_break_even_month: Option<i32>,  // When monthly burn becomes positive
    pub net_burn_rate_usd: i64,  // revenue - burn (can be negative = profitable)
    
    // 36-Month Projection
    pub runway_36m_projection: Vec<RunwayMonth>,
    pub critical_cash_thresholds: Vec<CashThreshold>,
    pub funding_requirement_timeline: Vec<FundingRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunwayMonth {
    pub month_num: i32,  // 1-36
    pub month_date: DateTime<Utc>,
    pub cash_balance_usd: u64,
    pub monthly_burn_usd: i64,
    pub monthly_revenue_usd: u64,
    pub headcount: i32,
    pub status: String,  // "healthy", "caution", "critical", "funded"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashThreshold {
    pub threshold_name: String,  // "Caution Zone", "Critical", "Runway Out"
    pub threshold_usd: u64,
    pub months_until_threshold: f32,
    pub recommended_action: String,  // "Accelerate fundraise", "Cut costs", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRequirement {
    pub round_number: i32,
    pub estimated_raise_date: DateTime<Utc>,
    pub minimum_raise_usd: u64,  // To reach next milestone
    pub target_raise_usd: u64,
    pub optimal_raise_usd: u64,  // For 24-month runway
    pub planned_use_of_proceeds: Vec<UseOfProceeds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseOfProceeds {
    pub category: String,  // "product", "sales", "operations", "team"
    pub percentage_allocation: f32,
    pub amount_usd: u64,
    pub timeline: String,
}

// ================================================
// HELPER FUNCTIONS
// ================================================

impl FundingWarPlan {
    pub fn new(company_name: String) -> Self {
        FundingWarPlan {
            id: Uuid::new_v4().to_string(),
            version: "v1.0".to_string(),
            company_name,
            creation_date: Utc::now(),
            last_updated: Utc::now(),
            executive_summary: WarPlanExecutiveSummary {
                headline: "Fundraising Strategy".to_string(),
                business_focus: "".to_string(),
                market_size: "".to_string(),
                growth_stage: "seed".to_string(),
                key_metrics: HashMap::new(),
                narrative: "".to_string(),
            },
            market_context: MarketContext {
                total_addressable_market: "".to_string(),
                serviceable_addressable_market: "".to_string(),
                market_growth_rate: 0.0,
                market_trends: vec![],
                regulatory_environment: "".to_string(),
                competitive_landscape: "".to_string(),
            },
            product_differentiation: vec![],
            competitive_advantages: vec![],
            target_raise_usd: 0,
            current_raised_usd: 0,
            target_valuation_usd: 0,
            desired_equity_given: 0.0,
            pre_money_valuation_usd: None,
            fundraising_timeline: FundraisingTimeline {
                phase_1_intro_date: Utc::now(),
                phase_1_deadline: Utc::now() + Duration::weeks(4),
                phase_2_meetings_start: Utc::now() + Duration::weeks(4),
                phase_2_deadline: Utc::now() + Duration::weeks(12),
                phase_3_final_close_target: Utc::now() + Duration::weeks(16),
                total_expected_duration_weeks: 16,
            },
            key_milestones: vec![],
            critical_path: vec![],
            financial_projection_12m: FinancialProjection12m {
                projection_start_date: Utc::now(),
                base_case: FinancialScenario {
                    name: "Base".to_string(),
                    probability: 0.6,
                    year_1_revenue: 0,
                    year_1_gross_margin: 0.7,
                    year_1_burn_rate: -500000,  // $500K/month burn
                    runway_months: None,
                    break_even_month: None,
                    key_drivers: HashMap::new(),
                },
                bull_case: FinancialScenario {
                    name: "Bull".to_string(),
                    probability: 0.2,
                    year_1_revenue: 0,
                    year_1_gross_margin: 0.8,
                    year_1_burn_rate: -300000,
                    runway_months: None,
                    break_even_month: None,
                    key_drivers: HashMap::new(),
                },
                bear_case: FinancialScenario {
                    name: "Bear".to_string(),
                    probability: 0.2,
                    year_1_revenue: 0,
                    year_1_gross_margin: 0.5,
                    year_1_burn_rate: -750000,
                    runway_months: None,
                    break_even_month: None,
                    key_drivers: HashMap::new(),
                },
                monthly_projections: vec![],
                key_assumptions: vec![],
            },
            cap_table_simulation: CapTableSimulation {
                current_shares_outstanding: 10000000,  // 10M shares
                founder_equity: FounderEquityAllocation {
                    founder_1: ("Founder 1".to_string(), 40.0),
                    founder_2: Some(("Founder 2".to_string(), 30.0)),
                    founder_3: Some(("Founder 3".to_string(), 20.0)),
                    advisor_pool_percentage: 10.0,
                },
                employee_pool_percentage: 0.0,
                existing_investors: vec![],
                new_round_structure: NewRoundSimulation {
                    round_name: "Series A".to_string(),
                    raise_amount_usd: 5000000,
                    valuation_usd: 20000000,
                    shares_issued: 0,
                    price_per_share: 0.0,
                    investor_ownership_percentage: 0.0,
                    pro_rata_investor_list: vec![],
                },
                pro_forma_cap_table: vec![],
                dilution_scenarios: vec![],
            },
            treasury_runway_analysis: TreasuryRunwayAnalysis {
                current_cash_position_usd: 1000000,  // Start with $1M
                monthly_burn_rate_usd: -500000,  // Burn $500K/month
                monthly_revenue_usd: 0,
                contribution_margin_usd: -500000,
                months_of_runway: 2.0,
                cash_flow_break_even_month: None,
                net_burn_rate_usd: -500000,
                runway_36m_projection: vec![],
                critical_cash_thresholds: vec![],
                funding_requirement_timeline: vec![],
            },
            investor_strategy: InvestorStrategy {
                target_investor_types: vec![],
                lead_investor_target: None,
                co_investor_strategy: "broad_syndicate".to_string(),
                geographic_focus: vec![],
                total_investor_meetings_target: 30,
                expected_conversion_rate: 0.15,  // 15% of meetings → check
                average_check_size_usd: 250000,
                minimum_check_size_usd: 50000,
            },
            outreach_campaigns: vec![],
            key_risks: vec![],
            mitigation_strategies: vec![],
            status: "draft".to_string(),
            confidentiality_level: "confidential".to_string(),
        }
    }

    pub fn calculate_runway(&self) -> f32 {
        if self.treasury_runway_analysis.monthly_burn_rate_usd == 0 {
            0.0
        } else {
            self.treasury_runway_analysis.current_cash_position_usd as f32
                / (self.treasury_runway_analysis.monthly_burn_rate_usd.abs() as f32)
        }
    }

    pub fn get_war_plan_status(&self) -> String {
        let runway = self.calculate_runway();
        if runway < 3.0 {
            "CRITICAL: < 3 months runway, fundraise immediately".to_string()
        } else if runway < 6.0 {
            "URGENT: 3-6 months runway, accelerate closings".to_string()
        } else if runway < 12.0 {
            "ACTIVE: 6-12 months runway, maintain momentum".to_string()
        } else {
            "HEALTHY: 12+ months runway, optimize growth".to_string()
        }
    }
}

// ================================================
// TAURI COMMANDS (STUB SIGNATURES)
// ================================================

#[tauri::command]
pub fn crm_generate_funding_war_plan(
    company_name: String,
    target_raise_usd: u64,
    target_valuation_usd: u64,
) -> String {
    // Creates new FundingWarPlan and returns ID
    let plan = FundingWarPlan::new(company_name);
    plan.id
}

#[tauri::command]
pub fn crm_get_funding_war_plan(plan_id: String) -> Option<FundingWarPlan> {
    // Retrieves FundingWarPlan from database
    None
}

#[tauri::command]
pub fn crm_export_war_plan_as_pdf(plan_id: String) -> Result<String, String> {
    // Exports war plan as battle-ready PDF document
    Ok("war_plan.pdf".to_string())
}

#[tauri::command]
pub fn crm_calculate_financial_projection(
    current_revenue_usd: u64,
    monthly_burn_rate_usd: i64,
    growth_rate_percent: f32,
) -> FinancialProjection12m {
    // Calculates 12-month projection with scenarios
    FinancialProjection12m {
        projection_start_date: Utc::now(),
        base_case: FinancialScenario {
            name: "Base".to_string(),
            probability: 0.6,
            year_1_revenue: 0,
            year_1_gross_margin: 0.7,
            year_1_burn_rate: monthly_burn_rate_usd,
            runway_months: None,
            break_even_month: None,
            key_drivers: HashMap::new(),
        },
        bull_case: FinancialScenario {
            name: "Bull".to_string(),
            probability: 0.2,
            year_1_revenue: 0,
            year_1_gross_margin: 0.8,
            year_1_burn_rate: (monthly_burn_rate_usd as f32 * 0.6) as i64,
            runway_months: None,
            break_even_month: None,
            key_drivers: HashMap::new(),
        },
        bear_case: FinancialScenario {
            name: "Bear".to_string(),
            probability: 0.2,
            year_1_revenue: 0,
            year_1_gross_margin: 0.5,
            year_1_burn_rate: (monthly_burn_rate_usd as f32 * 1.5) as i64,
            runway_months: None,
            break_even_month: None,
            key_drivers: HashMap::new(),
        },
        monthly_projections: vec![],
        key_assumptions: vec![],
    }
}

#[tauri::command]
pub fn crm_simulate_cap_table(
    current_shares: u64,
    new_raise_usd: u64,
    valuation_usd: u64,
) -> CapTableSimulation {
    // Simulates cap table dilution and equity distribution
    CapTableSimulation {
        current_shares_outstanding: current_shares,
        founder_equity: FounderEquityAllocation {
            founder_1: ("Founder 1".to_string(), 40.0),
            founder_2: Some(("Founder 2".to_string(), 30.0)),
            founder_3: None,
            advisor_pool_percentage: 10.0,
        },
        employee_pool_percentage: 10.0,
        existing_investors: vec![],
        new_round_structure: NewRoundSimulation {
            round_name: "Series A".to_string(),
            raise_amount_usd: new_raise_usd,
            valuation_usd,
            shares_issued: (new_raise_usd as f64 / (valuation_usd as f64 / current_shares as f64)) as u64,
            price_per_share: (valuation_usd as f64 / current_shares as f64) as f32,
            investor_ownership_percentage: (new_raise_usd as f32 / valuation_usd as f32),
            pro_rata_investor_list: vec![],
        },
        pro_forma_cap_table: vec![],
        dilution_scenarios: vec![],
    }
}

#[tauri::command]
pub fn crm_calculate_treasury_runway(
    current_cash_usd: u64,
    monthly_burn_usd: i64,
    monthly_revenue_usd: u64,
) -> TreasuryRunwayAnalysis {
    // Calculates 36-month runway with thresholds and funding requirements
    let net_burn = monthly_burn_usd - (monthly_revenue_usd as i64);
    let months_of_runway = if net_burn == 0 {
        36.0
    } else {
        (current_cash_usd as f32 / net_burn.abs() as f32).min(36.0)
    };

    TreasuryRunwayAnalysis {
        current_cash_position_usd: current_cash_usd,
        monthly_burn_rate_usd: monthly_burn_usd,
        monthly_revenue_usd,
        contribution_margin_usd: net_burn,
        months_of_runway,
        cash_flow_break_even_month: None,
        net_burn_rate_usd: net_burn,
        runway_36m_projection: vec![],
        critical_cash_thresholds: vec![],
        funding_requirement_timeline: vec![],
    }
}
