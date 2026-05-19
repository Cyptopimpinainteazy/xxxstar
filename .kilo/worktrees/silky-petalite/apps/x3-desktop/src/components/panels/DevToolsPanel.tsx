import React, { useEffect, useRef, useState } from "react";
import { useIdeTelemetry } from "@/hooks/useIdeTelemetry";
import { PanelError, PanelLoading } from "@/components/panels/PanelStatus";

const STATUS_COLORS: Record<string, string> = {
  building: "#00b4ff",
  success: "#4caf50",
  failed: "#ef5350",
  queued: "#666",
  deployed: "#4caf50",
  pending: "#ff9800",
  ok: "#4caf50",
  err: "#ef5350",
};

const Indicator: React.FC<{ label: string; value: number; total: number; color: string }> = ({
  label,
  value,
  total,
  color,
}) => (
  <div className="text-center">
    <div className="text-[9px] font-mono uppercase tracking-wider text-[#666]">{label}</div>
    <div className="text-sm font-bold font-mono" style={{ color }}>
      {value}<span className="text-[#555]">/{total}</span>
    </div>
  </div>
);

const DevToolsPanel: React.FC = () => {
  const { data, loading, error } = useIdeTelemetry();
  const [tab, setTab] = useState<"builds" | "contracts" | "traces">("builds");
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!logRef.current) return;
    logRef.current.scrollTo({ top: logRef.current.scrollHeight, behavior: "smooth" });
  }, [data?.logLines]);

  if (error) return <PanelError message={error} />;
  if (loading || !data) return <PanelLoading label="Polling IDE telemetry…" />;

  const { builds, contracts, traces, logLines } = data;
  const successfulBuilds = builds.filter((b) => b.status === "success").length;
  const deployedContracts = contracts.filter((c) => c.status === "deployed").length;

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-[#e0e0e0] overflow-hidden">
      <div className="flex items-center gap-4 px-4 py-3 border-b border-[#ff4488]/15 bg-[#0d0d14]">
        <Indicator label="BUILDS" value={successfulBuilds} total={builds.length} color="#4caf50" />
        <Indicator label="CONTRACTS" value={deployedContracts} total={contracts.length} color="#ff4488" />
        <div className="flex-1" />
        <div className="flex gap-1">
          {(["builds", "contracts", "traces"] as const).map((value) => (
            <button
              key={value}
              onClick={() => setTab(value)}
              className={`px-2 py-1 text-[10px] font-mono uppercase rounded transition ${
                tab === value ? `bg-[#ff4488]/20 text-[#ff4488]` : "text-[#666] hover:text-[#999]"
              }`}
            >
              {value}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-hidden flex flex-col">
        {tab === "builds" && (
          <>
            <div className="px-3 py-2 space-y-1.5 overflow-y-auto flex-1 scrollbar-thin scrollbar-thumb-[#ff4488]/20">
              {builds.map((b) => (
                <div
                  key={b.id}
                  className="flex items-center gap-3 px-3 py-2 rounded border border-white/5 bg-white/[0.02] text-[11px] font-mono"
                >
                  <span
                    className={`w-2 h-2 rounded-full shrink-0 ${b.status === "building" ? "animate-pulse" : ""}`}
                    style={{ background: STATUS_COLORS[b.status] }}
                  />
                  <span className="flex-1 text-[#e0e0e0] truncate">{b.target}</span>
                  <span className="text-[#666]">{b.timestamp}</span>
                  <span className="text-[#888]">
                    {b.durationSeconds > 0
                      ? `${b.durationSeconds}s`
                      : b.status === "building"
                        ? "..."
                        : "—"}
                  </span>
                  <span
                    className="text-[9px] uppercase px-1.5 py-0.5 rounded"
                    style={{
                      color: STATUS_COLORS[b.status],
                      background: `${STATUS_COLORS[b.status]}15`,
                    }}
                  >
                    {b.status}
                  </span>
                </div>
              ))}
            </div>
            <div
              ref={logRef}
              className="h-28 shrink-0 border-t border-white/5 bg-black/40 px-3 py-2 overflow-y-auto font-mono text-[10px] text-[#4caf50]/80 scrollbar-thin scrollbar-thumb-[#4caf50]/20"
            >
              {logLines.map((line, i) => (
                <div
                  key={`${line}-${i}`}
                  className={
                    line.includes("Building")
                      ? "text-[#00b4ff]/80"
                      : line.includes("Finished")
                        ? "text-[#4caf50]"
                        : ""
                  }
                >
                  {line}
                </div>
              ))}
              <span className="animate-pulse">▌</span>
            </div>
          </>
        )}

        {tab === "contracts" && (
          <div className="px-3 py-2 space-y-2 overflow-y-auto flex-1 scrollbar-thin scrollbar-thumb-[#ff4488]/20">
            {contracts.map((c) => (
              <div key={c.name} className="p-3 rounded-lg border border-white/5 bg-white/[0.02]">
                <div className="flex items-center gap-2 mb-1">
                  <span className="w-2 h-2 rounded-full" style={{ background: STATUS_COLORS[c.status] }} />
                  <span className="font-bold text-xs text-[#ff4488]">{c.name}</span>
                  <span
                    className="text-[9px] uppercase ml-auto font-mono"
                    style={{ color: STATUS_COLORS[c.status] }}
                  >
                    {c.status}
                  </span>
                </div>
                <div className="text-[10px] font-mono text-[#888] space-y-0.5">
                  <div>
                    Address: <span className="text-[#e0e0e0]">{c.address}</span>
                  </div>
                  <div>
                    Network: <span className="text-[#00b4ff]">{c.network}</span>
                  </div>
                  {c.gasUsed > 0 && (
                    <div>
                      Gas: <span className="text-[#ff8c42]">{c.gasUsed.toLocaleString()}</span>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}

        {tab === "traces" && (
          <div className="px-3 py-2 space-y-1 overflow-y-auto flex-1 scrollbar-thin scrollbar-thumb-[#ff4488]/20">
            <div className="grid grid-cols-[60px_1fr_40px_80px_auto] gap-2 text-[9px] font-mono text-[#555] uppercase px-3 mb-1">
              <span>Block</span>
              <span>Extrinsic</span>
              <span>Result</span>
              <span>Gas</span>
              <span>State Root</span>
            </div>
            {traces.map((t, i) => (
              <div
                key={`${t.blockNum}-${t.extrinsic}-${i}`}
                className="grid grid-cols-[60px_1fr_40px_80px_auto] gap-2 px-3 py-1.5 rounded text-[10px] font-mono border border-white/5 bg-white/[0.02] items-center"
              >
                <span className="text-[#888]">#{t.blockNum}</span>
                <span className="text-[#e0e0e0] truncate">{t.extrinsic}</span>
                <span style={{ color: STATUS_COLORS[t.result] }}>{t.result.toUpperCase()}</span>
                <span className="text-[#ff8c42]">{t.gasUsed.toLocaleString()}</span>
                <span className="text-[#555] truncate">{t.stateRoot}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default DevToolsPanel;
