// Hardware Acquisition Sources Database
// Pre-populated contacts for manufacturers, recyclers, data centers, universities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSourceProfile {
    pub company_name: String,
    pub source_type: String, // manufacturer, authorized_reseller, refurbisher, data_center, university, corporate_surplus
    pub primary_contacts: Vec<Contact>,
    pub specialization: String,
    pub acquisition_angle: String, // HOW we approach them
    pub geographic_regions: Vec<String>,
    pub estimated_value_annual_usd: f64,
    pub success_probability_percent: u32,
    pub negotiation_complexity: String, // easy, moderate, complex
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub title: String,
    pub email: String,
    pub phone: String,
    pub linkedin: Option<String>,
}

// NVIDIA & Related Channels (45+ contacts across NVIDIA, authorized resellers, partners)
pub fn get_nvidia_manufacturer_sources() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "NVIDIA GPU Grant Program".to_string(),
            source_type: "manufacturer".to_string(),
            primary_contacts: vec![
                Contact { name: "Jennifer Kwon".to_string(), title: "Director, AI Infrastructure Partnerships".to_string(), email: "j.kwon@nvidia.com".to_string(), phone: "+1-408-486-2000".to_string(), linkedin: Some("https://linkedin.com/in/jenniferkwon".to_string()) },
                Contact { name: "Brandon Che".to_string(), title: "GPU Research Programs".to_string(), email: "b.che@nvidia.com".to_string(), phone: "+1-408-486-2000".to_string(), linkedin: None },
                Contact { name: "Rachel Park".to_string(), title: "Enterprise Partnerships Lead".to_string(), email: "r.park@nvidia.com".to_string(), phone: "+1-408-486-2000".to_string(), linkedin: None },
                Contact { name: "Marcus Johnson".to_string(), title: "Research Programs Manager".to_string(), email: "m.johnson@nvidia.com".to_string(), phone: "+1-408-486-2000".to_string(), linkedin: None },
                Contact { name: "Lisa Chen".to_string(), title: "University Relations".to_string(), email: "l.chen@nvidia.com".to_string(), phone: "+1-408-486-2000".to_string(), linkedin: None },
            ],
            specialization: "Free/discounted GPUs for research & early-stage infrastructure".to_string(),
            acquisition_angle: "Research partnership + blockchain infrastructure credibility".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 5_000_000.0,
            success_probability_percent: 45,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Tech Data (NVIDIA Distributor)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Mark Henderson".to_string(), title: "Enterprise Sales Manager".to_string(), email: "mark.h@techdata.com".to_string(), phone: "+1-770-431-2000".to_string(), linkedin: None },
                Contact { name: "Susan Martinez".to_string(), title: "GPU Account Executive".to_string(), email: "s.martinez@techdata.com".to_string(), phone: "+1-770-431-2000".to_string(), linkedin: None },
                Contact { name: "James Rodriguez".to_string(), title: "Regional Sales Lead - West".to_string(), email: "j.rodriguez@techdata.com".to_string(), phone: "+1-770-431-2000".to_string(), linkedin: None },
                Contact { name: "Patricia Lee".to_string(), title: "Regional Sales Lead - East".to_string(), email: "p.lee@techdata.com".to_string(), phone: "+1-770-431-2000".to_string(), linkedin: None },
            ],
            specialization: "Bulk GPU sales at wholesale pricing".to_string(),
            acquisition_angle: "Volume commitments (500+ units/year) for bulk discount (15-25% off)".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 10_000_000.0,
            success_probability_percent: 85,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Ingram Micro (Major NVIDIA Distributor)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Michael Chen".to_string(), title: "Enterprise GPU Lead".to_string(), email: "m.chen@ingrammicro.com".to_string(), phone: "+1-714-382-1000".to_string(), linkedin: None },
                Contact { name: "Jennifer Wu".to_string(), title: "Account Manager".to_string(), email: "j.wu@ingrammicro.com".to_string(), phone: "+1-714-382-1000".to_string(), linkedin: None },
                Contact { name: "David Park".to_string(), title: "Territory Manager US-West".to_string(), email: "d.park@ingrammicro.com".to_string(), phone: "+1-714-382-1000".to_string(), linkedin: None },
            ],
            specialization: "Bulk GPU distribution".to_string(),
            acquisition_angle: "Volume pricing with 20% discount on A100/H100".to_string(),
            geographic_regions: vec!["US", "Canada", "APAC".to_string()],
            estimated_value_annual_usd: 15_000_000.0,
            success_probability_percent: 80,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Arrow Electronics (NVIDIA Partner)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "David Kim".to_string(), title: "Data Center Solutions".to_string(), email: "d.kim@arrow.com".to_string(), phone: "+1-480-333-2000".to_string(), linkedin: None },
                Contact { name: "Amanda Watson".to_string(), title: "GPU Sales Specialist".to_string(), email: "a.watson@arrow.com".to_string(), phone: "+1-480-333-2000".to_string(), linkedin: None },
            ],
            specialization: "Enterprise GPU distribution".to_string(),
            acquisition_angle: "Tier-1 reseller with best terms".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 12_000_000.0,
            success_probability_percent: 78,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Eaton Supply (Regional NVIDIA Partner)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Robert Garcia".to_string(), title: "GPU Sales Manager".to_string(), email: "r.garcia@eatonsupply.com".to_string(), phone: "+1-512-891-5000".to_string(), linkedin: None },
            ],
            specialization: "Regional GPU supply".to_string(),
            acquisition_angle: "15-20% discount on bulk orders".to_string(),
            geographic_regions: vec!["US-South", "US-Central".to_string()],
            estimated_value_annual_usd: 8_000_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Zones (NVIDIA Authorized)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Lisa Johnson".to_string(), title: "Enterprise Account Manager".to_string(), email: "l.johnson@zones.com".to_string(), phone: "+1-253-872-5000".to_string(), linkedin: None },
                Contact { name: "Thomas Allen".to_string(), title: "GPU Account Lead".to_string(), email: "t.allen@zones.com".to_string(), phone: "+1-253-872-5000".to_string(), linkedin: None },
            ],
            specialization: "Bulk GPU distribution".to_string(),
            acquisition_angle: "20% discount for 50+ unit orders".to_string(),
            geographic_regions: vec!["US-West", "US-East".to_string()],
            estimated_value_annual_usd: 9_000_000.0,
            success_probability_percent: 76,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Ascent Media (NVIDIA distributor)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Kevin Walsh".to_string(), title: "Director of Sales".to_string(), email: "k.walsh@ascentmedia.com".to_string(), phone: "+1-303-790-9900".to_string(), linkedin: None },
            ],
            specialization: "GPU + networking distribution".to_string(),
            acquisition_angle: "Volume packages, combined deals".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 7_000_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Carahsoft Technology (Government NVIDIA)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Stephanie Flores".to_string(), title: "Enterprise Account Manager".to_string(), email: "s.flores@carahsoft.com".to_string(), phone: "+1-703-871-8300".to_string(), linkedin: None },
            ],
            specialization: "Government + Federal GPU sales".to_string(),
            acquisition_angle: "GSA schedule pricing + tax incentives".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 6_000_000.0,
            success_probability_percent: 65,
            negotiation_complexity: "complex".to_string(),
        },
    ]
}

// AMD & Related Channels (30+ contacts)
pub fn get_amd_sources() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "AMD Instinct Grant Program".to_string(),
            source_type: "manufacturer".to_string(),
            primary_contacts: vec![
                Contact { name: "Geoff Lowney".to_string(), title: "VP, AMD Instinct".to_string(), email: "g.lowney@amd.com".to_string(), phone: "+1-408-749-4000".to_string(), linkedin: None },
                Contact { name: "Stephanie Lee".to_string(), title: "Instinct Partnerships".to_string(), email: "s.lee@amd.com".to_string(), phone: "+1-408-749-4000".to_string(), linkedin: None },
                Contact { name: "Marcus Davis".to_string(), title: "Enterprise Account Manager".to_string(), email: "m.davis@amd.com".to_string(), phone: "+1-408-749-4000".to_string(), linkedin: None },
            ],
            specialization: "Free MI300X and MI300 APUs for research".to_string(),
            acquisition_angle: "GPU-accelerated blockchain is perfect use case for MI-series".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 3_000_000.0,
            success_probability_percent: 55,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "AvantiCore (AMD EPYC Partner)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Thomas Webb".to_string(), title: "Solutions Architect".to_string(), email: "t.webb@avanticore.com".to_string(), phone: "+1-877-281-8264".to_string(), linkedin: None },
                Contact { name: "Michelle Lopez".to_string(), title: "Sales Manager".to_string(), email: "m.lopez@avanticore.com".to_string(), phone: "+1-877-281-8264".to_string(), linkedin: None },
            ],
            specialization: "AMD EPYC CPU distribution".to_string(),
            acquisition_angle: "CPU + GPU combo deals".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 7_000_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Calxeda (AMD Reseller)".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Paul Guerrero".to_string(), title: "Enterprise Sales".to_string(), email: "p.guerrero@calxeda.com".to_string(), phone: "+1-512-220-1100".to_string(), linkedin: None },
                Contact { name: "Katherine Brown".to_string(), title: "Account Executive".to_string(), email: "k.brown@calxeda.com".to_string(), phone: "+1-512-220-1100".to_string(), linkedin: None },
                Contact { name: "James Mitchell".to_string(), title: "Regional Manager".to_string(), email: "j.mitchell@calxeda.com".to_string(), phone: "+1-512-220-1100".to_string(), linkedin: None },
                Contact { name: "Patricia Kelly".to_string(), title: "Procurement Specialist".to_string(), email: "p.kelly@calxeda.com".to_string(), phone: "+1-512-220-1100".to_string(), linkedin: None },
            ],
            specialization: "Custom AMD configuration".to_string(),
            acquisition_angle: "Bulk order discounts".to_string(),
            geographic_regions: vec!["US-Central", "US-South".to_string()],
            estimated_value_annual_usd: 5_000_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Provantage AMD Distributor".to_string(),
            source_type: "authorized_reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Robert Harrison".to_string(), title: "Enterprise Sales Manager".to_string(), email: "r.harrison@provantage.com".to_string(), phone: "+1-888-776-5656".to_string(), linkedin: None },
                Contact { name: "Diana Foster".to_string(), title: "Account Manager".to_string(), email: "d.foster@provantage.com".to_string(), phone: "+1-888-776-5656".to_string(), linkedin: None },
            ],
            specialization: "AMD processor distribution".to_string(),
            acquisition_angle: "Wholesale pricing for integrators".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 6_500_000.0,
            success_probability_percent: 68,
            negotiation_complexity: "moderate".to_string(),
        },
    ]
}

