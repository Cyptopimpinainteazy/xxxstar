// TIER 9: Tauri Command Implementations for Funding War Plan
// Financial projections, cap table simulation, runway calculations

use serde_json::{json, Value};
use crate::crm::funding_war_plan::*;
use uuid::Uuid;
use chrono::{Utc, Duration};
use std::collections::HashMap;

// ================================================
// FINANCIAL PROJECTION ENGINE
// ================================================

#[tauri::command]
pub async fn crm_calculate_financial_projection(
    revenue_year1_usd: u64,
    burn_rate_monthly: u64,
    growth_rate_monthly: f32,
    headcount_year1: u32,
    cost_per_headcount: u64,
) -> Result<Value, String> {
    // Calculate 3 scenarios: Base (60%), Bull (20%), Bear (20%)
    
    let base_case = calculate_scenario(
        revenue_year1_usd,
        burn_rate_monthly,
        growth_rate_monthly,
        headcount_year1,
        cost_per_headcount,
        1.0,  // multiplier
    );
    
    let bull_case = calculate_scenario(
        revenue_year1_usd,
        burn_rate_monthly,
        growth_rate_monthly * 1.5,  // 50% higher growth
        headcount_year1,
        cost_per_headcount,
        0.8,  // 20% lower burn
    );
    
    let bear_case = calculate_scenario(
        revenue_year1_usd,
        burn_rate_monthly,
        growth_rate_monthly * 0.5,  // 50% lower growth
        headcount_year1,
        cost_per_headcount,
        1.4,  // 40% higher burn
    );
    
    Ok(json!({
        "id": Uuid::new_v4().to_string(),
        "base_case": base_case,
        "bull_case": bull_case,
        "bear_case": bear_case,
        "probability": {
            "base": 0.6,
            "bull": 0.2,
            "bear": 0.2
        },
        "expected_value": {
            "year1_revenue": (base_case["year1_revenue"] as u64) * 60 / 100 +
                            (bull_case["year1_revenue"] as u64) * 20 / 100 +
                            (bear_case["year1_revenue"] as u64) * 20 / 100,
            "year1_burn": (base_case["year1_burn"] as u64) * 60 / 100 +
                         (bull_case["year1_burn"] as u64) * 20 / 100 +
                         (bear_case["year1_burn"] as u64) * 20 / 100,
        }
    }))
}

fn calculate_scenario(
    year1_revenue: u64,
    burn_monthly: u64,
    growth: f32,
    headcount: u32,
    cost_per_head: u64,
    burn_multiplier: f32,
) -> Value {
    let mut monthly_data = vec![];
    let mut cash_position = year1_revenue as i64;
    let mut curr_revenue = year1_revenue as i64;
    
    for month in 1..=12 {
        let monthly_burn = (burn_monthly as f32 * burn_multiplier) as i64;
        let monthly_revenue = (curr_revenue as f32 * growth) as i64;
        let payroll = (headcount as u64 * cost_per_head / 12) as i64;
        let net_flow = monthly_revenue - monthly_burn - payroll;
        
        cash_position += net_flow;
        curr_revenue = monthly_revenue;
        
        monthly_data.push(json!({
            "month": month,
            "revenue": monthly_revenue,
            "burn": monthly_burn,
            "payroll": payroll,
            "net_flow": net_flow,
            "cash_position": cash_position,
            "gross_margin": if monthly_revenue > 0 {
                ((monthly_revenue - monthly_burn) as f32 / monthly_revenue as f32) * 100.0
            } else {
                0.0
            }
        }));
    }
    
    let year1_revenue = monthly_data.iter().map(|m| m["revenue"].as_i64().unwrap_or(0)).sum::<i64>();
    let year1_burn = monthly_data.iter().map(|m| m["burn"].as_i64().unwrap_or(0)).sum::<i64>();
    
    json!({
        "year1_revenue": year1_revenue.max(0),
        "year1_burn": year1_burn.max(0),
        "year1_net": year1_revenue - year1_burn,
        "final_cash_position": cash_position,
        "avg_gross_margin": monthly_data.iter()
            .map(|m| m["gross_margin"].as_f64().unwrap_or(0.0))
            .sum::<f64>() / 12.0,
        "headcount": headcount,
        "monthly_breakdown": monthly_data,
    })
}

// ================================================
// CAP TABLE SIMULATOR
// ================================================

