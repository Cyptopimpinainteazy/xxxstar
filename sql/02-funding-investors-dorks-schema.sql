-- Database Schema: Funding, Investors, Grants, and Google Dorks Search
-- Models funding pipeline, investor discovery, grant opportunities, and search campaigns
-- This file is applied automatically on app startup if tables don't exist

-- ================================================
-- INVESTORS TABLE
-- Primary tracking for individual investors
-- ================================================
CREATE TABLE IF NOT EXISTS crm_investors (
  id TEXT PRIMARY KEY,
  firm_name TEXT NOT NULL,
  investor_type TEXT NOT NULL, -- 'venture_capital', 'angel', 'family_office', 'accelerator', 'corporate'
  first_name TEXT,
  last_name TEXT,
  email TEXT UNIQUE,
  phone TEXT,
  website TEXT,
  location TEXT,
  linkedin_url TEXT,
  twitter_handle TEXT,
  
  -- Investment Preferences
  focus_sectors TEXT, -- JSON array: ["AI", "SaaS", "FinTech"]
  stage_preference TEXT, -- 'seed', 'seed_to_series_a', 'series_a_plus', 'all'
  ticket_size_min INTEGER, -- in USD
  ticket_size_max INTEGER, -- in USD
  
  -- Background
  years_investing INTEGER,
  portfolio_count INTEGER DEFAULT 0,
  average_check_size INTEGER,
  total_invested INTEGER, -- cumulative across portfolio
  
  -- Status & Rating
  status TEXT DEFAULT 'new', -- 'new', 'warm', 'hot', 'pitched', 'rejected'
  rating TEXT DEFAULT 'cold', -- 'cold', 'warm', 'hot'
  last_contact_date DATETIME,
  
  -- Metadata
  source TEXT, -- 'crunchbase', 'angellist', 'linkedin', 'dorks_search', 'manual'
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  
  FOREIGN KEY (id) REFERENCES crm_contacts(id) ON DELETE CASCADE
);

CREATE INDEX idx_investors_firm ON crm_investors(firm_name);
CREATE INDEX idx_investors_type ON crm_investors(investor_type);
CREATE INDEX idx_investors_sectors ON crm_investors(focus_sectors);
CREATE INDEX idx_investors_status ON crm_investors(status);
CREATE INDEX idx_investors_rating ON crm_investors(rating);
CREATE INDEX idx_investors_location ON crm_investors(location);
CREATE INDEX idx_investors_created ON crm_investors(created_at);