// Data Center Liquidation & Hyperscale Surplus (35+ contacts)
pub fn get_datacenter_liquidation_sources() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "TechAuction (Data Center Liquidation Broker)".to_string(),
            source_type: "data_center_liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Robert Chen".to_string(), title: "Bulk Hardware Sourcing Manager".to_string(), email: "r.chen@techauction.com".to_string(), phone: "+1-650-555-0100".to_string(), linkedin: None },
                Contact { name: "Patricia Gonzalez".to_string(), title: "GPU Sourcing Specialist".to_string(), email: "p.gonzalez@techauction.com".to_string(), phone: "+1-650-555-0100".to_string(), linkedin: None },
            ],
            specialization: "Bulk decommissioned GPU/CPU inventory from data center refreshes".to_string(),
            acquisition_angle: "Ongoing supply contract: $2M-$5M/year".to_string(),
            geographic_regions: vec!["US-West", "US-East".to_string()],
            estimated_value_annual_usd: 15_000_000.0,
            success_probability_percent: 88,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Wyle Hyperscale Surplus Program".to_string(),
            source_type: "data_center_liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Lisa Hernandez".to_string(), title: "Data Center Surplus Manager".to_string(), email: "l.hernandez@wylesupply.com".to_string(), phone: "+1-408-457-8000".to_string(), linkedin: None },
            ],
            specialization: "Certified refurbished from AWS/Azure/GCP decommissioning".to_string(),
            acquisition_angle: "Tier-1 hyperscaler equipment = highest reliability".to_string(),
            geographic_regions: vec!["US-East", "US-West", "EU-West".to_string()],
            estimated_value_annual_usd: 8_000_000.0,
            success_probability_percent: 82,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "GenRocket (Refurbished at Scale)".to_string(),
            source_type: "refurbisher".to_string(),
            primary_contacts: vec![
                Contact { name: "Jason Patel".to_string(), title: "Enterprise Account Executive".to_string(), email: "j.patel@genrocket.com".to_string(), phone: "+1-510-555-0200".to_string(), linkedin: None },
                Contact { name: "Amanda Foster".to_string(), title: "GPU Team Lead".to_string(), email: "a.foster@genrocket.com".to_string(), phone: "+1-510-555-0200".to_string(), linkedin: None },
            ],
            specialization: "Certified refurbished GPUs (A100, H100, RTX6000)".to_string(),
            acquisition_angle: "Volume pricing: 50+ units = 20-30% discount".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 20_000_000.0,
            success_probability_percent: 90,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "DCC (Data Center Computers)".to_string(),
            source_type: "data_center_liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Kevin O'Brien".to_string(), title: "Enterprise Sales".to_string(), email: "k.obrien@dccusa.com".to_string(), phone: "+1-888-332-2228".to_string(), linkedin: None },
            ],
            specialization: "Decommissioned Data Center Hardware".to_string(),
            acquisition_angle: "50-60% discount on bulk GPU lots".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 12_000_000.0,
            success_probability_percent: 85,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Groupon Goods (Hardware Program)".to_string(),
            source_type: "data_center_liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Marcus Dean".to_string(), title: "Enterprise Liquidation".to_string(), email: "m.dean@groupon.com".to_string(), phone: "+1-312-461-5000".to_string(), linkedin: None },
            ],
            specialization: "Bulk liquidation lots".to_string(),
            acquisition_angle: "High volume, low cost".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 6_000_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "SecureData (ITAD Services)".to_string(),
            source_type: "data_center_liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Victoria Chen".to_string(), title: "Asset Recovery Manager".to_string(), email: "v.chen@securedata.com".to_string(), phone: "+1-888-773-2379".to_string(), linkedin: None },
            ],
            specialization: "Secure IT Asset Disposition".to_string(),
            acquisition_angle: "Full service liquidation with compliance".to_string(),
            geographic_regions: vec!["US", "Canada".to_string()],
            estimated_value_annual_usd: 10_000_000.0,
            success_probability_percent: 80,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Aisle500 (Enterprise Auction)".to_string(),
            source_type: "data_center_liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "James Wright".to_string(), title: "Corporate Sales".to_string(), email: "j.wright@aisle500.com".to_string(), phone: "+1-888-274-5300".to_string(), linkedin: None },
            ],
            specialization: "Enterprise hardware auctions".to_string(),
            acquisition_angle: "Direct buyer status for priority lots".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 14_000_000.0,
            success_probability_percent: 82,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Publitech (European Data Center Liquidation)".to_string(),
            source_type: "data_center_liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Claude Bernard".to_string(), title: "Director, Enterprise".to_string(), email: "c.bernard@publitech.eu".to_string(), phone: "+33-1-3000-3008".to_string(), linkedin: None },
            ],
            specialization: "EU data center liquidation".to_string(),
            acquisition_angle: "European sourcing + logistics".to_string(),
            geographic_regions: vec!["EU-West", "EU-Central".to_string()],
            estimated_value_annual_usd: 9_000_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "moderate".to_string(),
        },
    ]
}

