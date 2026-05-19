import React, { useState } from 'react';
import { AlertTriangle, Bell, BarChart3, Activity, CheckCircle2, Clock } from 'lucide-react';

interface ValidatorAlert {
  id: string;
  validatorName: string;
  validatorAddress: string;
  alertType: 'offline' | 'missed-blocks' | 'low-stake' | 'commission-change' | 'update-needed';
  severity: 'critical' | 'warning' | 'info';
  message: string;
  timestamp: string;
  status: 'active' | 'resolved';
}

export const ValidatorAlertsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'alerts' | 'rules' | 'history'>('alerts');
  const [selectedAlert, setSelectedAlert] = useState<string | null>(null);

  const alerts: ValidatorAlert[] = [
    {
      id: '1',
      validatorName: 'Validator-01 (GPU)',
      validatorAddress: 'x3:val-01',
      alertType: 'offline',
      severity: 'critical',
      message: 'Validator offline for 45 minutes. GPU node may be down.',
      timestamp: '2 mins ago',
      status: 'active',
    },
    {
      id: '2',
      validatorName: 'Validator-03',
      validatorAddress: 'x3:val-03',
      alertType: 'missed-blocks',
      severity: 'warning',
      message: 'Missed 3 blocks in last epoch. Expected: 12, Actual: 9',
      timestamp: '12 mins ago',
      status: 'active',
    },
    {
      id: '3',
      validatorName: 'Validator-05',
      validatorAddress: 'x3:val-05',
      alertType: 'low-stake',
      severity: 'warning',
      message: 'Stake dropped below ideal level (Current: 450K, Target: 500K)',
      timestamp: '1 hour ago',
      status: 'active',
    },
    {
      id: '4',
      validatorName: 'Validator-02',
      validatorAddress: 'x3:val-02',
      alertType: 'commission-change',
      severity: 'info',
      message: 'Commission increased from 5% to 6%. Takes effect next epoch.',
      timestamp: '3 hours ago',
      status: 'resolved',
    },
    {
      id: '5',
      validatorName: 'Validator-04 (You)',
      validatorAddress: 'x3:val-04',
      alertType: 'update-needed',
      severity: 'warning',
      message: 'New client version 1.2.5 available. Recommended update.',
      timestamp: '5 hours ago',
      status: 'active',
    },
  ];

  const alertRules = [
    {
      id: '1',
      name: 'Offline Detection',
      condition: 'No blocks produced in 30 minutes',
      action: 'Email + Push notification',
      enabled: true,
    },
    {
      id: '2',
      name: 'Block Miss Threshold',
      condition: 'Missed > 2 blocks per epoch',
      action: 'Email alert',
      enabled: true,
    },
    {
      id: '3',
      name: 'Stake Threshold',
      condition: 'Effective stake < 400K X3',
      action: 'Email + Slack',
      enabled: true,
    },
    {
      id: '4',
      name: 'Commission Change',
      condition: 'Commission modified',
      action: 'Info notification',
      enabled: true,
    },
    {
      id: '5',
      name: 'Update Available',
      condition: 'New client version released',
      action: 'Push notification',
      enabled: false,
    },
  ];

  const history = [
    {
      date: 'Feb 28, 2026 14:32',
      event: 'Validator-01 came back online',
      type: 'resolved',
    },
    {
      date: 'Feb 28, 2026 12:15',
      event: 'Validator-03 block production resumed',
      type: 'resolved',
    },
    {
      date: 'Feb 27, 2026 09:45',
      event: 'Delegator #2451 unstaked 50K X3',
      type: 'info',
    },
  ];

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical':
        return 'text-red-400 bg-red-500/20 border-red-500/30';
      case 'warning':
        return 'text-yellow-400 bg-yellow-500/20 border-yellow-500/30';
      default:
        return 'text-blue-400 bg-blue-500/20 border-blue-500/30';
    }
  };

  const getAlertIcon = (type: string) => {
    switch (type) {
      case 'offline':
        return '⚠️';
      case 'missed-blocks':
        return '📉';
      case 'low-stake':
        return '📊';
      case 'commission-change':
        return '💰';
      case 'update-needed':
        return '🔄';
      default:
        return '📌';
    }
  };

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-red-500/20 to-orange-500/20">
        <div className="flex items-center gap-3 mb-2">
          <Bell className="w-5 h-5 text-red-400" />
          <h1 className="text-lg font-bold text-white">Validator Alerts</h1>
        </div>
        <p className="text-sm text-gray-400">Real-time monitoring with configurable rules for 3 validators</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['alerts', 'rules', 'history'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-red-400 border-b-2 border-red-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'alerts' && `Alerts (${alerts.filter((a) => a.status === 'active').length})`}
            {tab === 'rules' && 'Rules'}
            {tab === 'history' && 'History'}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'alerts' && (
          <div className="p-6 space-y-3">
            {alerts.map((alert) => (
              <div
                key={alert.id}
                onClick={() => setSelectedAlert(alert.id)}
                className={`p-4 border rounded-lg cursor-pointer transition ${getSeverityColor(
                  alert.severity
                )} ${selectedAlert === alert.id ? 'ring-2 ring-offset-1' : ''}`}
              >
                <div className="flex justify-between items-start mb-2">
                  <div className="flex items-start gap-3 flex-1">
                    <span className="text-xl">{getAlertIcon(alert.alertType)}</span>
                    <div>
                      <h3 className="text-sm font-semibold text-white">{alert.validatorName}</h3>
                      <p className="text-xs text-gray-500 font-mono mt-1">{alert.validatorAddress}</p>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    {alert.status === 'resolved' && <CheckCircle2 className="w-4 h-4 text-emerald-400" />}
                    <span className={`px-2 py-1 text-xs rounded font-semibold ${
                      alert.severity === 'critical' ? 'bg-red-600' :
                      alert.severity === 'warning' ? 'bg-yellow-600' : 'bg-blue-600'
                    }`}>
                      {alert.severity.toUpperCase()}
                    </span>
                  </div>
                </div>
                <p className="text-sm text-gray-300 mb-2">{alert.message}</p>
                <div className="flex gap-3 text-xs">
                  <span className="text-gray-500">{alert.timestamp}</span>
                  <span className="text-gray-600">•</span>
                  <span className="text-gray-600">{alert.alertType.replace('-', ' ')}</span>
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'rules' && (
          <div className="p-6 space-y-3">
            {alertRules.map((rule) => (
              <div key={rule.id} className="p-4 border border-[#2a2a35] rounded-lg hover:border-red-500/30 transition">
                <div className="flex justify-between items-start mb-2">
                  <div>
                    <h3 className="text-sm font-semibold text-white">{rule.name}</h3>
                    <p className="text-xs text-gray-400 mt-1">{rule.condition}</p>
                  </div>
                  <div className={`w-3 h-3 rounded-full ${rule.enabled ? 'bg-emerald-500' : 'bg-gray-600'}`} />
                </div>
                <div className="text-xs text-gray-500 flex items-center gap-2">
                  <Activity className="w-3 h-3" />
                  Action: {rule.action}
                </div>
              </div>
            ))}
          </div>
        )}

        {activeTab === 'history' && (
          <div className="p-6 space-y-4">
            <div className="relative space-y-4">
              {history.map((item, idx) => (
                <div key={idx} className="flex gap-4 relative">
                  <div className="flex flex-col items-center">
                    <div className={`w-3 h-3 rounded-full ${item.type === 'resolved' ? 'bg-emerald-400' : 'bg-blue-400'}`} />
                    {idx < history.length - 1 && (
                      <div className="w-0.5 h-12 bg-gradient-to-b from-[#2a2a35] to-transparent" />
                    )}
                  </div>
                  <div className="flex-1 pb-4">
                    <p className="text-xs font-semibold text-gray-400">{item.date}</p>
                    <p className="text-sm text-gray-300 mt-1">{item.event}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default ValidatorAlertsPanel;
