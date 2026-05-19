import React, { useEffect, useRef } from "react";
import { useNetworkControl } from "@/hooks/useNetworkControl";
import { PanelError, PanelLoading } from "@/components/panels/PanelStatus";

function formatBytes(b: number): string {
  if (b < 1024) return `${b}B`;
  if (b < 1048576) return `${(b / 1024).toFixed(1)}KB`;
  return `${(b / 1048576).toFixed(1)}MB`;
}

const Stat: React.FC<{ label: string; value: string; color: string }> = ({ label, value, color }) => (
  <div className="text-center">
    <div className="text-[9px] font-mono uppercase tracking-wider text-[#666]">{label}</div>
    <div className="text-sm font-bold font-mono" style={{ color }}>{value}</div>
  </div>
);

const NetworkPanel: React.FC = () => {
  const { data, loading, error } = useNetworkControl();
  const logRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!logRef.current) return;
    logRef.current.scrollTo({ top: logRef.current.scrollHeight, behavior: "smooth" });
  }, [data?.logs]);

  if (error) return <PanelError message={error} />;
  if (loading || !data) return <PanelLoading label="Listening for peer metrics…" />;

  const connectedPeers = data.peers.filter((p) => p.status === "connected").length;
  const avgLatency = connectedPeers
    ? Math.round(data.peers.reduce((s, p) => s + p.latencyMs, 0) / connectedPeers)
    : 0;
  const totalCalls = data.rpcEndpoints.reduce((s, e) => s + e.calls, 0);

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-[#e0e0e0] overflow-hidden">
      <div className="flex items-center gap-4 px-4 py-3 border-b border-[#00b4ff]/15 bg-[#0d0d14]">
        <Stat label="PEERS" value={`${connectedPeers}/${data.peers.length}`} color="#4caf50" />
        <Stat label="RPC CALLS" value={totalCalls.toLocaleString()} color="#00b4ff" />
        <Stat label="AVG LATENCY" value={`${avgLatency}ms`} color="#ff8c42" />
        <div className="flex-1" />
        <div className="flex gap-1">
          {(["peers", "rpc", "log"] as const).map((tab) => (
            <span key={tab} className="px-2 py-1 text-[10px] font-mono uppercase text-[#666]">
              {tab}
            </span>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-2 scrollbar-thin scrollbar-thumb-[#00b4ff]/20">
        <div className="space-y-1.5">
          {data.peers.map((peer) => (
            <div
              key={peer.id}
              className="flex items-center gap-3 px-3 py-2 rounded border border-white/5 bg-white/[0.02] text-[11px] font-mono hover:bg-white/[0.04] transition"
            >
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{
                  background:
                    peer.status === "connected"
                      ? "#4caf50"
                      : peer.status === "stale"
                      ? "#ff9800"
                      : "#555",
                  boxShadow: peer.status === "connected" ? "0 0 6px #4caf5088" : "none",
                }}
              />
              <span className="w-44 truncate text-[#ff8c42]">{peer.addr}</span>
              <span className="w-8 text-[#666] uppercase">{peer.protocol}</span>
              <span className="w-14 text-right" style={{
                color: peer.latencyMs > 100 ? "#ef5350" : peer.latencyMs > 50 ? "#ff9800" : "#4caf50",
              }}>
                {peer.latencyMs > 0 ? `${peer.latencyMs}ms` : "—"}
              </span>
              <span className="w-16 text-right text-[#666]">↑{formatBytes(peer.bytesSent)}</span>
              <span className="w-16 text-right text-[#666]">↓{formatBytes(peer.bytesReceived)}</span>
            </div>
          ))}
        </div>

        <div className="space-y-2 pt-4">
          {data.rpcEndpoints.map((ep) => (
            <div
              key={ep.name}
              className="p-3 rounded-lg border border-white/5 bg-white/[0.02]"
            >
              <div className="flex items-center gap-2 mb-2">
                <span
                  className="w-2 h-2 rounded-full"
                  style={{
                    background:
                      ep.status === "active"
                        ? "#4caf50"
                        : ep.status === "degraded"
                        ? "#ff9800"
                        : "#ef5350",
                  }}
                />
                <span className="font-bold text-xs text-[#00b4ff]">{ep.name}</span>
                <span className="text-[10px] font-mono text-[#666] ml-auto">{ep.url}</span>
              </div>
              <div className="flex gap-6 text-[10px] font-mono text-[#888]">
                <span>Calls: <span className="text-[#e0e0e0]">{ep.calls.toLocaleString()}</span></span>
                <span>Avg: <span style={{ color: ep.avgMs > 100 ? "#ef5350" : "#4caf50" }}>{ep.avgMs}ms</span></span>
                <span className="uppercase">{ep.status}</span>
              </div>
            </div>
          ))}
        </div>

        <div
          ref={logRef}
          className="mt-4 space-y-0.5 font-mono text-[10px]"
        >
          {data.logs.map((log, i) => (
            <div key={i} className="flex gap-2 py-0.5">
              <span className="text-[#555] shrink-0">{log.ts}</span>
              <span
                className="shrink-0 w-10 text-right"
                style={{
                  color: log.level === "error" ? "#ef5350" : log.level === "warn" ? "#ff9800" : "#4caf50",
                }}
              >
                {log.level.toUpperCase()}
              </span>
              <span className="text-[#ccc]">{log.message}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default NetworkPanel;