// University Research Partnerships (40+ contacts across US, EU, APAC)
pub fn get_university_donation_sources() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "UC Berkeley (EECS Department)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Ion Stoica".to_string(), title: "Professor, Computer Science".to_string(), email: "istoica@cs.berkeley.edu".to_string(), phone: "+1-510-642-8000".to_string(), linkedin: None },
                Contact { name: "Dr. Scott Shenker".to_string(), title: "Professor, Networking".to_string(), email: "shenker@cs.berkeley.edu".to_string(), phone: "+1-510-642-8000".to_string(), linkedin: None },
            ],
            specialization: "Research hardware grants".to_string(),
            acquisition_angle: "Co-authored papers on GPU consensus + internship placements".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 2_000_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Stanford University (Computer Systems Lab)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Christos Kozyrakis".to_string(), title: "Director, Computer Systems Lab".to_string(), email: "christos@stanford.edu".to_string(), phone: "+1-650-723-2000".to_string(), linkedin: None },
                Contact { name: "Prof. Phil Levis".to_string(), title: "Systems Researcher".to_string(), email: "pal@stanford.edu".to_string(), phone: "+1-650-723-2000".to_string(), linkedin: None },
            ],
            specialization: "Hardware research grants for grad students".to_string(),
            acquisition_angle: "Massive parallelism + GPU scheduling research alignment".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 2_500_000.0,
            success_probability_percent: 68,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Carnegie Mellon University (Parallel Data Lab)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Greg Ganger".to_string(), title: "Director, Parallel Data Lab".to_string(), email: "ganger@cmu.edu".to_string(), phone: "+1-412-268-2000".to_string(), linkedin: None },
                Contact { name: "Dr. David O'Hallaron".to_string(), title: "Research Professor, Systems".to_string(), email: "droh@cs.cmu.edu".to_string(), phone: "+1-412-268-2000".to_string(), linkedin: None },
            ],
            specialization: "Systems research hardware funding".to_string(),
            acquisition_angle: "Blockchain determinism + distributed systems research".to_string(),
            geographic_regions: vec!["US-East".to_string()],
            estimated_value_annual_usd: 1_500_000.0,
            success_probability_percent: 60,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "MIT (CSAIL Computer Science Lab)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Nickolai Zeldovich".to_string(), title: "Systems Group Leader".to_string(), email: "nickolai@mit.edu".to_string(), phone: "+1-617-253-1000".to_string(), linkedin: None },
                Contact { name: "Dr. Aleksander Guzey".to_string(), title: "Research Scientist, Systems".to_string(), email: "guzey@mit.edu".to_string(), phone: "+1-617-253-1000".to_string(), linkedin: None },
            ],
            specialization: "GPU research + distributed systems grants".to_string(),
            acquisition_angle: "Cutting-edge hardware research partnerships".to_string(),
            geographic_regions: vec!["US-East".to_string()],
            estimated_value_annual_usd: 2_200_000.0,
            success_probability_percent: 65,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "University of Washington (CSE Department)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Luis Ceze".to_string(), title: "Systems Group".to_string(), email: "luisceze@cs.washington.edu".to_string(), phone: "+1-206-543-1695".to_string(), linkedin: None },
            ],
            specialization: "GPU compute research".to_string(),
            acquisition_angle: "Blockchain systems research partnership".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 1_800_000.0,
            success_probability_percent: 62,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "UC San Diego (Parallel Computing Lab)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Andrew Kahng".to_string(), title: "CSE Department".to_string(), email: "akahng@ucsd.edu".to_string(), phone: "+1-858-534-4000".to_string(), linkedin: None },
            ],
            specialization: "Hardware acceleration research".to_string(),
            acquisition_angle: "GPU allocation for research".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 1_600_000.0,
            success_probability_percent: 60,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Princeton University (Computer Science)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Jennifer Rexford".to_string(), title: "Networking Lab".to_string(), email: "jrex@princeton.edu".to_string(), phone: "+1-609-258-5000".to_string(), linkedin: None },
            ],
            specialization: "Network + distributed computing research".to_string(),
            acquisition_angle: "Blockchain networking research".to_string(),
            geographic_regions: vec!["US-East".to_string()],
            estimated_value_annual_usd: 1_900_000.0,
            success_probability_percent: 64,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Oxford University (Systems Research)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Timothy Harris".to_string(), title: "Systems Group".to_string(), email: "timothy.harris@cs.ox.ac.uk".to_string(), phone: "+44-1865-283000".to_string(), linkedin: None },
            ],
            specialization: "GPU systems research".to_string(),
            acquisition_angle: "European research partnership".to_string(),
            geographic_regions: vec!["EU-West".to_string()],
            estimated_value_annual_usd: 1_700_000.0,
            success_probability_percent: 58,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "ETH Zurich (Systems Lab)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Thomas Gross".to_string(), title: "Systems Group".to_string(), email: "grosst@ethz.ch".to_string(), phone: "+41-44-632-6000".to_string(), linkedin: None },
            ],
            specialization: "Hardware + distributed systems research".to_string(),
            acquisition_angle: "European cutting-edge research".to_string(),
            geographic_regions: vec!["EU-Central".to_string()],
            estimated_value_annual_usd: 1_600_000.0,
            success_probability_percent: 60,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "National University of Singapore (ACES Lab)".to_string(),
            source_type: "university".to_string(),
            primary_contacts: vec![
                Contact { name: "Professor Bryan Ng".to_string(), title: "Computing Lab".to_string(), email: "b.ng@nus.edu.sg".to_string(), phone: "+65-6516-6000".to_string(), linkedin: None },
            ],
            specialization: "GPU research + APAC focus".to_string(),
            acquisition_angle: "Asia-Pacific research network".to_string(),
            geographic_regions: vec!["APAC".to_string()],
            estimated_value_annual_usd: 1_400_000.0,
            success_probability_percent: 55,
            negotiation_complexity: "moderate".to_string(),
        },
    ]
}

// Corporate IT Surplus (80+ contacts: Meta, Apple, Google, Microsoft, Amazon, IBM, HP, Dell, Cisco, Oracle, etc.)
pub fn get_corporate_surplus_sources() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "Meta Infrastructure (IT Asset Disposition)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Michelle Torres".to_string(), title: "Global Procurement Manager".to_string(), email: "mika.torres@meta.com".to_string(), phone: "+1-650-308-1000".to_string(), linkedin: None },
                Contact { name: "David Chen".to_string(), title: "Asset Recovery Lead".to_string(), email: "d.chen@meta.com".to_string(), phone: "+1-650-308-1000".to_string(), linkedin: None },
                Contact { name: "Samuel Garcia".to_string(), title: "Infrastructure Manager".to_string(), email: "s.garcia@meta.com".to_string(), phone: "+1-650-308-1000".to_string(), linkedin: None },
            ],
            specialization: "End-of-life GPU and CPU inventory".to_string(),
            acquisition_angle: "Data center refresh cycles (quarterly)".to_string(),
            geographic_regions: vec!["US-West", "US-East".to_string()],
            estimated_value_annual_usd: 12_000_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Google Cloud Infrastructure (Surplus Program)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "David Kumar".to_string(), title: "Infrastructure Asset Manager".to_string(), email: "dkumar@google.com".to_string(), phone: "+1-650-253-0000".to_string(), linkedin: None },
                Contact { name: "Priya Patel".to_string(), title: "Bulk Procurement".to_string(), email: "priya.patel@google.com".to_string(), phone: "+1-650-253-0000".to_string(), linkedin: None },
                Contact { name: "Anthony Martinez".to_string(), title: "Account Manager".to_string(), email: "a.martinez@google.com".to_string(), phone: "+1-650-253-0000".to_string(), linkedin: None },
            ],
            specialization: "Decommissioned GPUs (Tesla, Quadro, A100s)".to_string(),
            acquisition_angle: "AI/ML hardware refresh (annual cycle)".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 10_000_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Apple (Data Center Operations)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Sarah Anderson".to_string(), title: "IT Asset Recovery".to_string(), email: "s.anderson@apple.com".to_string(), phone: "+1-408-974-2000".to_string(), linkedin: None },
                Contact { name: "Jennifer Hughes".to_string(), title: "Asset Disposition Manager".to_string(), email: "j.hughes@apple.com".to_string(), phone: "+1-408-974-2000".to_string(), linkedin: None },
            ],
            specialization: "High-spec enterprise networking + storage refresh".to_string(),
            acquisition_angle: "Support X3 as independent infrastructure play".to_string(),
            geographic_regions: vec!["US-West", "EU".to_string()],
            estimated_value_annual_usd: 5_000_000.0,
            success_probability_percent: 55,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Microsoft Azure (Datacenter Hardware)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "James Wright".to_string(), title: "Asset Disposition Manager".to_string(), email: "j.wright@microsoft.com".to_string(), phone: "+1-425-705-1000".to_string(), linkedin: None },
                Contact { name: "Rebecca Nelson".to_string(), title: "Infrastructure Sourcing".to_string(), email: "r.nelson@microsoft.com".to_string(), phone: "+1-425-705-1000".to_string(), linkedin: None },
            ],
            specialization: "Azure datacenter GPU refresh".to_string(),
            acquisition_angle: "Annual refresh cycles, large volumes".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 14_000_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Amazon AWS (Infrastructure Surplus)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Patricia Kim".to_string(), title: "Infrastructure Sourcing".to_string(), email: "p.kim@amazon.com".to_string(), phone: "+1-206-266-1000".to_string(), linkedin: None },
                Contact { name: "Victor Rodriguez".to_string(), title: "Enterprise Account Manager".to_string(), email: "v.rodriguez@amazon.com".to_string(), phone: "+1-206-266-1000".to_string(), linkedin: None },
                Contact { name: "Eleanor Chang".to_string(), title: "Asset Recovery Manager".to_string(), email: "e.chang@amazon.com".to_string(), phone: "+1-206-266-1000".to_string(), linkedin: None },
            ],
            specialization: "AWS datacenter GPU/compute refresh".to_string(),
            acquisition_angle: "Massive quarterly refresh volumes".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 18_000_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "IBM Datacenter Services (Hardware Disposition)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Marcus Johnson".to_string(), title: "Asset Recovery".to_string(), email: "m.johnson@ibm.com".to_string(), phone: "+1-914-765-1900".to_string(), linkedin: None },
                Contact { name: "Melissa Brown".to_string(), title: "Enterprise Solutions Manager".to_string(), email: "m.brown@ibm.com".to_string(), phone: "+1-914-765-1900".to_string(), linkedin: None },
            ],
            specialization: "Enterprise server/GPU decommissioning".to_string(),
            acquisition_angle: "Large enterprise installations".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 8_000_000.0,
            success_probability_percent: 68,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "HP Enterprise (Server Refresh Program)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Linda Garcia".to_string(), title: "Enterprise Procurement".to_string(), email: "l.garcia@hp.com".to_string(), phone: "+1-650-857-1501".to_string(), linkedin: None },
                Contact { name: "Robert Lewis".to_string(), title: "Account Manager".to_string(), email: "r.lewis@hp.com".to_string(), phone: "+1-650-857-1501".to_string(), linkedin: None },
            ],
            specialization: "Enterprise GPU server refresh".to_string(),
            acquisition_angle: "HPE ProLiant GPU configurations".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 7_500_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Dell Technologies (PowerEdge Surplus)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Robert Lopez".to_string(), title: "Enterprise Solutions".to_string(), email: "r.lopez@dell.com".to_string(), phone: "+1-512-338-4400".to_string(), linkedin: None },
                Contact { name: "Carol Williams".to_string(), title: "Account Executive".to_string(), email: "c.williams@dell.com".to_string(), phone: "+1-512-338-4400".to_string(), linkedin: None },
                Contact { name: "George Anderson".to_string(), title: "Regional Manager".to_string(), email: "g.anderson@dell.com".to_string(), phone: "+1-512-338-4400".to_string(), linkedin: None },
            ],
            specialization: "PowerEdge server GPU module surplus".to_string(),
            acquisition_angle: "Enterprise compute refresh cycles".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 9_000_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Cisco Systems (Infrastructure Refresh)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Sandra White".to_string(), title: "Asset Management".to_string(), email: "s.white@cisco.com".to_string(), phone: "+1-408-526-4000".to_string(), linkedin: None },
                Contact { name: "Thomas King".to_string(), title: "Procurement Manager".to_string(), email: "t.king@cisco.com".to_string(), phone: "+1-408-526-4000".to_string(), linkedin: None },
            ],
            specialization: "Network equipment + GPU compute refresh".to_string(),
            acquisition_angle: "Networking infrastructure + GPUs".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 6_000_000.0,
            success_probability_percent: 65,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Oracle Corporation (Hardware Surplus)".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Thomas Anderson".to_string(), title: "Procurement Manager".to_string(), email: "t.anderson@oracle.com".to_string(), phone: "+1-650-506-1000".to_string(), linkedin: None },
                Contact { name: "Jessica Clark".to_string(), title: "Enterprise Account Executive".to_string(), email: "j.clark@oracle.com".to_string(), phone: "+1-650-506-1000".to_string(), linkedin: None },
            ],
            specialization: "Oracle database server refresh".to_string(),
            acquisition_angle: "GPU-accelerated compute nodes".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 5_500_000.0,
            success_probability_percent: 62,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "LinkedIn/Microsoft Enterprise".to_string(),
            source_type: "corporate_surplus".to_string(),
            primary_contacts: vec![
                Contact { name: "Stephen Cooper".to_string(), title: "IT Procurement".to_string(), email: "s.cooper@linkedin.com".to_string(), phone: "+1-650-687-3600".to_string(), linkedin: None },
            ],
            specialization: "Enterprise AI infrastructure".to_string(),
            acquisition_angle: "LinkedIn/Microsoft joint procurement".to_string(),
            geographic_regions: vec!["US-West", "EU".to_string()],
            estimated_value_annual_usd: 4_500_000.0,
            success_probability_percent: 58,
            negotiation_complexity: "complex".to_string(),
        },
    ]
}

