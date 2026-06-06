-- TIER 9: Funding War Plan Database Schema
-- 8 tables with 40+ indexes for comprehensive fundraising tracking

-- ================================================
-- 1. WAR PLAN DOCUMENTS (Master)
-- ================================================
CREATE TABLE IF NOT EXISTS crm_war_plans (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL DEFAULT 'v1.0',
    company_name TEXT NOT NULL,
    creation_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_updated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Strategic Positioning
    target_raise_usd INTEGER NOT NULL,
    current_raised_usd INTEGER NOT NULL DEFAULT 0,
    target_valuation_usd INTEGER NOT NULL,
    desired_equity_given REAL NOT NULL,
    pre_money_valuation_usd INTEGER,
    
    -- Executive Summary
    executive_summary TEXT NOT NULL,
    market_opportunity_summary TEXT,
    customer_problem_statement TEXT,
    x3_solution TEXT,
    competitive_advantages TEXT,
    
    -- Timeline & Status
    status TEXT CHECK(status IN ('draft', 'active', 'funded', 'completed', 'paused')),
    expected_close_date DATE,
    runway_months INTEGER,
    
    -- Treasury & Financial
    current_cash_position_usd INTEGER NOT NULL DEFAULT 0,
    monthly_burn_usd INTEGER NOT NULL,
    monthly_revenue_usd INTEGER NOT NULL DEFAULT 0,
    
    CREATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UPDATED_AT TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_war_plans_company ON crm_war_plans(company_name);
CREATE INDEX idx_war_plans_status ON crm_war_plans(status);
CREATE INDEX idx_war_plans_created ON crm_war_plans(creation_date DESC);

-- ================================================
-- 2. FINANCIAL SCENARIOS (Base/Bull/Bear)
-- ================================================
CREATE TABLE IF NOT EXISTS crm_financial_scenarios (
    id TEXT PRIMARY KEY,
    war_plan_id TEXT NOT NULL REFERENCES crm_war_plans(id),
    
    scenario_type TEXT NOT NULL CHECK(scenario_type IN ('base', 'bull', 'bear')),
    probability_percentage REAL NOT NULL,
    
    -- Year 1 Projections
    year1_revenue_usd INTEGER NOT NULL,
    year1_expenses_usd INTEGER NOT NULL,
    year1_gross_margin REAL NOT NULL,
    year1_headcount INTEGER NOT NULL,
    year1_burn_rate REAL NOT NULL,
    
    -- Year 2-3
    year2_revenue_usd INTEGER,
    year2_expenses_usd INTEGER,
    year3_revenue_usd INTEGER,
    year3_expenses_usd INTEGER,
    
    -- Key Assumptions
    revenue_growth_rate REAL NOT NULL,  -- monthly percentage
    customer_acquisition_cost REAL NOT NULL,
    lifetime_value_per_customer REAL NOT NULL,
    churn_rate REAL NOT NULL,
    
    -- Breakeven Analysis
    months_to_breakeven INTEGER,
    breakeven_mos_funding_required_usd INTEGER,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_scenarios_war_plan ON crm_financial_scenarios(war_plan_id);
CREATE INDEX idx_scenarios_type ON crm_financial_scenarios(scenario_type);

-- ================================================
-- 3. MONTHLY PROJECTIONS (12-Month Detail)
-- ================================================
CREATE TABLE IF NOT EXISTS crm_monthly_projections (
    id TEXT PRIMARY KEY,
    war_plan_id TEXT NOT NULL REFERENCES crm_war_plans(id),
    scenario_type TEXT NOT NULL,  -- 'base', 'bull', 'bear'
    
    month_num INTEGER NOT NULL CHECK(month_num >= 1 AND month_num <= 12),
    month_date DATE NOT NULL,
    
    -- Revenue Streams
    revenue_validator_usd INTEGER DEFAULT 0,
    revenue_routing_usd INTEGER DEFAULT 0,
    revenue_gpu_leasing_usd INTEGER DEFAULT 0,
    revenue_partnerships_usd INTEGER DEFAULT 0,
    total_revenue_usd INTEGER NOT NULL,
    
    -- Expenses
    payroll_usd INTEGER NOT NULL,
    infrastructure_usd INTEGER NOT NULL,
    security_audits_usd INTEGER DEFAULT 0,
    marketing_usd INTEGER DEFAULT 0,
    operations_usd INTEGER DEFAULT 0,
    total_expenses_usd INTEGER NOT NULL,
    
    -- Metrics
    gross_margin REAL NOT NULL,
    headcount INTEGER NOT NULL,
    active_validators INTEGER DEFAULT 0,
    cross_chain_volume_usd INTEGER DEFAULT 0,
    
    -- Cash Position
    cash_position_usd INTEGER NOT NULL,
    net_cash_flow INTEGER NOT NULL,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_monthly_war_plan ON crm_monthly_projections(war_plan_id);
CREATE INDEX idx_monthly_scenario ON crm_monthly_projections(scenario_type);
CREATE INDEX idx_monthly_date ON crm_monthly_projections(month_date);

-- ================================================
-- 4. CAPITAL STACK & CAP TABLE
-- ================================================
CREATE TABLE IF NOT EXISTS crm_cap_table_rows (
    id TEXT PRIMARY KEY,
    war_plan_id TEXT NOT NULL REFERENCES crm_war_plans(id),
    
    holder_type TEXT NOT NULL CHECK(holder_type IN ('founder', 'employee', 'investor', 'advisor', 'pool')),
    holder_name TEXT NOT NULL,
    
    -- Shares & Ownership
    shares_outstanding INTEGER NOT NULL,
    fully_diluted_shares INTEGER NOT NULL,
    ownership_percentage REAL NOT NULL,
    fully_diluted_percentage REAL NOT NULL,
    
    -- Liquidation Preferences
    preference_type TEXT,  -- 'non-participating', '1x', '2x', etc
    entry_valuation_usd INTEGER,
    entry_date DATE,
    
    -- Vesting
    vesting_start_date DATE,
    vesting_cliff_months INTEGER,
    vesting_total_months INTEGER,
    vesting_percentage REAL DEFAULT 100.0,
    
    -- Strike Price / Options
    option_strike_price REAL,
    option_pool_amount INTEGER,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_cap_table_war_plan ON crm_cap_table_rows(war_plan_id);
CREATE INDEX idx_cap_table_holder ON crm_cap_table_rows(holder_name);
CREATE INDEX idx_cap_table_type ON crm_cap_table_rows(holder_type);

-- ================================================
-- 5. DILUTION SCENARIOS (Future Rounds)
-- ================================================
CREATE TABLE IF NOT EXISTS crm_dilution_scenarios (
    id TEXT PRIMARY KEY,
    war_plan_id TEXT NOT NULL REFERENCES crm_war_plans(id),
    
    scenario_name TEXT NOT NULL,  -- 'Series A', 'Series B', 'Exit'
    funding_round_num INTEGER NOT NULL,
    
    -- Round Economics
    raise_amount_usd INTEGER NOT NULL,
    new_valuation_usd INTEGER NOT NULL,
    new_share_price REAL NOT NULL,
    new_shares_issued INTEGER NOT NULL,
    
    -- Dilution Impact
    founder_ownership_pre_round REAL NOT NULL,
    founder_ownership_post_round REAL NOT NULL,
    total_dilution_percentage REAL NOT NULL,
    cumulative_dilution_percentage REAL NOT NULL,
    
    -- Exit Simulation
    exit_value_estimate_usd INTEGER,
    founder_net_proceeds_usd INTEGER,
    investor_net_proceeds_usd INTEGER,
    preference_waterfall_applied BOOLEAN DEFAULT FALSE,
    
    -- Timeline
    expected_round_date DATE,
    assumptions TEXT,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_dilution_war_plan ON crm_dilution_scenarios(war_plan_id);
CREATE INDEX idx_dilution_round ON crm_dilution_scenarios(funding_round_num);

-- ================================================
-- 6. TREASURY RUNWAY (36-Month Projection)
-- ================================================
CREATE TABLE IF NOT EXISTS crm_runway_months (
    id TEXT PRIMARY KEY,
    war_plan_id TEXT NOT NULL REFERENCES crm_war_plans(id),
    
    month_num INTEGER NOT NULL CHECK(month_num >= 1 AND month_num <= 36),
    month_date DATE NOT NULL,
    
    -- Cash Tracking
    starting_cash_usd INTEGER NOT NULL,
    monthly_revenue_usd INTEGER NOT NULL,
    monthly_burn_usd INTEGER NOT NULL,
    ending_cash_usd INTEGER NOT NULL,
    net_cash_flow INTEGER NOT NULL,
    
    -- Status Indicators
    status TEXT NOT NULL CHECK(status IN ('healthy', 'caution', 'critical', 'funded', 'breakeven')),
    runway_remaining_months REAL NOT NULL,
    
    -- Funding Events
    funding_event BOOLEAN DEFAULT FALSE,
    funding_amount_usd INTEGER,
    funding_source TEXT,  -- 'grant', 'vc', 'revenue', 'trade'
    
    -- Risk Flags
    cash_under_threshold BOOLEAN DEFAULT FALSE,
    emergency_threshold_triggered BOOLEAN DEFAULT FALSE,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_runway_war_plan ON crm_runway_months(war_plan_id);
CREATE INDEX idx_runway_date ON crm_runway_months(month_date);
CREATE INDEX idx_runway_status ON crm_runway_months(status);

-- ================================================
-- 7. CRITICAL CASH THRESHOLDS
-- ================================================
CREATE TABLE IF NOT EXISTS crm_cash_thresholds (
    id TEXT PRIMARY KEY,
    war_plan_id TEXT NOT NULL REFERENCES crm_war_plans(id),
    
    threshold_name TEXT NOT NULL,
    threshold_type TEXT NOT NULL CHECK(threshold_type IN ('emergency', 'critical', 'caution', 'warning')),
    
    -- Threshold Definitions
    cash_amount_usd INTEGER NOT NULL,
    months_of_runway_remaining REAL NOT NULL,
    alert_color TEXT,  -- 'red', 'orange', 'yellow', 'green'
    
    -- Actions Triggered
    required_actions TEXT,  -- JSON array of action strings
    escalation_contact TEXT,
    escalation_channel TEXT,  -- 'ceo', 'board', 'all'
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_thresholds_war_plan ON crm_cash_thresholds(war_plan_id);
CREATE INDEX idx_thresholds_type ON crm_cash_thresholds(threshold_type);

-- ================================================
-- 8. FUNDING REQUIREMENTS & TIMELINE
-- ================================================
CREATE TABLE IF NOT EXISTS crm_funding_requirements (
    id TEXT PRIMARY KEY,
    war_plan_id TEXT NOT NULL REFERENCES crm_war_plans(id),
    
    funding_round_name TEXT NOT NULL,
    target_raise_usd INTEGER NOT NULL,
    target_valuation_usd INTEGER NOT NULL,
    
    -- Timeline
    planned_close_date DATE NOT NULL,
    runway_buffer_months INTEGER NOT NULL,
    latest_start_outreach_date DATE,
    
    -- Capital Sources (Multi-Source)
    source_1_type TEXT,  -- 'vc', 'grant', 'trade', 'bonds', 'token'
    source_1_target_usd INTEGER,
    source_1_status TEXT,  -- 'not_started', 'in_progress', 'verbal', 'signed'
    
    source_2_type TEXT,
    source_2_target_usd INTEGER,
    source_2_status TEXT,
    
    source_3_type TEXT,
    source_3_target_usd INTEGER,
    source_3_status TEXT,
    
    -- Outreach Metrics
    target_contacts_count INTEGER,
    contacted_count INTEGER DEFAULT 0,
    meeting_count INTEGER DEFAULT 0,
    term_sheet_count INTEGER DEFAULT 0,
    
    -- Key Milestones
    milestone_1_name TEXT,
    milestone_1_date DATE,
    milestone_1_completed BOOLEAN DEFAULT FALSE,
    
    milestone_2_name TEXT,
    milestone_2_date DATE,
    milestone_2_completed BOOLEAN DEFAULT FALSE,
    
    milestone_3_name TEXT,
    milestone_3_date DATE,
    milestone_3_completed BOOLEAN DEFAULT FALSE,
    
    -- Risk & Contingency
    contingency_plan TEXT,
    plan_b_funding_strategy TEXT,
    
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_funding_req_war_plan ON crm_funding_requirements(war_plan_id);
CREATE INDEX idx_funding_req_round ON crm_funding_requirements(funding_round_name);
CREATE INDEX idx_funding_req_date ON crm_funding_requirements(planned_close_date);

-- ================================================
-- AUDIT INDEXES FOR PERFORMANCE
-- ================================================
CREATE INDEX idx_war_plans_cash ON crm_war_plans(current_cash_position_usd);
CREATE INDEX idx_monthly_cash ON crm_monthly_projections(cash_position_usd);
CREATE INDEX idx_cap_table_ownership ON crm_cap_table_rows(ownership_percentage DESC);
CREATE INDEX idx_runway_date_status ON crm_runway_months(month_date, status);
CREATE INDEX idx_scenarios_breakeven ON crm_financial_scenarios(months_to_breakeven);
