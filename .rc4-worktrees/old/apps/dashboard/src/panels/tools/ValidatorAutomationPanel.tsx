import React, { useState } from 'react';
import { Zap, Server, CheckCircle, AlertTriangle, TrendingUp, Settings } from 'lucide-react';

interface Validator {
  id: string;
  name: string;
  address: string;
  stake: number;
  commission: number;
  uptime: number;
  producerReputation: number;
  slashingRisk: number;
  status: 'active' | 'inactive' | 'jailed';
  autoCompound: boolean;
}

interface ValidatorMetrics {
  totalValidators: number;
  totalStaked: number;
  avgUptime: number;
  avgCommission: number;
  slashingFund: number;
  networkHealth: number;
}

interface SetupWizard {
  step: number;
  name: string;
  description: string;
  completed: boolean;
  estimatedTime: string;
  requirements: string[];
}

interface SlashingAlert {
  id: string;
  validatorId: string;
  validatorName: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  reason: string;
  timestamp: number;
  status: 'active' | 'resolved';
}

export const ValidatorAutomationPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'validators' | 'setup' | 'alerts' | 'metrics'>('validators');

  const [validators] = useState<Validator[]>([
    {
      id: 'val1',
      name: 'X3 Core Validator 1',
      address: 'x3val1...a2f8',
      stake: 1250000,
      commission: 8.5,
      uptime: 99.8,
      producerReputation: 98.2,
      slashingRisk: 0.1,
      status: 'active',
      autoCompound: true,
    },
    {
      id: 'val2',
      name: 'Constellation Validator',
      address: 'x3val2...b3g9',
      stake: 850000,
      commission: 6.2,
      uptime: 99.5,
      producerReputation: 95.7,
      slashingRisk: 0.3,
      status: 'active',
      autoCompound: true,
    },
    {
      id: 'val3',
      name: 'Secure Nodes Collective',
      address: 'x3val3...c4h0',
      stake: 520000,
      commission: 5.0,
      uptime: 98.9,
      producerReputation: 92.1,
      slashingRisk: 0.8,
      status: 'active',
      autoCompound: false,
    },
  ]);

  const [setupSteps] = useState<SetupWizard[]>([
    {
      step: 1,
      name: 'Environment Setup',
      description: 'Configure system and dependencies',
      completed: true,
      estimatedTime: '5 min',
      requirements: ['Linux/Mac', '+5GB storage', '4+ cores', '8GB RAM'],
    },
    {
      step: 2,
      name: 'Key Generation',
      description: 'Create validator signing keys securely',
      completed: true,
      estimatedTime: '3 min',
      requirements: ['HSM optional', 'Secure backup', 'Key rotation ready'],
    },
    {
      step: 3,
      name: 'Node Setup',
      description: 'Deploy and sync full validator node',
      completed: true,
      estimatedTime: '45 min',
      requirements: ['Network connection', 'Disk sync', 'RPC access'],
    },
    {
      step: 4,
      name: 'Validator Registration',
      description: 'Register with network and stake tokens',
      completed: true,
      estimatedTime: '10 min',
      requirements: ['Initial stake', 'Registration fee', 'Active account'],
    },
    {
      step: 5,
      name: 'Monitoring Setup',
      description: 'Configure alerts and dashboards',
      completed: true,
      estimatedTime: '15 min',
      requirements: ['Prometheus', 'Grafana', 'Email alerts'],
    },
  ]);

  const [slashingAlerts] = useState<SlashingAlert[]>([
    {
      id: 'alert1',
      validatorId: 'val3',
      validatorName: 'Secure Nodes Collective',
      severity: 'medium',
      reason: 'Downtime detected: 8.2 hours',
      timestamp: Date.now() - 86400000,
      status: 'resolved',
    },
    {
      id: 'alert2',
      validatorId: 'val2',
      validatorName: 'Constellation Validator',
      severity: 'low',
      reason: 'Commission rate adjustment warning',
      timestamp: Date.now() - 86400000 * 3,
      status: 'active',
    },
  ]);

  const [metrics] = useState<ValidatorMetrics>({
    totalValidators: 342,
    totalStaked: 125000000,
    avgUptime: 99.1,
    avgCommission: 6.8,
    slashingFund: 2500000,
    networkHealth: 96.4,
  });

  const myTotalStake = validators.reduce((sum, v) => sum + v.stake, 0);
  const myAvgUptime = validators.reduce((sum, v) => sum + v.uptime, 0) / validators.length;
  const myAvgCommission = validators.reduce((sum, v) => sum + v.commission, 0) / validators.length;

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-yellow-400 to-orange-500 mb-2">
              Validator Automation
            </h1>
            <p className="text-gray-400">One-Click Setup • Real Metrics • Slashing Alerts • Auto Compound</p>
          </div>
          <Zap className="w-12 h-12 text-yellow-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">My Total Stake</div>
            <div className="text-2xl font-bold text-yellow-400">{(myTotalStake / 1000000).toFixed(2)}M</div>
            <div className="text-xs text-gray-500 mt-2">Across 3 validators</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Avg Uptime</div>
            <div className="text-2xl font-bold text-green-400">{myAvgUptime.toFixed(1)}%</div>
            <div className="text-xs text-gray-500 mt-2">Network average: {metrics.avgUptime}%</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Avg Commission</div>
            <div className="text-2xl font-bold text-blue-400">{myAvgCommission.toFixed(1)}%</div>
            <div className="text-xs text-gray-500 mt-2">Network average: {metrics.avgCommission}%</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Network Health</div>
            <div className="text-2xl font-bold text-purple-400">{metrics.networkHealth}%</div>
            <div className="text-xs text-gray-500 mt-2">All validators: {metrics.totalValidators}</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['validators', 'setup', 'alerts', 'metrics'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-yellow-400 border-b-2 border-yellow-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'validators' && 'My Validators'}
              {tab === 'setup' && 'Setup Wizard'}
              {tab === 'alerts' && 'Slashing Alerts'}
              {tab === 'metrics' && 'Network Metrics'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'validators' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Active Validators</h3>
              <div className="space-y-4">
                {validators.map((val) => (
                  <div key={val.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <h4 className="text-white font-semibold">{val.name}</h4>
                        <p className="text-xs text-gray-500 font-mono">{val.address}</p>
                      </div>
                      <div
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          val.status === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : 'bg-red-500/20 text-red-400'
                        }`}
                      >
                        {val.status.toUpperCase()}
                      </div>
                    </div>
                    <div className="grid grid-cols-6 gap-4 text-sm mb-3">
                      <div>
                        <div className="text-gray-400">Stake</div>
                        <div className="text-white font-semibold">${(val.stake / 1000000).toFixed(2)}M</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Commission</div>
                        <div className="text-white font-semibold">{val.commission}%</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Uptime</div>
                        <div className={val.uptime > 99.5 ? 'text-green-400 font-semibold' : 'text-yellow-400 font-semibold'}>
                          {val.uptime}%
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Reputation</div>
                        <div className="text-blue-400 font-semibold">{val.producerReputation}%</div>
                      </div>
                      <div>
                        <div className="text-gray-400">Slash Risk</div>
                        <div className={val.slashingRisk < 0.5 ? 'text-green-400 font-semibold' : 'text-orange-400 font-semibold'}>
                          {val.slashingRisk}%
                        </div>
                      </div>
                      <div>
                        <div className="text-gray-400">Auto Compound</div>
                        <div className="flex items-center gap-1">
                          {val.autoCompound ? (
                            <>
                              <CheckCircle className="w-4 h-4 text-green-400" /> On
                            </>
                          ) : (
                            <>
                              <AlertTriangle className="w-4 h-4 text-yellow-400" /> Off
                            </>
                          )}
                        </div>
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <button className="flex-1 bg-blue-500/20 text-blue-400 px-3 py-2 rounded text-xs font-semibold hover:bg-blue-500/30">
                        Manage Stake
                      </button>
                      <button className="flex-1 bg-purple-500/20 text-purple-400 px-3 py-2 rounded text-xs font-semibold hover:bg-purple-500/30">
                        Configure
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'setup' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">One-Click Setup Wizard</h3>
              <div className="space-y-4">
                {setupSteps.map((step) => (
                  <div key={step.step} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex items-start gap-4">
                        <div className={`flex-shrink-0 w-10 h-10 rounded-full flex items-center justify-center font-bold ${
                          step.completed ? 'bg-green-500/20 text-green-400' : 'bg-gray-500/20 text-gray-400'
                        }`}>
                          {step.completed ? '✓' : step.step}
                        </div>
                        <div>
                          <h4 className="text-white font-semibold">{step.name}</h4>
                          <p className="text-sm text-gray-400">{step.description}</p>
                        </div>
                      </div>
                      <div className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        step.completed ? 'bg-green-500/20 text-green-400' : 'bg-blue-500/20 text-blue-400'
                      }`}>
                        {step.completed ? 'COMPLETE' : `~${step.estimatedTime}`}
                      </div>
                    </div>
                    <div className="ml-14">
                      <div className="flex flex-wrap gap-2">
                        {step.requirements.map((req) => (
                          <span key={req} className="bg-gray-500/20 text-gray-400 px-2 py-1 rounded text-xs">
                            {req}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
              <button className="mt-6 w-full bg-gradient-to-r from-yellow-500 to-orange-500 text-white px-6 py-3 rounded-lg font-semibold hover:from-yellow-600 hover:to-orange-600">
                Start New Validator Setup
              </button>
            </div>
          )}

          {activeTab === 'alerts' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">Slashing Alerts & Warnings</h3>
              {slashingAlerts.length === 0 ? (
                <div className="text-center py-8">
                  <CheckCircle className="w-12 h-12 text-green-400 mx-auto mb-3" />
                  <p className="text-gray-400">No active slashing alerts</p>
                </div>
              ) : (
                <div className="space-y-4">
                  {slashingAlerts.map((alert) => (
                    <div
                      key={alert.id}
                      className={`border rounded-lg p-4 ${
                        alert.severity === 'critical'
                          ? 'bg-red-500/10 border-red-500/30'
                          : alert.severity === 'high'
                            ? 'bg-orange-500/10 border-orange-500/30'
                            : 'bg-yellow-500/10 border-yellow-500/30'
                      }`}
                    >
                      <div className="flex items-start justify-between">
                        <div>
                          <h4 className="text-white font-semibold">{alert.validatorName}</h4>
                          <p className="text-sm text-gray-300 mt-1">{alert.reason}</p>
                        </div>
                        <div
                          className={`px-3 py-1 rounded-full text-xs font-semibold ${
                            alert.severity === 'critical'
                              ? 'bg-red-500/20 text-red-400'
                              : alert.severity === 'high'
                                ? 'bg-orange-500/20 text-orange-400'
                                : 'bg-yellow-500/20 text-yellow-400'
                          }`}
                        >
                          {alert.severity.toUpperCase()}
                        </div>
                      </div>
                      <p className="text-xs text-gray-500 mt-3">{Math.round((Date.now() - alert.timestamp) / 86400000)}d ago</p>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {activeTab === 'metrics' && (
            <div className="grid grid-cols-3 gap-6">
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Network Stats</h4>
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Total Validators</span>
                    <span className="text-white font-semibold">{metrics.totalValidators.toLocaleString()}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Total Staked</span>
                    <span className="text-green-400 font-semibold">${(metrics.totalStaked / 1000000).toFixed(0)}M</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Network Health</span>
                    <span className="text-blue-400 font-semibold">{metrics.networkHealth}%</span>
                  </div>
                  <div className="flex justify-between border-t border-[#2a2a35] pt-3 mt-3">
                    <span className="text-gray-300">Avg Uptime</span>
                    <span className="text-green-400 font-semibold">{metrics.avgUptime}%</span>
                  </div>
                </div>
              </div>
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Commission Tracking</h4>
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Network Avg</span>
                    <span className="text-white font-semibold">{metrics.avgCommission}%</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">My Average</span>
                    <span className="text-cyan-400 font-semibold">{myAvgCommission.toFixed(1)}%</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Lowest</span>
                    <span className="text-green-400 font-semibold">2.5%</span>
                  </div>
                  <div className="flex justify-between border-t border-[#2a2a35] pt-3 mt-3">
                    <span className="text-gray-300">Highest</span>
                    <span className="text-orange-400 font-semibold">12.0%</span>
                  </div>
                </div>
              </div>
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <h4 className="text-white font-semibold mb-4">Slashing Fund</h4>
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Total Fund</span>
                    <span className="text-white font-semibold">${(metrics.slashingFund / 1000000).toFixed(1)}M</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Claims (YTD)</span>
                    <span className="text-yellow-400 font-semibold">$245K</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Coverage</span>
                    <span className="text-green-400 font-semibold">98.0%</span>
                  </div>
                  <div className="flex justify-between border-t border-[#2a2a35] pt-3 mt-3">
                    <span className="text-gray-300">Status</span>
                    <span className="text-green-400 font-semibold">Healthy</span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default ValidatorAutomationPanel;