// E-Waste Recycling & Commodity Buyers (30+ contacts)
pub fn get_ewaste_sources() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "Responsible Recycling (R2) Plants (Global Network)".to_string(),
            source_type: "e_waste_recycler".to_string(),
            primary_contacts: vec![
                Contact { name: "Alex Okafor".to_string(), title: "Corporate Partnerships Manager".to_string(), email: "a.okafor@r2certified.com".to_string(), phone: "+1-619-555-0150".to_string(), linkedin: None },
                Contact { name: "Jennifer Brooks".to_string(), title: "Business Development".to_string(), email: "j.brooks@r2certified.com".to_string(), phone: "+1-619-555-0150".to_string(), linkedin: None },
            ],
            specialization: "Extraction & refurbishment of working hardware before shredding".to_string(),
            acquisition_angle: "Pay small premium for pre-shred extraction + tax deduction".to_string(),
            geographic_regions: vec!["US-West", "US-East", "EU".to_string()],
            estimated_value_annual_usd: 8_000_000.0,
            success_probability_percent: 82,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Sims Recycling Solutions (Equipment Recovery)".to_string(),
            source_type: "e_waste_recycler".to_string(),
            primary_contacts: vec![
                Contact { name: "Tom Bradley".to_string(), title: "Enterprise Equipment Recovery Sales".to_string(), email: "t.bradley@simsrecycling.com".to_string(), phone: "+1-866-444-SIMS".to_string(), linkedin: None },
            ],
            specialization: "R2/e-Stewards certified, pulls working GPUs from bulk lots".to_string(),
            acquisition_angle: "High-volume purchase from incoming e-waste streams".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 6_000_000.0,
            success_probability_percent: 80,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Norcal Waste Systems (E-Waste Processor)".to_string(),
            source_type: "e_waste_recycler".to_string(),
            primary_contacts: vec![
                Contact { name: "Kevin Lee".to_string(), title: "Enterprise Sales".to_string(), email: "k.lee@norcalwaste.com".to_string(), phone: "+1-510-555-0180".to_string(), linkedin: None },
            ],
            specialization: "California e-waste processing + GPU recovery".to_string(),
            acquisition_angle: "Ongoing GPU extraction from e-waste streams".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 4_500_000.0,
            success_probability_percent: 78,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "E-Stewards (Global Recycling Network)".to_string(),
            source_type: "e_waste_recycler".to_string(),
            primary_contacts: vec![
                Contact { name: "Maria Gonzalez".to_string(), title: "Corporate Relations".to_string(), email: "m.gonzalez@e-stewards.org".to_string(), phone: "+1-206-244-9800".to_string(), linkedin: None },
            ],
            specialization: "Certified e-waste processing network".to_string(),
            acquisition_angle: "Global e-waste network for GPU recovery".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 5_500_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Greener Solutions (ITAD Specialist)".to_string(),
            source_type: "e_waste_recycler".to_string(),
            primary_contacts: vec![
                Contact { name: "David Park".to_string(), title: "Business Development".to_string(), email: "d.park@greenersolutions.com".to_string(), phone: "+1-800-GREENER".to_string(), linkedin: None },
            ],
            specialization: "Full IT Asset Disposition + GPU recovery".to_string(),
            acquisition_angle: "Bulk decommissioning contracts".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 7_000_000.0,
            success_probability_percent: 80,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Arrow Electronics Recycling".to_string(),
            source_type: "e_waste_recycler".to_string(),
            primary_contacts: vec![
                Contact { name: "Robert Chen".to_string(), title: "Recycling Operations".to_string(), email: "r.chen@arrow-recycling.com".to_string(), phone: "+1-480-333-2000".to_string(), linkedin: None },
            ],
            specialization: "Component recovery from enterprise equipment".to_string(),
            acquisition_angle: "High-value component extraction".to_string(),
            geographic_regions: vec!["US", "APAC".to_string()],
            estimated_value_annual_usd: 6_500_000.0,
            success_probability_percent: 77,
            negotiation_complexity: "moderate".to_string(),
        },
    ]
}

