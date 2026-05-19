/**
 * Enterprise Integration Panel
 * Track: Industry vertical, Required TPS, Latency tolerance, Regulatory needs, Chain integrations, Security tier, Revenue potential
 */

import React, { useState } from 'react';

interface EnterpriseClient {
  id: string;
  name: string;
  vertical: string;
  requiredTps: number;
  latencyTolerance: number;
  chainIntegrations: string[];
  securityTier: 'basic' | 'standard' | 'enterprise' | 'institutional';
  revenuePotential: number;
  status: 'prospect' | 'negotiating' | 'deployed' | 'scaling';
}

const EnterpriseIntegrationPanel: React.FC = () => {
  const [enterprises] = useState<EnterpriseClient[]>([
    {
      id: 'ent-001',
      name: 'TradeFi Global',
      vertical: 'Trading',
      requiredTps: 500000,
      latencyTolerance: 15,
      chainIntegrations: ['Ethereum', 'Polygon', 'Arbitrum'],
      securityTier: 'institutional',
      revenuePotential: 500000,
      status: 'deployed',
    },
    {
      id: 'ent-002',
      name: 'DeFi Protocol Alpha',
      vertical: 'DeFi',
      requiredTps: 250000,
      latencyTolerance: 25,
      chainIntegrations: ['Ethereum', 'Optimism'],
      securityTier: 'enterprise',
      revenuePotential: 250000,
      status: 'negotiating',
    },
    {
      id: 'ent-003',
      name: 'Payment Gateway Inc',
      vertical: 'Payments',
      requiredTps: 1000000,
      latencyTolerance: 10,
      chainIntegrations: ['Ethereum', 'Polygon', 'Solana', 'Optimism', 'Arbitrum'],
      securityTier: 'institutional',
      revenuePotential: 750000,
      status: 'prospect',
    },
  ]);

  const getStatusColor = (status: string) => {
    const colors: Record<string, string> = {
      prospect: '#ffc107',
      negotiating: '#00d4ff',
      deployed: '#00e676',
      scaling: '#9d4edd',
    };
    return colors[status] || '#a0a0a0';
  };

  return (
    <div className="crm-panel enterprise-panel">
      <div className="panel-header">
        <h2>Enterprise Integration Mapping</h2>
        <p className="panel-subtitle">Match enterprise demand with available capacity</p>
      </div>

      <div className="enterprise-table">
        <table>
          <thead>
            <tr>
              <th>Company</th>
              <th>Vertical</th>
              <th>Required TPS</th>
              <th>Latency (ms)</th>
              <th>Chains</th>
              <th>Security Tier</th>
              <th>Revenue Potential</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {enterprises.map((ent) => (
              <tr key={ent.id}>
                <td className="company-name">{ent.name}</td>
                <td>{ent.vertical}</td>
                <td>{(ent.requiredTps / 1000).toFixed(0)}k</td>
                <td>{ent.latencyTolerance}ms</td>
                <td>
                  <div className="chain-tags">
                    {ent.chainIntegrations.slice(0, 2).map((chain) => (
                      <span key={chain} className="chain-tag">{chain}</span>
                    ))}
                    {ent.chainIntegrations.length > 2 && (
                      <span className="chain-tag">+{ent.chainIntegrations.length - 2}</span>
                    )}
                  </div>
                </td>
                <td>{ent.securityTier}</td>
                <td className="revenue">${(ent.revenuePotential / 1000).toFixed(0)}k</td>
                <td>
                  <span className="status-badge" style={{ color: getStatusColor(ent.status) }}>
                    {ent.status}
                  </span>
                </td>
                <td>
                  <button className="table-action-btn">View</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <style>{`
        .enterprise-panel {
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

        .enterprise-table {
          overflow-x: auto;
        }

        table {
          width: 100%;
          border-collapse: collapse;
          background: rgba(255, 255, 255, 0.02);
        }

        thead {
          background: rgba(0, 212, 255, 0.05);
          border-bottom: 2px solid rgba(0, 212, 255, 0.2);
        }

        th {
          padding: 12px 16px;
          text-align: left;
          font-size: 11px;
          font-weight: 600;
          color: #00d4ff;
          text-transform: uppercase;
          letter-spacing: 0.5px;
        }

        td {
          padding: 12px 16px;
          border-bottom: 1px solid rgba(0, 212, 255, 0.1);
          font-size: 12px;
          color: #c0c0c0;
        }

        tbody tr:hover {
          background: rgba(0, 212, 255, 0.05);
        }

        .company-name {
          color: #00d4ff;
          font-weight: 600;
        }

        .chain-tags {
          display: flex;
          gap: 6px;
          flex-wrap: wrap;
        }

        .chain-tag {
          background: rgba(0, 212, 255, 0.2);
          color: #00d4ff;
          padding: 2px 6px;
          border-radius: 3px;
          font-size: 10px;
          font-weight: 600;
        }

        .revenue {
          color: #00e676;
          font-weight: 600;
        }

        .status-badge {
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.5px;
          font-size: 10px;
        }

        .table-action-btn {
          padding: 4px 8px;
          background: rgba(0, 212, 255, 0.1);
          border: 1px solid rgba(0, 212, 255, 0.3);
          color: #00d4ff;
          border-radius: 3px;
          font-size: 10px;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .table-action-btn:hover {
          background: rgba(0, 212, 255, 0.2);
        }
      `}</style>
    </div>
  );
};

export default EnterpriseIntegrationPanel;
