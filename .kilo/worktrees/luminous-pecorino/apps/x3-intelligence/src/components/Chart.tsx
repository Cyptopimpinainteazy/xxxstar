// Chart wrapper components using Recharts

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
  ComposedChart,
} from "recharts";

export interface ChartDataPoint {
  [key: string]: number | string;
}

interface TimeSeriesChartProps {
  data: ChartDataPoint[];
  dataKey: string;
  label: string;
  color?: string;
  height?: number;
}

export function TimeSeriesChart({
  data,
  dataKey,
  label,
  color = "#00d4aa",
  height = 300,
}: TimeSeriesChartProps) {
  return (
    <div style={{ width: "100%", height }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={data} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id={`gradient-${dataKey}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={color} stopOpacity={0.3} />
              <stop offset="95%" stopColor={color} stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
          <XAxis dataKey="name" stroke="#8a8a8e" style={{ fontSize: 12 }} />
          <YAxis stroke="#8a8a8e" style={{ fontSize: 12 }} />
          <Tooltip
            contentStyle={{
              backgroundColor: "#111113",
              border: "1px solid #2a2a2e",
              borderRadius: "4px",
            }}
            labelStyle={{ color: "#e0e0e0" }}
          />
          <Area
            type="monotone"
            dataKey={dataKey}
            stroke={color}
            fill={`url(#gradient-${dataKey})`}
            strokeWidth={2}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

interface MultiLineChartProps {
  data: ChartDataPoint[];
  lines: Array<{ dataKey: string; color: string; name: string }>;
  height?: number;
}

export function MultiLineChart({
  data,
  lines,
  height = 300,
}: MultiLineChartProps) {
  return (
    <div style={{ width: "100%", height }}>
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
          <XAxis dataKey="name" stroke="#8a8a8e" style={{ fontSize: 12 }} />
          <YAxis stroke="#8a8a8e" style={{ fontSize: 12 }} />
          <Tooltip
            contentStyle={{
              backgroundColor: "#111113",
              border: "1px solid #2a2a2e",
              borderRadius: "4px",
            }}
            labelStyle={{ color: "#e0e0e0" }}
          />
          <Legend />
          {lines.map((line) => (
            <Line
              key={line.dataKey}
              type="monotone"
              dataKey={line.dataKey}
              stroke={line.color}
              name={line.name}
              strokeWidth={2}
              dot={false}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

interface BarChartProps {
  data: ChartDataPoint[];
  dataKey: string;
  label: string;
  color?: string;
  height?: number;
}

export function BarChartComponent({
  data,
  dataKey,
  label,
  color = "#4488ff",
  height = 300,
}: BarChartProps) {
  return (
    <div style={{ width: "100%", height }}>
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#2a2a2e" />
          <XAxis dataKey="name" stroke="#8a8a8e" style={{ fontSize: 12 }} />
          <YAxis stroke="#8a8a8e" style={{ fontSize: 12 }} />
          <Tooltip
            contentStyle={{
              backgroundColor: "#111113",
              border: "1px solid #2a2a2e",
              borderRadius: "4px",
            }}
            labelStyle={{ color: "#e0e0e0" }}
          />
          <Bar dataKey={dataKey} fill={color} radius={[4, 4, 0, 0]} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

interface PieChartProps {
  data: Array<{ name: string; value: number }>;
  colors?: string[];
  height?: number;
}

export function PieChartComponent({
  data,
  colors = ["#00d4aa", "#ff4444", "#ffaa00", "#4488ff"],
  height = 300,
}: PieChartProps) {
  return (
    <div style={{ width: "100%", height }}>
      <ResponsiveContainer width="100%" height="100%">
        <PieChart>
          <Pie
            data={data}
            cx="50%"
            cy="50%"
            labelLine={false}
            label={({ name, value }) => `${name}: ${value}`}
            outerRadius={80}
            fill="#8884d8"
            dataKey="value"
          >
            {data.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
            ))}
          </Pie>
          <Tooltip
            contentStyle={{
              backgroundColor: "#111113",
              border: "1px solid #2a2a2e",
              borderRadius: "4px",
            }}
            labelStyle={{ color: "#e0e0e0" }}
          />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
}
