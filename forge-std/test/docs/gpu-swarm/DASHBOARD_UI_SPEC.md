# Dashboard UI Implementation Guide

## Overview

The GPU Swarm Dashboard provides real-time monitoring of the distributed swarm network, GPU resources, task execution, and economic metrics through a web-based interface.

**Architecture**: 
- Frontend: React/TypeScript with Vite
- State Management: TanStack Query + Zustand
- Real-time Updates: WebSocket connections to `swarm/api_server.py`
- Styling: Tailwind CSS + Headless UI
- Charts: Recharts for time-series and distribution visualizations

## Project Structure

```
apps/swarm-dashboard/
├── src/
│   ├── components/
│   │   ├── Layout/
│   │   │   ├── Header.tsx          # Navigation and settings
│   │   │   ├── Sidebar.tsx         # Menu and quick stats
│   │   │   └── Footer.tsx          # Footer with status
│   │   ├── Dashboard/
│   │   │   ├── Overview.tsx        # Main dashboard view
│   │   │   ├── MetricsPanel.tsx    # Metrics display
│   │   │   └── HealthStatus.tsx    # Node health status
│   │   ├── GPU/
│   │   │   ├── GpuUtilization.tsx  # Real-time GPU metrics
│   │   │   ├── BackendComparison.tsx
│   │   │   └── DeviceList.tsx      # Available GPU devices
│   │   ├── Tasks/
│   │   │   ├── TaskQueue.tsx       # Active task queue
│   │   │   ├── TaskDetail.tsx      # Task execution details
│   │   │   └── TaskHistory.tsx     # Completed tasks
│   │   ├── Network/
│   │   │   ├── PeerGraph.tsx       # Network topology visualization
│   │   │   ├── PeerList.tsx        # Connected peers
│   │   │   └── Statistics.tsx      # Network metrics
│   │   ├── Economics/
│   │   │   ├── RewardChart.tsx     # Reward distribution
│   │   │   ├── StakingPanel.tsx    # Staking interface
│   │   │   └── SlashingLog.tsx     # Slashing events
│   │   └── Common/
│   │       ├── Card.tsx
│   │       ├── Chart.tsx
│   │       └── LoadingSpinner.tsx
│   ├── hooks/
│   │   ├── useWebSocket.ts         # WebSocket connection
│   │   ├── useMetrics.ts           # Metrics fetching
│   │   ├── useGpuStatus.ts         # GPU monitoring
│   │   ├── useTasks.ts             # Task tracking
│   │   ├── usePeers.ts             # Peer monitoring
│   │   └── useRewards.ts           # Reward tracking
│   ├── services/
│   │   ├── api.ts                  # HTTP API calls
│   │   ├── websocket.ts            # WebSocket client
│   │   └── storage.ts              # Local storage utilities
│   ├── store/
│   │   ├── metricsStore.ts         # Metrics state
│   │   ├── gpuStore.ts             # GPU state
│   │   ├── taskStore.ts            # Task state
│   │   ├── networkStore.ts         # Network state
│   │   └── userStore.ts            # User/settings state
│   ├── pages/
│   │   ├── Home.tsx
│   │   ├── Dashboard.tsx
│   │   ├── GpuMonitoring.tsx
│   │   ├── TaskManagement.tsx
│   │   ├── NetworkTopology.tsx
│   │   ├── Economics.tsx
│   │   ├── Governance.tsx
│   │   └── Settings.tsx
│   ├── types/
│   │   ├── api.ts                  # API response types
│   │   ├── metrics.ts              # Metrics types
│   │   └── common.ts               # Common types
│   ├── utils/
│   │   ├── formatters.ts           # Number/date formatting
│   │   ├── calculations.ts         # Metric calculations
│   │   └── validators.ts           # Form validation
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css
├── public/
│   ├── favicon.ico
│   └── index.html
├── vite.config.ts
├── tsconfig.json
├── package.json
└── docs/root/README.md
```

