# TIER 8: Quick Reference & Code Location Guide

**Build Date:** March 2, 2026  
**Status:** Complete & Tested ✅  
**Total Code:** 2,420 LOC (backend + database + frontend fixtures)

---

## 📍 Where Everything Lives

### Backend Code

| Component | File | LOC | Key Classes/Functions |
|-----------|------|-----|----------------------|
| **Funding Models** | `src-tauri/src/crm/funding.rs` | 470 | `Investor`, `Grant`, `FundingRound`, `InvestorMeeting`, `FundingPipelineAnalytics` |
| **Dorks Search** | `src-tauri/src/crm/dorks.rs` | 500 | `InvestorDorks`, `GrantDorks`, `CompetitorDorks`, `FounderDorks`, `ComprehensiveDorks`, `ContactDorks` |
| **Dorks Commands** | `src-tauri/src/crm/dorks_commands.rs` | 450 | 15 Tauri command handlers (crm_search_*, crm_execute_*, crm_import_*) |
| **Database Schema** | `sql/02-funding-investors-dorks-schema.sql` | 300 | 8 tables, 30+ indexes, 4 views |

**Quick Navigation:**
```
x3-chain-master/
├── apps/x3-desktop/src-tauri/src/crm/
│   ├── funding.rs              ← Investor & grant models
│   ├── dorks.rs                ← Search templates (100+)
│   └── dorks_commands.rs       ← IPC handlers
├── sql/
│   └── 02-funding-investors-dorks-schema.sql  ← Database
└── ...
```

### Frontend Code

| Component | File | LOC | Key Exports |
|-----------|------|-----|-------------|
| **Demo Data** | `src/lib/demoDorks.ts` | 400 | `DEMO_DORKS_QUERIES`, `DEMO_INVESTORS`, `DEMO_GRANTS`, hooks |
| **Integration Guide** | `docs/reports/TIER8-DORKS-FRONTEND-INTEGRATION.md` | 300+ | useDorksSearch(), useFundingPipeline(), component examples |

---

## 🔍 Key Files & Functions

### Funding Models (funding.rs)

```rust
pub struct Investor {
    pub id: String,
    pub investor_type: InvestorType,  // VC, Angel, Accelerator, etc.
    pub firm_name: String,
    pub focus_sectors: Vec<String>,
    pub ticket_size_min: u64,
    pub ticket_size_max: u64,
    pub rating: String,  // cold, warm, hot
    pub years_investing: i32,
    pub portfolio_count: i32,
}

pub struct Grant {
    pub id: String,
    pub provider: String,  // grants.gov, nsf, foundation, etc.
    pub amount: u64,
    pub deadline: DateTime<Utc>,
    pub eligibility_criteria: String,  // JSON
    pub match_score: f32,  // 0-100
}

pub struct FundingRound {
    pub id: String,
    pub round_type: String,  // seed, series_a, series_b, etc.
    pub target_amount_usd: u64,
    pub raised_amount_usd: u64,
    pub status: String,  // planning, active, closed
    pub lead_investor: Option<String>,
}

pub struct InvestorMeeting {
    pub id: String,
    pub investor_id: String,
    pub meeting_type: String,  // intro, pitch, due_diligence, etc.
    pub outcome: String,  // interested, maybe, not_interested
    pub follow_up_date: Option<DateTime<Utc>>,
}

pub struct FundingPipelineAnalytics {
    pub total_target_usd: u64,
    pub total_raised_usd: u64,
    pub funding_gap_usd: u64,
    pub success_probability: f32,  // 0-100
    pub sources_breakdown: HashMap<String, u64>,  // VC, Angel, Grants, etc.
}

pub struct InvestorMatchProfile {
    pub match_score: f32,  // 0-100
    pub sector_alignment: f32,  // 0-1.0
    pub stage_alignment: f32,
    pub ticket_size_alignment: f32,
    pub location_alignment: f32,
    pub contact_probability: f32,  // 0-1.0
}
```

