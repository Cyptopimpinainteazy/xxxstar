import React, { useCallback, useEffect, useState } from "react";
import {
  DiskMetrics,
  EVENT_NODE_STATUS,
  EVENT_SYSTEM_METRICS,
  NodeStatusData,
  SystemMetricsData,
  fetchNodeStatus,
  fetchSystemMetrics,
  formatBytes,
  formatPercent,
  isTauri,
  subscribeNodeStatus,
  subscribeSystemMetrics,
} from "../lib/telemetry";

type ConnectionState =
  | { type: "connecting" }
  | { type: "live" }
  | { type: "offline"; reason: string };

/**
 * Live operator telemetry for the X3 Atomic Star OS shell.
 *
 * This panel is NOT fed by mock generators — it renders the real snapshots
 * the Rust backend publishes over `os:node_status` / `os:system_metrics`
 * (sysinfo CPU/memory/disk + node JSON-RPC peer state). If no Tauri host is
 * present (e.g. plain browser dev) it shows an explicit offline state instead
 * of inventing values.
 */
export function LiveTelemetryPanel() {
  const [metrics, setMetrics] = useState<SystemMetricsData | null>(null);
  const [node, setNode] = useState<NodeStatusData | null>(null);
  const [conn, setConn] = useState<ConnectionState>({ type: "connecting" });

  useEffect(() => {
    if (!isTauri()) {
      setConn({ type: "offline", reason: "not running inside Tauri (browser preview)" });
      return;
    }

    let disposed = false;
    const unlisteners: (() => void)[] = [];

    (async () => {
      // Seed with the current snapshots, then keep listening for pushes.
      try {
        const [m, n] = await Promise.all([fetchSystemMetrics(), fetchNodeStatus()]);
        if (disposed) return;
        setMetrics(m);
        setNode(n);
        setConn({ type: "live" });
      } catch {
        if (disposed) return;
        setConn({ type: "offline", reason: "backend metrics not reachable" });
      }

      try {
        const u1 = await subscribeSystemMetrics((m) => {
          setMetrics(m);
          setConn({ type: "live" });
        });
        const u2 = await subscribeNodeStatus((n) => {
          setNode(n);
          setConn({ type: "live" });
        });
        if (!disposed) {
          unlisteners.push(u1, u2);
        } else {
          u1();
          u2();
        }
      } catch (err) {
        if (!disposed) {
          setConn({
            type: "offline",
            reason: err instanceof Error ? err.message : String(err),
          });
        }
      }
    })();

    return () => {
      disposed = true;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  return (
    <div style={{ padding: 24, fontFamily: "Inter, sans-serif", lineHeight: 1.5 }}>
      <h1>Live Operator Telemetry</h1>
      <p>
        Real CPU / memory / storage from <code>sysinfo</code> and node state from the
        local RPC endpoint — streamed via Tauri events every ~5s. No mock data.
      </p>

      {conn.type === "connecting" && <p>Connecting to backend telemetry…</p>}
      {conn.type === "offline" && (
        <p style={{ color: "#b45309" }}>
          Backend offline: {conn.reason}. Data will appear once the OS shell backend is
          running (node + swarm reachable).
        </p>
      )}

      {metrics && (
        <div style={{ display: "grid", gap: 16, gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))" }}>
          <GaugeCard title={`CPU ${formatPercent(metrics.cpu.usagePercent)}`} percent={metrics.cpu.usagePercent}>
            <div>
              {metrics.cpu.cores} cores · {metrics.cpu.frequency} MHz
            </div>
            <div style={{ fontSize: 12, color: "#555" }}>Last update: {metrics.updatedAt}</div>
          </GaugeCard>

          <GaugeCard title={`Memory ${formatPercent(metrics.memory.usagePercent)}`} percent={metrics.memory.usagePercent}>
            <div>
              {formatBytes(metrics.memory.used)} used / {formatBytes(metrics.memory.total)} total
            </div>
            <div style={{ fontSize: 12, color: "#555" }}>Last update: {metrics.updatedAt}</div>
          </GaugeCard>

          <StorageCard disks={metrics.disk} />
          <NodeCard node={node} />
          <Button refresh={() => refreshOnce(setMetrics, setNode, setConn)} />
        </div>
      )}

      {!metrics && (
        <p style={{ marginTop: 16, color: "#6b7280" }}>
          Waiting for the first system-metrics snapshot from the OS shell backend…
        </p>
      )}
    </div>
  );
}

async function refreshOnce(
  setMetrics: React.Dispatch<React.SetStateAction<SystemMetricsData | null>>,
  setNode: React.Dispatch<React.SetStateAction<NodeStatusData | null>>,
  setConn: React.Dispatch<React.SetStateAction<ConnectionState>>,
) {
  if (!isTauri()) return;
  try {
    const [m, n] = await Promise.all([fetchSystemMetrics(), fetchNodeStatus()]);
    setMetrics(m);
    setNode(n);
    setConn({ type: "live" });
  } catch {
    setConn({ type: "offline", reason: "backend metrics not reachable" });
  }
}

function Button({ refresh }: { refresh: () => void }) {
  return (
    <div style={{ padding: 16, border: "1px solid #ddd", borderRadius: 12 }}>
      <h3>Controls</h3>
      <button onClick={refresh} style={{ padding: "8px 14px" }}>
        Refresh now
      </button>
      <p style={{ fontSize: 12, color: "#555", marginTop: 8 }}>
        Panels update automatically every ~5s from the backend event stream.
      </p>
    </div>
  );
}

function GaugeCard({
  title,
  percent,
  children,
}: {
  title: string;
  percent: number;
  children: React.ReactNode;
}) {
  const clamped = Math.max(0, Math.min(100, percent));
  return (
    <div style={{ padding: 16, border: "1px solid #ddd", borderRadius: 12 }}>
      <h3 style={{ marginTop: 0 }}>{title}</h3>
      <svg width="100%" height={18} viewBox="0 0 240 18" preserveAspectRatio="none">
        <rect x={0} y={0} width={240} height={18} rx={9} fill="#ececec" />
        <rect
          x={0}
          y={0}
          width={(clamped / 100) * 240}
          height={18}
          rx={9}
          fill={clamped > 90 ? "#dc2626" : clamped > 70 ? "#f59e0b" : "#16a34a"}
        />
      </svg>
      {children}
    </div>
  );
}

function StorageCard({ disks }: { disks: DiskMetrics[] }) {
  return (
    <div style={{ padding: 16, border: "1px solid #ddd", borderRadius: 12 }}>
      <h3 style={{ marginTop: 0 }}>Storage</h3>
      {disks.length === 0 && <p>No disk reported by sysinfo.</p>}
      {disks.map((d) => (
        <div key={d.name} style={{ marginBottom: 12 }}>
          <GaugeRow label={d.name} percent={d.usagePercent} detail={`${formatBytes(d.used)} / ${formatBytes(d.total)}`} />
        </div>
      ))}
    </div>
  );
}

function GaugeRow({ label, percent, detail }: { label: string; percent: number; detail: string }) {
  const clamped = Math.max(0, Math.min(100, percent || 0));
  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 13 }}>
        <strong>{label}</strong>
        <span>{formatPercent(percent || 0)}</span>
      </div>
      <svg width="100%" height={10} viewBox="0 0 240 10" preserveAspectRatio="none" style={{ margin: "2px 0" }}>
        <rect x={0} y={0} width={240} height={10} rx={5} fill="#ececec" />
        <rect
          x={0}
          y={0}
          width={(clamped / 100) * 240}
          height={10}
          rx={5}
          fill={clamped > 90 ? "#dc2626" : clamped > 70 ? "#f59e0b" : "#3b82f6"}
        />
      </svg>
      <div style={{ fontSize: 12, color: "#555" }}>{detail}</div>
    </div>
  );
}

function NodeCard({ node }: { node: NodeStatusData | null }) {
  return (
    <div style={{ padding: 16, border: "1px solid #ddd", borderRadius: 12 }}>
      <h3 style={{ marginTop: 0 }}>Node Status</h3>
      {node ? (
        <div>
          <p>
            Status:{" "}
            <span style={{ color: node.running ? "#16a34a" : "#b45309" }}>
              {node.running ? "running" : "offline / syncing"}
            </span>
          </p>
          <p>Peers: {node.peerCount}</p>
          <p>Block height: {node.blockHeight}</p>
          {node.pid !== null && <p>PID: {node.pid}</p>}
          <p style={{ fontSize: 12, color: "#555" }}>Last check: {node.updatedAt}</p>
        </div>
      ) : (
        <p style={{ color: "#6b7280" }}>No node status received yet.</p>
      )}
      <p style={{ fontSize: 12, color: "#555", marginTop: 8 }}>
        Polled from the local chain JSON-RPC (127.0.0.1:9933) by the Rust backend.
      </p>
    </div>
  );
}

/* ——————— end file ——————— */