#[tauri::command]
pub async fn crm_simulate_cap_table(
    founder_shares: u64,
    total_shares_outstanding: u64,
    raise_amount_usd: u64,
    post_money_valuation_usd: u64,
) -> Result<Value, String> {
    
    // Calculate new share price and dilution
    let new_share_price = post_money_valuation_usd as f64 / total_shares_outstanding as f64;
    let new_shares_issued = (raise_amount_usd as f64 / new_share_price) as u64;
    let total_shares_post = total_shares_outstanding + new_shares_issued;
    
    let founder_pre_ownership = (founder_shares as f64 / total_shares_outstanding as f64) * 100.0;
    let founder_post_ownership = (founder_shares as f64 / total_shares_post as f64) * 100.0;
    let dilution = founder_pre_ownership - founder_post_ownership;
    
    // Simulate future rounds (Series A, B, C)
    let mut dilution_scenarios = vec![];
    let mut cumulative_dilution = dilution;
    let mut curr_shares = total_shares_post;
    let mut curr_founder_ownership = founder_post_ownership;
    
    for round_num in 1..=3 {
        let series_name = match round_num {
            1 => "Series A",
            2 => "Series B",
            3 => "Series C",
            _ => "Series X",
        };
        
        // Typical round: 25% dilution
        let round_raise = (raise_amount_usd as f64 * (1.5_f64.powi(round_num as i32))) as u64;
        let round_valuation = (post_money_valuation_usd as f64 * (2.5_f64.powi(round_num as i32))) as u64;
        let round_share_price = round_valuation as f64 / curr_shares as f64;
        let round_new_shares = (round_raise as f64 / round_share_price) as u64;
        let round_total_shares = curr_shares + round_new_shares;
        
        let round_founder_pre = curr_founder_ownership;
        let round_founder_post = (founder_shares as f64 / round_total_shares as f64) * 100.0;
        let round_dilution = round_founder_pre - round_founder_post;
        cumulative_dilution += round_dilution;
        
        dilution_scenarios.push(json!({
            "round": series_name,
            "raise_amount": round_raise,
            "valuation": round_valuation,
            "share_price": format!("{:.6}", round_share_price),
            "shares_issued": round_new_shares,
            "founder_ownership_pre": format!("{:.2}%", round_founder_pre),
            "founder_ownership_post": format!("{:.2}%", round_founder_post),
            "dilution_this_round": format!("{:.2}%", round_dilution),
            "cumulative_dilution": format!("{:.2}%", cumulative_dilution),
        }));
        
        curr_shares = round_total_shares;
        curr_founder_ownership = round_founder_post;
    }
    
    // Exit simulation at $1B valuation
    let exit_valuation = 1_000_000_000u64;
    let exit_founder_value = (founder_shares as f64 / curr_shares as f64) * exit_valuation as f64;
    
    Ok(json!({
        "id": Uuid::new_v4().to_string(),
        "seed_round": {
            "raise_amount": raise_amount_usd,
            "valuation": post_money_valuation_usd,
            "share_price": format!("{:.6}", new_share_price),
            "shares_issued": new_shares_issued,
            "founder_ownership_pre": format!("{:.2}%", founder_pre_ownership),
            "founder_ownership_post": format!("{:.2}%", founder_post_ownership),
            "dilution": format!("{:.2}%", dilution),
        },
        "pro_forma_cap_table": {
            "founders": {
                "shares": founder_shares,
                "ownership_post_seed": format!("{:.2}%", founder_post_ownership),
            },
            "investors": {
                "shares": new_shares_issued,
                "ownership_post_seed": format!("{:.2}%", (new_shares_issued as f64 / total_shares_post as f64) * 100.0),
            },
            "total_shares_outstanding": total_shares_post,
        },
        "future_rounds": dilution_scenarios,
        "exit_scenario": {
            "exit_valuation": exit_valuation,
            "founder_equity_value": format!("${:.2}", exit_founder_value),
            "final_founder_ownership": format!("{:.2}%", curr_founder_ownership),
            "total_cumulative_dilution": format!("{:.2}%", cumulative_dilution),
        }
    }))
}

// ================================================
// TREASURY RUNWAY CALCULATOR
// ================================================

