import React, { useState } from 'react';
import { TrendingUp, Calendar, AlertCircle, CheckCircle2, LineChart, BarChart3 } from 'lucide-react';

interface ReportMetric {
  label: string;
  value: string;
  change: number;
  trend: 'up' | 'down';
}

export const AnalyticsReportingPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'overview' | 'detailed' | 'export'>('overview');
  const [dateRange, setDateRange] = useState<'day' | 'week' | 'month' | 'year'>('month');

  const metrics: ReportMetric[] = [
    { label: 'Total Transactions', value: '45,230', change: 12.5, trend: 'up' },
    { label: 'Average Block Time', value: '12.3s', change: -2.1, trend: 'down' },
    { label: 'Network Throughput', value: '8,450 TPS', change: 8.3, trend: 'up' },
    { label: 'Validator Uptime', value: '99.87%', change: 0.05, trend: 'up' },
  ];

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Analytics & Reporting
            </h1>
            <p className="text-gray-400">Comprehensive metrics, trends, and performance reports</p>
          </div>
          <LineChart className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Date Range Selector */}
        <div className="flex gap-2 mb-6">
          {(['day', 'week', 'month', 'year'] as const).map((range) => (
            <button
              key={range}
              onClick={() => setDateRange(range)}
              className={`px-4 py-2 rounded-lg font-semibold transition ${
                dateRange === range
                  ? 'bg-cyan-600 text-white'
                  : 'bg-[#1a1a2e] border border-[#2a2a35] text-gray-400 hover:border-cyan-400'
              }`}
            >
              {range.charAt(0).toUpperCase() + range.slice(1)}
            </button>
          ))}
        </div>

        {/* Key Metrics */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          {metrics.map((metric, idx) => (
            <div key={idx} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
              <div className="text-gray-400 text-xs font-semibold mb-2">{metric.label}</div>
              <div className="flex items-end justify-between">
                <div>
                  <div className="text-2xl font-bold text-cyan-400 mb-1">{metric.value}</div>
                  <div
                    className={`text-xs font-semibold flex items-center gap-1 ${
                      metric.trend === 'up' ? 'text-green-400' : 'text-red-400'
                    }`}
                  >
                    {metric.trend === 'up' ? '↑' : '↓'} {Math.abs(metric.change)}%
                  </div>
                </div>
                {metric.trend === 'up' ? (
                  <TrendingUp className="w-5 h-5 text-green-400 opacity-30" />
                ) : (
                  <TrendingUp className="w-5 h-5 text-red-400 opacity-30 rotate-180" />
                )}
              </div>
            </div>
          ))}
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['overview', 'detailed', 'export'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'overview' && 'Overview'}
              {tab === 'detailed' && 'Detailed Analysis'}
              {tab === 'export' && 'Export Report'}
            </button>
          ))}
        </div>

        {/* Overview Tab */}
        {activeTab === 'overview' && (
          <div className="grid grid-cols-2 gap-6">
            {/* Transaction Chart */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <h2 className="text-white font-bold mb-4 flex items-center gap-2">
                <BarChart3 className="w-5 h-5" /> Transaction Volume
              </h2>
              <div className="h-64 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                <svg className="w-full h-full" viewBox="0 0 400 200">
                  <polyline
                    points="10,150 40,120 70,100 100,90 130,95 160,80 190,70 220,85 250,60 280,75 310,50 340,65 370,40"
                    fill="none"
                    stroke="#06b6d4"
                    strokeWidth="2"
                  />
                  <line x1="0" y1="160" x2="400" y2="160" stroke="#2a2a35" strokeWidth="1" />
                </svg>
              </div>
            </div>

            {/* Network Health */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <h2 className="text-white font-bold mb-4 flex items-center gap-2">
                <CheckCircle2 className="w-5 h-5" /> Network Health
              </h2>
              <div className="space-y-4">
                {[
                  { label: 'Active Validators', value: 1234, status: 'healthy' },
                  { label: 'Byzantine Fault Tolerance', value: 99.99, status: 'optimal' },
                  { label: 'Average Latency', value: 45, status: 'good' },
                  { label: 'Missing Blocks', value: 2, status: 'warning' },
                ].map((item, idx) => (
                  <div key={idx} className="flex items-center justify-between">
                    <div>
                      <p className="text-gray-400 text-sm">{item.label}</p>
                      <p className="text-white font-bold">
                        {item.value}
                        {item.label === 'Average Latency' ? 'ms' : item.label === 'Byzantine Fault Tolerance' ? '%' : ''}
                      </p>
                    </div>
                    <div
                      className={`w-3 h-3 rounded-full ${
                        item.status === 'healthy'
                          ? 'bg-green-500'
                          : item.status === 'optimal'
                            ? 'bg-cyan-500'
                            : item.status === 'good'
                              ? 'bg-blue-500'
                              : 'bg-yellow-500'
                      }`}
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Detailed Analysis Tab */}
        {activeTab === 'detailed' && (
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <h2 className="text-white font-bold mb-6">Detailed Performance Analysis</h2>
            <div className="space-y-6">
              {[
                {
                  title: 'Block Production Analysis',
                  stats: [
                    { label: 'Total Blocks Produced', value: '45,230' },
                    { label: 'Average Block Size', value: '128 KB' },
                    { label: 'Max Block Size', value: '256 KB' },
                    { label: 'Block Production Rate', value: '99.8%' },
                  ],
                },
                {
                  title: 'Transaction Analysis',
                  stats: [
                    { label: 'Total Transactions', value: '2,456,789' },
                    { label: 'Failed Transactions', value: '12 (0.005%)' },
                    { label: 'Average Gas Used', value: '45,230 units' },
                    { label: 'Peak TPS', value: '9,250 TPS' },
                  ],
                },
              ].map((section, idx) => (
                <div key={idx} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <h3 className="text-cyan-400 font-semibold mb-4">{section.title}</h3>
                  <div className="grid grid-cols-2 gap-4">
                    {section.stats.map((stat, sidx) => (
                      <div key={sidx} className="border-b border-[#2a2a35] pb-3">
                        <p className="text-gray-500 text-xs mb-1">{stat.label}</p>
                        <p className="text-white font-bold">{stat.value}</p>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Export Report Tab */}
        {activeTab === 'export' && (
          <div className="space-y-4 max-w-2xl">
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <h2 className="text-white font-bold mb-6">Export Reports</h2>
              <div className="space-y-3">
                {[
                  {
                    title: 'Monthly Performance Report',
                    description: 'Complete performance metrics for the current month',
                    format: 'PDF',
                    size: '2.4 MB',
                  },
                  {
                    title: 'Transaction History Export',
                    description: 'All transactions in CSV format for analysis',
                    format: 'CSV',
                    size: '8.7 MB',
                  },
                  {
                    title: 'Block Data Export',
                    description: 'Detailed block-by-block information',
                    format: 'JSON',
                    size: '15.2 MB',
                  },
                  {
                    title: 'Validator Statistics',
                    description: 'Comprehensive validator performance data',
                    format: 'Excel',
                    size: '3.1 MB',
                  },
                ].map((report, idx) => (
                  <div key={idx} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4 flex items-center justify-between">
                    <div className="flex-1">
                      <h3 className="text-white font-semibold mb-1">{report.title}</h3>
                      <p className="text-gray-400 text-sm">{report.description}</p>
                      <div className="flex gap-4 mt-2 text-xs text-gray-500">
                        <span>Format: {report.format}</span>
                        <span>Size: {report.size}</span>
                      </div>
                    </div>
                    <button className="px-4 py-2 bg-cyan-600 hover:bg-cyan-700 text-white rounded-lg font-semibold transition whitespace-nowrap">
                      Download
                    </button>
                  </div>
                ))}
              </div>
            </div>

            {/* Email Report Option */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <h2 className="text-white font-bold mb-4">Email Report</h2>
              <p className="text-gray-400 text-sm mb-4">
                Receive reports automatically every month to your email
              </p>
              <div className="flex gap-2">
                <input
                  type="email"
                  placeholder="your@email.com"
                  className="flex-1 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-cyan-400"
                />
                <button className="px-4 py-2 bg-cyan-600 hover:bg-cyan-700 text-white rounded-lg font-semibold transition">
                  Subscribe
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default AnalyticsReportingPanel;