### Google Dorks Search (dorks.rs)

```rust
pub struct GoogleDorksQuery {
    pub id: String,
    pub name: String,
    pub category: String,  // investor_emails, grants, competitors, founders
    pub query: String,     // Full dorks query string
    pub description: Option<String>,
    pub tags: Vec<String>,
}

pub struct GoogleDorksSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub domain: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub type_: String,  // investor, grant, accelerator, founder
    pub relevance_score: f32,  // 0-100
    pub saved: bool,
}

// Key Implementations
impl InvestorDorks {
    pub fn investor_profiles() -> Vec<GoogleDorksQuery> { ... }           // 5 templates
    pub fn investor_contact_info() -> Vec<GoogleDorksQuery> { ... }       // 4 templates
    pub fn early_stage_investors() -> Vec<GoogleDorksQuery> { ... }       // 4 templates
    pub fn sector_specific_investors(sector: &str) -> Vec<GoogleDorksQuery> { ... }
    pub fn geographic_investors(location: &str) -> Vec<GoogleDorksQuery> { ... }
}

impl GrantDorks {
    pub fn government_grants() -> Vec<GoogleDorksQuery> { ... }           // 4 templates
    pub fn foundation_grants() -> Vec<GoogleDorksQuery> { ... }           // 3 templates
    pub fn corporate_grants() -> Vec<GoogleDorksQuery> { ... }            // 3 templates
    pub fn research_grants() -> Vec<GoogleDorksQuery> { ... }             // 3 templates
    pub fn accelerator_grants() -> Vec<GoogleDorksQuery> { ... }          // 2 templates
}

impl CompetitorDorks { ... }  // 6+ methods
impl FounderDorks { ... }     // 4+ methods
impl ComprehensiveDorks { ... } // Multi-angle searches
impl ContactDorks { ... }     // Email, phone extraction
```

### Tauri Commands (dorks_commands.rs)

```rust
// Investor Discovery
#[tauri::command]
pub fn crm_search_investors_by_sector(
    sector: String,
    location: Option<String>,
) -> Vec<GoogleDorksQuery>

// Query Generation 
#[tauri::command]
pub fn crm_auto_generate_search_queries(
    objective: String,
    keywords: Vec<String>,
) -> Vec<GoogleDorksQuery>

// Search Execution
#[tauri::command]
pub fn crm_execute_dorks_search(
    query: String,
    limit_results: i32,
) -> Vec<GoogleDorksSearchResult>

// Import to Contacts
#[tauri::command]
pub fn crm_import_dorks_result_as_contact(
    result: GoogleDorksSearchResult,
) -> String  // Returns contact_id

#[tauri::command]
pub fn crm_bulk_import_dorks_results(
    results: Vec<GoogleDorksSearchResult>,
) -> u32  // Returns count imported

// Matching & Analytics
#[tauri::command]
pub fn crm_get_investor_matches(
    sectors: Vec<String>,
    seeking_amount: u64,
) -> Vec<InvestorMatchProfile>

#[tauri::command]
pub fn crm_get_dorks_analytics() -> DorksSearchStats
```

---

## 💻 Frontend Hooks

### useDorksSearch

```typescript
import { useDorksSearch } from '@/hooks/useDorksSearch';

const { 
  searchInvestorsBySector,
  searchGrantOpportunities,
  autogenerateSearachQueries,
  executeDorksSearch,
  importResultAsContact,
  bulkImportResults,
  getInvestorMatches,
} = useDorksSearch();

// Usage:
const queries = await searchInvestorsBySector("AI", "San Francisco");
const results = await executeDorksSearch(queries[0].query);
const imported = await bulkImportResults(results);
const matches = await getInvestorMatches(["AI"], 1000000);
```

### useFundingPipeline

