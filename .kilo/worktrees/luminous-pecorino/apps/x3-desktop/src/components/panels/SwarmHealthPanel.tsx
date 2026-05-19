import React from "react";
import { useSwarmHealth } from "@/hooks/useSwarmHealth";
import { PanelError, PanelLoading } from "@/components/panels/PanelStatus";
import type { SwarmNodeStatus } from "@/types/panelTelemetry";

const Bar: React.FC<{ value: number; max?: number; color?: string; label?: string }> = ({
  value,
  max = 100,
  color = "#ff6b35",
  label,
}) => {
  const pct = Math.min(100, (value / max) * 100);
  return (
    <div className="flex items-center gap-2 text-[10px] font-mono">
      {label && <span className="w-10 text-right text-[#a8a8a8]">{label}</span>}
      <div className="flex-1 h-2 bg-white/5 rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-700"
          style={{ width: `${pct}%`, background: pct > 85 ? "#ef5350" : color }}
        />
      </div>
      <span className="w-10 text-[#e0e0e0]">{Math.round(pct)}%</span>
    </div>
  );
};

const StatusDot: React.FC<{ status: SwarmNodeStatus }> = ({ status }) => {
  const colors: Record<SwarmNodeStatus, string> = {
    online: "#4caf50",
    idle: "#ff9800",
    offline: "#666",
    slashed: "#ef5350",
  };

  return (
    <span
      className="inline-block w-2 h-2 rounded-full"
      style={{ background: colors[status], boxShadow: `0 0 6px ${colors[status]}88` }}
    />
  );
};

const Metric: React.FC<{ label: string; value: string; color: string }> = ({ label, value, color }) => (
  <div className="text-center">
    <div className="text-[9px] font-mono uppercase tracking-wider text-[#666]">{label}</div>
    <div className="text-sm font-bold font-mono" style={{ color }}>{value}</div>
  </div>
);

const SwarmHealthPanel: React.FC = () => {
  const { data, loading, error } = useSwarmHealth();

  if (error) return <PanelError message={error} />;
  if (loading || !data) return <PanelLoading label="Gathering GPU telemetry…" />;

  const { summary, nodes } = data;
  const vramUsagePct = (summary.totalVramUsed / Math.max(1, summary.totalVramCapacity)) * 100;

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-[#e0e0e0] overflow-hidden">
      <div className="flex items-center gap-4 px-4 py-3 border-b border-[#ff6b35]/15 bg-[#0d0d14]">
        <Metric label="NODES" value={`${summary.onlineNodes}/${summary.totalNodes}`} color="#4caf50" />
        <Metric label="AVG GPU" value={`${Math.round(summary.avgGpuUtil)}%`} color="#ff6b35" />
        <Metric
          label="VRAM"
          value={`${(summary.totalVramUsed / 1024 / 1024).toFixed(1)}M / ${(summary.totalVramCapacity / 1024 / 1024).toFixed(0)}M`}
          color="#00b4ff"
        />
        <Metric label="JOBS" value={String(summary.queuedJobs)} color="#ff8c42" />
        <div className="flex-1" />
        <span className="text-[9px] font-mono text-[#666] animate-pulse">LIVE</span>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-2 space-y-2 scrollbar-thin scrollbar-thumb-[#ff6b35]/20">
        <div className="px-2 mb-2 text-[9px] font-mono text-[#666]">
          Total GPU memory usage: {vramUsagePct.toFixed(1)}%
        </div>
        {nodes.map((node) => (
          <div
            key={node.id}
            className="p-3 rounded-lg border border-white/5 bg-white/[0.02] hover:bg-white/[0.04] transition"
          >
            <div className="flex items-center gap-2 mb-2">
              <StatusDot status={node.status} />
              <span className="font-mono text-xs font-bold text-[#ff8c42]">{node.name}</span>
              <span className="text-[9px] uppercase ml-auto text-[#888] font-mono">{node.status}</span>
            </div>

            <div className="space-y-1">
              <Bar value={node.gpuUtil} label="GPU" color="#ff6b35" />
              <Bar value={node.vramUsed} max={node.vramCapacity} label="VRAM" color="#00b4ff" />
              <div className="flex items-center justify-between text-[10px] font-mono text-[#888] mt-1">
                <span>🌡 {node.temperature}°C</span>
                <span>⏱ {node.uptimeHours}h</span>
                <span style={{ color: node.sla < 90 ? "#ef5350" : "#4caf50" }}>SLA {node.sla}%</span>
                <span>📋 {node.jobs} jobs</span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default SwarmHealthPanel;