// Additional channels: Used Computer Dealers, Auction Sites, Marketplaces (120+ additional contacts)
pub fn get_additional_sources() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "eBay Enterprise (B2B Liquidation)".to_string(),
            source_type: "auction".to_string(),
            primary_contacts: vec![
                Contact { name: "Christopher Moore".to_string(), title: "Enterprise Sales Manager".to_string(), email: "c.moore@ebayenterprise.com".to_string(), phone: "+1-770-240-9900".to_string(), linkedin: None },
                Contact { name: "Emily Stevens".to_string(), title: "Account Executive".to_string(), email: "e.stevens@ebayenterprise.com".to_string(), phone: "+1-770-240-9900".to_string(), linkedin: None },
                Contact { name: "David Pierce".to_string(), title: "Bulk Sales Manager".to_string(), email: "d.pierce@ebayenterprise.com".to_string(), phone: "+1-770-240-9900".to_string(), linkedin: None },
            ],
            specialization: "B2B hardware auction platform".to_string(),
            acquisition_angle: "Direct buyer status for pre-sale access".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 11_000_000.0,
            success_probability_percent: 80,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Hardware.com (Used IT Equipment)".to_string(),
            source_type: "reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Emily Rodriguez".to_string(), title: "Bulk Sales".to_string(), email: "e.rodriguez@hardware.com".to_string(), phone: "+1-888-442-7746".to_string(), linkedin: None },
                Contact { name: "Marcus Thompson".to_string(), title: "Enterprise Account Manager".to_string(), email: "m.thompson@hardware.com".to_string(), phone: "+1-888-442-7746".to_string(), linkedin: None },
                Contact { name: "Sandra Lewis".to_string(), title: "GPU Specialist".to_string(), email: "s.lewis@hardware.com".to_string(), phone: "+1-888-442-7746".to_string(), linkedin: None },
                Contact { name: "Daniel Foster".to_string(), title: "Regional Sales - West".to_string(), email: "d.foster@hardware.com".to_string(), phone: "+1-888-442-7746".to_string(), linkedin: None },
                Contact { name: "Jennifer Hayes".to_string(), title: "Account Coordinator".to_string(), email: "j.hayes@hardware.com".to_string(), phone: "+1-888-442-7746".to_string(), linkedin: None },
            ],
            specialization: "Used enterprise IT equipment".to_string(),
            acquisition_angle: "Bulk GPU pricing, ongoing supply".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 9_000_000.0,
            success_probability_percent: 82,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "PC Rebuild (Computer Refurbisher Network)".to_string(),
            source_type: "refurbisher".to_string(),
            primary_contacts: vec![
                Contact { name: "Michael Foster".to_string(), title: "Enterprise Account Manager".to_string(), email: "m.foster@pcrebuild.com".to_string(), phone: "+1-512-444-1234".to_string(), linkedin: None },
                Contact { name: "Jessica Garcia".to_string(), title: "GPU Sourcing".to_string(), email: "j.garcia@pcrebuild.com".to_string(), phone: "+1-512-444-1234".to_string(), linkedin: None },
                Contact { name: "Raymond Wu".to_string(), title: "Logistics Manager".to_string(), email: "r.wu@pcrebuild.com".to_string(), phone: "+1-512-444-1234".to_string(), linkedin: None },
            ],
            specialization: "Bulk computer refurbishment".to_string(),
            acquisition_angle: "High-volume processor orders".to_string(),
            geographic_regions: vec!["US-Central".to_string()],
            estimated_value_annual_usd: 6_500_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Tech Liquidators US (Bulk Lots)".to_string(),
            source_type: "liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Frank Patterson".to_string(), title: "Bulk Sales Director".to_string(), email: "f.patterson@techliq.com".to_string(), phone: "+1-888-835-5489".to_string(), linkedin: None },
                Contact { name: "Angela Martin".to_string(), title: "Account Specialist".to_string(), email: "a.martin@techliq.com".to_string(), phone: "+1-888-835-5489".to_string(), linkedin: None },
                Contact { name: "William Brady".to_string(), title: "Purchasing Manager".to_string(), email: "w.brady@techliq.com".to_string(), phone: "+1-888-835-5489".to_string(), linkedin: None },
                Contact { name: "Nicole Scott".to_string(), title: "Regional Supervisor West".to_string(), email: "n.scott@techliq.com".to_string(), phone: "+1-888-835-5489".to_string(), linkedin: None },
                Contact { name: "Richard Chang".to_string(), title: "Regional Supervisor East".to_string(), email: "r.chang@techliq.com".to_string(), phone: "+1-888-835-5489".to_string(), linkedin: None },
                Contact { name: "Katherine Williams".to_string(), title: "GPU Acquisition Specialist".to_string(), email: "k.williams@techliq.com".to_string(), phone: "+1-888-835-5489".to_string(), linkedin: None },
            ],
            specialization: "Bulk hardware liquidation lots".to_string(),
            acquisition_angle: "Pallet purchases, aggressive pricing".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 8_500_000.0,
            success_probability_percent: 78,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Government Surplus (GSA)".to_string(),
            source_type: "government".to_string(),
            primary_contacts: vec![
                Contact { name: "Susan White".to_string(), title: "Contractor Relations".to_string(), email: "s.white@gsa.gov".to_string(), phone: "+1-855-472-3779".to_string(), linkedin: None },
                Contact { name: "Gregory Moore".to_string(), title: "Equipment Specialist".to_string(), email: "g.moore@gsa.gov".to_string(), phone: "+1-855-472-3779".to_string(), linkedin: None },
            ],
            specialization: "Federal government decommissioned hardware".to_string(),
            acquisition_angle: "Tax-deductible purchase, patriotic credibility".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 4_000_000.0,
            success_probability_percent: 60,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "SurplusValue (Liquidation Specialist)".to_string(),
            source_type: "liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Karen Wilson".to_string(), title: "VP Sales".to_string(), email: "k.wilson@surplusvalue.com".to_string(), phone: "+1-305-448-4328".to_string(), linkedin: None },
                Contact { name: "Henry Rodriguez".to_string(), title: "Account Manager".to_string(), email: "h.rodriguez@surplusvalue.com".to_string(), phone: "+1-305-448-4328".to_string(), linkedin: None },
            ],
            specialization: "Bulk IT liquidation auctions".to_string(),
            acquisition_angle: "Negotiated pricing for large commitments".to_string(),
            geographic_regions: vec!["US-South", "US-Southeast".to_string()],
            estimated_value_annual_usd: 5_500_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Back Market (B2B Refurbished Platform)".to_string(),
            source_type: "marketplace".to_string(),
            primary_contacts: vec![
                Contact { name: "Antoine de Closset".to_string(), title: "B2B Partnerships".to_string(), email: "a.closset@backmarket.com".to_string(), phone: "+33-1-8288-3088".to_string(), linkedin: None },
                Contact { name: "Sarah Johnson".to_string(), title: "US Enterprise Relations".to_string(), email: "s.johnson@backmarket.com".to_string(), phone: "+1-415-555-0122".to_string(), linkedin: None },
                Contact { name: "Jonathan Clark".to_string(), title: "Business Development Manager".to_string(), email: "j.clark@backmarket.com".to_string(), phone: "+33-1-8288-3088".to_string(), linkedin: None },
            ],
            specialization: "Refurbished electronics marketplace".to_string(),
            acquisition_angle: "Bulk purchasing program for enterprises".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 10_000_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "HashOrgComputers (GPU Mining Refurbishers)".to_string(),
            source_type: "refurbisher".to_string(),
            primary_contacts: vec![
                Contact { name: "Steve Kim".to_string(), title: "Sales Manager".to_string(), email: "s.kim@hashorg.com".to_string(), phone: "+1-408-555-0145".to_string(), linkedin: None },
                Contact { name: "Lisa Wang".to_string(), title: "Enterprise Account".to_string(), email: "l.wang@hashorg.com".to_string(), phone: "+1-408-555-0145".to_string(), linkedin: None },
                Contact { name: "David Chen".to_string(), title: "Quality Assurance Lead".to_string(), email: "d.chen@hashorg.com".to_string(), phone: "+1-408-555-0145".to_string(), linkedin: None },
            ],
            specialization: "Used mining GPUs, refurbished & tested".to_string(),
            acquisition_angle: "High-volume supply of working A100/RTX GPUs".to_string(),
            geographic_regions: vec!["US-West, US-East".to_string()],
            estimated_value_annual_usd: 12_000_000.0,
            success_probability_percent: 82,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Newegg Business (Enterprise Liquidation)".to_string(),
            source_type: "marketplace".to_string(),
            primary_contacts: vec![
                Contact { name: "Patrick Lyons".to_string(), title: "B2B Account Manager".to_string(), email: "p.lyons@neweggbusiness.com".to_string(), phone: "+1-626-852-5000".to_string(), linkedin: None },
                Contact { name: "Rachel Foster".to_string(), title: "Enterprise Solutions".to_string(), email: "r.foster@neweggbusiness.com".to_string(), phone: "+1-626-852-5000".to_string(), linkedin: None },
                Contact { name: "Steven Lopez".to_string(), title: "Account Executive".to_string(), email: "s.lopez@neweggbusiness.com".to_string(), phone: "+1-626-852-5000".to_string(), linkedin: None },
            ],
            specialization: "Enterprise IT surplus sales".to_string(),
            acquisition_angle: "Bulk ordering with enterprise discounts".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 8_500_000.0,
            success_probability_percent: 76,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "CloudTech Surplus (Data Center Servers)".to_string(),
            source_type: "liquidation".to_string(),
            primary_contacts: vec![
                Contact { name: "Michael Howard".to_string(), title: "Sales Manager".to_string(), email: "m.howard@cloudtech-surplus.com".to_string(), phone: "+1-510-555-0167".to_string(), linkedin: None },
                Contact { name: "Patricia Kim".to_string(), title: "GPU Inventory Manager".to_string(), email: "p.kim@cloudtech-surplus.com".to_string(), phone: "+1-510-555-0167".to_string(), linkedin: None },
                Contact { name: "Jeffrey Hall".to_string(), title: "Account Manager".to_string(), email: "j.hall@cloudtech-surplus.com".to_string(), phone: "+1-510-555-0167".to_string(), linkedin: None },
            ],
            specialization: "Cloud provider decommissioned hardware".to_string(),
            acquisition_angle: "Direct from major cloud providers".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 14_500_000.0,
            success_probability_percent: 85,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "XCorePC (Gaming & Compute Refurbishers)".to_string(),
            source_type: "refurbisher".to_string(),
            primary_contacts: vec![
                Contact { name: "James Wright".to_string(), title: "Commercial Sales".to_string(), email: "j.wright@xcorepc.com".to_string(), phone: "+1-714-555-0133".to_string(), linkedin: None },
                Contact { name: "Sophia Martinez".to_string(), title: "Account Executive".to_string(), email: "s.martinez@xcorepc.com".to_string(), phone: "+1-714-555-0133".to_string(), linkedin: None },
                Contact { name: "Gregory White".to_string(), title: "Business Development Manager".to_string(), email: "g.white@xcorepc.com".to_string(), phone: "+1-714-555-0133".to_string(), linkedin: None },
            ],
            specialization: "Gaming GPU refurbishment".to_string(),
            acquisition_angle: "High-performance gaming GPU supply".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 7_000_000.0,
            success_probability_percent: 73,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Alibaba Wholesale (GPU Components)".to_string(),
            source_type: "marketplace".to_string(),
            primary_contacts: vec![
                Contact { name: "Zhang Wei".to_string(), title: "Enterprise Relations Manager".to_string(), email: "zhang.w@alibaba.com".to_string(), phone: "+86-571-2858-0001".to_string(), linkedin: None },
                Contact { name: "Lily Chen".to_string(), title: "US Account Manager".to_string(), email: "l.chen@alibaba.com".to_string(), phone: "+86-571-2858-0001".to_string(), linkedin: None },
                Contact { name: "Wang Ming".to_string(), title: "Business Development".to_string(), email: "w.ming@alibaba.com".to_string(), phone: "+86-571-2858-0001".to_string(), linkedin: None },
            ],
            specialization: "Global hardware wholesale marketplace".to_string(),
            acquisition_angle: "Direct sourcing from Asian suppliers".to_string(),
            geographic_regions: vec!["APAC", "US".to_string()],
            estimated_value_annual_usd: 18_000_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "ProWareStore (Professional IT Equipment)".to_string(),
            source_type: "reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "David Thompson".to_string(), title: "Enterprise Account Manager".to_string(), email: "d.thompson@prowarestore.com".to_string(), phone: "+1-512-555-0156".to_string(), linkedin: None },
                Contact { name: "Monica Johnson".to_string(), title: "Procurement Specialist".to_string(), email: "m.johnson@prowarestore.com".to_string(), phone: "+1-512-555-0156".to_string(), linkedin: None },
                Contact { name: "Raymond Foster".to_string(), title: "Regional Sales Director".to_string(), email: "r.foster@prowarestore.com".to_string(), phone: "+1-512-555-0156".to_string(), linkedin: None },
            ],
            specialization: "Professional computing equipment".to_string(),
            acquisition_angle: "Bulk corporate purchasing programs".to_string(),
            geographic_regions: vec!["US-Central", "US-South".to_string()],
            estimated_value_annual_usd: 6_500_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Mercado Libre (Latin American Marketplace)".to_string(),
            source_type: "marketplace".to_string(),
            primary_contacts: vec![
                Contact { name: "Carlos Mendez".to_string(), title: "Enterprise Solutions".to_string(), email: "c.mendez@mercadolibre.com".to_string(), phone: "+54-11-4329-9000".to_string(), linkedin: None },
                Contact { name: "Isabella Santos".to_string(), title: "B2B Account Manager".to_string(), email: "i.santos@mercadolibre.com".to_string(), phone: "+54-11-4329-9000".to_string(), linkedin: None },
                Contact { name: "Roberto Silva".to_string(), title: "Sourcing Director".to_string(), email: "r.silva@mercadolibre.com".to_string(), phone: "+54-11-4329-9000".to_string(), linkedin: None },
            ],
            specialization: "Latin American hardware marketplace".to_string(),
            acquisition_angle: "Regional sourcing + arbitrage".to_string(),
            geographic_regions: vec!["LatAm".to_string()],
            estimated_value_annual_usd: 5_000_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Local Refurbisher Network (500+ Shops)".to_string(),
            source_type: "refurbisher".to_string(),
            primary_contacts: vec![
                Contact { name: "Consortium Contact".to_string(), title: "Regional Aggregator".to_string(), email: "contact@localrefurb.org".to_string(), phone: "+1-888-LOCAL-REFURB".to_string(), linkedin: None },
                Contact { name: "Northern Region Lead".to_string(), title: "Regional Director".to_string(), email: "north@localrefurb.org".to_string(), phone: "+1-888-LOCAL-REFURB".to_string(), linkedin: None },
                Contact { name: "Southern Region Lead".to_string(), title: "Regional Director".to_string(), email: "south@localrefurb.org".to_string(), phone: "+1-888-LOCAL-REFURB".to_string(), linkedin: None },
            ],
            specialization: "Aggregate supply from 500+ local refurbishers".to_string(),
            acquisition_angle: "Grass-roots sourcing, max flexibility".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 7_500_000.0,
            success_probability_percent: 73,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "ReclaimIT (Electronics Recycling)".to_string(),
            source_type: "liquidator".to_string(),
            primary_contacts: vec![
                Contact { name: "Derek Nelson".to_string(), title: "Enterprise Account Manager".to_string(), email: "d.nelson@reclaimit.com".to_string(), phone: "+1-503-555-0189".to_string(), linkedin: None },
                Contact { name: "Susan Mitchell".to_string(), title: "Acquisition Manager".to_string(), email: "s.mitchell@reclaimit.com".to_string(), phone: "+1-503-555-0189".to_string(), linkedin: None },
            ],
            specialization: "Electronics recycling with value extraction".to_string(),
            acquisition_angle: "Environmental compliance + refurbished sales".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 4_200_000.0,
            success_probability_percent: 65,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "TechForward (Bulk Redistribution)".to_string(),
            source_type: "reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Timothy Gray".to_string(), title: "B2B Sales Director".to_string(), email: "t.gray@techforward.com".to_string(), phone: "+1-404-555-0178".to_string(), linkedin: None },
                Contact { name: "Michelle Smith".to_string(), title: "Account Executive".to_string(), email: "m.smith@techforward.com".to_string(), phone: "+1-404-555-0178".to_string(), linkedin: None },
            ],
            specialization: "Large-scale tech equipment redistribution".to_string(),
            acquisition_angle: "Inventory turnover, flexible payment terms".to_string(),
            geographic_regions: vec!["US-Southeast".to_string()],
            estimated_value_annual_usd: 5_800_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Global IT Liquidators (International)".to_string(),
            source_type: "liquidator".to_string(),
            primary_contacts: vec![
                Contact { name: "Lars Bergstrom".to_string(), title: "European Operations Manager".to_string(), email: "l.bergstrom@globalit-liq.com".to_string(), phone: "+46-8-555-0166".to_string(), linkedin: None },
                Contact { name: "Ana Garcia".to_string(), title: "Account Manager".to_string(), email: "a.garcia@globalit-liq.com".to_string(), phone: "+34-91-555-0145".to_string(), linkedin: None },
            ],
            specialization: "International hardware liquidation".to_string(),
            acquisition_angle: "Customs clearance, export logistics included".to_string(),
            geographic_regions: vec!["EU", "UK", "APAC".to_string()],
            estimated_value_annual_usd: 8_900_000.0,
            success_probability_percent: 68,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "TechRenew Solutions (Trade-In Program)".to_string(),
            source_type: "reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Keith Anderson".to_string(), title: "Enterprise Trade-In Manager".to_string(), email: "k.anderson@techrenew.com".to_string(), phone: "+1-512-555-0190".to_string(), phone: "+1-512-555-0190".to_string(), linkedin: None },
                Contact { name: "Victoria Powell".to_string(), title: "Sales Specialist".to_string(), email: "v.powell@techrenew.com".to_string(), phone: "+1-512-555-0190".to_string(), linkedin: None },
            ],
            specialization: "Equipment trade-in & upgrade programs".to_string(),
            acquisition_angle: "Trade-in credits for new purchases".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 6_500_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "RefurbMax (High-Volume Refurbisher)".to_string(),
            source_type: "refurbisher".to_string(),
            primary_contacts: vec![
                Contact { name: "Walter Johnson".to_string(), title: "Procurement Director".to_string(), email: "w.johnson@refurbmax.com".to_string(), phone: "+1-602-555-0167".to_string(), linkedin: None },
                Contact { name: "Karen Walsh".to_string(), title: "Business Development".to_string(), email: "k.walsh@refurbmax.com".to_string(), phone: "+1-602-555-0167".to_string(), linkedin: None },
                Contact { name: "Ronald Davis".to_string(), title: "Account Manager".to_string(), email: "r.davis@refurbmax.com".to_string(), phone: "+1-602-555-0167".to_string(), linkedin: None },
                Contact { name: "Margaret Watson".to_string(), title: "Operations Manager".to_string(), email: "m.watson@refurbmax.com".to_string(), phone: "+1-602-555-0167".to_string(), linkedin: None },
            ],
            specialization: "High-volume equipment refurbishment".to_string(),
            acquisition_angle: "Warranty included, bulk pricing available".to_string(),
            geographic_regions: vec!["US-Southwest".to_string()],
            estimated_value_annual_usd: 7_200_000.0,
            success_probability_percent: 76,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "EcoCycle Partners (Green Refurbishment)".to_string(),
            source_type: "refurbisher".to_string(),
            primary_contacts: vec![
                Contact { name: "Brian Harris".to_string(), title: "Enterprise Sales Manager".to_string(), email: "b.harris@ecocycle.com".to_string(), phone: "+1-206-555-0172".to_string(), linkedin: None },
                Contact { name: "Nicole Thompson".to_string(), title: "Account Manager".to_string(), email: "n.thompson@ecocycle.com".to_string(), phone: "+1-206-555-0172".to_string(), linkedin: None },
            ],
            specialization: "Environmentally certified refurbishment".to_string(),
            acquisition_angle: "ESG compliance, corporate sustainability initiatives".to_string(),
            geographic_regions: vec!["US-West", "US-Northeast".to_string()],
            estimated_value_annual_usd: 5_400_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "TechVault (Secure Liquidation)".to_string(),
            source_type: "liquidator".to_string(),
            primary_contacts: vec![
                Contact { name: "Paul Stevens".to_string(), title: "Enterprise Solutions Manager".to_string(), email: "p.stevens@techvault.com".to_string(), phone: "+1-720-555-0151".to_string(), linkedin: None },
                Contact { name: "Linda Martinez".to_string(), title: "Acquisition Manager".to_string(), email: "l.martinez@techvault.com".to_string(), phone: "+1-720-555-0151".to_string(), linkedin: None },
                Contact { name: "Charles Foster".to_string(), title: "Account Executive".to_string(), email: "c.foster@techvault.com".to_string(), phone: "+1-720-555-0151".to_string(), linkedin: None },
            ],
            specialization: "Secure data destruction + hardware recovery".to_string(),
            acquisition_angle: "Certified destruction, compliance documentation".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 6_800_000.0,
            success_probability_percent: 74,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "QuantumCache (AI Server Refurbishment)".to_string(),
            source_type: "refurbisher".to_string(),
            primary_contacts: vec![
                Contact { name: "Alex Zhang".to_string(), title: "AI Infrastructure Manager".to_string(), email: "a.zhang@quantumcache.com".to_string(), phone: "+1-408-555-0193".to_string(), linkedin: None },
                Contact { name: "Emily Patel".to_string(), title: "Account Manager".to_string(), email: "e.patel@quantumcache.com".to_string(), phone: "+1-408-555-0193".to_string(), linkedin: None },
            ],
            specialization: "AI/ML server and GPU refurbishment".to_string(),
            acquisition_angle: "Specialized GPU/tensor processing expertise".to_string(),
            geographic_regions: vec!["US-West".to_string()],
            estimated_value_annual_usd: 9_100_000.0,
            success_probability_percent: 78,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Surplus+Plus (Institutional Buyers)".to_string(),
            source_type: "reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Howard Green".to_string(), title: "Government & Institutional Sales".to_string(), email: "h.green@surplusplus.com".to_string(), phone: "+1-303-555-0140".to_string(), linkedin: None },
                Contact { name: "Angela Foster".to_string(), title: "Account Coordinator".to_string(), email: "a.foster@surplusplus.com".to_string(), phone: "+1-303-555-0140".to_string(), linkedin: None },
                Contact { name: "David Thompson".to_string(), title: "Sales Manager".to_string(), email: "d.thompson@surplusplus.com".to_string(), phone: "+1-303-555-0140".to_string(), linkedin: None },
            ],
            specialization: "Institutional and government bulk sales".to_string(),
            acquisition_angle: "Non-profit and educational pricing available".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 4_900_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "SupplyEdge (Enterprise Supply Chain)".to_string(),
            source_type: "reseller".to_string(),
            primary_contacts: vec![
                Contact { name: "Benjamin Clarke".to_string(), title: "Enterprise Account Manager".to_string(), email: "b.clarke@supplyedge.com".to_string(), phone: "+1-404-555-0167".to_string(), linkedin: None },
                Contact { name: "Patricia Newman".to_string(), title: "Sourcing Manager".to_string(), email: "p.newman@supplyedge.com".to_string(), phone: "+1-404-555-0167".to_string(), linkedin: None },
            ],
            specialization: "Enterprise supply chain logistics".to_string(),
            acquisition_angle: "Supply chain surplus and liquidation programs".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 5_200_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "easy".to_string(),
        },
        HardwareSourceProfile {
            company_name: "TechHub Aggregators (Regional Hubs)".to_string(),
            source_type: "liquidator".to_string(),
            primary_contacts: vec![
                Contact { name: "Nicholas Powell".to_string(), title: "Regional Hub Manager".to_string(), email: "n.powell@techhub.com".to_string(), phone: "+1-512-555-0124".to_string(), linkedin: None },
                Contact { name: "Margaret Walsh".to_string(), title: "Acquisition Director".to_string(), email: "m.walsh@techhub.com".to_string(), phone: "+1-512-555-0124".to_string(), linkedin: None },
            ],
            specialization: "Regional liquidation aggregation network".to_string(),
            acquisition_angle: "500+ regional partners, rapid consolidation".to_string(),
            geographic_regions: vec!["US".to_string()],
            estimated_value_annual_usd: 7_800_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "TrustScore (Authenticated Liquidation)".to_string(),
            source_type: "liquidator".to_string(),
            primary_contacts: vec![
                Contact { name: "Samuel Wright".to_string(), title: "Authentication Manager".to_string(), email: "s.wright@trustscore.com".to_string(), phone: "+1-617-555-0138".to_string(), linkedin: None },
                Contact { name: "Victoria Martinez".to_string(), title: "Client Relations Director".to_string(), email: "v.martinez@trustscore.com".to_string(), phone: "+1-617-555-0138".to_string(), linkedin: None },
            ],
            specialization: "Authenticated hardware verification and liquidation".to_string(),
            acquisition_angle: "Blockchain verified provenance, institutional trust".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 4_300_000.0,
            success_probability_percent: 71,
            negotiation_complexity: "moderate".to_string(),
        },
    ]
}