## Core Components

### 1. Main Dashboard View (Overview.tsx)

Displays high-level swarm status with key metrics:

```typescript
// src/pages/Dashboard.tsx
import React, { useEffect } from 'react';
import { useMetrics, useGpuStatus, usePeers } from '../hooks';
import MetricsPanel from '../components/Dashboard/MetricsPanel';
import HealthStatus from '../components/Dashboard/HealthStatus';
import Header from '../components/Layout/Header';
import Sidebar from '../components/Layout/Sidebar';

export const Dashboard: React.FC = () => {
  const { metrics, isLoading } = useMetrics();
  const { gpuDevices } = useGpuStatus();
  const { peerCount } = usePeers();

  return (
    <div className="flex h-screen bg-gray-900">
      <Sidebar />
      <div className="flex-1 flex flex-col">
        <Header title="Swarm Dashboard" />
        
        <main className="flex-1 overflow-auto p-8">
          {/* KPI Cards */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8">
            <MetricCard 
              title="Active Tasks"
              value={metrics?.tasksCompleted || 0}
              unit="tasks"
              trend={5.2}
            />
            <MetricCard 
              title="GPU Utilization"
              value={metrics?.gpuUtilization || 0}
              unit="%"
              trend={-2.1}
            />
            <MetricCard 
              title="Connected Peers"
              value={peerCount}
              unit="peers"
              trend={12.5}
            />
            <MetricCard 
              title="Floor Rewards"
              value={metrics?.rewardsDistributed || 0}
              unit="X3"
              trend={8.3}
            />
          </div>

          {/* Charts Grid */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
            <MetricsPanel metrics={metrics} />
            <HealthStatus gpuDevices={gpuDevices} />
          </div>

          {/* Detailed Sections */}
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <GpuUtilizationChart />
            <PeerNetworkStats />
            <RecentTasksTable />
          </div>
        </main>
      </div>
    </div>
  );
};

interface MetricCardProps {
  title: string;
  value: number;
  unit: string;
  trend: number;
}

const MetricCard: React.FC<MetricCardProps> = ({ title, value, unit, trend }) => (
  <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
    <h3 className="text-gray-400 text-sm font-medium">{title}</h3>
    <div className="mt-2 flex items-baseline justify-between">
      <span className="text-3xl font-bold text-white">
        {value.toLocaleString()}
      </span>
      <span className="text-gray-400 text-sm ml-2">{unit}</span>
    </div>
    <div className={`mt-2 text-sm font-semibold ${trend >= 0 ? 'text-green-500' : 'text-red-500'}`}>
      {trend >= 0 ? '↑' : '↓'} {Math.abs(trend).toFixed(1)}%
    </div>
  </div>
);
```

### 2. GPU Utilization Chart (GpuUtilization.tsx)

Real-time GPU metrics with time-series visualization:

