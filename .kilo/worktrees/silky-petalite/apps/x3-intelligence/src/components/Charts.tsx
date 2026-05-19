// X3 Intelligence — Chart Components
// Professional visualizations powered by Recharts

import React from "react";
import {
  LineChart,
  Line,
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
} from "recharts";

// ─── Volume & Success Trend Chart ─────────────────────────────

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
          contentStyle={{ backgroundColor: "#1a1a1d", border: "1px solid #2a2a2e" }}
          labelStyle={{ color: "#e0e0e0" }}
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

// ─── Success Rate Multi-Line Chart ────────────────────────────

export interface SuccessPoint {
  agent: string;
  timestamp: string;
  rate: number;
}

export function SuccessRateChart({ data }: { data: SuccessPoint[] }) {
  const agents = [...new Set(data.map((d) => d.agent))];
  const colors = ["#00d4aa", "#4488ff", "#ffaa00", "#ff4444"];

  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart
        data={data}
        margin={{ top: 10, right: 30, left: 0, bottom: 0 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
        <XAxis dataKey="timestamp" stroke="#8a8a8e" />
        <YAxis stroke="#8a8a8e" domain={[0, 100]} />
        <Tooltip
          contentStyle={{ backgroundColor: "#1a1a1d", border: "1px solid #2a2a2e" }}
          labelStyle={{ color: "#e0e0e0" }}
        />
        <Legend />
        {agents.map((agent, i) => (
          <Line
            key={agent}
            type="monotone"
            dataKey="rate"
            data={data.filter((d) => d.agent === agent)}
            stroke={colors[i % colors.length]}
            name={agent}
            strokeWidth={2}
            dot={false}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
}

// ─── Intent State Distribution Pie ───────────────────────────

export interface StateDistribution {
  name: string;
  value: number;
  color?: string;
}

export function IntentStatePie({
  data,
}: {
  data: StateDistribution[];
}) {
  const colors = [
    "#00d4aa", // Finalized
    "#4488ff", // Executing
    "#ffaa00", // Expired
    "#ff4444", // Slashed
    "#555559", // Cancelled
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
          {data.map((entry, index) => (
            <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
          ))}
        </Pie>
        <Tooltip
          contentStyle={{ backgroundColor: "#1a1a1d", border: "1px solid #2a2a2e" }}
          labelStyle={{ color: "#e0e0e0" }}
        />
      </PieChart>
    </ResponsiveContainer>
  );
}

// ─── Agent Reputation Bar Chart ──────────────────────────────

export interface AgentReputation {
  agent: string;
  reputation: number;
  successRate: number;
}

export function AgentReputationChart({
  data,
}: {
  data: AgentReputation[];
}) {
  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart data={data} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
        <XAxis dataKey="agent" stroke="#8a8a8e" />
        <YAxis stroke="#8a8a8e" />
        <Tooltip
          contentStyle={{ backgroundColor: "#1a1a1d", border: "1px solid #2a2a2e" }}
          labelStyle={{ color: "#e0e0e0" }}
        />
        <Legend />
        <Bar dataKey="reputation" fill="#00d4aa" name="Reputation" />
        <Bar dataKey="successRate" fill="#4488ff" name="Success Rate %" />
      </BarChart>
    </ResponsiveContainer>
  );
}

// ─── Slash Severity Distribution ─────────────────────────────

export interface SlashCounts {
  severity: string;
  count: number;
}

export function SlashSeverityChart({ data }: { data: SlashCounts[] }) {
  const colors = ["#ffaa00", "#ff8800", "#ff4444", "#cc0000"];

  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart data={data} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
        <XAxis dataKey="severity" stroke="#8a8a8e" />
        <YAxis stroke="#8a8a8e" />
        <Tooltip
          contentStyle={{ backgroundColor: "#1a1a1d", border: "1px solid #2a2a2e" }}
          labelStyle={{ color: "#e0e0e0" }}
        />
        <Bar dataKey="count" fill="#ff4444" name="Slashes">
          {data.map((entry, index) => (
            <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
          ))}
        </Bar>
      </BarChart>
    </ResponsiveContainer>
  );
}

// ─── Bond Distribution ───────────────────────────────────────

export interface BondEntry {
  agent: string;
  bond: number;
}

export function BondDistributionChart({ data }: { data: BondEntry[] }) {
  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart
        data={data}
        margin={{ top: 10, right: 30, left: 0, bottom: 0 }}
        layout="vertical"
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
        <XAxis type="number" stroke="#8a8a8e" />
        <YAxis dataKey="agent" type="category" stroke="#8a8a8e" />
        <Tooltip
          contentStyle={{ backgroundColor: "#1a1a1d", border: "1px solid #2a2a2e" }}
          labelStyle={{ color: "#e0e0e0" }}
        />
        <Bar dataKey="bond" fill="#00d4aa" name="Bond (USDC)" />
      </BarChart>
    </ResponsiveContainer>
  );
}
