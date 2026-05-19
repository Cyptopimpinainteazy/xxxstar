/**
 * Validator Capacity Panel
 * Track: Node ID, Region, GPU Type, GPU Count, VRAM, Latency, Uptime %, TPS Capacity, TPS Sustained, Hardware Verified, Risk Score, Economic Tier, Onboarding Status
 */

import React, { useState } from 'react';

interface ValidatorNode {
  id: string;
  region: string;
  gpuType: string;
  gpuCount: number;
  vram: number;
  latency: number;
  uptimePercent: number;
  tpsCapacity: number;
  tpsSustained: number;
  hardwareVerified: boolean;
  riskScore: number;
  economicTier: 'basic' | 'standard' | 'enterprise';
  onboardingStatus: 'pending' | 'verified' | 'active' | 'suspended';
}

const ValidatorCapacityPanel: React.FC = () => {
  const [validators] = useState<ValidatorNode[]>([
    {
      id: 'val-001',
      region: 'us-east-1',
      gpuType: 'H100',
      gpuCount: 8,
      vram: 640,
      latency: 12,
      uptimePercent: 99.95,
      tpsCapacity: 500000,
      tpsSustained: 450000,
      hardwareVerified: true,
      riskScore: 2,
      economicTier: 'enterprise',
      onboardingStatus: 'active',
    },
    {
      id: 'val-002',
      region: 'eu-west-1',
      gpuType: 'L40S',
      gpuCount: 4,
      vram: 192,
      latency: 18,
      uptimePercent: 99.87,
      tpsCapacity: 250000,
      tpsSustained: 220000,
      hardwareVerified: true,
      riskScore: 4,
      economicTier: 'standard',
      onboardingStatus: 'active',
    },
    {
      id: 'val-003',
      region: 'ap-south-1',
      gpuType: 'A100',
      gpuCount: 2,
      vram: 80,
      latency: 45,
      uptimePercent: 98.2,
      tpsCapacity: 100000,
      tpsSustained: 80000,
      hardwareVerified: false,
      riskScore: 8,
      economicTier: 'basic',
      onboardingStatus: 'pending',
    },
  ]);

  const getRiskBadgeColor = (score: number) => {
    if (score <= 3) return '#00e676';
    if (score <= 6) return '#ffc107';
    return '#ff6b6b';
  };

  const getStatusBadge = (status: string) => {
    const badges: Record<string, { bg: string; color: string }> = {
      active: { bg: 'rgba(0, 230, 118, 0.2)', color: '#00e676' },
      verified: { bg: 'rgba(0, 212, 255, 0.2)', color: '#00d4ff' },
      pending: { bg: 'rgba(255, 193, 7, 0.2)', color: '#ffc107' },
      suspended: { bg: 'rgba(255, 107, 107, 0.2)', color: '#ff6b6b' },
    };
    return badges[status] || badges.pending;
  };

  return (
    <div className="crm-panel validator-panel">
      <div className="panel-header">
        <h2>Validator Node Capacity</h2>
        <p className="panel-subtitle">Track GPU capacity, TPS, latency, uptime, and risk scores</p>
      </div>

      <div className="validator-grid">
        {validators.map((validator) => (
          <div key={validator.id} className="validator-card">
            <div className="card-header">
              <div className="validator-id">{validator.id}</div>
              <div className="validator-status" style={getStatusBadge(validator.onboardingStatus)}>
                {validator.onboardingStatus}
              </div>
            </div>

            <div className="card-body">
              <div className="metric-row">
                <span className="label">Region</span>
                <span className="value">{validator.region}</span>
              </div>

              <div className="metric-row">
                <span className="label">GPU Type</span>
                <span className="value">{validator.gpuType} × {validator.gpuCount}</span>
              </div>

              <div className="metric-row">
                <span className="label">VRAM</span>
                <span className="value">{validator.vram}GB</span>
              </div>

              <div className="metric-row">
                <span className="label">Latency</span>
                <span className="value">{validator.latency}ms</span>
              </div>

              <div className="metric-row">
                <span className="label">Uptime</span>
                <span className="value">{validator.uptimePercent}%</span>
              </div>

              <div className="metric-row">
                <span className="label">TPS Capacity</span>
                <span className="value">{(validator.tpsCapacity / 1000).toFixed(0)}k</span>
              </div>

              <div className="metric-row">
                <span className="label">TPS Sustained</span>
                <span className="value">{(validator.tpsSustained / 1000).toFixed(0)}k</span>
              </div>

              <div className="metric-row">
                <span className="label">Hardware</span>
                <span className="value">
                  {validator.hardwareVerified ? '✓ Verified' : '⚠ Pending'}
                </span>
              </div>

              <div className="metric-row">
                <span className="label">Risk Score</span>
                <span className="value" style={{ color: getRiskBadgeColor(validator.riskScore) }}>
                  {validator.riskScore}/10
                </span>
              </div>

              <div className="metric-row">
                <span className="label">Economic Tier</span>
                <span className="value">{validator.economicTier.toUpperCase()}</span>
              </div>
            </div>

            <div className="card-actions">
              <button className="action-btn-small">Edit</button>
              <button className="action-btn-small">Monitor</button>
              <button className="action-btn-small">Details</button>
            </div>
          </div>
        ))}
      </div>

      <div className="panel-footer">
        <button className="add-btn">+ Add New Validator Node</button>
      </div>

      <style>{`
        .validator-panel {
          padding: 0;
        }

        .panel-header {
          padding: 20px 24px;
          border-bottom: 1px solid rgba(0, 212, 255, 0.2);
        }

        .panel-header h2 {
          margin: 0 0 4px 0;
          font-size: 18px;
          color: #e0e0e0;
        }

        .panel-subtitle {
          margin: 0;
          font-size: 12px;
          color: #a0a0a0;
        }

        .validator-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
          gap: 16px;
          padding: 24px;
        }

        .validator-card {
          background: rgba(255, 255, 255, 0.05);
          border: 1px solid rgba(0, 212, 255, 0.2);
          border-radius: 8px;
          overflow: hidden;
          transition: all 0.2s ease;
        }

        .validator-card:hover {
          background: rgba(255, 255, 255, 0.08);
          border-color: rgba(0, 212, 255, 0.4);
          box-shadow: 0 4px 12px rgba(0, 212, 255, 0.1);
        }

        .card-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 12px 16px;
          background: rgba(0, 212, 255, 0.05);
          border-bottom: 1px solid rgba(0, 212, 255, 0.1);
        }

        .validator-id {
          font-weight: 600;
          color: #00d4ff;
          font-family: 'Monaco', 'Menlo', monospace;
          font-size: 12px;
        }

        .validator-status {
          padding: 2px 8px;
          border-radius: 4px;
          font-size: 10px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.5px;
        }

        .card-body {
          padding: 16px;
        }

        .metric-row {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 6px 0;
          font-size: 12px;
          border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        }

        .metric-row:last-child {
          border-bottom: none;
        }

        .metric-row .label {
          color: #a0a0a0;
        }

        .metric-row .value {
          color: #00d4ff;
          font-weight: 600;
          font-family: 'Monaco', 'Menlo', monospace;
        }

        .card-actions {
          display: flex;
          gap: 8px;
          padding: 12px 16px;
          background: rgba(0, 0, 0, 0.2);
          border-top: 1px solid rgba(0, 212, 255, 0.1);
        }

        .action-btn-small {
          flex: 1;
          padding: 6px 8px;
          background: rgba(0, 212, 255, 0.1);
          border: 1px solid rgba(0, 212, 255, 0.3);
          color: #00d4ff;
          border-radius: 4px;
          font-size: 10px;
          font-weight: 600;
          cursor: pointer;
          transition: all 0.2s ease;
          text-transform: uppercase;
          letter-spacing: 0.5px;
        }

        .action-btn-small:hover {
          background: rgba(0, 212, 255, 0.2);
          border-color: rgba(0, 212, 255, 0.5);
        }

        .panel-footer {
          padding: 16px 24px;
          border-top: 1px solid rgba(0, 212, 255, 0.1);
          display: flex;
          justify-content: flex-end;
        }

        .add-btn {
          padding: 8px 16px;
          background: rgba(0, 230, 118, 0.2);
          border: 1px solid rgba(0, 230, 118, 0.4);
          color: #00e676;
          border-radius: 6px;
          font-size: 12px;
          font-weight: 600;
          cursor: pointer;
          transition: all 0.2s ease;
          text-transform: uppercase;
          letter-spacing: 0.5px;
        }

        .add-btn:hover {
          background: rgba(0, 230, 118, 0.3);
          border-color: rgba(0, 230, 118, 0.6);
        }
      `}</style>
    </div>
  );
};

export default ValidatorCapacityPanel;
