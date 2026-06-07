-- X3 Hardware Acquisition & Logistics System
-- Complete schema for tracking free/cheap hardware sourcing

-- Master hardware acquisition campaigns
CREATE TABLE hw_acquisition_campaigns (
  id TEXT PRIMARY KEY,
  campaign_name TEXT NOT NULL,
  campaign_type TEXT CHECK(campaign_type IN ('manufacturer', 'retailer', 'recycler', 'datacenter', 'university', 'enterprise')) NOT NULL,
  status TEXT CHECK(status IN ('planning', 'active', 'paused', 'completed', 'failed')) DEFAULT 'planning',
  target_hardware TEXT NOT NULL, -- e.g., "NVIDIA A100 GPUs", "AMD EPYC CPUs", "DDR5 RAM"
  unit_count INTEGER,
  total_estimated_value_usd DECIMAL(12,2),
  start_date TEXT,
  end_date TEXT,
  notes TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Hardware sources (manufacturers, retailers, recyclers, data centers)
CREATE TABLE hw_sources (
  id TEXT PRIMARY KEY,
  source_type TEXT CHECK(source_type IN ('manufacturer', 'authorized_partner', 'reseller', 'refurbisher', 'data_center_liquidation', 'university', 'corporate_surplus', 'e_waste_recycler')) NOT NULL,
  company_name TEXT NOT NULL UNIQUE,
  website TEXT,
  primary_contact_name TEXT,
  email TEXT,
  phone TEXT,
  geographic_region TEXT, -- e.g., "US-West", "EU", "APAC"
  specialization TEXT, -- e.g., "GPU reseller", "data center liquidation", "refurbished servers"
  acquisition_angle TEXT, -- HOW we approach them (partnership, donation, research collab, B2B deal)
  negotiation_status TEXT CHECK(negotiation_status IN ('not_contacted', 'inquiry_sent', 'in_discussion', 'deal_proposed', 'closed', 'rejected')) DEFAULT 'not_contacted',
  deal_value_usd DECIMAL(12,2),
  deal_terms TEXT, -- e.g., "Free + shipping covered by X3", "50% discount", "Donation for tax credit"
  success_rate_percent INTEGER DEFAULT 0,
  last_contact_date TEXT,
  response_time_days INTEGER,
  reliability_score DECIMAL(3,1) DEFAULT 5.0, -- 1-10 scale
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Individual hardware units tracked
CREATE TABLE hw_units (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES hw_sources(id),
  campaign_id TEXT NOT NULL REFERENCES hw_acquisition_campaigns(id),
  hardware_category TEXT CHECK(hardware_category IN ('gpu', 'cpu', 'memory', 'storage', 'network', 'cooling', 'psu', 'motherboard', 'other')) NOT NULL,
  hardware_model TEXT NOT NULL, -- e.g., "NVIDIA A100 80GB", "AMD EPYC 7713"
  condition TEXT CHECK(condition IN ('new', 'refurbished', 'used_good', 'used_fair', 'salvage')) DEFAULT 'used_good',
  acquisition_cost_usd DECIMAL(10,2),
  estimated_market_value_usd DECIMAL(10,2),
  bandwidth_gbps DECIMAL(8,2),
  power_consumption_w INTEGER,
  quantity_received INTEGER DEFAULT 1,
  arrival_date TEXT,
  location_warehouse TEXT,
  operational_status TEXT CHECK(operational_status IN ('pending_test', 'operational', 'needs_repair', 'scrapped', 'deployed')) DEFAULT 'pending_test',
  deployment_target TEXT, -- e.g., "validator-node-5", "gpu-pool-cluster", "benchmark-lab"
  performance_metrics TEXT, -- JSON: {hashrate, memory_bandwidth, power_efficiency}
  notes TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Outreach campaigns (per source or per hardware type)
CREATE TABLE hw_outreach_campaigns (
  id TEXT PRIMARY KEY,
  campaign_name TEXT NOT NULL,
  source_id TEXT NOT NULL REFERENCES hw_sources(id),
  template_type TEXT CHECK(template_type IN ('donation_solicitation', 'partnership_proposal', 'research_collaboration', 'bulk_purchase', 'e_waste_recovery', 'education_program')) NOT NULL,
  message_campaign TEXT NOT NULL, -- Full personalized message
  key_value_prop TEXT, -- Main selling point for THIS source
  success_metrics TEXT, -- What we're measuring (units received, cost savings, response rate)
  outreach_date TEXT,
  follow_up_dates TEXT, -- JSON array of follow-up dates
  status TEXT CHECK(status IN ('draft', 'sent', 'in_progress', 'scheduled_followup', 'closed_won', 'closed_lost')) DEFAULT 'draft',
  response_received BOOLEAN DEFAULT FALSE,
  response_text TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Shipping & logistics
CREATE TABLE hw_shipments (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES hw_sources(id),
  shipment_status TEXT CHECK(shipment_status IN ('arranged', 'in_transit', 'delivered', 'inspection_passed', 'inspection_failed', 'received_damaged')) DEFAULT 'arranged',
  origin_address TEXT,
  destination_address TEXT, -- X3 warehouse/lab address
  carrier TEXT, -- FedEx, UPS, DHL, local courier
  tracking_number TEXT,
  shipment_date TEXT,
  estimated_delivery_date TEXT,
  actual_delivery_date TEXT,
  shipping_cost_usd DECIMAL(10,2),
  insurance_cost_usd DECIMAL(10,2),
  customs_duties_usd DECIMAL(10,2),
  inspected_by TEXT,
  inspection_notes TEXT,
  units_received INTEGER,
  units_damaged INTEGER,
  return_cost_usd DECIMAL(10,2),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Hardware inventory & deployment
CREATE TABLE hw_inventory (
  id TEXT PRIMARY KEY,
  hardware_model TEXT NOT NULL,
  category TEXT NOT NULL,
  total_units_acquired INTEGER DEFAULT 0,
  total_units_operational INTEGER DEFAULT 0,
  total_acquisition_cost_usd DECIMAL(12,2) DEFAULT 0,
  total_market_value_usd DECIMAL(12,2) DEFAULT 0,
  deployment_locations TEXT, -- JSON array: [{location, units, utilization_percent}]
  utilization_percent DECIMAL(5,2) DEFAULT 0,
  maintenance_cost_annual_usd DECIMAL(10,2) DEFAULT 0,
  power_cost_monthly_usd DECIMAL(10,2) DEFAULT 0,
  cooling_cost_monthly_usd DECIMAL(10,2) DEFAULT 0,
  estimated_useful_life_years INTEGER DEFAULT 5,
  roi_timeline_months INTEGER,
  notes TEXT,
  last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Acquisition success metrics
CREATE TABLE hw_acquisition_metrics (
  id TEXT PRIMARY KEY,
  metric_date TEXT NOT NULL,
  total_hardware_value_acquired_usd DECIMAL(12,2),
  total_acquisition_cost_usd DECIMAL(12,2),
  roi_percent DECIMAL(6,2), -- (value - cost) / cost * 100
  outreach_attempts_sent INTEGER,
  positive_responses INTEGER,
  deals_closed INTEGER,
  average_negotiation_days INTEGER,
  cost_per_unit_acquired_usd DECIMAL(10,2),
  sources_engaged INTEGER,
  hardware_categories_acquired TEXT, -- JSON: count by category
  expected_gpu_capacity_tflops DECIMAL(10,1),
  expected_monthly_validator_revenue_usd DECIMAL(12,2),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Donor/partner database (follow-up potential)
CREATE TABLE hw_donor_relationships (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES hw_sources(id),
  relationship_type TEXT CHECK(relationship_type IN ('one_off', 'ongoing_partnership', 'annual_donation', 'equipment_lease')) DEFAULT 'one_off',
  total_value_received_usd DECIMAL(12,2),
  total_deals_closed INTEGER DEFAULT 0,
  last_acquisition_date TEXT,
  next_contact_date TEXT,
  renewal_likelihood_percent INTEGER, -- 0-100
  strategic_importance TEXT, -- e.g., "critical_for_gpu_supply", "exploratory"
  long_term_potential TEXT,
  relationship_owner TEXT, -- Team member responsible
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Tax deductions & CSR documentation
CREATE TABLE hw_acquisition_documentation (
  id TEXT PRIMARY KEY,
  hardware_unit_id TEXT NOT NULL REFERENCES hw_units(id),
  doc_type TEXT CHECK(doc_type IN ('donation_letter', 'receipt', 'valuation_cert', 'inspection_report', 'shipment_proof', 'csr_report')) NOT NULL,
  doc_url TEXT, -- Link to PDF in storage
  doc_date TEXT,
  issuing_party TEXT, -- Donor or valuation service
  third_party_valuation_usd DECIMAL(10,2), -- For tax purposes
  tax_deduction_eligible BOOLEAN DEFAULT FALSE,
  notes TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for fast queries
CREATE INDEX idx_hw_campaigns_type_status ON hw_acquisition_campaigns(campaign_type, status);
CREATE INDEX idx_hw_campaigns_dates ON hw_acquisition_campaigns(start_date, end_date);
CREATE INDEX idx_hw_sources_type_region ON hw_sources(source_type, geographic_region);
CREATE INDEX idx_hw_sources_negotiation ON hw_sources(negotiation_status);
CREATE INDEX idx_hw_units_campaign ON hw_units(campaign_id);
CREATE INDEX idx_hw_units_source ON hw_units(source_id);
CREATE INDEX idx_hw_units_category ON hw_units(hardware_category);
CREATE INDEX idx_hw_units_status ON hw_units(operational_status);
CREATE INDEX idx_hw_units_location ON hw_units(location_warehouse);
CREATE INDEX idx_hw_shipments_status ON hw_shipments(shipment_status);
CREATE INDEX idx_hw_shipments_dates ON hw_shipments(shipment_date, actual_delivery_date);
CREATE INDEX idx_hw_inventory_model ON hw_inventory(hardware_model);
CREATE INDEX idx_hw_inventory_category ON hw_inventory(category);
CREATE INDEX idx_hw_metrics_date ON hw_acquisition_metrics(metric_date);
CREATE INDEX idx_hw_donors_type ON hw_donor_relationships(relationship_type);
CREATE INDEX idx_hw_donors_potential ON hw_donor_relationships(renewal_likelihood_percent DESC);