```typescript
// src/components/GPU/GpuUtilization.tsx
import React, { useEffect } from 'react';
import {
  LineChart, Line, AreaChart, Area, BarChart, Bar,
  XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer
} from 'recharts';
import { useWebSocket } from '../../hooks/useWebSocket';

interface GpuMetricData {
  timestamp: string;
  device: string;
  utilization: number;    // 0-100%
  memory: number;         // MB
  temperature: number;    // Celsius
  power: number;          // Watts
  throughput: number;     // GFLOPS
}

export const GpuUtilization: React.FC = () => {
  const [data, setData] = React.useState<GpuMetricData[]>([]);
  const [selectedDevice, setSelectedDevice] = React.useState<string>('all');
  
  const { message } = useWebSocket('ws://localhost:9000/ws/metrics');

  useEffect(() => {
    if (message?.type === 'gpu_metrics') {
      setData(prev => {
        const updated = [...prev, message.data];
        // Keep only last 60 points (5 minutes at 5s intervals)
        return updated.slice(-60);
      });
    }
  }, [message]);

  const filteredData = selectedDevice === 'all'
    ? data
    : data.filter(d => d.device === selectedDevice);

  return (
    <div className="bg-gray-800 rounded-lg p-6 border border-gray-700 col-span-2">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold text-white">GPU Performance</h2>
        <select
          value={selectedDevice}
          onChange={(e) => setSelectedDevice(e.target.value)}
          className="bg-gray-700 text-white px-4 py-2 rounded border border-gray-600"
        >
          <option value="all">All Devices</option>
          <option value="cuda:0">CUDA:0 - RTX 4090</option>
          <option value="cuda:1">CUDA:1 - RTX 4080</option>
          <option value="vulkan:0">Vulkan:0 - RX 7900 XTX</option>
        </select>
      </div>

      {/* Utilization Trend */}
      <div className="mb-8">
        <h3 className="text-sm font-semibold text-gray-300 mb-3">Utilization (%)</h3>
        <ResponsiveContainer width="100%" height={250}>
          <AreaChart data={filteredData}>
            <defs>
              <linearGradient id="colorUtil" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#10b981" stopOpacity={0.8}/>
                <stop offset="95%" stopColor="#10b981" stopOpacity={0.1}/>
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
            <XAxis dataKey="timestamp" stroke="#9ca3af" />
            <YAxis stroke="#9ca3af" domain={[0, 100]} />
            <Tooltip
              contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #4b5563' }}
              labelStyle={{ color: '#fff' }}
            />
            <Area
              type="monotone"
              dataKey="utilization"
              stroke="#10b981"
              fillOpacity={1}
              fill="url(#colorUtil)"
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      {/* Memory Usage */}
      <div className="mb-8">
        <h3 className="text-sm font-semibold text-gray-300 mb-3">Memory Usage (MB)</h3>
        <ResponsiveContainer width="100%" height={200}>
          <LineChart data={filteredData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
            <XAxis dataKey="timestamp" stroke="#9ca3af" />
            <YAxis stroke="#9ca3af" />
            <Tooltip
              contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #4b5563' }}
              labelStyle={{ color: '#fff' }}
            />
            <Line
              type="monotone"
              dataKey="memory"
              stroke="#3b82f6"
              strokeWidth={2}
              dot={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Temperature & Power */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <h3 className="text-sm font-semibold text-gray-300 mb-3">Temperature (°C)</h3>
          <ResponsiveContainer width="100%" height={150}>
            <BarChart data={filteredData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="timestamp" stroke="#9ca3af" />
              <YAxis stroke="#9ca3af" />
              <Bar dataKey="temperature" fill="#f59e0b" />
            </BarChart>
          </ResponsiveContainer>
        </div>
        <div>
          <h3 className="text-sm font-semibold text-gray-300 mb-3">Power (Watts)</h3>
          <ResponsiveContainer width="100%" height={150}>
            <BarChart data={filteredData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
              <XAxis dataKey="timestamp" stroke="#9ca3af" />
              <YAxis stroke="#9ca3af" />
              <Bar dataKey="power" fill="#ef4444" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>
    </div>
  );
};
```

### 3. Task Queue Monitoring (TaskQueue.tsx)

Monitor active and pending tasks:

