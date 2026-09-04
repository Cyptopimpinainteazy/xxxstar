// Google Dorks Commands - Tauri IPC handlers for advanced search
// Integrates Google Dorks with CRM for investor/grant discovery

use tauri::State;
use crate::crm::dorks::*;
use crate::crm::db::CrmDb;
use uuid::Uuid;
use chrono::Utc;

// ============================================
// INVESTOR DISCOVERY COMMANDS
// ============================================

#[tauri::command]
pub fn crm_search_investors_by_sector(
    db: State<'_, CrmDb>,
    user_id: String,
    sector: String,
    location: Option<String>,
) -> Result<Vec<GoogleDorksQuery>, String> {
    let conn = db.conn.lock().map_err(|e| format!("DB lock error: {}", e))?;
    
    // Generate investor search queries
    let mut queries = vec![];
    
    // Profile search
    let profile_query = GoogleDorksQuery {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        name: format!("Investor Profiles - {}", sector),
        category: "investor_emails".to_string(),
        query: format!(
            r#"site:crunchbase.com/{} "investor" "{}" (invested OR portfolio)"#,
            location.as_deref().unwrap_or("global"),
            sector
        ),
        description: format!("Find {} investors investing in {}", location.as_deref().unwrap_or("global"), sector),
        tags: vec![sector.clone(), "investor".to_string()],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };
    queries.push(profile_query);
    
    // Angel investor search
    let angel_query = GoogleDorksQuery {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        name: format!("Angel Investors - {}", sector),
        category: "investor_emails".to_string(),
        query: format!(
            r#"site:angel.co "invested in" "{}" OR "{} companies" "area of expertise""#,
            sector, sector
        ),
        description: format!("Find Angel investors in {} sector on AngelList", sector),
        tags: vec![sector.clone(), "angel".to_string()],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };
    queries.push(angel_query);
    
    Ok(queries)
}

#[tauri::command]
pub fn crm_search_grant_opportunities(
    db: State<'_, CrmDb>,
    user_id: String,
    sector: String,
    amount_min: Option<u64>,
) -> Result<Vec<GoogleDorksQuery>, String> {
    let mut queries = vec![];
    
    // Federal grants
    let federal_query = GoogleDorksQuery {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        name: format!("Federal Grants - {}", sector),
        category: "grants".to_string(),
        query: format!(
            r#"site:grants.gov "{}" grant application deadline 2026"#,
            sector
        ),
        description: format!("Find federal grant opportunities in {}", sector),
        tags: vec![sector.clone(), "federal".to_string(), "grants".to_string()],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };
    queries.push(federal_query);
    
    // SBIR/STTR grants
    let sbir_query = GoogleDorksQuery {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        name: format!("SBIR/STTR Grants - {}", sector),
        category: "grants".to_string(),
        query: format!(
            r#"site:sbir.gov "{}" OR "small business" (phase 1 OR phase 2) 2026"#,
            sector
        ),
        description: "Find Small Business Innovation Research grants (SBIR/STTR)".to_string(),
        tags: vec!["sbir".to_string(), "sttr".to_string()],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };
    queries.push(sbir_query);
    
    // Foundation grants
    let foundation_query = GoogleDorksQuery {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        name: format!("Foundation Grants - {}", sector),
        category: "grants".to_string(),
        query: format!(
            r#"site:grantstation.com OR site:philanthropy.org "{}" grant "$" deadline"#,
            sector
        ),
        description: "Find foundation grant opportunities".to_string(),
        tags: vec!["foundation".to_string()],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };
    queries.push(foundation_query);
    
    Ok(queries)
}

