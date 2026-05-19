/**
 * Capacity Forecast Panel
 * Regional TPS saturation forecasting, new GPU deployment predictions, scaling requirements
 */

import React from 'react';

const CapacityForecastPanel: React.FC = () => {
  return (
    <div className="crm-panel forecast-panel">
      <div className="panel-header">
        <h2>Infrastructure Capacity Forecast</h2>
        <p className="panel-subtitle">Predict when regional TPS saturates, GPU deployment needs</p>
      </div>

      <div className="forecast-content">
        <div className="forecast-chart">
          <h3>Global TPS Utilization Projection (12 months)</h3>
          <div className="chart-placeholder">
            <div style={{ opacity: 0.5, color: '#a0a0a0', padding: '40px', textAlign: 'center' }}>
              📊 Chart visualization will render here<br />
              Current: 72% | Forecast: 89% (Month 12)
            </div>
          </div>
        </div>

        <div className="forecast-grid">
          <div className="forecast-card">
            <h4>🟢 EU West</h4>
            <div className="metric"><span>Current Util:</span> <span className="value">68%</span></div>
            <div className="metric"><span>Saturation (Est):</span> <span className="value">Month 8</span></div>
            <div className="metric"><span>New GPUs Needed:</span> <span className="value">+24 H100s</span></div>
            <button className="action-btn-sm">Plan Deployment →</button>
          </div>

          <div className="forecast-card">
            <h4>🟡 US East</h4>
            <div className="metric"><span>Current Util:</span> <span className="value">81%</span></div>
            <div className="metric"><span>Saturation (Est):</span> <span className="value">Month 4</span></div>
            <div className="metric"><span>New GPUs Needed:</span> <span className="value">+32 H100s</span></div>
            <button className="action-btn-sm">Urgent: Plan Now →</button>
          </div>

          <div className="forecast-card">
            <h4>🟢 AP South</h4>
            <div className="metric"><span>Current Util:</span> <span className="value">45%</span></div>
            <div className="metric"><span>Saturation (Est):</span> <span className="value">Month 14</span></div>
            <div className="metric"><span>New GPUs Needed:</span> <span className="value">+12 A100s</span></div>
            <button className="action-btn-sm">Monitor</button>
          </div>
        </div>

        <div className="forecast-insights">
          <h3>Intelligence</h3>
          <ul>
            <li>🔴 Production Alert: US East will saturate in <strong>4 months</strong> at current growth</li>
            <li>💰 GPU ROI: Average breakeven timeline is <strong>8.4 months</strong></li>
            <li>📈 Growth Rate: +15% monthly (enterprise contracts)</li>
            <li>⚡ Recommended: Deploy 96 additional H100 GPUs in next 60 days</li>
          </ul>
        </div>
      </div>

      <style>{`
        .forecast-panel {
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

        .forecast-content {
          padding: 24px;
        }

        .forecast-chart {
          margin-bottom: 24px;
          background: rgba(255, 255, 255, 0.05);
          border: 1px solid rgba(0, 212, 255, 0.2);
          border-radius: 8px;
          padding: 16px;
        }

        .forecast-chart h3 {
          margin: 0 0 12px 0;
          font-size: 14px;
          color: #00d4ff;
        }

        .chart-placeholder {
          background: rgba(0, 0, 0, 0.3);
          border-radius: 6px;
          min-height: 200px;
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .forecast-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
          gap: 16px;
          margin-bottom: 24px;
        }

        .forecast-card {
          background: rgba(255, 255, 255, 0.05);
          border: 1px solid rgba(0, 212, 255, 0.2);
          border-radius: 8px;
          padding: 16px;
        }

        .forecast-card h4 {
          margin: 0 0 12px 0;
          font-size: 14px;
          color: #00d4ff;
        }

        .forecast-card .metric {
          display: flex;
          justify-content: space-between;
          padding: 6px 0;
          font-size: 12px;
          border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        }

        .forecast-card .metric span:first-child {
          color: #a0a0a0;
        }

        .forecast-card .metric .value {
          color: #00e676;
          font-weight: 600;
        }

        .action-btn-sm {
          width: 100%;
          margin-top: 12px;
          padding: 6px 8px;
          background: rgba(0, 212, 255, 0.1);
          border: 1px solid rgba(0, 212, 255, 0.3);
          color: #00d4ff;
          border-radius: 4px;
          font-size: 11px;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .action-btn-sm:hover {
          background: rgba(0, 212, 255, 0.2);
        }

        .forecast-insights {
          background: rgba(0, 230, 118, 0.05);
          border: 1px solid rgba(0, 230, 118, 0.2);
          border-radius: 8px;
          padding: 16px;
        }

        .forecast-insights h3 {
          margin: 0 0 12px 0;
          font-size: 14px;
          color: #00e676;
        }

        .forecast-insights ul {
          margin: 0;
          padding-left: 18px;
          list-style: none;
        }

        .forecast-insights li {
          margin: 6px 0;
          font-size: 12px;
          color: #c0c0c0;
          line-height: 1.5;
        }

        .forecast-insights strong {
          color: #00e676;
        }
      `}</style>
    </div>
  );
};

export default CapacityForecastPanel;