```typescript
// src/components/Tasks/TaskQueue.tsx
import React, { useEffect } from 'react';
import { useTasks } from '../../hooks/useTasks';
import { formatDuration, formatBytes } from '../../utils/formatters';

interface Task {
  id: string;
  type: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  submittedAt: number;
  startedAt?: number;
  completedAt?: number;
  gpuBackend: string;
  estimatedGpuMemory: number;
  reward: number;
  executedBy?: string;
}

export const TaskQueue: React.FC = () => {
  const { tasks, isLoading } = useTasks();

  const stats = {
    total: tasks.length,
    queued: tasks.filter(t => t.status === 'pending').length,
    running: tasks.filter(t => t.status === 'running').length,
    completed: tasks.filter(t => t.status === 'completed').length,
    failed: tasks.filter(t => t.status === 'failed').length,
  };

  return (
    <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold text-white">Task Queue</h2>
        <div className="flex gap-4 text-sm">
          <div><span className="text-gray-400">Queue:</span> <span className="text-white font-bold">{stats.queued}</span></div>
          <div><span className="text-gray-400">Running:</span> <span className="text-green-500 font-bold">{stats.running}</span></div>
          <div><span className="text-gray-400">Failed:</span> <span className="text-red-500 font-bold">{stats.failed}</span></div>
        </div>
      </div>

      {/* Queue Depth Chart */}
      <div className="mb-6 h-32 bg-gray-700 rounded flex items-center justify-center">
        <QueueDepthChart tasks={tasks} />
      </div>

      {/* Task Table */}
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-700">
              <th className="text-left py-3 px-4 text-gray-400 font-semibold">Task ID</th>
              <th className="text-left py-3 px-4 text-gray-400 font-semibold">Type</th>
              <th className="text-left py-3 px-4 text-gray-400 font-semibold">Status</th>
              <th className="text-left py-3 px-4 text-gray-400 font-semibold">GPU Backend</th>
              <th className="text-left py-3 px-4 text-gray-400 font-semibold">Memory</th>
              <th className="text-left py-3 px-4 text-gray-400 font-semibold">Reward</th>
              <th className="text-left py-3 px-4 text-gray-400 font-semibold">Progress</th>
            </tr>
          </thead>
          <tbody>
            {tasks.slice(0, 20).map(task => (
              <tr key={task.id} className="border-b border-gray-700 hover:bg-gray-700 transition">
                <td className="py-3 px-4 text-blue-400 font-mono text-xs">{task.id.slice(0, 8)}...</td>
                <td className="py-3 px-4 text-white">{task.type}</td>
                <td className="py-3 px-4">
                  <span className={`px-2 py-1 rounded text-xs font-semibold ${
                    task.status === 'running' ? 'bg-green-900 text-green-300' :
                    task.status === 'completed' ? 'bg-blue-900 text-blue-300' :
                    task.status === 'failed' ? 'bg-red-900 text-red-300' :
                    'bg-gray-700 text-gray-300'
                  }`}>
                    {task.status}
                  </span>
                </td>
                <td className="py-3 px-4 text-white">{task.gpuBackend}</td>
                <td className="py-3 px-4 text-gray-400">{formatBytes(task.estimatedGpuMemory)}</td>
                <td className="py-3 px-4 text-green-400 font-semibold">{task.reward} X3</td>
                <td className="py-3 px-4">
                  <div className="w-24 h-2 bg-gray-700 rounded-full">
                    <div
                      className={`h-full rounded-full transition-all ${
                        task.status === 'completed' ? 'bg-green-500 w-full' :
                        task.status === 'running' ? 'bg-blue-500' :
                        'bg-gray-600 w-0'
                      }`}
                      style={{
                        width: task.status === 'running'
                          ? `${Math.random() * 80 + 10}%`
                          : '0%'
                      }}
                    />
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {tasks.length > 20 && (
        <div className="mt-4 text-center text-gray-400 text-sm">
          ... and {tasks.length - 20} more tasks
        </div>
      )}
    </div>
  );
};

const QueueDepthChart: React.FC<{ tasks: Task[] }> = ({ tasks }) => {
  // Simple queue depth visualization
  const pending = tasks.filter(t => t.status === 'pending').length;
  const maxDepth = Math.max(...tasks.map(() => 1), 50);
  
  return (
    <div className="flex items-end justify-center h-full gap-1 px-4">
      <div
        className="bg-gray-500 rounded-t"
        style={{ height: `${(pending / maxDepth) * 100}%`, width: '20px' }}
        title={`${pending} pending`}
      />
      <span className="text-gray-300 text-xs">{pending} queued</span>
    </div>
  );
};
```

