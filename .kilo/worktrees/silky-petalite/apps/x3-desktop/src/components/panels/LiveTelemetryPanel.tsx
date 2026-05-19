import React, { useEffect, useMemo, useState } from "react";
import { useTelemetryStream } from "@/hooks/useTelemetryStream";
import { PanelError, PanelLoading } from "@/components/panels/PanelStatus";

const HEATMAP_COLS = 4;
const HISTORY_LENGTH = 24;

function heatColor(value: number): string {
  if (value < 30) return "rgba(56, 189, 248, 0.5)";
  if (value < 60) return "rgba(250, 204, 21, 0.6)";
  if (value < 85) return "rgba(248, 113, 113, 0.7)";
  return "rgba(239, 68, 68, 0.85)";
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)}MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)}GB`;
}

const LiveTelemetryPanel: React.FC = () => {
  const { data, loading, error } = useTelemetryStream();
  const [history, setHistory] = useState<number[]>([]);

  useEffect(() => {
    if (!data) return;
    const usedPct = (data.storage.usedBytes / Math.max(1, data.storage.capacityBytes)) * 100;
    setHistory((prev) => {
      const next = [...prev, usedPct];
      return next.slice(-HISTORY_LENGTH);
    });
  }, [data?.storage.usedBytes, data?.storage.capacityBytes, data]);

  const heatmapCells = useMemo(() => {
    if (!data) return [];
    return data.swarm.nodes.map((node) => ({
      id: node.id,
      label: node.name,
      value: node.gpuUtil,
      status: node.status,
    }));
  }, [data]);

  const storageStats = useMemo(() => {
    if (!data) return { usedPct: 0, used: 0, capacity: 0 };
    const usedPct = (data.storage.usedBytes / Math.max(1, data.storage.capacityBytes)) * 100;
    return {
      usedPct,
      used: data.storage.usedBytes,
      capacity: data.storage.capacityBytes,
    };
  }, [data]);

  const sparkPath = useMemo(() => {
    if (history.length === 0) return "";
    const step = 100 / Math.max(1, history.length - 1);
    const points = history.map((value, idx) => {
      const x = idx * step;
      const y = 100 - Math.min(100, Math.max(0, value));
      return `${x},${y}`;
    });
    return `M ${points.join(" L ")}`;
  }, [history]);

  if (error) return <PanelError message={error} />;
  if (loading || !data) return <PanelLoading label="Streaming telemetry..." />;

  return (
    <div className="h-full flex flex-col bg-[#06060b] text-[#e6e6e6] overflow-hidden">
      <div className="flex items-center gap-4 px-4 py-3 border-b border-[#ff6b35]/20 bg-[#0a0a12]">
        <div>
          <div className="text-[9px] font-mono uppercase tracking-wider text-[#666]">Telemetry Core</div>
          <div className="text-sm font-semibold text-[#ff6b35]">Live Command Grid</div>
        </div>
        <div className="flex-1" />
        <div className="text-[10px] font-mono text-[#888]">Updated {new Date(data.updatedAt).toLocaleTimeString()}</div>
      </div>

      <div className="flex-1 grid grid-cols-2 gap-4 p-4 overflow-auto">
        <section className="rounded-xl border border-white/5 bg-[#0b0b14] p-4">
          <div className="flex items-center justify-between mb-3">
            <div>
              <div className="text-[9px] font-mono uppercase tracking-wider text-[#666]">GPU Swarm</div>
              <div className="text-sm font-semibold text-[#ff8c42]">Utilization Heatmap</div>
            </div>
            <div className="text-[10px] font-mono text-[#888]">Avg {Math.round(data.swarm.summary.avgGpuUtil)}%</div>
          </div>
          <div className="grid gap-2" style={{ gridTemplateColumns: `repeat(${HEATMAP_COLS}, minmax(0, 1fr))` }}>
            {heatmapCells.map((cell) => (
              <div
                key={cell.id}
                className="rounded-lg p-2 border border-white/5 text-[10px] font-mono"
                style={{ background: heatColor(cell.value) }}
              >
                <div className="text-[11px] font-semibold text-white/90 truncate">{cell.label}</div>
                <div className="flex items-center justify-between mt-1">
                  <span>{Math.round(cell.value)}%</span>
                  <span className="uppercase text-[9px] text-white/70">{cell.status}</span>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="rounded-xl border border-white/5 bg-[#0b0b14] p-4">
          <div className="flex items-center justify-between mb-3">
            <div>
              <div className="text-[9px] font-mono uppercase tracking-wider text-[#666]">Storage Layer</div>
              <div className="text-sm font-semibold text-[#8b5cf6]">Utilization Graph</div>
            </div>
            <div className="text-[10px] font-mono text-[#888]">
              {storageStats.usedPct.toFixed(1)}% used
            </div>
          </div>

          <div className="h-32 w-full rounded-lg border border-white/5 bg-[#07070d] p-3">
            <svg viewBox="0 0 100 100" className="w-full h-full">
              <path
                d={sparkPath}
                fill="none"
                stroke="#8b5cf6"
                strokeWidth="2"
              />
              <line x1="0" y1="100" x2="100" y2="100" stroke="rgba(255,255,255,0.08)" strokeWidth="1" />
            </svg>
          </div>

          <div className="mt-3 flex items-center justify-between text-[10px] font-mono text-[#aaa]">
            <span>Used: {formatBytes(storageStats.used)}</span>
            <span>Cap: {formatBytes(storageStats.capacity)}</span>
          </div>
        </section>

        <section className="rounded-xl border border-white/5 bg-[#0b0b14] p-4 col-span-2">
          <div className="flex items-center justify-between mb-3">
            <div>
              <div className="text-[9px] font-mono uppercase tracking-wider text-[#666]">Signal Lanes</div>
              <div className="text-sm font-semibold text-[#00b4ff]">Network + IDE Snapshot</div>
            </div>
            <div className="text-[10px] font-mono text-[#888]">Queue {data.swarm.summary.queuedJobs} jobs</div>
          </div>
          <div className="grid grid-cols-3 gap-3 text-[10px] font-mono">
            <div className="rounded-lg border border-white/5 p-3 bg-[#07070d]">
              <div className="text-[#666] uppercase text-[9px] mb-1">Peers Online</div>
              <div className="text-xl text-[#4caf50]">
                {data.network.peers.filter((peer) => peer.status === "connected").length}
              </div>
              <div className="text-[#888]">RPC Calls {data.network.rpcEndpoints.reduce((sum, ep) => sum + ep.calls, 0)}</div>
            </div>
            <div className="rounded-lg border border-white/5 p-3 bg-[#07070d]">
              <div className="text-[#666] uppercase text-[9px] mb-1">Build Queue</div>
              <div className="text-xl text-[#ff4488]">
                {data.ide.builds.filter((b) => b.status === "building").length}
              </div>
              <div className="text-[#888]">Contracts {data.ide.contracts.length}</div>
            </div>
            <div className="rounded-lg border border-white/5 p-3 bg-[#07070d]">
              <div className="text-[#666] uppercase text-[9px] mb-1">Recent Proofs</div>
              <div className="text-xl text-[#ff8c42]">{data.storage.proofs.length}</div>
              <div className="text-[#888]">Pinned {data.storage.pins.length}</div>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
};

export default LiveTelemetryPanel;