-- ================================================
-- GRANTS TABLE
-- Track available grant opportunities
-- ================================================
CREATE TABLE IF NOT EXISTS crm_grants (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  provider TEXT NOT NULL, -- 'grants.gov', 'nsf', 'foundation', 'corporate', 'accelerator'
  provider_name TEXT,
  award_type TEXT, -- 'grant', 'loan', 'equity', 'prize'
  
  amount_min_usd INTEGER,
  amount_max_usd INTEGER,
  
  deadline_date DATE NOT NULL,
  announcement_date DATE,
  result_date DATE,
  
  -- Eligibility
  eligibility_criteria TEXT, -- JSON object with requirements
  company_stage TEXT, -- 'pre-seed', 'seed', 'series_a', 'public'
  focus_areas TEXT, -- JSON array: ["AI", "Climate"]
  location_focus TEXT, -- 'USA', 'Global', specific region
  company_size_max INTEGER, -- max employees
  revenue_max_usd INTEGER,
  
  -- Application Status
  application_status TEXT DEFAULT 'open', -- 'open', 'applied', 'shortlisted', 'awarded', 'rejected'
  match_score REAL DEFAULT 0.0, -- 0-100, our relevance
  submitted_date DATETIME,
  result_notification_date DATETIME,
  
  -- Details
  description TEXT,
  application_url TEXT,
  documentation_url TEXT,
  contact_person TEXT,
  contact_email TEXT,
  contact_phone TEXT,
  
  source TEXT DEFAULT 'dorks_search', -- 'dorks_search', 'manual', 'grant_site'
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_grants_provider ON crm_grants(provider);
CREATE INDEX idx_grants_deadline ON crm_grants(deadline_date);
CREATE INDEX idx_grants_amount ON crm_grants(amount_min_usd, amount_max_usd);
CREATE INDEX idx_grants_focus ON crm_grants(focus_areas);
CREATE INDEX idx_grants_status ON crm_grants(application_status);
CREATE INDEX idx_grants_match ON crm_grants(match_score);

-- ================================================
-- FUNDING ROUNDS TABLE
-- Track company funding history and targets
-- ================================================
CREATE TABLE IF NOT EXISTS crm_funding_rounds (
  id TEXT PRIMARY KEY,
  round_name TEXT NOT NULL, -- "Seed", "Series A", "Series B", etc.
  round_type TEXT NOT NULL, -- 'pre_seed', 'seed', 'series_a', 'series_b', etc.
  status TEXT DEFAULT 'planning', -- 'planning', 'active', 'closed', 'failed'
  
  target_amount_usd INTEGER NOT NULL,
  target_date DATE,
  raised_amount_usd INTEGER DEFAULT 0,
  actual_close_date DATE,
  
  -- Investor Tracking
  lead_investor TEXT, -- investor_id reference
  lead_investor_name TEXT,
  follow_investors TEXT, -- JSON array of investor_ids
  investors_count INTEGER DEFAULT 0,
  
  -- Terms
  valuation_usd INTEGER,
  valuation_cap_usd INTEGER,
  discount_rate REAL,
  
  -- Documents
  fundraising_deck_url TEXT,
  pitch_deck_url TEXT,
  data_room_url TEXT,
  
  -- Timeline
  start_date DATETIME,
  end_date DATETIME,
  
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_funding_rounds_status ON crm_funding_rounds(status);
CREATE INDEX idx_funding_rounds_target_date ON crm_funding_rounds(target_date);
CREATE INDEX idx_funding_rounds_lead ON crm_funding_rounds(lead_investor);

-- ================================================
-- INVESTOR MEETINGS TABLE
-- Detailed tracking of investor interactions
-- ================================================
CREATE TABLE IF NOT EXISTS crm_investor_meetings (
  id TEXT PRIMARY KEY,
  investor_id TEXT NOT NULL,
  funding_round_id TEXT,
  
  meeting_type TEXT NOT NULL, -- 'intro', 'pitch', 'due_diligence', 'follow_up', 'closing'
  meeting_date DATETIME NOT NULL,
  duration_minutes INTEGER,
  location_type TEXT, -- 'phone', 'video', 'in_person'
  
  attendees_text TEXT, -- JSON: names of attendees
  agenda TEXT,
  notes TEXT,
  materials_shared TEXT, -- JSON: [deck, financials, etc.]
  
  -- Outcome
  outcome TEXT, -- 'interested', 'maybe', 'not_interested', 'pass', 'committed'
  next_steps TEXT,
  follow_up_date DATETIME,
  probability_percentage INTEGER, -- 0-100
  
  -- Sentiment
  overall_sentiment TEXT, -- 'positive', 'neutral', 'negative'
  key_concerns TEXT,
  
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  
  FOREIGN KEY (investor_id) REFERENCES crm_investors(id),
  FOREIGN KEY (funding_round_id) REFERENCES crm_funding_rounds(id)
);

CREATE INDEX idx_meetings_investor ON crm_investor_meetings(investor_id);
CREATE INDEX idx_meetings_date ON crm_investor_meetings(meeting_date);
CREATE INDEX idx_meetings_outcome ON crm_investor_meetings(outcome);
CREATE INDEX idx_meetings_funding_round ON crm_investor_meetings(funding_round_id);

-- ================================================
-- FUNDING ANALYTICS TABLE
-- Aggregated funding pipeline metrics
-- ================================================
CREATE TABLE IF NOT EXISTS crm_funding_analytics (
  id TEXT PRIMARY KEY,
  period_start DATE NOT NULL,
  period_end DATE NOT NULL,
  
  -- Funding Status
  total_target_usd INTEGER,
  total_raised_usd INTEGER,
  funding_gap_usd INTEGER,
  funding_percentage REAL, -- 0-100
  
  -- Sources breakdown
  from_vc_usd INTEGER DEFAULT 0,
  from_angel_usd INTEGER DEFAULT 0,
  from_grants_usd INTEGER DEFAULT 0,
  from_corporate_usd INTEGER DEFAULT 0,
  from_other_usd INTEGER DEFAULT 0,
  
  -- Pipeline
  investors_in_pipeline INTEGER DEFAULT 0,
  investors_interested INTEGER DEFAULT 0,
  investors_committed INTEGER DEFAULT 0,
  
  grants_found INTEGER DEFAULT 0,
  grants_applied INTEGER DEFAULT 0,
  grants_awarded INTEGER DEFAULT 0,
  
  -- Forecast
  estimated_close_date DATE,
  success_probability_percentage REAL, -- 0-100
  months_to_close INTEGER,
  
  -- Investor Stats
  avg_check_size_usd INTEGER,
  median_check_size_usd INTEGER,
  min_check_size_usd INTEGER,
  max_check_size_usd INTEGER,
  
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_analytics_period ON crm_funding_analytics(period_start, period_end);

-- ================================================
-- GOOGLE DORKS QUERIES TABLE
-- Track all dorks search queries executed
-- ================================================
CREATE TABLE IF NOT EXISTS crm_dorks_queries (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  campaign_id TEXT,
  
  name TEXT NOT NULL,
  category TEXT, -- 'investor_emails', 'grants', 'competitors', 'founders'
  query TEXT NOT NULL, -- Full dorks query string
  description TEXT,
  tags TEXT, -- JSON array
  
  -- Execution
  executed_at DATETIME,
  results_count INTEGER DEFAULT 0,
  execution_time_seconds REAL,
  
  -- Status
  status TEXT DEFAULT 'saved', -- 'saved', 'executed', 'results_reviewed'
  
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_dorks_category ON crm_dorks_queries(category);
CREATE INDEX idx_dorks_campaign ON crm_dorks_queries(campaign_id);
CREATE INDEX idx_dorks_user ON crm_dorks_queries(user_id);
CREATE INDEX idx_dorks_status ON crm_dorks_queries(status);
CREATE INDEX idx_dorks_created ON crm_dorks_queries(created_at);

-- ================================================
-- GOOGLE DORKS RESULTS TABLE
-- Cache search results and import status
-- ================================================
CREATE TABLE IF NOT EXISTS crm_dorks_results (
  id TEXT PRIMARY KEY,
  query_id TEXT NOT NULL,
  
  title TEXT,
  url TEXT UNIQUE,
  snippet TEXT,
  domain TEXT,
  
  -- Extracted Information
  email TEXT,
  phone TEXT,
  linkedin_url TEXT,
  
  -- Classification
  result_type TEXT, -- 'investor', 'grant', 'accelerator', 'founder', 'competitor'
  relevance_score REAL, -- 0-100
  
  -- Import Status
  imported_as_contact_id TEXT,
  imported_at DATETIME,
  
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  
  FOREIGN KEY (query_id) REFERENCES crm_dorks_queries(id),
  FOREIGN KEY (imported_as_contact_id) REFERENCES crm_contacts(id)
);

CREATE INDEX idx_dorks_results_query ON crm_dorks_results(query_id);
CREATE INDEX idx_dorks_results_type ON crm_dorks_results(result_type);
CREATE INDEX idx_dorks_results_domain ON crm_dorks_results(domain);
CREATE INDEX idx_dorks_results_imported ON crm_dorks_results(imported_at);

-- ================================================
-- DORKS SEARCH CAMPAIGNS TABLE
-- Organize multiple searches into campaigns
-- ================================================
CREATE TABLE IF NOT EXISTS crm_dorks_campaigns (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  
  name TEXT NOT NULL,
  objective TEXT, -- 'find_investors', 'find_grants', 'market_research', 'partnership_search'
  description TEXT,
  
  target_keywords TEXT, -- JSON array
  location_focus TEXT,
  sector_focus TEXT, -- JSON array
  
  -- Progress
  queries_count INTEGER DEFAULT 0,
  results_found INTEGER DEFAULT 0,
  contacts_created INTEGER DEFAULT 0,
  
  status TEXT DEFAULT 'active', -- 'planning', 'active', 'completed', 'archived'
  
  start_date DATETIME DEFAULT CURRENT_TIMESTAMP,
  end_date DATETIME,
  
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_campaigns_user ON crm_dorks_campaigns(user_id);
CREATE INDEX idx_campaigns_objective ON crm_dorks_campaigns(objective);
CREATE INDEX idx_campaigns_status ON crm_dorks_campaigns(status);

-- ================================================
-- INDEX OPTIMIZATION
-- Critical multi-column indexes for fast queries
-- ================================================

-- Performance optimization for funding pipeline queries
CREATE INDEX idx_funding_pipeline ON crm_investors(status, rating, location);

-- Fast search for grant matching
CREATE INDEX idx_grants_matching ON crm_grants(application_status, deadline_date, match_score);

-- Dorks campaign performance
CREATE INDEX idx_dorks_campaign_performance ON crm_dorks_campaigns(status, objective, start_date);

-- Text search optimization (SQLite FTS would be better but requires setup)
CREATE INDEX idx_contact_search ON crm_contacts(first_name, last_name, email);

-- Temporal queries
CREATE INDEX idx_contacts_temporal ON crm_contacts(created_at, updated_at);
CREATE INDEX idx_activities_temporal ON crm_activities(created_at);

-- ================================================
-- VIEWS for Analytics
-- Helpful aggregations for dashboards
-- ================================================

CREATE VIEW IF NOT EXISTS vw_investor_pipeline AS
SELECT 
  status,
  rating,
  COUNT(*) as investor_count,
  COUNT(CASE WHEN rating = 'hot' THEN 1 END) as hot_count,
  COUNT(CASE WHEN rating = 'warm' THEN 1 END) as warm_count,
  COUNT(CASE WHEN rating = 'cold' THEN 1 END) as cold_count,
  SUM(CASE WHEN ticket_size_min IS NOT NULL THEN ticket_size_min ELSE 0 END) as min_capital,
  AVG(CAST(ticket_size_max AS DECIMAL)) as avg_ticket_size
FROM crm_investors
GROUP BY status, rating;

CREATE VIEW IF NOT EXISTS vw_grant_opportunities AS
SELECT 
  provider,
  application_status,
  COUNT(*) as grant_count,
  SUM(amount_max_usd) as total_potential_usd,
  MIN(deadline_date) as next_deadline,
  AVG(CAST(match_score AS DECIMAL)) as avg_relevance
FROM crm_grants
GROUP BY provider, application_status;

CREATE VIEW IF NOT EXISTS vw_dorks_campaign_summary AS
SELECT 
  id,
  name,
  objective,
  queries_count,
  results_found,
  contacts_created,
  ROUND(CAST(contacts_created AS DECIMAL) / NULLIF(results_found, 0), 2) as conversion_rate,
  DATEDIFF(day, start_date, COALESCE(end_date, CURRENT_TIMESTAMP)) as duration_days
FROM crm_dorks_campaigns;

CREATE VIEW IF NOT EXISTS vw_funding_health AS
SELECT 
  fr.round_name,
  fr.target_amount_usd,
  fr.raised_amount_usd,
  (fr.target_amount_usd - fr.raised_amount_usd) as funding_gap,
  ROUND(100.0 * fr.raised_amount_usd / NULLIF(fr.target_amount_usd, 0), 1) as funding_percentage,
  COUNT(DISTINCT im.investor_id) as active_investors,
  COUNT(CASE WHEN im.outcome IN ('interested', 'committed') THEN 1 END) as positive_outcomes
FROM crm_funding_rounds fr
LEFT JOIN crm_investor_meetings im ON fr.id = im.funding_round_id
GROUP BY fr.id, fr.round_name, fr.target_amount_usd, fr.raised_amount_usd;