### 4. Network Topology Visualization (PeerGraph.tsx)

Display connected peer network:

```typescript
// src/components/Network/PeerGraph.tsx
import React, { useEffect, useRef } from 'react';
import { usePeers } from '../../hooks/usePeers';
import * as d3 from 'd3';

interface PeerNode {
  id: string;
  reputation: number;        // 0-100
  isBlacklisted: boolean;
  capabilities: string[];
  lastSeen: number;
}

export const PeerGraph: React.FC = () => {
  const svgRef = useRef<SVGSVGElement>(null);
  const { peers } = usePeers();

  useEffect(() => {
    if (!svgRef.current || peers.length === 0) return;

    const width = svgRef.current.clientWidth;
    const height = svgRef.current.clientHeight;

    // Build graph data
    const nodes = peers.map(p => ({
      id: p.id,
      reputation: p.reputation,
      isBlacklisted: p.isBlacklisted,
    }));

    const links = [];
    for (let i = 0; i < peers.length; i++) {
      for (let j = i + 1; j < Math.min(i + 5, peers.length); j++) {
        links.push({ source: peers[i].id, target: peers[j].id });
      }
    }

    // Remove previous content
    d3.select(svgRef.current).selectAll('*').remove();

    const svg = d3.select(svgRef.current)
      .attr('width', width)
      .attr('height', height);

    // Define arrow markers for links
    svg.append('defs')
      .selectAll('marker')
      .data(['arrowhead'])
      .enter()
      .append('marker')
      .attr('id', 'arrowhead')
      .attr('markerWidth', 10)
      .attr('markerHeight', 10)
      .attr('refX', 9)
      .attr('refY', 3)
      .attr('orient', 'auto')
      .append('polygon')
      .attr('points', '0 0, 10 3, 0 6')
      .attr('fill', '#6b7280');

    // Create force simulation
    const simulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(links).id((d: any) => d.id).distance(100))
      .force('charge', d3.forceManyBody().strength(-300))
      .force('center', d3.forceCenter(width / 2, height / 2));

    // Draw links
    const link = svg.append('g')
      .selectAll('line')
      .data(links)
      .enter()
      .append('line')
      .attr('stroke', '#4b5563')
      .attr('stroke-width', 1.5)
      .attr('marker-end', 'url(#arrowhead)');

    // Draw nodes
    const node = svg.append('g')
      .selectAll('circle')
      .data(nodes)
      .enter()
      .append('circle')
      .attr('r', (d: any) => {
        if (d.isBlacklisted) return 6;
        return Math.max(8, (d.reputation / 100) * 20);
      })
      .attr('fill', (d: any) => {
        if (d.isBlacklisted) return '#ef4444';
        if (d.reputation < 30) return '#f59e0b';
        return '#10b981';
      })
      .attr('stroke', '#e5e7eb')
      .attr('stroke-width', 2)
      .call(d3.drag()
        .on('start', dragstarted)
        .on('drag', dragged)
        .on('end', dragended));

    // Add labels
    const labels = svg.append('g')
      .selectAll('text')
      .data(nodes)
      .enter()
      .append('text')
      .attr('font-size', 10)
      .attr('fill', '#e5e7eb')
      .attr('text-anchor', 'middle')
      .text((d: any) => d.id.slice(0, 4));

    // Add tooltips
    node.append('title')
      .text((d: any) => `Reputation: ${d.reputation}/100`);

    // Update positions on tick
    simulation.on('tick', () => {
      link
        .attr('x1', (d: any) => d.source.x)
        .attr('y1', (d: any) => d.source.y)
        .attr('x2', (d: any) => d.target.x)
        .attr('y2', (d: any) => d.target.y);

      node
        .attr('cx', (d: any) => Math.max(10, Math.min(width - 10, d.x)))
        .attr('cy', (d: any) => Math.max(10, Math.min(height - 10, d.y)));

      labels
        .attr('x', (d: any) => Math.max(10, Math.min(width - 10, d.x)))
        .attr('y', (d: any) => Math.max(10, Math.min(height - 10, d.y)) + 4);
    });

    function dragstarted(event: any, d: any) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      d.fx = d.x;
      d.fy = d.y;
    }

    function dragged(event: any, d: any) {
      d.fx = event.x;
      d.fy = event.y;
    }

    function dragended(event: any, d: any) {
      if (!event.active) simulation.alphaTarget(0);
      d.fx = null;
      d.fy = null;
    }
  }, [peers]);

  return (
    <div className="bg-gray-800 rounded-lg p-6 border border-gray-700 col-span-2">
      <h2 className="text-xl font-bold text-white mb-4">Peer Network Topology</h2>
      <svg
        ref={svgRef}
        className="w-full border border-gray-700 rounded bg-gray-900"
        style={{ height: '400px' }}
      />
      <div className="mt-4 flex gap-6 text-sm">
        <div className="flex items-center gap-2">
          <div className="w-4 h-4 bg-green-500 rounded-full" />
          <span className="text-gray-300">Good reputation (≥30)</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-4 h-4 bg-yellow-500 rounded-full" />
          <span className="text-gray-300">Low reputation (&lt;30)</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-4 h-4 bg-red-500 rounded-full" />
          <span className="text-gray-300">Blacklisted</span>
        </div>
      </div>
    </div>
  );
};
```

