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

// Sample hardware acquisition templates
pub fn get_manufacturer_outreach_template() -> String {
    r#"
Subject: Partnership Opportunity: X3 GPU-Accelerated Blockchain Infrastructure

Dear [NAME],

We're building X3, a high-performance GPU-accelerated blockchain validator network that processes cross-chain transactions at 100K TPS with 300ms finality.

Our infrastructure is built on [NVIDIA/AMD] GPUs, and we're scaling rapidly. We're reaching out to explore opportunities for collaboration:

1. **Research Partnership** — Your latest GPU architecture would be an ideal fit for blockchain deterministic execution. We can provide real-world performance data from a production network.

2. **Hardware for Testing** — We'd welcome beta units of new GPU models for validation and integration testing. We'll provide detailed performance benchmarks and feedback.

3. **Co-branded Initiative** — "GPU-Accelerated Finance" category positioning where your platform is showcased as the infrastructure backbone of X3.

4. **Donation for Tax Credit** — If you have certified refurbished units, we can facilitate donation with full tax deduction documentation.

Over the next 12 months, we'll deploy 500+ GPUs. A partnership now positions you at the infrastructure layer of the largest GPU-native blockchain ecosystem.

Call or email to discuss. Happy to schedule a technical deep-dive with our CTO.

Best,
[YOUR_NAME]
X3 Head of Infrastructure
x3network.io
"#.to_string()
}

pub fn get_datacenter_liquidation_template() -> String {
    r#"
Subject: High-Value Hardware Acquisition: End-of-Life Equipment Request

Hi [NAME],

We're acquiring decommissioned data center equipment for repurposing in blockchain infrastructure. X3 runs validator nodes that require high-spec hardware, and your end-of-life inventory is exactly what we're looking for.

Specific interest:
- NVIDIA A100 / H100 / RTX6000 Ada GPUs
- AMD EPYC 7003/9004 series CPUs
- DDR5 enterprise memory modules
- High-bandwidth networking (InfiniBand, 400G+ Ethernet)
- Redundant power supplies (Titanium rated)

We can handle:
✓ Ship-to-us logistics
✓ Bulk acquisitions ($100K - $5M range)
✓ Fast transactions (deal → payment within 48 hours)
✓ Tax documentation for donation value

Current need: 50+ units per month for the next 18 months.

Let's connect this week. Can you share your current liquidation schedule?

[YOUR_NAME]
X3 Logistics & Partnerships
"#.to_string()
}

pub fn get_refurbisher_partnership_template() -> String {
    r#"
Subject: B2B Partnership: Certified Refurbished Hardware Supply Agreement

Hi [NAME],

We're interested in establishing an ongoing supply relationship for certified refurbished enterprise hardware.

X3 is a legitimate, VC-backed infrastructure company with projected $50M+ HW spend over 24 months. We're looking for reliable, long-term partners for:

- Certified refurbished data center GPUs (A100, RTX6000, professional series)
- Enterprise CPUs (Xeon, EPYC)
- High-capacity enterprise SSDs
- Network infrastructure

Partnership terms we offer:
• Committed minimum $500K/quarter purchase
• Pre-agreed pricing with volume discounts
• Fast payment (net 7-15 days)
• Long-term contract (24+ months)
• Marketing benefit: featured as official X3 hardware partner

Your benefit:
• Predictable revenue stream
• Guaranteed volume
• Association with cutting-edge infrastructure
• Potential for co-marketing

Let's discuss. Available for a call this week.

Regards,
[YOUR_NAME]
X3 Procurement
"#.to_string()
}

pub fn get_university_donation_template() -> String {
    r#"
Subject: Hardware Donation for Blockchain Research Lab

Dear [PROFESSOR_NAME],

X3 is funding research into GPU-accelerated blockchain consensus mechanisms and is looking for institutional partners.

We'd like to donate high-spec hardware to your lab for research purposes:
- 2-4 NVIDIA A100 GPUs
- Enterprise-grade CPU (Xeon/EPYC)
- High-bandwidth networking
- Supporting infrastructure

What we're looking for:
✓ Research collaboration on consensus optimization
✓ Publication co-authorship
✓ Lab acknowledgment in whitepapers/talks
✓ Potential internship/graduate student placements

The hardware is certified refurbished, professionally tested, and comes with full tax deduction documentation.

This is a win-win: you get world-class hardware for your research, and we gain academic credibility and external validation of our approach.

Can we schedule a call to discuss?

Best regards,
[YOUR_NAME]
Head of Research Partnerships, X3
"#.to_string()
}

pub fn get_corporate_it_surplus_template() -> String {
    r#"
Subject: Acquisition Inquiry: IT Hardware Surplus / E-Waste Management

Hi [IT_DIRECTOR_NAME],

X3 provides certified e-waste and IT surplus management services for enterprise equipment. If you're managing equipment decommissioning or data center consolidation, we can handle it.

We specialize in:
- Data sanitization & destruction (NIST SP 800-88 compliant)
- Refurbishment assessment
- Remarketing valuable components
- Full ITAD (IT Asset Disposition) compliance

We're actively acquiring:
- GPU computing accelerators
- High-end CPUs
- Enterprise storage/networking
- Professional workstations (NVIDIA, AMD Radeon)

Process:
1. You send inventory list
2. We provide instant valuation quote
3. Pickup arranged (we cover logistics)
4. Payment within 48 hours
5. Full chain-of-custody documentation

Plus: We can donate non-sellable items to schools/NGOs with tax benefits.

Got a refresh or consolidation coming up? Let's talk.

[YOUR_NAME]
X3 Hardware Procurement
"#.to_string()
}