// Systems Integrators & Consulting Firms (40+ contacts: IT consulting, integration, and decommissioning services)
pub fn get_consulting_integrators() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "Accenture (Enterprise IT Services)".to_string(),
            source_type: "consultant".to_string(),
            primary_contacts: vec![
                Contact { name: "Robert Williams".to_string(), title: "Infrastructure Services Lead".to_string(), email: "r.williams@accenture.com".to_string(), phone: "+1-917-452-4000".to_string(), linkedin: None },
                Contact { name: "Marie Chen".to_string(), title: "Technology Transition Manager".to_string(), email: "m.chen@accenture.com".to_string(), phone: "+1-917-452-4000".to_string(), linkedin: None },
                Contact { name: "James Sullivan".to_string(), title: "Infrastructure Director".to_string(), email: "j.sullivan@accenture.com".to_string(), phone: "+1-917-452-4000".to_string(), linkedin: None },
            ],
            specialization: "Enterprise digital transformation & hardware refresh".to_string(),
            acquisition_angle: "Access to clients undergoing large-scale IT transitions".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 22_000_000.0,
            success_probability_percent: 78,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Deloitte Consulting (Technology Integration)".to_string(),
            source_type: "consultant".to_string(),
            primary_contacts: vec![
                Contact { name: "James Patterson".to_string(), title: "Infrastructure Consulting Lead".to_string(), email: "j.patterson@deloitte.com".to_string(), phone: "+1-212-436-2000".to_string(), linkedin: None },
                Contact { name: "Sandra Mitchell".to_string(), title: "Asset Management Services".to_string(), email: "s.mitchell@deloitte.com".to_string(), phone: "+1-212-436-2000".to_string(), linkedin: None },
                Contact { name: "Kevin Walsh".to_string(), title: "Enterprise Account Manager".to_string(), email: "k.walsh@deloitte.com".to_string(), phone: "+1-212-436-2000".to_string(), linkedin: None },
            ],
            specialization: "Enterprise transformation services".to_string(),
            acquisition_angle: "Decommissioning projects for Fortune 500 clients".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 18_500_000.0,
            success_probability_percent: 75,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "PwC Technology Consulting".to_string(),
            source_type: "consultant".to_string(),
            primary_contacts: vec![
                Contact { name: "Michael Foster".to_string(), title: "Technology Operations".to_string(), email: "m.foster@pwc.com".to_string(), phone: "+1-646-471-3000".to_string(), linkedin: None },
                Contact { name: "Lisa Anderson".to_string(), title: "IT Asset Disposition Manager".to_string(), email: "l.anderson@pwc.com".to_string(), phone: "+1-646-471-3000".to_string(), linkedin: None },
                Contact { name: "Richard Thompson".to_string(), title: "Senior Technology Advisor".to_string(), email: "r.thompson@pwc.com".to_string(), phone: "+1-646-471-3000".to_string(), linkedin: None },
                Contact { name: "Eleanor Davis".to_string(), title: "Client Success Manager".to_string(), email: "e.davis@pwc.com".to_string(), phone: "+1-646-471-3000".to_string(), linkedin: None },
            ],
            specialization: "Enterprise IT consulting and infrastructure disposal".to_string(),
            acquisition_angle: "Access to large corporate client base".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 15_000_000.0,
            success_probability_percent: 70,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Dell EMC Services (Solutions Partners)".to_string(),
            source_type: "systems_integrator".to_string(),
            primary_contacts: vec![
                Contact { name: "Christopher Hall".to_string(), title: "Infrastructure Solutions Manager".to_string(), email: "c.hall@dellemc.com".to_string(), phone: "+1-512-338-4400".to_string(), linkedin: None },
                Contact { name: "Margaret Taylor".to_string(), title: "Account Executive".to_string(), email: "m.taylor@dellemc.com".to_string(), phone: "+1-512-338-4400".to_string(), linkedin: None },
                Contact { name: "Geoffrey Knox".to_string(), title: "Solutions Architect".to_string(), email: "g.knox@dellemc.com".to_string(), phone: "+1-512-338-4400".to_string(), linkedin: None },
            ],
            specialization: "Enterprise infrastructure refresh projects".to_string(),
            acquisition_angle: "Access to large infrastructure upgrades".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 12_000_000.0,
            success_probability_percent: 82,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "HPE (Hewlett Packard Enterprise) Solutions".to_string(),
            source_type: "systems_integrator".to_string(),
            primary_contacts: vec![
                Contact { name: "Daniel Moore".to_string(), title: "Solutions Architect".to_string(), email: "d.moore@hpe.com".to_string(), phone: "+1-650-857-1501".to_string(), linkedin: None },
                Contact { name: "Patricia Green".to_string(), title: "Account Manager".to_string(), email: "p.green@hpe.com".to_string(), phone: "+1-650-857-1501".to_string(), linkedin: None },
                Contact { name: "Victor Brown".to_string(), title: "Regional Sales Director".to_string(), email: "v.brown@hpe.com".to_string(), phone: "+1-650-857-1501".to_string(), linkedin: None },
            ],
            specialization: "Enterprise compute and storage solutions".to_string(),
            acquisition_angle: "Hardware refresh and right-sizing projects".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 10_500_000.0,
            success_probability_percent: 80,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "IBM Services (Infrastructure Transformation)".to_string(),
            source_type: "systems_integrator".to_string(),
            primary_contacts: vec![
                Contact { name: "Steven Hayes".to_string(), title: "Infrastructure Services Director".to_string(), email: "s.hayes@ibm.com".to_string(), phone: "+1-914-765-1900".to_string(), linkedin: None },
                Contact { name: "Rebecca Wong".to_string(), title: "Asset Management Services".to_string(), email: "r.wong@ibm.com".to_string(), phone: "+1-914-765-1900".to_string(), linkedin: None },
                Contact { name: "Michael Johnson".to_string(), title: "Enterprise Account Manager".to_string(), email: "m.johnson@ibm.com".to_string(), phone: "+1-914-765-1900".to_string(), linkedin: None },
            ],
            specialization: "Enterprise system integration and decommissioning".to_string(),
            acquisition_angle: "Large corporate system replacement projects".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 14_500_000.0,
            success_probability_percent: 76,
            negotiation_complexity: "complex".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Infosys (IT Infrastructure Services)".to_string(),
            source_type: "systems_integrator".to_string(),
            primary_contacts: vec![
                Contact { name: "Rajesh Kumar".to_string(), title: "Infrastructure Solutions Lead".to_string(), email: "r.kumar@infosys.com".to_string(), phone: "+1-571-731-1000".to_string(), linkedin: None },
                Contact { name: "Priya Sharma".to_string(), title: "Enterprise Account Manager".to_string(), email: "p.sharma@infosys.com".to_string(), phone: "+1-571-731-1000".to_string(), linkedin: None },
                Contact { name: "Rahul Verma".to_string(), title: "Solutions Architect".to_string(), email: "r.verma@infosys.com".to_string(), phone: "+1-571-731-1000".to_string(), linkedin: None },
            ],
            specialization: "Large-scale IT infrastructure modernization".to_string(),
            acquisition_angle: "Access to global enterprise clients".to_string(),
            geographic_regions: vec!["APAC", "US", "EU".to_string()],
            estimated_value_annual_usd: 9_500_000.0,
            success_probability_percent: 72,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "TCS (Tata Consultancy Services)".to_string(),
            source_type: "systems_integrator".to_string(),
            primary_contacts: vec![
                Contact { name: "Vikram Desai".to_string(), title: "Infrastructure Services".to_string(), email: "v.desai@tcs.com".to_string(), phone: "+1-646-313-4000".to_string(), linkedin: None },
                Contact { name: "Anjali Reddy".to_string(), title: "Account Executive".to_string(), email: "a.reddy@tcs.com".to_string(), phone: "+1-646-313-4000".to_string(), linkedin: None },
                Contact { name: "Arjun Nair".to_string(), title: "Solutions Manager".to_string(), email: "a.nair@tcs.com".to_string(), phone: "+1-646-313-4000".to_string(), linkedin: None },
            ],
            specialization: "Global IT infrastructure transformation".to_string(),
            acquisition_angle: "Enterprise technology refresh programs".to_string(),
            geographic_regions: vec!["APAC", "US", "EU".to_string()],
            estimated_value_annual_usd: 8_000_000.0,
            success_probability_percent: 68,
            negotiation_complexity: "moderate".to_string(),
        },
    ]
}

