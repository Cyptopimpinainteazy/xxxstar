// TIER 9: React Dashboard Components for Funding War Plan
// Financial projections, cap table simulator, runway visualization

import React, { useState, useEffect } from 'react';
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from 'recharts';

// ================================================
// FINANCIAL PROJECTION DASHBOARD
// ================================================

export const FinancialProjectionChart: React.FC<{ projectionData: any }> = ({
  projectionData,
}) => {
  const monthlyData = projectionData?.monthly_breakdown || [];

  return (
    <div style={{ width: '100%', height: 600 }}>
      <h3>12-Month Financial Projection (Base Case)</h3>

      {/* Revenue vs Burn Chart */}
      <ResponsiveContainer width="100%" height={350}>
        <AreaChart data={monthlyData}>
          <defs>
            <linearGradient id="colorRevenue" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#8884d8" stopOpacity={0.8} />
              <stop offset="95%" stopColor="#8884d8" stopOpacity={0} />
            </linearGradient>
            <linearGradient id="colorBurn" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#ff7300" stopOpacity={0.8} />
              <stop offset="95%" stopColor="#ff7300" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="month" />
          <YAxis />
          <Tooltip
            formatter={(value) => `$${(Number(value) / 1000).toFixed(1)}k`}
            labelFormatter={(label) => `Month ${label}`}
          />
          <Legend />
          <Area
            type="monotone"
            dataKey="revenue"
            stroke="#8884d8"
            fillOpacity={1}
            fill="url(#colorRevenue)"
            name="Revenue"
          />
          <Area
            type="monotone"
            dataKey="burn"
            stroke="#ff7300"
            fillOpacity={1}
            fill="url(#colorBurn)"
            name="Burn Rate"
          />
        </AreaChart>
      </ResponsiveContainer>

      {/* Cash Position Chart */}
      <ResponsiveContainer width="100%" height={250}>
        <LineChart data={monthlyData}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="month" />
          <YAxis />
          <Tooltip formatter={(value) => `$${(Number(value) / 1000000).toFixed(1)}M`} />
          <Legend />
          <Line
            type="monotone"
            dataKey="cash_position"
            stroke="#82ca9d"
            strokeWidth={2}
            name="Cash Position"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
};

// ================================================
// SCENARIO COMPARISON (Base/Bull/Bear)
// ================================================

export const ScenarioComparison: React.FC<{ scenarios: any }> = ({
  scenarios,
}) => {
  const scenarioData = [
    {
      name: 'Base Case',
      revenue: scenarios?.base_case?.year1_revenue || 0,
      burn: scenarios?.base_case?.year1_burn || 0,
      net: (scenarios?.base_case?.year1_net || 0),
      probability: '60%',
    },
    {
      name: 'Bull Case',
      revenue: scenarios?.bull_case?.year1_revenue || 0,
      burn: scenarios?.bull_case?.year1_burn || 0,
      net: (scenarios?.bull_case?.year1_net || 0),
      probability: '20%',
    },
    {
      name: 'Bear Case',
      revenue: scenarios?.bear_case?.year1_revenue || 0,
      burn: scenarios?.bear_case?.year1_burn || 0,
      net: (scenarios?.bear_case?.year1_net || 0),
      probability: '20%',
    },
  ];

  return (
    <div style={{ width: '100%', marginTop: 20 }}>
      <h3>Scenario Analysis (Year 1)</h3>
      <ResponsiveContainer width="100%" height={350}>
        <BarChart data={scenarioData}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="name" />
          <YAxis />
          <Tooltip formatter={(value) => `$${(Number(value) / 1000000).toFixed(1)}M`} />
          <Legend />
          <Bar dataKey="revenue" fill="#8884d8" name="Revenue" />
          <Bar dataKey="burn" fill="#ff7300" name="Burn" />
          <Bar dataKey="net" fill="#82ca9d" name="Net Income" />
        </BarChart>
      </ResponsiveContainer>

      <table style={{ marginTop: 20, width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr style={{ borderBottom: '2px solid #ddd' }}>
            <th style={{ padding: 10, textAlign: 'left' }}>Scenario</th>
            <th style={{ padding: 10, textAlign: 'right' }}>Revenue</th>
            <th style={{ padding: 10, textAlign: 'right' }}>Burn</th>
            <th style={{ padding: 10, textAlign: 'right' }}>Net</th>
            <th style={{ padding: 10, textAlign: 'right' }}>Probability</th>
          </tr>
        </thead>
        <tbody>
          {scenarioData.map((row) => (
            <tr
              key={row.name}
              style={{
                borderBottom: '1px solid #eee',
                backgroundColor: row.name === 'Base Case' ? '#f0f0f0' : 'white',
              }}
            >
              <td style={{ padding: 10 }}>{row.name}</td>
              <td style={{ padding: 10, textAlign: 'right' }}>
                ${(row.revenue / 1000000).toFixed(1)}M
              </td>
              <td style={{ padding: 10, textAlign: 'right' }}>
                ${(row.burn / 1000000).toFixed(1)}M
              </td>
              <td
                style={{
                  padding: 10,
                  textAlign: 'right',
                  color: row.net >= 0 ? 'green' : 'red',
                  fontWeight: 'bold',
                }}
              >
                ${(row.net / 1000000).toFixed(1)}M
              </td>
              <td style={{ padding: 10, textAlign: 'right' }}>{row.probability}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

// ================================================
// CAP TABLE SIMULATOR
// ================================================

export const CapTableSimulator: React.FC<{ capTable: any }> = ({ capTable }) => {
  const [showDilution, setShowDilution] = useState(false);

  const seedRound = capTable?.seed_round;
  const futureRounds = capTable?.future_rounds || [];
  const exitScenario = capTable?.exit_scenario;

  const pieData = [
    {
      name: 'Founders',
      value: parseFloat(seedRound?.founder_ownership_post) || 0,
    },
    {
      name: 'Investors',
      value: 100 - (parseFloat(seedRound?.founder_ownership_post) || 0),
    },
  ];

  const COLORS = ['#82ca9d', '#8884d8'];

  return (
    <div style={{ width: '100%', marginTop: 20 }}>
      <h3>Cap Table Analysis</h3>

      {/* Seed Round Details */}
      <div
        style={{
          backgroundColor: '#f5f5f5',
          padding: 15,
          borderRadius: 8,
          marginBottom: 20,
        }}
      >
        <h4>Seed Round</h4>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 15 }}>
          <div>
            <strong>Raise Amount:</strong>
            <p>${seedRound?.raise_amount || 0}</p>
          </div>
          <div>
            <strong>Valuation:</strong>
            <p>${seedRound?.valuation || 0}</p>
          </div>
          <div>
            <strong>Share Price:</strong>
            <p>${seedRound?.share_price || 0}</p>
          </div>
          <div>
            <strong>Founder Ownership (Pre):</strong>
            <p>{seedRound?.founder_ownership_pre}</p>
          </div>
          <div>
            <strong>Founder Ownership (Post):</strong>
            <p>{seedRound?.founder_ownership_post}</p>
          </div>
          <div>
            <strong>Dilution:</strong>
            <p style={{ color: 'red' }}>{seedRound?.dilution}</p>
          </div>
        </div>
      </div>

      {/* Ownership Pie Chart */}
      <ResponsiveContainer width="100%" height={300}>
        <PieChart>
          <Pie
            data={pieData}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={({ name, value }) => `${name}: ${Number(value ?? 0).toFixed(1)}%`}
            outerRadius={80}
            fill="#8884d8"
            dataKey="value"
          >
            {pieData.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip formatter={(value) => `${Number(value).toFixed(1)}%`} />
        </PieChart>
      </ResponsiveContainer>

      {/* Future Rounds Dilution */}
      <button
        onClick={() => setShowDilution(!showDilution)}
        style={{
          marginTop: 20,
          padding: 10,
          backgroundColor: '#007bff',
          color: 'white',
          border: 'none',
          borderRadius: 4,
          cursor: 'pointer',
        }}
      >
        {showDilution ? 'Hide' : 'Show'} Future Rounds Dilution
      </button>

      {showDilution && (
        <div style={{ marginTop: 20 }}>
          <h4>Future Rounds & Dilution Scenarios</h4>
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr style={{ borderBottom: '2px solid #ddd', backgroundColor: '#f0f0f0' }}>
                <th style={{ padding: 10, textAlign: 'left' }}>Round</th>
                <th style={{ padding: 10, textAlign: 'right' }}>Raise</th>
                <th style={{ padding: 10, textAlign: 'right' }}>Valuation</th>
                <th style={{ padding: 10, textAlign: 'right' }}>Founder Pre %</th>
                <th style={{ padding: 10, textAlign: 'right' }}>Founder Post %</th>
                <th style={{ padding: 10, textAlign: 'right' }}>Dilution</th>
                <th style={{ padding: 10, textAlign: 'right' }}>Cumulative</th>
              </tr>
            </thead>
            <tbody>
              {futureRounds.map((round: any, idx: number) => (
                <tr key={idx} style={{ borderBottom: '1px solid #eee' }}>
                  <td style={{ padding: 10 }}>{round.round}</td>
                  <td style={{ padding: 10, textAlign: 'right' }}>
                    ${(round.raise_amount / 1000000).toFixed(0)}M
                  </td>
                  <td style={{ padding: 10, textAlign: 'right' }}>
                    ${(round.valuation / 1000000).toFixed(0)}M
                  </td>
                  <td style={{ padding: 10, textAlign: 'right' }}>
                    {round.founder_ownership_pre}
                  </td>
                  <td style={{ padding: 10, textAlign: 'right' }}>
                    {round.founder_ownership_post}
                  </td>
                  <td style={{ padding: 10, textAlign: 'right', color: 'red' }}>
                    {round.dilution_this_round}
                  </td>
                  <td style={{ padding: 10, textAlign: 'right', fontWeight: 'bold' }}>
                    {round.cumulative_dilution}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          {/* Exit Scenario */}
          {exitScenario && (
            <div
              style={{
                backgroundColor: '#e8f5e9',
                padding: 15,
                borderRadius: 8,
                marginTop: 20,
              }}
            >
              <h4>Exit Scenario (1B Valuation)</h4>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 15 }}>
                <div>
                  <strong>Founder Equity Value:</strong>
                  <p>{exitScenario.founder_equity_value}</p>
                </div>
                <div>
                  <strong>Final Founder Ownership:</strong>
                  <p>{exitScenario.final_founder_ownership}</p>
                </div>
                <div>
                  <strong>Total Dilution:</strong>
                  <p style={{ color: 'red' }}>
                    {exitScenario.total_cumulative_dilution}
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

// ================================================
// TREASURY RUNWAY VISUALIZATION
// ================================================

export const TreasuryRunwayDashboard: React.FC<{ runwayData: any }> = ({
  runwayData,
}) => {
  const monthlyBreakdown = runwayData?.breakdown_36m || [];
  const funding = runwayData?.funding_strategy || {};
  const thresholds = runwayData?.cash_thresholds || {};

  // Color code by status
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'healthy':
        return '#82ca9d';
      case 'caution':
        return '#ffc658';
      case 'critical':
        return '#ff7300';
      case 'funded_or_out':
        return '#1f77b4';
      default:
        return '#8884d8';
    }
  };

  return (
    <div style={{ width: '100%', marginTop: 20 }}>
      <h3>36-Month Treasury Runway Analysis</h3>

      {/* Key Metrics */}
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1fr 1fr 1fr',
          gap: 15,
          marginBottom: 20,
        }}
      >
        <div style={{ backgroundColor: '#f0f0f0', padding: 15, borderRadius: 8 }}>
          <strong>Current Cash:</strong>
          <p style={{ fontSize: 18, color: '#1f77b4', marginTop: 5 }}>
            ${(runwayData?.current_cash / 1000000).toFixed(1)}M
          </p>
        </div>
        <div style={{ backgroundColor: '#f0f0f0', padding: 15, borderRadius: 8 }}>
          <strong>Monthly Burn:</strong>
          <p style={{ fontSize: 18, color: '#ff7300', marginTop: 5 }}>
            ${(runwayData?.monthly_burn / 1000000).toFixed(1)}M
          </p>
        </div>
        <div style={{ backgroundColor: '#f0f0f0', padding: 15, borderRadius: 8 }}>
          <strong>Current Runway:</strong>
          <p style={{ fontSize: 18, color: '#82ca9d', marginTop: 5 }}>
            {runwayData?.current_runway_months} months
          </p>
        </div>
        <div
          style={{
            backgroundColor: '#ffe6e6',
            padding: 15,
            borderRadius: 8,
            borderLeft: '4px solid red',
          }}
        >
          <strong>Status:</strong>
          <p style={{ fontSize: 16, color: 'red', marginTop: 5, fontWeight: 'bold' }}>
            {funding?.recommendation || 'N/A'}
          </p>
        </div>
      </div>

      {/* Runway Chart (36 months) */}
      <ResponsiveContainer width="100%" height={350}>
        <AreaChart data={monthlyBreakdown}>
          <defs>
            <linearGradient id="colorRunway" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#82ca9d" stopOpacity={0.8} />
              <stop offset="95%" stopColor="#82ca9d" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="month" />
          <YAxis />
          <Tooltip
            formatter={(value) => `$${(Number(value) / 1000000).toFixed(1)}M`}
            labelFormatter={(label) => `Month ${label}`}
          />
          <Area
            type="monotone"
            dataKey="cash_balance"
            stroke="#82ca9d"
            fillOpacity={1}
            fill="url(#colorRunway)"
            name="Cash Balance"
          />
        </AreaChart>
      </ResponsiveContainer>

      {/* Critical Thresholds */}
      <div style={{ marginTop: 20 }}>
        <h4>Cash Thresholds</h4>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr 1fr', gap: 15 }}>
          <div style={{ backgroundColor: '#ffe6e6', padding: 10, borderRadius: 4 }}>
            <strong>Emergency:</strong>
            <p>${(thresholds?.emergency / 1000000).toFixed(1)}M</p>
          </div>
          <div style={{ backgroundColor: '#fff3cd', padding: 10, borderRadius: 4 }}>
            <strong>Critical:</strong>
            <p>${(thresholds?.critical / 1000000).toFixed(1)}M</p>
          </div>
          <div style={{ backgroundColor: '#fff9e6', padding: 10, borderRadius: 4 }}>
            <strong>Caution:</strong>
            <p>${(thresholds?.caution / 1000000).toFixed(1)}M</p>
          </div>
          <div style={{ backgroundColor: '#e8f5e9', padding: 10, borderRadius: 4 }}>
            <strong>Healthy:</strong>
            <p>${(thresholds?.healthy / 1000000).toFixed(1)}M</p>
          </div>
        </div>
      </div>

      {/* Funding Requirements */}
      <div
        style={{
          backgroundColor: '#e3f2fd',
          padding: 15,
          borderRadius: 8,
          marginTop: 20,
        }}
      >
        <h4>Funding Requirements</h4>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 15 }}>
          <div>
            <strong>For 24-Month Runway:</strong>
            <p>${(funding?.funding_for_24m_runway / 1000000).toFixed(1)}M additional</p>
          </div>
          <div>
            <strong>For 36-Month Runway:</strong>
            <p>${(funding?.funding_for_36m_runway / 1000000).toFixed(1)}M additional</p>
          </div>
          <div>
            <strong>Recommendation:</strong>
            <p style={{ fontWeight: 'bold', color: '#d32f2f' }}>
              {funding?.recommendation}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

// ================================================
// MAIN WAR PLAN DASHBOARD
// ================================================

export const WarPlanDashboard: React.FC<{ planId: string }> = ({ planId }) => {
  const [projectionData, setProjectionData] = useState(null);
  const [capTableData, setCapTableData] = useState(null);
  const [runwayData, setRunwayData] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Load data from Tauri commands
    loadPlanData();
  }, [planId]);

  const loadPlanData = async () => {
    try {
      // This would call actual Tauri commands
      // const projection = await invoke('crm_calculate_financial_projection', {...});
      setLoading(false);
    } catch (error) {
      console.error('Failed to load plan:', error);
      setLoading(false);
    }
  };

  if (loading) return <div>Loading...</div>;

  return (
    <div style={{ padding: 20, maxWidth: 1400, margin: '0 auto' }}>
      <h1>Funding War Plan v1.0 Dashboard</h1>
      <p style={{ color: '#666', marginBottom: 30 }}>
        Comprehensive financial modeling, cap table analysis, and runway tracking
      </p>

      {projectionData && <FinancialProjectionChart projectionData={projectionData} />}
      {projectionData && <ScenarioComparison scenarios={projectionData} />}
      {capTableData && <CapTableSimulator capTable={capTableData} />}
      {runwayData && <TreasuryRunwayDashboard runwayData={runwayData} />}
    </div>
  );
};

export default WarPlanDashboard;