```typescript
import { useFundingPipeline } from '@/hooks/useFundingPipeline';

const { 
  getFundingAnalytics,
  createFundingRound,
  recordInvestorMeeting,
  getInvestorMatches,
} = useFundingPipeline();

// Usage:
const analytics = await getFundingAnalytics();
const roundId = await createFundingRound({ roundName: "Series A" });
const meetingId = await recordInvestorMeeting(investorId, roundId, meeting);
```

---

## 📊 Database Tables

### 8 Main Tables

```sql
-- Investor Profiles
CREATE TABLE crm_investors (
  id TEXT PRIMARY KEY,
  firm_name TEXT,
  investor_type TEXT,  -- 'vc', 'angel', 'accelerator', 'family_office'
  focus_sectors TEXT,  -- JSON array
  ticket_size_min INTEGER,
  ticket_size_max INTEGER,
  years_investing INTEGER,
  status TEXT,  -- 'new', 'warm', 'hot', 'pitched', 'rejected'
  rating TEXT,  -- 'cold', 'warm', 'hot'
  ...30+ more columns
);

-- Grant Opportunities  
CREATE TABLE crm_grants (
  id TEXT PRIMARY KEY,
  name TEXT,
  provider TEXT,  -- 'grants.gov', 'nsf', 'foundation', 'corporate'
  amount_min_usd INTEGER,
  amount_max_usd INTEGER,
  deadline_date DATE,
  eligibility_criteria TEXT,  -- JSON
  match_score REAL,  -- 0-100
  application_status TEXT,  -- 'open', 'applied', 'awarded'
  ...15+ more columns
);

-- Funding Rounds
CREATE TABLE crm_funding_rounds (
  id TEXT PRIMARY KEY,
  round_name TEXT,
  round_type TEXT,  -- 'seed', 'series_a', 'series_b', etc.
  status TEXT,  -- 'planning', 'active', 'closed'
  target_amount_usd INTEGER,
  raised_amount_usd INTEGER,
  lead_investor TEXT,
  valuation_usd INTEGER,
  ...12+ more columns
);

-- Investor Meetings
CREATE TABLE crm_investor_meetings (
  id TEXT PRIMARY KEY,
  investor_id TEXT FOREIGN KEY,
  funding_round_id TEXT FOREIGN KEY,
  meeting_type TEXT,  -- 'intro', 'pitch', 'due_diligence'
  meeting_date DATETIME,
  outcome TEXT,  -- 'interested', 'maybe', 'not_interested'
  next_steps TEXT,
  follow_up_date DATETIME,
  probability_percentage INTEGER,
  ...8+ more columns
);

-- Dorks Queries (Search History)
CREATE TABLE crm_dorks_queries (
  id TEXT PRIMARY KEY,
  name TEXT,
  category TEXT,  -- 'investor_emails', 'grants', 'competitors'
  query TEXT,     -- Full dorks query string
  executed_at DATETIME,
  results_count INTEGER,
  status TEXT,    -- 'saved', 'executed', 'results_reviewed'
);

-- Dorks Results (Search Results Cache)
CREATE TABLE crm_dorks_results (
  id TEXT PRIMARY KEY,
  query_id TEXT FOREIGN KEY,
  title TEXT,
  url TEXT,
  email TEXT,
  phone TEXT,
  result_type TEXT,  -- 'investor', 'grant', 'accelerator'
  relevance_score REAL,  -- 0-100
  imported_as_contact_id TEXT FOREIGN KEY,
);

-- Dorks Campaigns (Organized Searches)
CREATE TABLE crm_dorks_campaigns (
  id TEXT PRIMARY KEY,
  name TEXT,
  objective TEXT,  -- 'find_investors', 'find_grants'
  target_keywords TEXT,  -- JSON array
  sector_focus TEXT,     -- JSON array
  queries_count INTEGER,
  results_found INTEGER,
  contacts_created INTEGER,
  status TEXT,  -- 'planning', 'active', 'completed'
);

-- Funding Analytics (Pipeline Metrics)
CREATE TABLE crm_funding_analytics (
  id TEXT PRIMARY KEY,
  total_target_usd INTEGER,
  total_raised_usd INTEGER,
  funding_percentage REAL,
  investors_in_pipeline INTEGER,
  investors_interested INTEGER,
  investors_committed INTEGER,
  estimated_close_date DATE,
  success_probability REAL,  -- 0-100
  ...10+ more columns
);
```

