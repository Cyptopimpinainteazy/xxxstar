/**
 * X3 Protocol-Native CRM Dashboard
 * 
 * Infrastructure Capacity Architect's Command Center
 * - Validator Node tracking (GPU capacity, TPS, uptime, risk)
 * - Enterprise Integration mapping (demand ↔ capacity)
 * - Infrastructure Capacity forecasting
 * - Revenue intelligence & deal tracking
 * - Real-time blockchain metrics integration
 */

import React, { useState } from 'react';
import './crm.css';
import ValidatorCapacityPanel from './panels/ValidatorCapacityPanel';
import EnterpriseIntegrationPanel from './panels/EnterpriseIntegrationPanel';
import CapacityForecastPanel from './panels/CapacityForecastPanel';
import RevenueIntelligencePanel from './panels/RevenueIntelligencePanel';
import PipelineKanbanPanel from './panels/PipelineKanbanPanel';

type ViewTab = 'overview' | 'validators' | 'enterprise' | 'forecast' | 'revenue' | 'pipeline';

const X3CRMDashboard: React.FC = () => {
  const [activeTab, setActiveTab] = useState<ViewTab>('overview');

  const tabs: Array<{ id: ViewTab; label: string; icon: string; description: string }> = [
    { id: 'overview', label: 'Overview', icon: '📊', description: 'Key metrics snapshot' },
    { id: 'validators', label: 'Validators', icon: '⚙️', description: 'Node capacity & health' },
    { id: 'enterprise', label: 'Enterprise', icon: '🏢', description: 'Client demand mapping' },
    { id: 'forecast', label: 'Forecast', icon: '📈', description: 'Capacity predictions' },
    { id: 'revenue', label: 'Revenue', icon: '💰', description: 'Deal intelligence' },
    { id: 'pipeline', label: 'Pipeline', icon: '🎯', description: 'Kanban deal tracking' },
  ];

  return (
    <div className="x3-crm-container">
      {/* Header */}
      <div className="crm-header">
        <div className="crm-header-content">
          <h1 className="crm-title">
            <span className="crm-icon">🏗️</span>
            X3 Protocol-Native CRM
          </h1>
          <p className="crm-subtitle">
            Infrastructure Capacity Architect's Command Center
          </p>
        </div>
        <div className="crm-status">
          <div className="status-item">
            <span className="status-label">Global TPS</span>
            <span className="status-value">1.85M</span>
          </div>
          <div className="status-item">
            <span className="status-label">Active Validators</span>
            <span className="status-value">487</span>
          </div>
          <div className="status-item">
            <span className="status-label">Enterprise Deals</span>
            <span className="status-value">23</span>
          </div>
        </div>
      </div>

      {/* Tab Navigation */}
      <div className="crm-tabs">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            className={`crm-tab ${activeTab === tab.id ? 'active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
            title={tab.description}
          >
            <span className="tab-icon">{tab.icon}</span>
            <span className="tab-label">{tab.label}</span>
          </button>
        ))}
      </div>

      {/* Content Area */}
      <div className="crm-content">
        {activeTab === 'overview' && (
          <OverviewPanel />
        )}
        {activeTab === 'validators' && (
          <ValidatorCapacityPanel />
        )}
        {activeTab === 'enterprise' && (
          <EnterpriseIntegrationPanel />
        )}
        {activeTab === 'forecast' && (
          <CapacityForecastPanel />
        )}
        {activeTab === 'revenue' && (
          <RevenueIntelligencePanel />
        )}
        {activeTab === 'pipeline' && (
          <PipelineKanbanPanel />
        )}
      </div>
    </div>
  );
};

/**
 * Overview Panel — Key metrics snapshot
 */
const OverviewPanel: React.FC = () => {
  return (
    <div className="crm-panel overview-panel">
      <h2>Dashboard Overview</h2>
      
      <div className="overview-grid">
        {/* Infrastructure Capacity */}
        <div className="overview-card">
          <h3>🏗️ Infrastructure Capacity</h3>
          <div className="metric">
            <span>Global TPS Capacity</span>
            <span className="value">1,850,000 TPS</span>
          </div>
          <div className="metric">
            <span>Regional Headroom (EU West)</span>
            <span className="value">28%</span>
          </div>
          <div className="metric">
            <span>Available Validators</span>
            <span className="value">487 nodes</span>
          </div>
          <div className="progress-bar">
            <div className="progress" style={{ width: '72%' }}></div>
            <span className="progress-label">72% utilized</span>
          </div>
        </div>

        {/* Enterprise Pipeline */}
        <div className="overview-card">
          <h3>🏢 Enterprise Pipeline</h3>
          <div className="metric">
            <span>Active Deals</span>
            <span className="value">23</span>
          </div>
          <div className="metric">
            <span>Potential Revenue (ARR)</span>
            <span className="value">$4.2M</span>
          </div>
          <div className="metric">
            <span>Average Deal Size</span>
            <span className="value">$182k</span>
          </div>
          <div className="pipeline-stage">
            <span>Stage:</span>
            <span className="badge">Contract Review</span>
          </div>
        </div>

        {/* Revenue Intelligence */}
        <div className="overview-card">
          <h3>💰 Revenue Intelligence</h3>
          <div className="metric">
            <span>Revenue per TPS</span>
            <span className="value">$2.27/TPS/year</span>
          </div>
          <div className="metric">
            <span>GPU ROI Timeline</span>
            <span className="value">8.4 months</span>
          </div>
          <div className="metric">
            <span>Monthly Recurring Revenue</span>
            <span className="value">$185k</span>
          </div>
        </div>

        {/* Risk & Performance */}
        <div className="overview-card">
          <h3>⚠️ Risk & Performance</h3>
          <div className="metric">
            <span>Nodes with Risk Score &gt; 7</span>
            <span className="value danger">12</span>
          </div>
          <div className="metric">
            <span>Average Uptime</span>
            <span className="value">99.87%</span>
          </div>
          <div className="metric">
            <span>Slashing Events (7d)</span>
            <span className="value">2</span>
          </div>
        </div>
      </div>

      {/* Quick Actions */}
      <div className="quick-actions">
        <h3>Quick Actions</h3>
        <div className="action-buttons">
          <button className="action-btn primary">+ Add Validator Node</button>
          <button className="action-btn primary">+ Create Enterprise Deal</button>
          <button className="action-btn secondary">🔍 Analyze Capacity</button>
          <button className="action-btn secondary">📊 Export Report</button>
        </div>
      </div>
    </div>
  );
};

export default X3CRMDashboard;
