// Hardware Acquisition Dashboard Components
// Complete UI for tracking free/cheap hardware sourcing

import React, { useState, useEffect } from 'react';
import { LineChart, Line, BarChart, Bar, PieChart, Pie, Cell, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

interface Campaign {
  id: string;
  campaign_name: string;
  campaign_type: string;
  status: string;
  target_hardware: string;
  unit_count: number;
  total_estimated_value_usd: number;
  start_date: string;
}

interface HardwareMetrics {
  metric_date: string;
  total_hardware_value_usd: number;
  total_cost_usd: number;
  roi_percent: number;
  outreach_attempts: number;
  positive_responses: number;
  deals_closed: number;
  sources_engaged: number;
}

// Component 1: Campaign Tracker
export const HardwareAcquisitionTracker: React.FC = () => {
  const [campaigns, setCampaigns] = useState<Campaign[]>([
    {
      id: '1',
      campaign_name: 'NVIDIA GPU Acquisition Q1 2026',
      campaign_type: 'manufacturer',
      status: 'active',
      target_hardware: 'NVIDIA A100 / H100',
      unit_count: 50,
      total_estimated_value_usd: 2_500_000,
      start_date: '2026-01-15',
    },
    {
      id: '2',
      campaign_name: 'Data Center Liquidation - Mid-Year',
      campaign_type: 'datacenter',
      status: 'active',
      target_hardware: 'Mixed enterprise GPUs/CPUs',
      unit_count: 200,
      total_estimated_value_usd: 4_000_000,
      start_date: '2026-02-01',
    },
    {
      id: '3',
      campaign_name: 'University Research Partnership',
      campaign_type: 'university',
      status: 'planning',
      target_hardware: 'High-spec GPUs for labs',
      unit_count: 12,
      total_estimated_value_usd: 1_200_000,
      start_date: '2026-03-01',
    },
  ]);

  return (
    <div style={{ padding: '20px', backgroundColor: '#f5f5f5' }}>
      <h2>Hardware Acquisition Campaigns</h2>
      <div style={{ marginTop: '20px' }}>
        {campaigns.map((campaign) => (
          <div
            key={campaign.id}
            style={{
              backgroundColor: 'white',
              padding: '16px',
              borderRadius: '8px',
              marginBottom: '12px',
              border: '1px solid #e0e0e0',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <div>
                <h3 style={{ margin: '0 0 8px 0' }}>{campaign.campaign_name}</h3>
                <p style={{ margin: '4px 0', color: '#666', fontSize: '14px' }}>
                  <strong>Target:</strong> {campaign.target_hardware} ({campaign.unit_count} units)
                </p>
                <p style={{ margin: '4px 0', color: '#666', fontSize: '14px' }}>
                  <strong>Estimated Value:</strong> ${(campaign.total_estimated_value_usd / 1_000_000).toFixed(2)}M
                </p>
              </div>
              <div style={{ textAlign: 'right' }}>
                <span
                  style={{
                    backgroundColor: campaign.status === 'active' ? '#4CAF50' : '#FFC107',
                    color: 'white',
                    padding: '6px 12px',
                    borderRadius: '4px',
                    fontSize: '12px',
                    fontWeight: 'bold',
                  }}
                >
                  {campaign.status.toUpperCase()}
                </span>
                <p style={{ margin: '12px 0 0 0', fontSize: '12px', color: '#999' }}>
                  Since {campaign.start_date}
                </p>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

// Component 2: ROI and Acquisition Metrics
export const AcquisitionMetricsPanel: React.FC = () => {
  const metrics: HardwareMetrics = {
    metric_date: '2026-03-02',
    total_hardware_value_usd: 7_700_000,
    total_cost_usd: 1_200_000,
    roi_percent: 541.67,
    outreach_attempts: 24,
    positive_responses: 18,
    deals_closed: 8,
    sources_engaged: 6,
  };

  const responseRate = ((metrics.positive_responses / metrics.outreach_attempts) * 100).toFixed(1);
  const conversionRate = ((metrics.deals_closed / metrics.positive_responses) * 100).toFixed(1);

  return (
    <div style={{ padding: '20px', backgroundColor: 'white', borderRadius: '8px' }}>
      <h2>Acquisition Metrics & ROI</h2>

      {/* Key Metrics Cards */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px', marginTop: '20px' }}>
        <div style={{ padding: '16px', backgroundColor: '#E8F5E9', borderRadius: '8px' }}>
          <p style={{ color: '#666', fontSize: '12px', margin: '0 0 8px 0' }}>TOTAL VALUE ACQUIRED</p>
          <h3 style={{ margin: '0', color: '#2E7D32', fontSize: '24px' }}>
            ${(metrics.total_hardware_value_usd / 1_000_000).toFixed(1)}M
          </h3>
        </div>

        <div style={{ padding: '16px', backgroundColor: '#FFF3E0', borderRadius: '8px' }}>
          <p style={{ color: '#666', fontSize: '12px', margin: '0 0 8px 0' }}>ACQUISITION COST</p>
          <h3 style={{ margin: '0', color: '#E65100', fontSize: '24px' }}>
            ${(metrics.total_cost_usd / 1_000_000).toFixed(2)}M
          </h3>
        </div>

        <div style={{ padding: '16px', backgroundColor: '#E3F2FD', borderRadius: '8px' }}>
          <p style={{ color: '#666', fontSize: '12px', margin: '0 0 8px 0' }}>ROI</p>
          <h3 style={{ margin: '0', color: '#1565C0', fontSize: '24px' }}>
            {metrics.roi_percent.toFixed(0)}%
          </h3>
          <p style={{ color: '#666', fontSize: '11px', margin: '4px 0 0 0' }}>
            Free/discounted hardware
          </p>
        </div>

        <div style={{ padding: '16px', backgroundColor: '#F3E5F5', borderRadius: '8px' }}>
          <p style={{ color: '#666', fontSize: '12px', margin: '0 0 8px 0' }}>DEALS CLOSED</p>
          <h3 style={{ margin: '0', color: '#6A1B9A', fontSize: '24px' }}>
            {metrics.deals_closed}
          </h3>
          <p style={{ color: '#666', fontSize: '11px', margin: '4px 0 0 0' }}>
            {responseRate}% response rate
          </p>
        </div>
      </div>

      {/* Detailed Metrics */}
      <div style={{ marginTop: '24px', paddingTop: '20px', borderTop: '1px solid #e0e0e0' }}>
        <h3>Outreach Performance</h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '16px', marginTop: '12px' }}>
          <div>
            <p style={{ color: '#666', fontSize: '13px', margin: '0 0 8px 0' }}>Outreach Attempts</p>
            <p style={{ margin: '0', fontSize: '20px', fontWeight: 'bold' }}>24</p>
            <div style={{ width: '100%', height: '6px', backgroundColor: '#e0e0e0', borderRadius: '3px', marginTop: '8px' }}>
              <div style={{ width: '100%', height: '100%', backgroundColor: '#2196F3', borderRadius: '3px' }} />
            </div>
          </div>

          <div>
            <p style={{ color: '#666', fontSize: '13px', margin: '0 0 8px 0' }}>Positive Responses</p>
            <p style={{ margin: '0', fontSize: '20px', fontWeight: 'bold', color: '#4CAF50' }}>
              18 ({responseRate}%)
            </p>
            <div style={{ width: '100%', height: '6px', backgroundColor: '#e0e0e0', borderRadius: '3px', marginTop: '8px' }}>
              <div style={{ width: `${parseFloat(responseRate) * 4.17}%`, height: '100%', backgroundColor: '#4CAF50', borderRadius: '3px' }} />
            </div>
          </div>

          <div>
            <p style={{ color: '#666', fontSize: '13px', margin: '0 0 8px 0' }}>Conversion Rate</p>
            <p style={{ margin: '0', fontSize: '20px', fontWeight: 'bold', color: '#FF9800' }}>
              {conversionRate}%
            </p>
            <div style={{ width: '100%', height: '6px', backgroundColor: '#e0e0e0', borderRadius: '3px', marginTop: '8px' }}>
              <div style={{ width: `${parseFloat(conversionRate) * 4.17}%`, height: '100%', backgroundColor: '#FF9800', borderRadius: '3px' }} />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

// Component 3: Hardware Inventory by Source Type
export const InventoryBySourceChart: React.FC = () => {
  const data = [
    { name: 'Manufacturer Direct', value: 2_500_000, percent: 32 },
    { name: 'Data Center', value: 2_900_000, percent: 38 },
    { name: 'Refurbisher', value: 1_500_000, percent: 19 },
    { name: 'University', value: 600_000, percent: 8 },
    { name: 'E-Waste', value: 200_000, percent: 3 },
  ];

  const COLORS = ['#2196F3', '#FF9800', '#4CAF50', '#9C27B0', '#F44336'];

  return (
    <div style={{ padding: '20px', backgroundColor: 'white', borderRadius: '8px' }}>
      <h2>Acquisition Value by Source Type</h2>
      <ResponsiveContainer width="100%" height={300}>
        <PieChart>
          <Pie data={data} cx="50%" cy="50%" labelLine={false} label={({ name, percent }) => `${name} (${percent}%)`} outerRadius={100} fill="#8884d8" dataKey="value">
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip formatter={(value) => `$${(value as number / 1_000_000).toFixed(1)}M`} />
        </PieChart>
      </ResponsiveContainer>
      <div style={{ marginTop: '20px', display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: '12px' }}>
        {data.map((item, idx) => (
          <div key={idx} style={{ padding: '12px', backgroundColor: '#f5f5f5', borderRadius: '6px', borderLeft: `4px solid ${COLORS[idx]}` }}>
            <p style={{ margin: '0 0 6px 0', fontSize: '12px', color: '#666' }}>{item.name}</p>
            <p style={{ margin: '0', fontSize: '14px', fontWeight: 'bold' }}>${(item.value / 1_000_000).toFixed(1)}M</p>
          </div>
        ))}
      </div>
    </div>
  );
};

// Component 4: Monthly Acquisition Timeline
export const AcquisitionTimeline: React.FC = () => {
  const data = [
    { month: 'Jan', acquired_usd: 400_000, cost_usd: 80_000, roi_percent: 400 },
    { month: 'Feb', acquired_usd: 1_200_000, cost_usd: 180_000, roi_percent: 567 },
    { month: 'Mar', acquired_usd: 2_100_000, cost_usd: 420_000, roi_percent: 400 },
    { month: 'Apr (Est.)', acquired_usd: 2_400_000, cost_usd: 480_000, roi_percent: 400 },
    { month: 'May (Est.)', acquired_usd: 2_700_000, cost_usd: 540_000, roi_percent: 400 },
    { month: 'Jun (Est.)', acquired_usd: 3_000_000, cost_usd: 600_000, roi_percent: 400 },
  ];

  return (
    <div style={{ padding: '20px', backgroundColor: 'white', borderRadius: '8px' }}>
      <h2>Hardware Acquisition Timeline</h2>
      <div style={{ marginTop: '20px', display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '20px' }}>
        {/* Acquired Value Timeline */}
        <div>
          <h4 style={{ margin: '0 0 12px 0' }}>Cumulative Value Acquired</h4>
          <ResponsiveContainer width="100%" height={250}>
            <LineChart data={data}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
              <XAxis dataKey="month" />
              <YAxis />
              <Tooltip formatter={(value) => `$${(value as number / 1_000_000).toFixed(1)}M`} />
              <Line type="monotone" dataKey="acquired_usd" stroke="#4CAF50" strokeWidth={2} name="Hardware Value" />
            </LineChart>
          </ResponsiveContainer>
        </div>

        {/* Cost vs Savings */}
        <div>
          <h4 style={{ margin: '0 0 12px 0' }}>Cost vs Savings</h4>
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={data}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e0e0e0" />
              <XAxis dataKey="month" />
              <YAxis />
              <Tooltip formatter={(value) => `$${(value as number / 1_000_000).toFixed(1)}M`} />
              <Legend />
              <Bar dataKey="cost_usd" fill="#FF9800" name="Acquisition Cost" />
              <Bar dataKey="acquired_usd" fill="#4CAF50" name="Hardware Value" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>
    </div>
  );
};

// Master Component: Hardware Acquisition Dashboard
export const HardwareAcquisitionDashboard: React.FC = () => {
  return (
    <div style={{ padding: '24px', backgroundColor: '#fafafa' }}>
      <h1 style={{ margin: '0 0 24px 0', color: '#333' }}>Hardware Acquisition & Logistics</h1>

      {/* Top Section: Active Campaigns */}
      <div style={{ marginBottom: '24px' }}>
        <HardwareAcquisitionTracker />
      </div>

      {/* Metrics Row */}
      <div style={{ marginBottom: '24px' }}>
        <AcquisitionMetricsPanel />
      </div>

      {/* Charts Row */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '24px', marginBottom: '24px' }}>
        <InventoryBySourceChart />
        <AcquisitionTimeline />
      </div>

      {/* Bottom: Quick Stats */}
      <div style={{ backgroundColor: 'white', padding: '20px', borderRadius: '8px' }}>
        <h3>Next Steps & Opportunities</h3>
        <div style={{ marginTop: '12px' }}>
          <div style={{ backgroundColor: '#F3E5F5', padding: '12px', borderRadius: '4px', marginBottom: '8px', borderLeft: '4px solid #9C27B0' }}>
            <strong>University Partnership:</strong> Follow up with Stanford & CMU labs on hardware donation (12 GPUs, $1.2M value)
          </div>
          <div style={{ backgroundColor: '#E3F2FD', padding: '12px', borderRadius: '4px', marginBottom: '8px', borderLeft: '4px solid #2196F3' }}>
            <strong>Data Center Q2 Refresh:</strong> Schedule calls with TechAuction & GenRocket for end-of-life inventory
          </div>
          <div style={{ backgroundColor: '#FFF3E0', padding: '12px', borderRadius: '4px', borderLeft: '4px solid #FF9800' }}>
            <strong>Corporate Surplus Prospecting:</strong> Reach out to Apple, Meta for Q2-Q3 refresh cycles
          </div>
        </div>
      </div>
    </div>
  );
};

export default HardwareAcquisitionDashboard;