## WebSocket Integration

WebSocket service for real-time updates from `swarm/api_server.py`:

```typescript
// src/hooks/useWebSocket.ts
import { useEffect, useRef, useState } from 'react';

interface WebSocketMessage {
  type: string;
  data: any;
  timestamp: number;
}

export const useWebSocket = (url: string) => {
  const [message, setMessage] = useState<WebSocketMessage | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const ws = useRef<WebSocket | null>(null);

  useEffect(() => {
    // Ensure we have a protocol prefix
    const wsUrl = url.startsWith('ws')
      ? url
      : `ws://${window.location.host}${url}`;

    ws.current = new WebSocket(wsUrl);

    ws.current.onopen = () => {
      setIsConnected(true);
      console.log(`Connected to ${wsUrl}`);
    };

    ws.current.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        setMessage(data);
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e);
      }
    };

    ws.current.onerror = (error) => {
      console.error('WebSocket error:', error);
      setIsConnected(false);
    };

    ws.current.onclose = () => {
      setIsConnected(false);
      // Reconnect after 3 seconds
      setTimeout(() => {
        // Trigger reconnection
      }, 3000);
    };

    return () => {
      if (ws.current?.readyState === WebSocket.OPEN) {
        ws.current.close();
      }
    };
  }, [url]);

  return { message, isConnected };
};
```

## API Integration

Fetch data from swarm API server:

```typescript
// src/services/api.ts
const API_BASE = 'http://localhost:5000/api';