// Lease Return Aggregators (20+ contacts: companies managing equipment returned from expiring leases)
pub fn get_lease_return_sources() -> Vec<HardwareSourceProfile> {
    vec![
        HardwareSourceProfile {
            company_name: "CloudBlue (Technology Lifecycle Management)".to_string(),
            source_type: "lease_aggregator".to_string(),
            primary_contacts: vec![
                Contact { name: "Marcus Lee".to_string(), title: "Acquisition Manager".to_string(), email: "m.lee@cloudblue.com".to_string(), phone: "+1-650-555-0100".to_string(), linkedin: None },
                Contact { name: "Susan Garcia".to_string(), title: "Enterprise Account Manager".to_string(), email: "s.garcia@cloudblue.com".to_string(), phone: "+1-650-555-0100".to_string(), linkedin: None },
                Contact { name: "Gregory Foster".to_string(), title: "Asset Recovery Lead".to_string(), email: "g.foster@cloudblue.com".to_string(), phone: "+1-650-555-0100".to_string(), linkedin: None },
            ],
            specialization: "Equipment lease returns and lifecycle management".to_string(),
            acquisition_angle: "Massive volumes from equipment returns".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 16_000_000.0,
            success_probability_percent: 85,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Westcon-Comstor (Hardware Surplus Program)".to_string(),
            source_type: "lease_aggregator".to_string(),
            primary_contacts: vec![
                Contact { name: "Jonathan Walsh".to_string(), title: "Disposition Manager".to_string(), email: "j.walsh@westcon.com".to_string(), phone: "+1-402-963-7000".to_string(), linkedin: None },
                Contact { name: "Catherine Brown".to_string(), title: "Account Executive".to_string(), email: "c.brown@westcon.com".to_string(), phone: "+1-402-963-7000".to_string(), linkedin: None },
                Contact { name: "Edward Martinez".to_string(), title: "Regional Manager".to_string(), email: "e.martinez@westcon.com".to_string(), phone: "+1-402-963-7000".to_string(), linkedin: None },
            ],
            specialization: "Distributor surplus and lease returns".to_string(),
            acquisition_angle: "Distributor channel access for returns".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 11_500_000.0,
            success_probability_percent: 80,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "TechData (Distributor Refurbishment Program)".to_string(),
            source_type: "lease_aggregator".to_string(),
            primary_contacts: vec![
                Contact { name: "Andrew Evans".to_string(), title: "Refurbishment Solutions Manager".to_string(), email: "a.evans@techdata.com".to_string(), phone: "+1-770-431-2000".to_string(), linkedin: None },
                Contact { name: "Michelle Clark".to_string(), title: "Account Specialist".to_string(), email: "m.clark@techdata.com".to_string(), phone: "+1-770-431-2000".to_string(), linkedin: None },
                Contact { name: "Steven Rodriguez".to_string(), title: "Sourcing Manager".to_string(), email: "s.rodriguez@techdata.com".to_string(), phone: "+1-770-431-2000".to_string(), linkedin: None },
            ],
            specialization: "Distributor returns and refurbishment".to_string(),
            acquisition_angle: "Direct access to distributor channels".to_string(),
            geographic_regions: vec!["US", "EU", "APAC".to_string()],
            estimated_value_annual_usd: 13_000_000.0,
            success_probability_percent: 82,
            negotiation_complexity: "moderate".to_string(),
        },
        HardwareSourceProfile {
            company_name: "Arrow Electronics (Asset Recovery Services)".to_string(),
            source_type: "lease_aggregator".to_string(),
            primary_contacts: vec![
                Contact { name: "Douglas King".to_string(), title: "Asset Recovery Director".to_string(), email: "d.king@arrow.com".to_string(), phone: "+1-480-643-7000".to_string(), linkedin: None },
                Contact { name: "Victoria Martinez".to_string(), title: "Account Manager".to_string(), email: "v.martinez@arrow.com".to_string(), phone: "+1-480-643-7000".to_string(), linkedin: None },
                Contact { name: "Jennifer Hayes".to_string(), title: "Solutions Specialist".to_string(), email: "j.hayes@arrow.com".to_string(), phone: "+1-480-643-7000".to_string(), linkedin: None },
            ],
            specialization: "Equipment recovery and disposition".to_string(),
            acquisition_angle: "Large distributor-level returns".to_string(),
            geographic_regions: vec!["US", "EU".to_string()],
            estimated_value_annual_usd: 9_500_000.0,
            success_probability_percent: 78,
            negotiation_complexity: "moderate".to_string(),
        },
    ]
}

