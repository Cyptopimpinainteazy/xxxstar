'use client';

import React from 'react';
import {
  AreaChart,
  Area,
  BarChart,
  Bar,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from 'recharts';

export interface TrendPoint {
  timestamp: string;
  volume: number;
  successRate: number;
}

export function VolumeTrendChart({ data }: { data: TrendPoint[] }) {
  return (
    <ResponsiveContainer width="100%" height={300}>
      <AreaChart data={data} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
        <defs>
          <linearGradient id="colorVolume" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor="#00d4aa" stopOpacity={0.8} />
            <stop offset="95%" stopColor="#00d4aa" stopOpacity={0} />
          </linearGradient>
        </defs>
        <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
        <XAxis dataKey="timestamp" stroke="#8a8a8e" />
        <YAxis stroke="#8a8a8e" />
        <Tooltip
          contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }}
          labelStyle={{ color: '#e0e0e0' }}
        />
        <Area
          type="monotone"
          dataKey="volume"
          stroke="#00d4aa"
          fillOpacity={1}
          fill="url(#colorVolume)"
          name="Volume (USDC)"
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}

export interface StateDistribution {
  name: string;
  value: number;
  color?: string;
}

export function IntentStatePie({ data }: { data: StateDistribution[] }) {
  const colors = [
    '#00d4aa', // Finalized
    '#4488ff', // Executing
    '#ffaa00', // Expired
    '#ff4444', // Slashed
    '#555559', // Cancelled
  ];

  return (
    <ResponsiveContainer width="100%" height={280}>
      <PieChart>
        <Pie
          data={data}
          cx="50%"
          cy="50%"
          labelLine={false}
          label={({ name, value }: any) => `${name}: ${value}`}
          outerRadius={80}
          fill="#8884d8"
          dataKey="value"
        >
          {data.map((_, index) => (
            <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
          ))}
        </Pie>
        <Tooltip
          contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }}
          labelStyle={{ color: '#e0e0e0' }}
        />
      </PieChart>
    </ResponsiveContainer>
  );
}

export interface AgentRepData {
  agent: string;
  reputation: number;
  successRate: number;
}

export function AgentReputationChart({ data }: { data: AgentRepData[] }) {
  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart data={data} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
        <XAxis dataKey="agent" stroke="#8a8a8e" />
        <YAxis stroke="#8a8a8e" />
        <Tooltip
          contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }}
          labelStyle={{ color: '#e0e0e0' }}
        />
        <Legend />
        <Bar dataKey="reputation" fill="#00d4aa" name="Reputation" />
        <Bar dataKey="successRate" fill="#4488ff" name="Success Rate %" />
      </BarChart>
    </ResponsiveContainer>
  );
}

export interface SlashCounts {
  severity: string;
  count: number;
}

export function SlashSeverityChart({ data }: { data: SlashCounts[] }) {
  return (
    <ResponsiveContainer width="100%" height={280}>
      <BarChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
        <XAxis dataKey="severity" stroke="#8a8a8e" />
        <YAxis stroke="#8a8a8e" />
        <Tooltip
          contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }}
          labelStyle={{ color: '#e0e0e0' }}
        />
        <Bar dataKey="count" fill="#ff4444" name="Count" />
      </BarChart>
    </ResponsiveContainer>
  );
}

export interface BondDistData {
  agent: string;
  bond: number;
}

export function BondDistributionChart({ data }: { data: BondDistData[] }) {
  return (
    <ResponsiveContainer width="100%" height={280}>
      <BarChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
        <XAxis dataKey="agent" stroke="#8a8a8e" />
        <YAxis stroke="#8a8a8e" />
        <Tooltip
          contentStyle={{ backgroundColor: '#1a1a1d', border: '1px solid #2a2a2e' }}
          labelStyle={{ color: '#e0e0e0' }}
        />
        <Bar dataKey="bond" fill="#00d4aa" name="Bond (USDC)" />
      </BarChart>
    </ResponsiveContainer>
  );
}
