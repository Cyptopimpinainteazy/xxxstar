// Google Dorks Search Module for X3 CRM
// Advanced search queries for finding investors, grants, and funding opportunities

use serde::{Deserialize, Serialize};

// ============================================
// GOOGLE DORKS SEARCH ENGINE
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleDorksQuery {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub category: String,           // investor_emails, investor_websites, grants, competitor_tech, founder_profiles
    pub query: String,              // the actual dorks query
    pub description: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleDorksSearchResult {
    pub id: String,
    pub query_id: String,
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub domain: String,
    pub email: Option<String>,      // if found
    pub phone: Option<String>,       // if found
    pub type_: String,              // investor, grant, competitor, founder, accelerator
    pub relevance_score: f32,       // 0-100
    pub saved: bool,                // whether imported to contacts
    pub contact_id: Option<String>, // if linked to a contact
    pub notes: String,
    pub search_date: String,
    pub last_verified: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteGoogleDorksInput {
    pub query_name: String,
    pub query: String,
    pub category: String,
    pub limit_results: Option<u32>,  // max results to return
}

// ============================================
// INVESTOR DISCOVERY DORKS
// ============================================

pub struct InvestorDorks;

impl InvestorDorks {
    /// Find investor profile pages
    pub fn investor_profiles() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Angel List Investor Profiles", 
             r#"site:angel.co/investors "invested in" OR "portfolio" OR "area of expertise""#),
            
            ("Crunchbase Investor Pages",
             r#"site:crunchbase.com/person OR site:crunchbase.com/entity "investor""#),
            
            ("LinkedIn Investor Profiles",
             r#"site:linkedin.com "venture capital" OR "investor" location:"{location}" -company"#),
            
            ("Medium VC Articles",
             r#"site:medium.com "venture capital" OR "VC" OR "startup investing" author investment thesis"#),
            
            ("Twitter Verified VCs",
             r#"site:twitter.com verified account:"VC" OR "Venture Capital" OR "Angel Investor""#),
        ]
    }

    /// Find investor contact information
    pub fn investor_contact_info() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Investor Email Addresses",
             r#"site:{investor_domain} OR site:{investor_domain}.com OR site:{investor_domain}.io "contact" OR "email" OR "@{investor_domain}""#),
            
            ("Venture Fund Contact Pages",
             r#"(venture capital OR "VC fund") (contact us OR email OR phone) {location} filetype:pdf OR filetype:html"#),
            
            ("Fund Management Team",
             r#"site:{fund_website} "team" OR "partners" OR "investors" OR "contact""#),
            
            ("Email patterns site:crunchbase.com",
             r#"site:crunchbase.com "{firm_name}" (contact OR email OR linkedin.com OR email:)"#),
        ]
    }

    /// Find early-stage investors
    pub fn early_stage_investors() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Seed Stage Investors",
             r#"("seed capital" OR "seed funding" OR "seed stage") ("$100k" OR "$250k" OR "$500k") site:crunchbase.com OR site:angel.co"#),
            
            ("Angel Investor Networks",
             r#"site:angellist.com OR site:gust.com "looking for" OR "accepting pitch" location:"{location}""#),
            
            ("Startup Accelerators",
             r#"(Y Combinator OR "500 Global" OR Techstars OR "Plug and Play") location:"{location}" apply OR cohort"#),
            
            ("Micro VCs",
             r#"site:crunchbase.com ("micro vc" OR "micro fund") "$1M" OR "$5M" OR "$10M" focus:"{sector}""#),
        ]
    }

    /// Find sector-specific investors
    pub fn sector_specific_investors(sector: &str) -> String {
        format!(
            r#"("{}" OR "{}_sector" OR "{}_ investors") site:crunchbase.com OR site:angel.co ("investing in" OR "portfolio" OR "focus area")"#,
            sector, sector, sector
        )
    }

    /// Geographic investor search
    pub fn geographic_investors(location: &str) -> String {
        format!(
            r#"(venture capital OR "VC firm") location:"{}" OR "based in" {} site:crunchbase.com OR site:linkedin.com"#,
            location, location
        )
    }
}

// ============================================
// GRANT DISCOVERY DORKS
// ============================================

pub struct GrantDorks;

impl GrantDorks {
    /// Find government grants
    pub fn government_grants() -> Vec<(&'static str, &'static str)> {
        vec![
            ("US Federal Grants",
             r#"site:grants.gov "{sector}" OR "{industry}" grant application deadline"#),
            
            ("SBIR/STTR Grants",
             r#"site:sbir.gov OR site:nsf.gov "SBIR" OR "STTR" phase 1 OR phase 2 "{sector}""#),
            
            ("NSF Grants",
             r#"site:nsf.gov grant program "{sector}" OR "research" application deadline"#),
            
            ("EPA Grants",
             r#"site:epa.gov "grant funding" OR "environmental" "{sector}" deadline"#),
        ]
    }

    /// Find foundation grants
    pub fn foundation_grants() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Major Foundations",
             r#"site:grantstation.com OR site:philanthropy.org "{sector}" grant "$" deadline"#),
            
            ("Foundation Directories",
             r#"(foundation grants OR charitable giving) "{sector}" OR "{industry}" site:foundationcenter.org"#),
            
            ("Community Foundations",
             r#"site:communityfoundations.org OR site:cfgreatlakes.org grant "{sector}" "{location}""#),
        ]
    }

    /// Find corporate grants
    pub fn corporate_grants() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Tech Company Grants",
             r#"(Google OR Microsoft OR Amazon OR Meta OR Apple) "grants program" OR "startup grant" "{sector}""#),
            
            ("Corporate Foundations",
             r#"site:{company}.com OR site:{company}foundation.org grant program application"#),
            
            ("Corporate Matching Gifts",
             r#"site:matchinggifts.org OR site:{company}.com "matching gift" OR "corporate grant""#),
        ]
    }

    /// Find innovation & research grants
    pub fn research_grants() -> Vec<(&'static str, &'static str)> {
        vec![
            ("R&D Tax Credits",
             r#"site:irs.gov OR site:sba.gov "R&D credit" OR "research and development" eligible"{#),
            
            ("Innovation Challenge Grants",
             r#"(innovation OR research) challenge grant "$" "{sector}" deadline 2026"#),
            
            ("Climate & Sustainability Grants",
             r#"site:climate.org OR site:greenfund.org (climate OR sustainability) grant "{sector}""#),
        ]
    }

    /// Find accelerator & incubator grants
    pub fn accelerator_grants() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Accelerator Grant Programs",
             r#"(Y Combinator OR Techstars OR "500 Global") grant OR funding OR fellowship "{sector}""#),
            
            ("University Startup Grants",
             r#"site:.edu startup grant OR accelerator program "{sector}" OR "entrepreneurship""#),
        ]
    }
}

// ============================================
// COMPETITOR & MARKET INTELLIGENCE DORKS
// ============================================

pub struct CompetitorDorks;

impl CompetitorDorks {
    /// Find competitors using similar tech
    pub fn tech_stack_search(tech: &str) -> String {
        format!(
            r#"("built with" OR "powered by" OR "uses") {} site:linkedin.com OR site:crunchbase.com OR site:github.com"#,
            tech
        )
    }

    /// Find companies in specific market
    pub fn market_search(market: &str, keyword: &str) -> String {
        format!(
            r#"({} OR "{}") (company OR startup OR platform) founded:2020-2026 site:crunchbase.com"#,
            market, keyword
        )
    }

    /// Find funding announcements
    pub fn funding_announcements(sector: &str) -> String {
        format!(
            r#"("{}" OR "{} startup") ("raises" OR "funding" OR "seed round" OR "series") 2026"#,
            sector, sector
        )
    }

    /// Find recent IPOs and exits
    pub fn exits_and_ipos() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Recent IPOs",
             r#"(IPO OR "initial public offering") 2025 OR 2026 site:nasdaq.com OR site:nyse.com"#),
            
            ("Recent Acquisitions",
             r#"(acquired by OR "acquired for") 2025 OR 2026 site:crunchbase.com OR site:techcrunch.com"#),
            
            ("Unicorn Exits",
             r#"unicorn ("exit" OR "acquired" OR "IPO") site:crunchbase.com 2025 OR 2026"#),
        ]
    }
}

// ============================================
// FOUNDER & TEAM INTELLIGENCE DORKS
// ============================================

pub struct FounderDorks;

impl FounderDorks {
    /// Find founder social profiles
    pub fn founder_profiles(name: &str) -> Vec<(&'static str, String)> {
        vec![
            ("LinkedIn Profile", format!(r#"site:linkedin.com "{}" founder OR CEO OR CTO"#, name)),
            ("Twitter Profile", format!(r#"site:twitter.com "{}" -"this account""#, name)),
            ("GitHub Profile", format!(r#"site:github.com "{}" (repositories OR projects)"#, name)),
            ("Medium Articles", format!(r#"site:medium.com/@{} OR site:medium.com author:"{}" OR "{}" startup"#, name.to_lowercase().replace(" ", ""), name, name)),
        ]
    }

    /// Find founder expertise and background
    pub fn founder_expertise(expertise: &str) -> String {
        format!(
            r#"("{}" OR "{} expert") founder OR "serial entrepreneur" (blog OR article OR speaking)"#,
            expertise, expertise
        )
    }

    /// Find founder networks and connections
    pub fn founder_networks() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Founder Communities",
             r#"site:linkedin.com OR site:facebook.com (founder group OR "founder network" OR "entrepreneur group") location:"{location}""#),
            
            ("Industry Founders",
             r#"site:twitter.com OR site:linkedin.com "{sector} founder" OR "{industry} entrepreneur""#),
        ]
    }
}

// ============================================
// MULTI-SEARCH INTELLIGENCE DORKS
// ============================================

pub struct ComprehensiveDorks;

impl ComprehensiveDorks {
    /// Find all opportunities for a specific company type
    pub fn company_opportunity_scan(company_keywords: &[&str], location: &str) -> Vec<String> {
        let mut queries = vec![];

        // Investor search
        for keyword in company_keywords {
            queries.push(format!(
                r#"(investor OR "venture capital" OR VC) "{}" {} site:crunchbase.com OR site:angel.co OR site:linkedin.com"#,
                keyword, location
            ));
        }

        // Grant search
        for keyword in company_keywords {
            queries.push(format!(
                r#"(grant OR funding OR "financial aid") "{}" {} site:grants.gov OR site:sbir.gov"#,
                keyword, location
            ));
        }

        // Accelerator search
        for keyword in company_keywords {
            queries.push(format!(
                r#"(accelerator OR incubator) "{}" {} 2026 application"#,
                keyword, location
            ));
        }

        queries
    }

    /// Find partnership opportunities
    pub fn partnership_search(company_type: &str, sector: &str) -> String {
        format!(
            r#"("{}" OR "{} company") "partner" OR "partnership" OR "collaboration" "{}" site:linkedin.com OR site:crunchbase.com"#,
            company_type, sector, sector
        )
    }

    /// Find strategic buyer prospects
    pub fn strategic_buyer_search(sector: &str) -> String {
        format!(
            r#"(large tech OR publicly traded) "{}" sector (acquisition OR partnership OR investment) site:crunchbase.com OR site:linkedin.com"#,
            sector
        )
    }
}

// ============================================
// CONTACT EXTRACTION DORKS
// ============================================

pub struct ContactDorks;

impl ContactDorks {
    /// Find email patterns
    pub fn email_extraction(domain: &str) -> Vec<(&'static str, String)> {
        vec![
            ("Common Email Pattern", format!(r#"site:{} "@{}" OR "contact" OR "email""#, domain, domain)),
            ("Team Page Email", format!(r#"site:{}/team OR site:{}/about "@{}" OR "contact""#, domain, domain, domain)),
            ("Email Directory", format!(r#"site:{} filetype:pdf "email" OR "contact" OR "@{}" team"#, domain, domain)),
        ]
    }

    /// Find phone numbers
    pub fn phone_extraction(company_name: &str) -> String {
        format!(
            r#""{}" ("phone" OR "call" OR "contact") ("+1" OR area code) -site:yellowpages.com"#,
            company_name
        )
    }

    /// Find LinkedIn URLs
    pub fn linkedin_urls(company_name: &str) -> String {
        format!(
            r#"site:linkedin.com/in "{}" OR site:linkedin.com/company "{}" founder OR CEO"#,
            company_name, company_name
        )
    }
}

// ============================================
// SAVED DORKS TEMPLATES
// ============================================

pub fn get_default_dorks() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // Investors
        ("Angel Investors by Sector", "site:angel.co \"invested in\" \"{sector}\"", "investor_emails"),
        ("VC Firms Contact Pages", "site:{vc_firm}.com OR site:{vc_firm}.com/contact", "investor_websites"),
        ("Series A Investors", "site:crunchbase.com \"series a\" invested \"{sector}\" 2025 2026", "investor_emails"),
        
        // Grants
        ("Federal Research Grants", "site:grants.gov \"{sector}\" grant deadline 2026", "grants"),
        ("Small Business Innovation Grants", "site:sbir.gov \"{sector}\" phase 1 OR phase 2", "grants"),
        ("Foundation Grants Database", "site:grantstation.com \"{sector}\" \"{location}\" grant", "grants"),
        
        // Accelerators
        ("Tech Accelerators", "(Y Combinator OR Techstars OR \"500 Global\") \"{sector}\" apply 2026", "accelerators"),
        ("University Startup Programs", "site:.edu accelerator OR startup-grant \"{sector}\"", "accelerators"),
        
        // Competitors
        ("Funded Competitors", "site:crunchbase.com \"{sector}\" funding 2025 2026 raised", "competitor_tech"),
        ("Recent Exits in Sector", "site:crunchbase.com \"{sector}\" (acquired OR IPO) 2025 2026", "competitor_tech"),
        
        // Founders
        ("Founder Profiles", "site:linkedin.com \"{name}\" founder CEO OR CTO \"{company}\"", "founder_profiles"),
        ("Founder Twitter", "site:twitter.com \"{name}\" founder -\"this account\"", "founder_profiles"),
    ]
}

// ============================================
// ADVANCED SEARCH STRATEGIES
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DorksSearchCampaign {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub objective: String,        // "find_investors", "find_grants", "competitor_analysis", "market_research"
    pub target_keywords: Vec<String>,
    pub location_focus: Option<String>,
    pub sector_focus: Vec<String>,
    pub queries: Vec<GoogleDorksQuery>,
    pub results_found: u32,
    pub contacts_created: u32,
    pub status: String,           // active, paused, completed
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSearchCampaignInput {
    pub name: String,
    pub objective: String,
    pub target_keywords: Vec<String>,
    pub location_focus: Option<String>,
    pub sector_focus: Option<Vec<String>>,
    pub auto_generate_queries: Option<bool>,
}

// ============================================
// BATCH PROCESSING
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DorksBatchJob {
    pub id: String,
    pub user_id: String,
    pub queries: Vec<String>,
    pub status: String,           // pending, running, completed, failed
    pub total_results: u32,
    pub progress_percent: u32,
    pub results_file_url: Option<String>,
    pub started_at: String,
    pub estimated_completion: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DorksSearchStats {
    pub total_queries_run: u32,
    pub total_results_found: u32,
    pub average_results_per_query: f32,
    pub top_result_domains: Vec<(String, u32)>,
    pub results_by_type: std::collections::HashMap<String, u32>,
    pub avg_search_time_seconds: f32,
}
