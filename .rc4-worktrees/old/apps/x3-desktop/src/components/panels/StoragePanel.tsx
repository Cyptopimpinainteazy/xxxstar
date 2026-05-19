import React from "react";
import { useStorageMonitor } from "@/hooks/useStorageMonitor";
import { PanelError, PanelLoading } from "@/components/panels/PanelStatus";

function formatSize(b: number): string {
  if (b < 1024) return `${b}B`;
  if (b < 1048576) return `${(b / 1024).toFixed(1)}KB`;
  if (b < 1073741824) return `${(b / 1048576).toFixed(1)}MB`;
  return `${(b / 1073741824).toFixed(2)}GB`;
}

const Stat: React.FC<{ label: string; value: string; color: string }> = ({ label, value, color }) => (
  <div className="text-center shrink-0">
    <div className="text-[9px] font-mono uppercase tracking-wider text-[#666]">{label}</div>
    <div className="text-sm font-bold font-mono" style={{ color }}>{value}</div>
  </div>
);

const STATUS_COLORS: Record<string, string> = {
  snapshot: "#8b5cf6",
  artifact: "#ff6b35",
  "agent-memory": "#00b4ff",
  contract: "#ff4488",
  dataset: "#4caf50",
};

const StoragePanel: React.FC = () => {
  const { data, loading, error } = useStorageMonitor();

  if (error) return <PanelError message={error} />;
  if (loading || !data) return <PanelLoading label="Streaming storage health…" />;

  const { pins, proofs, capacityBytes, usedBytes } = data;
  const usedPct = (usedBytes / Math.max(1, capacityBytes)) * 100;
  const pinnedCount = pins.filter((p) => p.status === "pinned").length;

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-[#e0e0e0] overflow-hidden">
      <div className="flex items-center gap-4 px-4 py-3 border-b border-[#8b5cf6]/15 bg-[#0d0d14]">
        <Stat label="PINNED" value={`${pinnedCount}/${pins.length}`} color="#4caf50" />
        <Stat label="TOTAL" value={formatSize(usedBytes)} color="#8b5cf6" />
        <div className="flex-1">
          <div className="text-[9px] font-mono text-[#666] mb-0.5 text-right">
            {usedPct.toFixed(1)}% · {formatSize(capacityBytes)} cap
          </div>
          <div className="h-2 bg-white/5 rounded-full overflow-hidden">
            <div
              className="h-full rounded-full transition-all"
              style={{
                width: `${Math.min(usedPct, 100)}%`,
                background: usedPct > 85 ? "#ef5350" : "linear-gradient(90deg, #8b5cf6, #ff6b35)",
              }}
            />
          </div>
        </div>
        <div className="flex gap-1 ml-2">
          {(["pins", "proofs"] as const).map((t) => (
            <span key={t} className="px-2 py-1 text-[10px] font-mono uppercase text-[#666]">
              {t}
            </span>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-2 scrollbar-thin scrollbar-thumb-[#8b5cf6]/20">
        <div className="space-y-1.5">
          {pins.map((pin) => (
            <div
              key={pin.cid}
              className="flex items-center gap-3 px-3 py-2.5 rounded border border-white/5 bg-white/[0.02] hover:bg-white/[0.04] transition"
            >
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{
                  background:
                    pin.status === "pinned"
                      ? "#4caf50"
                      : pin.status === "pinning"
                      ? "#ff9800"
                      : pin.status === "failed"
                      ? "#ef5350"
                      : "#555",
                  boxShadow: pin.status === "pinned" ? "0 0 6px #4caf5066" : "none",
                }}
              />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-[11px] font-mono font-bold text-[#e0e0e0] truncate">{pin.name}</span>
                  <span
                    className="text-[8px] px-1.5 py-0.5 rounded uppercase font-mono"
                    style={{
                      color: STATUS_COLORS[pin.type],
                      background: `${STATUS_COLORS[pin.type]}15`,
                      border: `1px solid ${STATUS_COLORS[pin.type]}30`,
                    }}
                  >
                    {pin.type}
                  </span>
                </div>
                <div className="text-[9px] font-mono text-[#666] mt-0.5">{pin.cid}</div>
              </div>
              <div className="text-right shrink-0 text-[10px] font-mono">
                <div className="text-[#e0e0e0]">{formatSize(pin.size)}</div>
                <div className="text-[#666]">
                  {pin.replicas}x · {pin.proofAgeMinutes < 60 ? `${pin.proofAgeMinutes}m` : `${Math.floor(pin.proofAgeMinutes / 60)}h`}
                </div>
              </div>
            </div>
          ))}
        </div>

        <div className="mt-4 space-y-1">
          {proofs.map((proof, i) => (
            <div
              key={`${proof.cid}-${proof.epoch}-${i}`}
              className="flex items-center gap-3 px-3 py-2 rounded border border-white/5 bg-white/[0.02] text-[11px] font-mono"
            >
              <span className="text-[#555]">{proof.timestamp}</span>
              <span className="text-[#888]">#{proof.epoch}</span>
              <span className="text-[#666] truncate flex-1">{proof.cid}</span>
              <span
                className="shrink-0 px-1.5 py-0.5 rounded text-[9px] uppercase"
                style={{
                  color:
                    proof.result === "valid"
                      ? "#4caf50"
                      : proof.result === "challenged"
                      ? "#ff9800"
                      : "#ef5350",
                  background:
                    proof.result === "valid"
                      ? "#4caf5015"
                      : proof.result === "challenged"
                      ? "#ff980015"
                      : "#ef535015",
                }}
              >
                {proof.result}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default StoragePanel;