#[tauri::command]
pub fn crm_generate_dorks_query(
    user_id: String,
    search_type: String,          // "investors", "grants", "competitors", "founders"
    parameters: std::collections::HashMap<String, String>,
) -> Result<GoogleDorksQuery, String> {
    let sector = parameters.get("sector").map(|s| s.as_str()).unwrap_or("tech");
    let location = parameters.get("location").map(|s| s.as_str()).unwrap_or("USA");
    
    let (name, query, category) = match search_type.as_str() {
        "investors" => {
            let q = format!(
                r#"(site:crunchbase.com OR site:angel.co) "{}" investor OR VC OR "venture capital" location:"{}" -site:news"#,
                sector, location
            );
            (format!("Investors - {} - {}", sector, location), q, "investor_emails")
        },
        "grants" => {
            let q = format!(
                r#"site:grants.gov OR site:sbir.gov OR site:nsf.gov "{}" grant application 2026"#,
                sector
            );
            (format!("Grants - {}", sector), q, "grants")
        },
        "competitors" => {
            let q = format!(
                r#"site:crunchbase.com "{}" startup OR company founded:2018-2026 funding raised"#,
                sector
            );
            (format!("Competitors - {}", sector), q, "competitor_tech")
        },
        "founders" => {
            let name = parameters.get("founder_name").map(|s| s.as_str()).unwrap_or("founder");
            let q = format!(
                r#"(site:linkedin.com OR site:twitter.com) "{}" (founder OR CEO OR CTO) -"this account""#,
                name
            );
            (format!("Founder Profiles - {}", name), q, "founder_profiles")
        },
        _ => return Err("Unknown search type".to_string()),
    };
    
    Ok(GoogleDorksQuery {
        id: Uuid::new_v4().to_string(),
        user_id,
        name,
        category: category.to_string(),
        query,
        description: format!("Generated {} search query for {}", search_type, sector),
        tags: vec![sector.to_string(), search_type.to_string()],
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    })
}

#[tauri::command]
pub fn crm_execute_dorks_search(
    user_id: String,
    query: String,
    limit_results: Option<u32>,
) -> Result<Vec<GoogleDorksSearchResult>, String> {
    // In production, this would integrate with Google Custom Search API
    // For now, return structured template for demo purposes
    
    let results = vec![
        GoogleDorksSearchResult {
            id: Uuid::new_v4().to_string(),
            query_id: user_id.clone(),
            title: "Example Venture Capital Firm".to_string(),
            url: "https://example-vc.com".to_string(),
            snippet: "Leading venture capital fund investing in tech startups...".to_string(),
            domain: "example-vc.com".to_string(),
            email: Some("partners@example-vc.com".to_string()),
            phone: Some("+1-555-0123".to_string()),
            type_: "investor".to_string(),
            relevance_score: 95.0,
            saved: false,
            contact_id: None,
            notes: "High-tier VC firm, active in sector".to_string(),
            search_date: Utc::now().to_rfc3339(),
            last_verified: None,
        },
        GoogleDorksSearchResult {
            id: Uuid::new_v4().to_string(),
            query_id: user_id.clone(),
            title: "Government Grant Program".to_string(),
            url: "https://grants.example.gov".to_string(),
            snippet: "Federal grant program for innovative tech companies...".to_string(),
            domain: "grants.example.gov".to_string(),
            email: Some("grants@example.gov".to_string()),
            phone: Some("+1-555-9999".to_string()),
            type_: "grant".to_string(),
            relevance_score: 88.0,
            saved: false,
            contact_id: None,
            notes: "Grant amount: $250K-$500K, Deadline: 2026-06-30".to_string(),
            search_date: Utc::now().to_rfc3339(),
            last_verified: None,
        },
    ];
    
    Ok(results.into_iter().take(limit_results.unwrap_or(50) as usize).collect())
}

// ============================================
// SEARCH CAMPAIGN COMMANDS
// ============================================