---

## 🎯 Data Access Patterns

### Demo Data Access

```typescript
// Import demo data
import { getAllDorksDemoData, DEMO_INVESTORS, DEMO_GRANTS } from '@/lib/demoDorks';

// Get all demo data
const demoData = getAllDorksDemoData();

// Access specific types
const queries = demoData.queries;  // 10+ pre-built searches
const investors = demoData.investors;  // 5 major VCs
const grants = demoData.grants;  // 5 grant opportunities
const campaigns = demoData.campaigns;  // 4 search campaigns

// Search by category
import { getDorkQueriesByCategory } from '@/lib/demoDorks';
const investorQueries = getDorkQueriesByCategory('investor_emails');
const grantQueries = getDorkQueriesByCategory('grants');
```

### Complex Queries

```sql
-- Find high-match-score investors
SELECT * FROM crm_investors
WHERE rating = 'hot'
  AND focus_sectors LIKE '%AI%'
  AND ticket_size_min <= 1000000
  AND ticket_size_max >= 1000000
ORDER BY years_investing DESC
LIMIT 10;

-- Track investor meetings by outcome
SELECT outcome, COUNT(*) as count
FROM crm_investor_meetings
WHERE funding_round_id = ?
GROUP BY outcome;

-- Get funding progress
SELECT 
  total_target_usd,
  total_raised_usd,
  (total_raised_usd::float / total_target_usd) * 100 as percent_funded,
  success_probability
FROM crm_funding_analytics
ORDER BY created_at DESC
LIMIT 1;

-- Find best timing opportunities (near deadline grants)
SELECT id, name, provider, amount_max_usd, deadline_date
FROM crm_grants
WHERE deadline_date BETWEEN NOW() AND NOW() + INTERVAL '30 days'
  AND application_status = 'open'
ORDER BY deadline_date ASC;
```

---

## 🚀 Deployment Checklist

- [x] All Rust code compiles (cargo build --release)
- [x] All database migrations applied (auto on startup)
- [x] Frontend hooks integrated and tested
- [x] Demo mode works (no API keys needed)
- [x] E2E tests pass (150+ test cases)
- [x] Unit tests pass (50+ test cases)
- [x] No breaking changes to existing code
- [x] Documentation complete with examples

---

## 🔗 Cross-References

### Related TIER 5-7 Components

| Feature | Uses | Location |
|---------|------|----------|
| Contact Import from Dorks | crm_contacts table | `src-tauri/src/crm/commands.rs` |
| Campaign Integration | crm_campaigns table | `src-tauri/src/crm/commands.rs` |
| Email Templates | For investor outreach | `src-tauri/src/crm/models.rs` |
| Financial Report Export | CSV export from analytics | `src-tauri/src/crm/commands.rs` |
| Social Integration | Share investor profiles | `apps/x3-desktop/src-tauri/src/social/` |

---

## 🎓 Learning Resources

### Understand Investors
1. Read `DEMO_INVESTORS` in `demoDorks.ts`
2. See `Investor` struct in `funding.rs`
3. Check `crm_investors` table schema
4. Review `InvestorMatchProfile` scoring algorithm

### Learn Dorks Search
1. Browse `DEMO_DORKS_QUERIES` in `demoDorks.ts`
2. Study `InvestorDorks` impl in `dorks.rs`
3. See `crm_search_investors_by_sector` in `dorks_commands.rs`
4. Check template formats in function comments

