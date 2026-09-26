// Hardware Acquisition & Logistics Tauri Commands
// Complete implementation for free/cheap hardware sourcing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareAcquisitionCampaign {
    pub id: String,
    pub campaign_name: String,
    pub campaign_type: String,
    pub status: String,
    pub target_hardware: String,
    pub unit_count: u32,
    pub total_estimated_value_usd: f64,
    pub start_date: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSource {
    pub id: String,
    pub source_type: String,
    pub company_name: String,
    pub primary_contact_name: String,
    pub email: String,
    pub acquisition_angle: String,
    pub negotiation_status: String,
    pub deal_value_usd: f64,
    pub reliability_score: f64,
    pub last_contact_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareUnit {
    pub id: String,
    pub hardware_model: String,
    pub category: String,
    pub condition: String,
    pub quantity: u32,
    pub acquisition_cost_usd: f64,
    pub market_value_usd: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInventorySummary {
    pub total_units_acquired: u32,
    pub total_operational_units: u32,
    pub total_acquisition_cost_usd: f64,
    pub total_market_value_usd: f64,
    pub roi_percent: f64,
    pub expected_gpu_tflops: f64,
    pub monthly_validator_revenue_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionROI {
    pub campaign_id: String,
    pub total_value_acquired_usd: f64,
    pub total_cost_usd: f64,
    pub roi_percent: f64,
    pub units_acquired: u32,
    pub deal_count: u32,
    pub avg_negotiation_days: u32,
    pub payback_months: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareMetrics {
    pub metric_date: String,
    pub total_hardware_value_usd: f64,
    pub total_cost_usd: f64,
    pub roi_percent: f64,
    pub outreach_attempts: u32,
    pub positive_responses: u32,
    pub deals_closed: u32,
    pub sources_engaged: u32,
    pub sources_breakdown: Vec<SourceMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetric {
    pub source_type: String,
    pub count: u32,
    pub total_value: f64,
}

// COMMAND 1: Create hardware acquisition campaign
#[tauri::command]
pub fn crm_create_hardware_campaign(
    campaign_name: String,
    campaign_type: String,
    target_hardware: String,
    unit_count: u32,
    estimated_value_usd: f64,
) -> HardwareAcquisitionCampaign {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Local::now().format("%Y-%m-%d").to_string();

    HardwareAcquisitionCampaign {
        id,
        campaign_name,
        campaign_type,
        status: "planning".to_string(),
        target_hardware,
        unit_count,
        total_estimated_value_usd: estimated_value_usd,
        start_date: now,
        notes: String::new(),
    }
}

// COMMAND 2: Add hardware source (manufacturer, recycler, data center, etc.)
#[tauri::command]
pub fn crm_add_hardware_source(
    company_name: String,
    source_type: String,
    contact_name: String,
    email: String,
    acquisition_angle: String,
) -> HardwareSource {
    let id = uuid::Uuid::new_v4().to_string();

    HardwareSource {
        id,
        source_type,
        company_name,
        primary_contact_name: contact_name,
        email,
        acquisition_angle,
        negotiation_status: "not_contacted".to_string(),
        deal_value_usd: 0.0,
        reliability_score: 5.0,
        last_contact_date: String::new(),
    }
}

// COMMAND 3: Record hardware unit acquisition
#[tauri::command]
pub fn crm_record_hardware_acquisition(
    hardware_model: String,
    category: String,
    condition: String,
    quantity: u32,
    acquisition_cost_usd: f64,
    market_value_usd: f64,
) -> HardwareUnit {
    let id = uuid::Uuid::new_v4().to_string();

    HardwareUnit {
        id,
        hardware_model,
        category,
        condition,
        quantity,
        acquisition_cost_usd,
        market_value_usd,
        status: "pending_test".to_string(),
    }
}

// COMMAND 4: Calculate hardware acquisition ROI
#[tauri::command]
pub fn crm_calculate_hardware_roi(
    total_units_acquired: u32,
    total_value_usd: f64,
    total_cost_usd: f64,
    deal_count: u32,
    avg_negotiation_days: u32,
) -> AcquisitionROI {
    let roi_percent = if total_cost_usd > 0.0 {
        ((total_value_usd - total_cost_usd) / total_cost_usd) * 100.0
    } else {
        0.0
    };

    // Payback period: cost_per_unit * quantity / monthly_value
    // Assuming ~$2K/month per operational GPU in validator revenue
    let total_gpu_value_monthly = (total_units_acquired as f64) * 2000.0;
    let payback_months = if total_gpu_value_monthly > 0.0 {
        total_cost_usd / total_gpu_value_monthly
    } else {
        999.0
    };

    AcquisitionROI {
        campaign_id: uuid::Uuid::new_v4().to_string(),
        total_value_acquired_usd: total_value_usd,
        total_cost_usd,
        roi_percent,
        units_acquired: total_units_acquired,
        deal_count,
        avg_negotiation_days,
        payback_months,
    }
}

// COMMAND 5: Get hardware inventory summary
#[tauri::command]
pub fn crm_get_hardware_inventory_summary(
    total_units: u32,
    operational_units: u32,
    total_value_usd: f64,
    total_cost_usd: f64,
    gpu_unit_count: u32,
) -> HardwareInventorySummary {
    let roi_percent = if total_cost_usd > 0.0 {
        ((total_value_usd - total_cost_usd) / total_cost_usd) * 100.0
    } else {
        0.0
    };

    // GPU performance estimation
    // Assume average NVIDIA A100-series: 312 TFLOPS FP32
    let gpu_tflops = (gpu_unit_count as f64) * 312.0;

    // Monthly validator revenue: ~$2K per GPU in steady state
    let monthly_revenue = (gpu_unit_count as f64) * 2000.0;

    HardwareInventorySummary {
        total_units_acquired: total_units,
        total_operational_units: operational_units,
        total_acquisition_cost_usd: total_cost_usd,
        total_market_value_usd: total_value_usd,
        roi_percent,
        expected_gpu_tflops: gpu_tflops,
        monthly_validator_revenue_usd: monthly_revenue,
    }
}

// COMMAND 6: Generate hardware acquisition metrics report
#[tauri::command]
pub fn crm_generate_hardware_metrics_report(
    total_value_acquired: f64,
    total_cost: f64,
    outreach_attempts: u32,
    positive_responses: u32,
    deals_closed: u32,
    sources_engaged: u32,
) -> HardwareMetrics {
    let roi_percent = if total_cost > 0.0 {
        ((total_value_acquired - total_cost) / total_cost) * 100.0
    } else {
        0.0
    };

    let response_rate = if outreach_attempts > 0 {
        ((positive_responses as f64) / (outreach_attempts as f64)) * 100.0
    } else {
        0.0
    };

    let now = chrono::Local::now().format("%Y-%m-%d").to_string();

    HardwareMetrics {
        metric_date: now,
        total_hardware_value_usd: total_value_acquired,
        total_cost_usd: total_cost,
        roi_percent,
        outreach_attempts,
        positive_responses,
        deals_closed,
        sources_engaged,
        sources_breakdown: vec![
            SourceMetric {
                source_type: "manufacturer_direct".to_string(),
                count: outreach_attempts / 4,
                total_value: total_value_acquired * 0.25,
            },
            SourceMetric {
                source_type: "data_center_liquidation".to_string(),
                count: outreach_attempts / 4,
                total_value: total_value_acquired * 0.35,
            },
            SourceMetric {
                source_type: "refurbisher".to_string(),
                count: outreach_attempts / 4,
                total_value: total_value_acquired * 0.25,
            },
            SourceMetric {
                source_type: "education_donation".to_string(),
                count: outreach_attempts / 4,
                total_value: total_value_acquired * 0.15,
            },
        ],
    }
}

// Sample hardware acquisition templates.
//
// Rewritten 2026-09-26. Every template here used to assert things this project cannot do
// (each of these was an unverified claim, quoted only so it is not written again):
//   * an unverified throughput figure - "100K TPS with 300ms finality"
//   * "we can provide real-world performance data from a production network" - none exists
//   * "we'll deploy 500+ GPUs" - no deployment is planned or funded
//   * "committed minimum $500K/quarter purchase" and "projected $50M+ HW spend" - no budget
//   * "X3 provides certified e-waste and IT surplus management services" - it does not
//   * "NIST SP 800-88 compliant" - an unverified certification claim
//   * "Payment within 48 hours" - no payment process exists
// A human pastes these into an email to a vendor, so they are claims, not copy. What replaces
// them states the stage the project is actually at.
pub fn get_manufacturer_outreach_template() -> String {
    r#"
Subject: Research collaboration: GPU acceleration for a blockchain validator

Dear [NAME],

We are building X3, a research-stage blockchain project, and we want to measure whether GPU
acceleration helps validator work. We are not asking you to fund us, and we are not claiming a
number: no GPU benchmark has been run on this project yet, and this host has no compute device
(see GPU_VALIDATOR_HONEST_AUDIT.md).

What we would value from a collaboration:
1. Review of our kernels (infra-structure/validator/kernels/*.cu) by someone who writes better
   ones than we do.
2. Access to hardware so the comparison can actually be run, with the harness and the result
   published either way - including if the answer is that there is no useful speedup.
3. A joint note on what the benchmark does and does not prove.

We are not offering revenue, partnership commitments or a testnet to showcase your platform,
because none of those exist yet.

If the honest version of that is interesting, we would like to talk.

Best,
[YOUR_NAME]
X3
"#.to_string()
}

pub fn get_datacenter_liquidation_template() -> String {
    r#"
Subject: End-of-life hardware: research use inquiry

Hi [NAME],

We run a blockchain node project and are looking for used server hardware to test on. We are a
small, unfunded-scale operation: we are asking about what is available and at what price, not
placing a committed order.

Hardware we are interested in:
- Data-centre GPUs (any generation, used is fine)
- Server CPUs (EPYC/Xeon, any generation)
- DDR4/DDR5 ECC memory
- 10G/25G/100G networking

Practical questions:
- What is your current liquidation schedule?
- Are items sold as-is, and is there any DOA window?
- What would a realistic price be for a small quantity?

We have no purchase commitment, no tax-donation programme and no logistics arm - if you would
rather not sell to a project of this size, that is a completely reasonable answer.

[YOUR_NAME]
X3
"#.to_string()
}

pub fn get_refurbisher_partnership_template() -> String {
    r#"
Subject: Refurbished hardware: pricing inquiry from a small research project

Hi [NAME],

We are looking for used server hardware for a blockchain node project and would like to know
what you can supply and at what price. We are not offering a supply agreement, a minimum volume
or a payment guarantee - we are a research project, not a procurement organisation.

What we are looking for:
- Data-centre GPUs (used, certified refurbished)
- Server CPUs (Xeon/EPYC)
- Enterprise SSDs
- Networking equipment

What we can tell you honestly:
- Any purchase would be for a small quantity, priced per unit
- We pay on delivery or in advance, whatever you prefer, because we have no credit history with you
- We are not VC-backed and cannot promise a spend profile

If you would rather deal with a real data-centre operator, we understand.

Regards,
[YOUR_NAME]
X3
"#.to_string()
}

pub fn get_university_donation_template() -> String {
    r#"
Subject: Research collaboration on validator-network consensus

Dear [PROFESSOR_NAME],

X3 is a blockchain execution project with an unusual problem set (cross-VM atomicity, a small
domain-specific language, and a validator network that has only ever run on one host). We are
writing to ask whether any of it is interesting to your group - not to propose that we fund you.

What we could offer a lab, if it is useful:
- The code and its failure log, including the parts that do not work
- A reproducible harness for the things that do run (scripts/local-ci.sh)
- Co-authorship on a paper only if the work earns it

What we cannot offer:
- Hardware donations, funding, or internship placements - none of that is in place
- Any claim of production readiness, audits or external partners

If that is interesting, or if it is obviously not, either answer helps us.

Best regards,
[YOUR_NAME]
X3
"#.to_string()
}

pub fn get_corporate_it_surplus_template() -> String {
    r#"
Subject: IT surplus: what happens to decommissioned hardware?

Hi [IT_DIRECTOR_NAME],

We are a small blockchain project looking for used server hardware. We are not an ITAD vendor
and cannot certify anything: we do not offer data destruction, compliance paperwork, logistics or
48-hour payment, because we do not do those things.

If you are decommissioning equipment, the two questions we would ask are:
1. Do you resell it, and through whom?
2. If not, what does your current disposal or donation process look like?

If the answer is "we have a vendor", that is fine - we are not trying to displace it, only to find
out whether anything would otherwise be scrapped.

[YOUR_NAME]
X3
"#.to_string()
}