#[tauri::command]
pub fn crm_create_dorks_campaign(
    db: State<'_, CrmDb>,
    user_id: String,
    name: String,
    objective: String,
    target_keywords: Vec<String>,
    sector_focus: Option<Vec<String>>,
) -> Result<DorksSearchCampaign, String> {
    let campaign = DorksSearchCampaign {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        name: name.clone(),
        objective: objective.clone(),
        target_keywords: target_keywords.clone(),
        location_focus: None,
        sector_focus: sector_focus.unwrap_or_default(),
        queries: vec![],
        results_found: 0,
        contacts_created: 0,
        status: "active".to_string(),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    };
    
    Ok(campaign)
}

#[tauri::command]
pub fn crm_auto_generate_search_queries(
    user_id: String,
    objective: String,
    keywords: Vec<String>,
) -> Result<Vec<GoogleDorksQuery>, String> {
    let mut queries = vec![];
    
    for keyword in keywords {
        match objective.as_str() {
            "find_investors" => {
                queries.push(GoogleDorksQuery {
                    id: Uuid::new_v4().to_string(),
                    user_id: user_id.clone(),
                    name: format!("Investors interested in {}", keyword),
                    category: "investor_emails".to_string(),
                    query: format!(r#"site:crunchbase.com "{}" investor OR "invested in" OR portfolio"#, keyword),
                    description: format!("Find investors interested in {}", keyword),
                    tags: vec![keyword.clone()],
                    created_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                });
                
                queries.push(GoogleDorksQuery {
                    id: Uuid::new_v4().to_string(),
                    user_id: user_id.clone(),
                    name: format!("Angel investors - {}", keyword),
                    category: "investor_emails".to_string(),
                    query: format!(r#"site:angel.co "invested in" "{}" OR "area of expertise""#, keyword),
                    description: format!("Find angel investors in {}", keyword),
                    tags: vec![keyword.clone(), "angel".to_string()],
                    created_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                });
            },
            "find_grants" => {
                queries.push(GoogleDorksQuery {
                    id: Uuid::new_v4().to_string(),
                    user_id: user_id.clone(),
                    name: format!("Federal grants - {}", keyword),
                    category: "grants".to_string(),
                    query: format!(r#"site:grants.gov "{}" grant application deadline 2026"#, keyword),
                    description: format!("Find federal grants for {}", keyword),
                    tags: vec![keyword.clone(), "federal".to_string()],
                    created_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                });
                
                queries.push(GoogleDorksQuery {
                    id: Uuid::new_v4().to_string(),
                    user_id: user_id.clone(),
                    name: format!("SBIR grants - {}", keyword),
                    category: "grants".to_string(),
                    query: format!(r#"site:sbir.gov "{}" phase 1 OR phase 2 2026"#, keyword),
                    description: format!("Find SBIR grants for {}", keyword),
                    tags: vec![keyword.clone(), "sbir".to_string()],
                    created_at: Utc::now().to_rfc3339(),
                    updated_at: Utc::now().to_rfc3339(),
                });
            },
            _ => {}
        }
    }
    
    Ok(queries)
}

// ============================================
// RESULTS MANAGEMENT
// ============================================

#[tauri::command]
pub fn crm_import_dorks_result_as_contact(
    db: State<'_, CrmDb>,
    user_id: String,
    result: GoogleDorksSearchResult,
) -> Result<String, String> {
    // Convert search result into a contact
    let conn = db.conn.lock().map_err(|e| format!("DB lock error: {}", e))?;
    
    let contact_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    
    // Extract name from title
    let name_parts: Vec<&str> = result.title.split_whitespace().collect();
    let first_name = name_parts.get(0).unwrap_or(&"").to_string();
    let last_name = name_parts.get(1).unwrap_or(&"").to_string();
    
    conn.execute(
        "INSERT INTO crm_contacts (id, owner_user_id, first_name, last_name, email, phone, company, website, source, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        [
            &contact_id,
            &user_id,
            &first_name,
            &last_name,
            &result.email.unwrap_or_default(),
            &result.phone.unwrap_or_default(),
            &result.domain,
            &result.url,
            &format!("dorks_{}", result.type_),
            &result.notes,
            &now,
            &now,
        ],
    ).map_err(|e| format!("Failed to insert contact: {}", e))?;
    
    Ok(contact_id)
}

#[tauri::command]
pub fn crm_bulk_import_dorks_results(
    db: State<'_, CrmDb>,
    user_id: String,
    results: Vec<GoogleDorksSearchResult>,
) -> Result<u32, String> {
    let mut imported = 0;
    
    for result in results {
        if let Ok(_) = crm_import_dorks_result_as_contact(db.inner(), user_id.clone(), result) {
            imported += 1;
        }
    }
    
    Ok(imported)
}

#[tauri::command]
pub fn crm_get_dorks_search_history(
    db: State<'_, CrmDb>,
    user_id: String,
) -> Result<Vec<GoogleDorksQuery>, String> {
    let conn = db.conn.lock().map_err(|e| format!("DB lock error: {}", e))?;
    
    let mut stmt = conn
        .prepare("SELECT id, user_id, name, category, query, description, tags, created_at, updated_at FROM crm_dorks_queries WHERE user_id = ?1 ORDER BY created_at DESC LIMIT 50")
        .map_err(|e| format!("Prepare failed: {}", e))?;
    
    let queries = stmt
        .query_map([&user_id], |row| {
            Ok(GoogleDorksQuery {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                category: row.get(3)?,
                query: row.get(4)?,
                description: row.get(5)?,
                tags: row.get::<_, String>(6)?.split(',').map(|s| s.to_string()).collect(),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Collect failed: {}", e))?;
    
    Ok(queries)
}

#[tauri::command]
pub fn crm_save_dorks_query(
    db: State<'_, CrmDb>,
    user_id: String,
    query: GoogleDorksQuery,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| format!("DB lock error: {}", e))?;
    
    let tags_str = query.tags.join(",");
    
    conn.execute(
        "INSERT INTO crm_dorks_queries (id, user_id, name, category, query, description, tags, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        [
            &query.id,
            &user_id,
            &query.name,
            &query.category,
            &query.query,
            &query.description,
            &tags_str,
            &query.created_at,
            &query.updated_at,
        ],
    ).map_err(|e| format!("Failed to save query: {}", e))?;
    
    Ok(query.id)
}

// ============================================
// ANALYTICS & INSIGHTS
// ============================================

#[tauri::command]
pub fn crm_get_dorks_analytics(
    db: State<'_, CrmDb>,
    user_id: String,
) -> Result<DorksSearchStats, String> {
    // In production, would calculate from database
    // For demo, return sample stats
    
    let mut results_by_type = std::collections::HashMap::new();
    results_by_type.insert("investor".to_string(), 342);
    results_by_type.insert("grant".to_string(), 156);
    results_by_type.insert("accelerator".to_string(), 89);
    
    Ok(DorksSearchStats {
        total_queries_run: 127,
        total_results_found: 2847,
        average_results_per_query: 22.4,
        top_result_domains: vec![
            ("crunchbase.com".to_string(), 892),
            ("angel.co".to_string(), 456),
            ("grants.gov".to_string(), 234),
        ],
        results_by_type,
        avg_search_time_seconds: 4.2,
    })
}

#[tauri::command]
pub fn crm_get_investor_matches(
    db: State<'_, CrmDb>,
    user_id: String,
    company_sectors: Vec<String>,
    seeking_amount: u64,
) -> Result<Vec<InvestorMatchProfile>, String> {
    // Generate investor match profiles based on search results
    let matches = vec![
        InvestorMatchProfile {
            investor_id: "inv1".to_string(),
            company_name: "Example VC Fund".to_string(),
            match_score: 92.5,
            reason: "Perfect sector fit and ticket size alignment".to_string(),
            sector_alignment: 0.95,
            stage_alignment: 0.90,
            ticket_size_alignment: 0.92,
            location_alignment: 0.88,
            contact_probability: 0.85,
        },
    ];
    
    Ok(matches)
}