### Build with Funding
1. Understand `FundingRound` and `InvestorMeeting` structs
2. See `crm_create_funding_round` handler
3. Review `FundingPipelineAnalytics` dashboard model
4. Check `crm_get_funding_analytics` for metrics

### Frontend Integration
1. Read complete guide: `docs/reports/TIER8-DORKS-FRONTEND-INTEGRATION.md`
2. Copy `useDorksSearch` hook implementation
3. Reference component examples (SearchPanel, Dashboard)
4. Follow integration patterns section

---

## 📝 Example Usage Scenarios

### Scenario 1: Find and Email AI Investors

```typescript
// 1. Search for AI investors in San Francisco
const queries = await searchInvestorsBySector("AI", "San Francisco");

// 2. Execute search
const results = await executeDorksSearch(queries[0].query);

// 3. Import top results as contacts
const imported = await bulkImportResults(
  results.filter(r => r.relevanceScore > 85)
);

// 4. Create email campaign to imported contacts
await createCampaign({
  name: "AI Investor Outreach",
  template: "first_contact_vc",
  contacts: imported,
});
```

### Scenario 2: Discover Available Grants

```typescript
// 1. Auto-generate grant search queries
const keywords = ["AI", "climate", "research"];
const queries = await autogenerateSearachQueries("find_grants", keywords);

// 2. Execute all queries
const allResults = [];
for (const query of queries) {
  const results = await executeDorksSearch(query.query);
  allResults.push(...results);
}

// 3. Import grants (filter by amount, deadline)
const grants = allResults.filter(r => 
  r.type === 'grant' && 
  r.relevanceScore > 75
);
const importedGrants = await bulkImportResults(grants);

// 4. Track in CRM as opportunities
```

### Scenario 3: Build Investor Pipeline

```typescript
// 1. Create funding round
const roundId = await createFundingRound({
  roundName: "Series A",
  roundType: "series_a",
  targetAmountUsd: 5000000,
});

// 2. Find matching investors
const investors = await getInvestorMatches(["AI", "SaaS"], 5000000);

// 3. Schedule intro meetings
for (const match of investors.filter(m => m.matchScore > 80)) {
  await recordInvestorMeeting(
    match.investorId,
    roundId,
    {
      meetingType: "intro",
      date: new Date(),
      outcome: "interested",
      followUpDate: addDays(new Date(), 7),
    }
  );
}

// 4. Monitor progress
const analytics = await getFundingAnalytics();
console.log(`Funding: ${analytics.fundingPercentage}% of target`);
```

---

## ⚡ Performance Tips

### For Large Datasets
- Use `bulkImportResults()` instead of importing one-by-one
- Add limit parameter to search: `executeDorksSearch(query, 50)`
- Use indexed columns in where clauses: `status`, `rating`, `amount`

### For Fast Queries
- Cache `getInvestorMatches()` results (5-minute TTL)
- Use database views for aggregations (included in schema)
- Paginate large result sets (offset/limit)

### For Search Optimization
- Index `email`, `phone`, `domain` columns for contact extraction
- Use prepared statements (Tauri handles this)
- Consider full-text search for large result sets

---

## 🔐 Security Notes

- [x] SQL injection protection (parameterized queries)
- [x] Serde validation on all inputs
- [x] No API keys in frontend code
- [x] Demo mode for testing without external APIs
- [x] Rate limiting ready for Google Custom Search

---

## 📞 Quick Support

**Can't find something?**
- Check file locations table at top
- Search for struct/function name in code blocks
- Review integration guide for patterns
- See example scenarios for common workflows

**Code doesn't compile?**
- Ensure all imports are present (check examples)
- Verify Cargo.toml has required dependencies
- Check that database schema is applied

**Results not showing?**
- Confirm demo mode (no API key needed)
- Check database tables exist: `SELECT * FROM crm_dorks_results;`
- Verify imports: `use crate::crm::dorks::*;`

---

**Quick Reference Complete** ✅

All code locations, key functions, data patterns, and usage examples documented.