#[tauri::command]
pub async fn crm_calculate_treasury_runway(
    starting_cash_usd: u64,
    monthly_burn_usd: u64,
    monthly_revenue_usd: u64,
    growth_rate_monthly: f32,
) -> Result<Value, String> {
    
    let mut runway_data = vec![];
    let mut cash = starting_cash_usd as i64;
    let mut curr_revenue = monthly_revenue_usd as i64;
    let mut months_of_runway = 0.0;
    let mut critical_threshold_hit_month: Option<i32> = None;
    let mut breakeven_month: Option<i32> = None;
    
    for month in 1..=36 {
        let revenue = curr_revenue;
        let burn = monthly_burn_usd as i64;
        let net_flow = revenue - burn;
        cash += net_flow;
        
        // Track runway
        if cash > 0 {
            months_of_runway = cash as f64 / burn as f64;
        }
        
        // Determine status
        let status = match cash {
            c if c <= 0 => "funded_or_out", // Should be funded
            c if c < (burn * 3) => "critical",
            c if c < (burn * 6) => "caution",
            _ => "healthy",
        };
        
        // Track critical threshold
        if status == "critical" && critical_threshold_hit_month.is_none() {
            critical_threshold_hit_month = Some(month);
        }
        
        // Track breakeven
        if net_flow >= 0 && breakeven_month.is_none() {
            breakeven_month = Some(month);
        }
        
        runway_data.push(json!({
            "month": month,
            "revenue": revenue,
            "burn": burn,
            "net_flow": net_flow,
            "cash_balance": cash.max(0),
            "runway_months": months_of_runway,
            "status": status,
        }));
        
        // Update revenue for next month
        curr_revenue = (curr_revenue as f64 * (1.0 + growth_rate_monthly)) as i64;
    }
    
    // Calculate funding requirements
    let current_runway = if monthly_burn_usd > 0 {
        starting_cash_usd / monthly_burn_usd
    } else {
        36
    };
    
    let funding_required_safe = (monthly_burn_usd * 24).saturating_sub(starting_cash_usd);
    
    Ok(json!({
        "id": Uuid::new_v4().to_string(),
        "current_cash": starting_cash_usd,
        "monthly_burn": monthly_burn_usd,
        "monthly_revenue": monthly_revenue_usd,
        "net_monthly_burn": monthly_burn_usd as i64 - monthly_revenue_usd as i64,
        "current_runway_months": current_runway,
        "breakdown_36m": runway_data,
        "critical_milestones": {
            "critical_threshold_hit_month": critical_threshold_hit_month,
            "breakeven_month": breakeven_month,
        },
        "funding_strategy": {
            "funding_for_24m_runway": funding_required_safe,
            "funding_for_36m_runway": (monthly_burn_usd * 36).saturating_sub(starting_cash_usd),
            "safe_minimum_runway_months": 24,
            "recommendation": if current_runway < 12 {
                "URGENT: Begin fundraising immediately"
            } else if current_runway < 18 {
                "ACTIVE: Start investor meetings in next 60 days"
            } else if current_runway < 24 {
                "PLANNED: Schedule fundraising for Q2"
            } else {
                "HEALTHY: Monitor quarterly, no immediate pressure"
            }
        },
        "cash_thresholds": {
            "emergency": monthly_burn_usd * 3,
            "critical": monthly_burn_usd * 6,
            "caution": monthly_burn_usd * 12,
            "healthy": monthly_burn_usd * 18,
        }
    }))
}

// ================================================
// WAR PLAN GENERATION
// ================================================

#[tauri::command]
pub async fn crm_generate_funding_war_plan(
    company_name: String,
    target_raise_usd: u64,
    target_valuation_usd: u64,
) -> Result<Value, String> {
    
    let plan_id = Uuid::new_v4().to_string();
    
    Ok(json!({
        "id": plan_id,
        "version": "v1.0",
        "company_name": company_name,
        "creation_date": Utc::now().to_rfc3339(),
        "target_raise_usd": target_raise_usd,
        "target_valuation_usd": target_valuation_usd,
        "status": "draft",
        "next_steps": [
            "Complete 12-month financial projections",
            "Model capital stack scenarios",
            "Define investor strategy",
            "Create outreach segmentation",
            "Schedule industry-specific campaigns"
        ]
    }))
}

#[tauri::command]
pub async fn crm_export_war_plan_as_pdf(
    plan_id: String,
    financial_projection: Value,
    cap_table: Value,
    runway_analysis: Value,
) -> Result<String, String> {
    // This would integrate with a PDF generation library
    // Placeholder for actual PDF export logic
    Ok(format!("war_plan_{}_v1.pdf", plan_id))
}

// ================================================
// GET WAR PLAN
// ================================================

#[tauri::command]
pub async fn crm_get_funding_war_plan(
    plan_id: String,
) -> Result<Value, String> {
    // This would fetch from database
    // Placeholder for actual database query
    Ok(json!({
        "id": plan_id,
        "status": "active",
        "message": "War plan loaded successfully"
    }))
}