export const apiClient = {
  async getMetrics() {
    const res = await fetch(`${API_BASE}/metrics`);
    return res.json();
  },

  async getTaskQueue() {
    const res = await fetch(`${API_BASE}/tasks/queue`);
    return res.json();
  },

  async getGpuStatus() {
    const res = await fetch(`${API_BASE}/gpu/status`);
    return res.json();
  },

  async getPeers() {
    const res = await fetch(`${API_BASE}/network/peers`);
    return res.json();
  },

  async getRewards(account: string) {
    const res = await fetch(`${API_BASE}/rewards/${account}`);
    return res.json();
  },

  async claimReward(taskId: string) {
    const res = await fetch(`${API_BASE}/rewards/claim`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ taskId }),
    });
    return res.json();
  },
};
```

## Package Configuration

```json
{
  "name": "@x3-chain/swarm-dashboard",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "lint": "eslint src --ext ts,tsx",
    "type-check": "tsc --noEmit"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "react-router-dom": "^6.16.0",
    "@tanstack/react-query": "^5.0.0",
    "zustand": "^4.4.0",
    "recharts": "^2.10.0",
    "d3": "^7.8.0",
    "@headlessui/react": "^1.7.0",
    "tailwindcss": "^3.3.0",
    "axios": "^1.5.0"
  },
  "devDependencies": {
    "typescript": "^5.1.0",
    "vite": "^5.0.0",
    "@vitejs/plugin-react": "^4.2.0",
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@types/d3": "^7.4.0",
    "autoprefixer": "^10.4.0",
    "postcss": "^8.4.0",
    "tailwindcss": "^3.3.0"
  }
}
```

## Development Workflow

```bash
# Install dependencies
npm install

# Start development server (hot reload)
npm run dev
# Accessible at http://localhost:5173

# Build for production
npm run build

# Type checking
npm run type-check

# Linting
npm run lint
```

## Key Metrics to Display

| Metric | Source | Update Frequency | Display |
|--------|--------|------------------|---------|
| Task Submission Rate | `/api/metrics/tasks/submitted` | 5s | Counter + Trend |
| Task Completion Rate | `/api/metrics/tasks/completed` | 5s | Counter + Trend |
| Task Failure Rate | `/api/metrics/tasks/failed` | 5s | Counter + Trend |
| GPU Utilization | `/api/metrics/gpu/utilization` | 2s | Line Chart (5min window) |
| GPU Memory | `/api/metrics/gpu/memory` | 2s | Line Chart (5min window) |
| GPU Temperature | `/api/metrics/gpu/temperature` | 5s | Gauge + Bar |
| Network Peers | `/api/metrics/network/peers` | 10s | Counter + Graph |
| Network Latency | `/api/metrics/network/latency` | 10s | Histogram |
| Verification Time | `/api/metrics/verification/time` | 5s | P50/P95/P99 Chart |
| Rewards Distributed | `/api/metrics/economics/rewards` | 60s | Counter |
| Slashing Events | `/api/metrics/economics/slashing` | 60s | Event Log |

## Testing Strategy

```typescript
// tests/Dashboard.test.tsx
import { render, screen, waitFor } from '@testing-library/react';
import { Dashboard } from '../pages/Dashboard';

test('displays metric cards with data', async () => {
  render(<Dashboard />);
  
  await waitFor(() => {
    expect(screen.getByText('Active Tasks')).toBeInTheDocument();
  });
  
  const taskCount = screen.getByText(/\d+ tasks/);
  expect(taskCount).toBeVisible();
});
```

## Performance Optimization

1. **Code Splitting**: Lazy load page components with React.lazy()
2. **Memoization**: Use React.memo() for expensive components
3. **WebSocket Debouncing**: Throttle metric updates to 1s
4. **Chart Data Limits**: Keep only last 60-300 points
5. **Virtual Scrolling**: For large task tables

## Deployment

```dockerfile
# Dockerfile for dashboard
FROM node:18-alpine as builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:18-alpine
WORKDIR /app
RUN npm install -g http-server
COPY --from=builder /app/dist ./dist
EXPOSE 3001
CMD ["http-server", "dist", "-p", "3001"]
```

```yaml
# kubernetes/dashboard-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: swarm-dashboard
spec:
  replicas: 2
  template:
    spec:
      containers:
      - name: dashboard
        image: x3-chain/swarm-dashboard:v1
        ports:
        - containerPort: 3001
        env:
        - name: REACT_APP_API_URL
          value: "http://coordinator:9000/api"
        - name: REACT_APP_WS_URL
          value: "ws://coordinator:9000/ws"
```

This dashboard provides complete visibility into the GPU Swarm ecosystem with real-time updates, performance monitoring, and intuitive visualizations.