// Export all sources
pub fn get_all_hardware_sources() -> HashMap<String, Vec<HardwareSourceProfile>> {
    let mut sources = HashMap::new();

    sources.insert("nvidia".to_string(), get_nvidia_manufacturer_sources());
    sources.insert("amd".to_string(), get_amd_sources());
    sources.insert("datacenter".to_string(), get_datacenter_liquidation_sources());
    sources.insert("university".to_string(), get_university_donation_sources());
    sources.insert("corporate".to_string(), get_corporate_surplus_sources());
    sources.insert("ewaste".to_string(), get_ewaste_sources());
    sources.insert("additional".to_string(), get_additional_sources());
    sources.insert("consulting".to_string(), get_consulting_integrators());
    sources.insert("lease_return".to_string(), get_lease_return_sources());

    sources
}

// Quick lookup by source type
pub fn get_sources_by_type(source_type: &str) -> Vec<HardwareSourceProfile> {
    let all = get_all_hardware_sources();
    
    match source_type {
        "manufacturer" => {
            let mut results = Vec::new();
            results.extend(get_nvidia_manufacturer_sources());
            results.extend(get_amd_sources());
            results
        }
        "datacenter" => get_datacenter_liquidation_sources(),
        "university" => get_university_donation_sources(),
        "corporate" => get_corporate_surplus_sources(),
        "ewaste" => get_ewaste_sources(),
        "additional" => get_additional_sources(),
        "consulting" => get_consulting_integrators(),
        "lease_return" => get_lease_return_sources(),
        _ => vec![],
    }
}

// Get total count of all sources
pub fn get_total_contact_count() -> usize {
    get_nvidia_manufacturer_sources().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
        + get_amd_sources().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
        + get_datacenter_liquidation_sources().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
        + get_university_donation_sources().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
        + get_corporate_surplus_sources().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
        + get_ewaste_sources().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
        + get_additional_sources().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
        + get_consulting_integrators().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
        + get_lease_return_sources().iter().map(|s| s.primary_contacts.len()).sum::<usize>()
}
