/**
 * AdminPanel — admin dashboard integrated into X3 Desktop window manager.
 *
 * Features:
 *   - System overview (hostname, kernel, uptime, platform)
 *   - Service health checks with latency
 *   - Allowlisted system commands (no arbitrary shell execution)
 *   - Command output viewer
 *
 * Matches the existing glass-panel / monospace dark theme.
 */
import React, { useState, useEffect, useCallback } from "react";
import {
  runSystemCommand,
  listAdminCommands,
  getSystemOverview,
  checkServices,
  type AllowedCommand,
  type ServiceHealth,
  type AdminSystemOverview,
} from "@/services/adminService";

type Tab = "overview" | "services" | "commands";

const AdminPanel: React.FC = () => {
  const [tab, setTab] = useState<Tab>("overview");
  const [overview, setOverview] = useState<AdminSystemOverview | null>(null);
  const [services, setServices] = useState<ServiceHealth[]>([]);
  const [commands, setCommands] = useState<AllowedCommand[]>([]);
  const [cmdOutput, setCmdOutput] = useState<string>("");
  const [runningCmd, setRunningCmd] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastUpdate, setLastUpdate] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [ov, svc, cmds] = await Promise.all([
        getSystemOverview().catch(() => null),
        checkServices().catch(() => []),
        listAdminCommands().catch(() => []),
      ]);
      if (ov) setOverview(ov);
      setServices(svc);
      setCommands(cmds);
      setLastUpdate(new Date().toLocaleTimeString());
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const iv = setInterval(refresh, 10_000);
    return () => clearInterval(iv);
  }, [refresh]);

  const executeCommand = async (cmdId: string) => {
    setRunningCmd(cmdId);
    setCmdOutput("");
    try {
      const result = await runSystemCommand(cmdId);
      setCmdOutput(`$ ${cmdId}\n\n${result}`);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setCmdOutput(`Error: ${msg}`);
    } finally {
      setRunningCmd(null);
    }
  };

  const healthyCount = services.filter((s) => s.healthy).length;
  const totalCount = services.length;

  const tabs: { key: Tab; label: string; icon: string }[] = [
    { key: "overview", label: "Overview", icon: "📊" },
    { key: "services", label: "Services", icon: "🏥" },
    { key: "commands", label: "Commands", icon: "⚡" },
  ];

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        background: "#0a0e17",
        color: "#e0e0e0",
        fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
        fontSize: "0.8rem",
        overflow: "hidden",
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "10px 14px",
          borderBottom: "1px solid #1a1f2e",
          flexShrink: 0,
        }}
      >
        <span style={{ fontSize: "1.1rem" }}>🛡️</span>
        <span style={{ fontWeight: 700, fontSize: "0.95rem" }}>Admin Dashboard</span>
        <div style={{ flex: 1 }} />
        {totalCount > 0 && (
          <span
            style={{
              fontSize: "0.72rem",
              color: healthyCount === totalCount ? "#10b981" : "#f59e0b",
            }}
          >
            {healthyCount}/{totalCount} services
          </span>
        )}
        <span style={{ fontSize: "0.65rem", color: "#555", marginLeft: 4 }}>
          {lastUpdate}
        </span>
        <button
          onClick={refresh}
          style={{
            background: "transparent",
            border: "1px solid #2a2f3e",
            borderRadius: 6,
            padding: "3px 8px",
            color: "#999",
            cursor: "pointer",
            fontSize: "0.72rem",
            marginLeft: 4,
          }}
        >
          ↻ Refresh
        </button>
      </div>

      {/* Tabs */}
      <div
        style={{
          display: "flex",
          gap: 2,
          padding: "6px 14px",
          borderBottom: "1px solid #1a1f2e",
          flexShrink: 0,
        }}
      >
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
              padding: "5px 12px",
              borderRadius: 6,
              border: "none",
              cursor: "pointer",
              fontSize: "0.75rem",
              fontWeight: 600,
              background: tab === t.key ? "#1e293b" : "transparent",
              color: tab === t.key ? "#fff" : "#6b7280",
              transition: "all 0.15s",
            }}
          >
            <span>{t.icon}</span>
            {t.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div style={{ flex: 1, overflow: "auto", padding: 14 }}>
        {loading ? (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              height: "100%",
              color: "#666",
            }}
          >
            Loading admin data…
          </div>
        ) : (
          <>
            {tab === "overview" && <OverviewTab overview={overview} services={services} />}
            {tab === "services" && <ServicesTab services={services} onRefresh={refresh} />}
            {tab === "commands" && (
              <CommandsTab
                commands={commands}
                cmdOutput={cmdOutput}
                runningCmd={runningCmd}
                onExecute={executeCommand}
              />
            )}
          </>
        )}
      </div>
    </div>
  );
};

/* ── Overview Tab ─────────────────────────────────── */

const OverviewTab: React.FC<{
  overview: AdminSystemOverview | null;
  services: ServiceHealth[];
}> = ({ overview, services }) => {
  const healthyCount = services.filter((s) => s.healthy).length;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      {/* System Info Cards */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
          gap: 10,
        }}
      >
        <InfoCard icon="🖥️" label="Hostname" value={overview?.hostname ?? "—"} />
        <InfoCard icon="🐧" label="Kernel" value={overview?.kernel ?? "—"} />
        <InfoCard icon="⏱️" label="Uptime" value={overview?.uptime ?? "—"} />
        <InfoCard icon="🏗️" label="Platform" value={`${overview?.platform ?? "—"} / ${overview?.arch ?? "—"}`} />
      </div>

      {/* Service Summary */}
      <div style={cardStyle}>
        <div style={cardTitleStyle}>🏥 Service Health Summary</div>
        <div style={{ display: "flex", alignItems: "center", gap: 16, marginTop: 8 }}>
          <div>
            <span
              style={{
                fontSize: "2rem",
                fontWeight: 700,
                color: healthyCount === services.length ? "#10b981" : "#f59e0b",
              }}
            >
              {healthyCount}
            </span>
            <span style={{ fontSize: "0.8rem", color: "#6b7280" }}>
              {" "}/ {services.length} healthy
            </span>
          </div>
          <div style={{ flex: 1 }} />
          <div style={{ display: "flex", gap: 6 }}>
            {services.map((s) => (
              <div
                key={s.name}
                title={`${s.name} — ${s.healthy ? "OK" : "Down"} (${s.latency_ms}ms)`}
                style={{
                  width: 12,
                  height: 12,
                  borderRadius: "50%",
                  background: s.healthy ? "#10b981" : "#ef4444",
                  cursor: "default",
                }}
              />
            ))}
          </div>
        </div>
      </div>

      {/* Quick Actions */}
      <div style={cardStyle}>
        <div style={cardTitleStyle}>⚡ Quick Info</div>
        <div style={{ fontSize: "0.72rem", color: "#9ca3af", lineHeight: 1.8 }}>
          <div>Admin panel provides allowlisted system commands for safe operations.</div>
          <div>Service health probes HTTP endpoints on localhost every 10 seconds.</div>
          <div>All commands execute with the Tauri process's permissions — no shell injection.</div>
        </div>
      </div>
    </div>
  );
};

/* ── Services Tab ─────────────────────────────────── */

const ServicesTab: React.FC<{
  services: ServiceHealth[];
  onRefresh: () => void;
}> = ({ services }) => (
  <div style={cardStyle}>
    <div style={cardTitleStyle}>🏥 Infrastructure Services</div>
    <div
      style={{
        display: "flex",
        padding: "6px 0",
        color: "#555",
        fontWeight: 600,
        fontSize: "0.7rem",
        borderBottom: "1px solid #1f2937",
      }}
    >
      <span style={{ width: 28 }} />
      <span style={{ flex: 1 }}>Service</span>
      <span style={{ width: 60, textAlign: "center" }}>Port</span>
      <span style={{ width: 70, textAlign: "center" }}>Status</span>
      <span style={{ width: 80, textAlign: "center" }}>Latency</span>
      <span style={{ width: 50, textAlign: "center" }}>HTTP</span>
    </div>
    {services.length === 0 ? (
      <div style={{ padding: 20, textAlign: "center", color: "#555" }}>
        No service data available
      </div>
    ) : (
      services.map((s) => (
        <div
          key={s.name}
          style={{
            display: "flex",
            alignItems: "center",
            padding: "8px 0",
            borderBottom: "1px solid #111827",
          }}
        >
          <span
            style={{
              width: 28,
              display: "flex",
              justifyContent: "center",
            }}
          >
            <span
              style={{
                display: "inline-block",
                width: 8,
                height: 8,
                borderRadius: "50%",
                background: s.healthy ? "#10b981" : "#ef4444",
              }}
            />
          </span>
          <span style={{ flex: 1, fontWeight: 600, fontSize: "0.78rem" }}>{s.name}</span>
          <span
            style={{
              width: 60,
              textAlign: "center",
              color: "#6b7280",
              fontSize: "0.72rem",
              fontFamily: "monospace",
            }}
          >
            {s.port}
          </span>
          <span
            style={{
              width: 70,
              textAlign: "center",
              fontSize: "0.7rem",
              fontWeight: 600,
              color: s.healthy ? "#10b981" : "#ef4444",
            }}
          >
            {s.healthy ? "ONLINE" : "DOWN"}
          </span>
          <span
            style={{
              width: 80,
              textAlign: "center",
              fontSize: "0.72rem",
              color: s.latency_ms > 200 ? "#f59e0b" : "#9ca3af",
              fontFamily: "monospace",
            }}
          >
            {s.healthy ? `${s.latency_ms}ms` : "—"}
          </span>
          <span
            style={{
              width: 50,
              textAlign: "center",
              fontSize: "0.68rem",
              color: "#6b7280",
              fontFamily: "monospace",
            }}
          >
            {s.status_code > 0 ? s.status_code : "—"}
          </span>
        </div>
      ))
    )}
  </div>
);

/* ── Commands Tab ─────────────────────────────────── */

const CommandsTab: React.FC<{
  commands: AllowedCommand[];
  cmdOutput: string;
  runningCmd: string | null;
  onExecute: (cmdId: string) => void;
}> = ({ commands, cmdOutput, runningCmd, onExecute }) => (
  <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
    <div style={cardStyle}>
      <div style={cardTitleStyle}>⚡ System Commands</div>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 8,
          marginTop: 8,
        }}
      >
        {commands.map((cmd) => (
          <button
            key={cmd.id}
            onClick={() => onExecute(cmd.id)}
            disabled={runningCmd !== null}
            style={{
              background: runningCmd === cmd.id ? "#1e40af" : "#111827",
              color: runningCmd === cmd.id ? "#93c5fd" : "#d1d5db",
              border: "1px solid #2a2f3e",
              borderRadius: 8,
              padding: "6px 14px",
              fontSize: "0.72rem",
              fontWeight: 600,
              cursor: runningCmd !== null ? "not-allowed" : "pointer",
              opacity: runningCmd !== null && runningCmd !== cmd.id ? 0.5 : 1,
              transition: "all 0.15s",
              fontFamily: "monospace",
            }}
          >
            {runningCmd === cmd.id ? "⏳ Running…" : cmd.id}
          </button>
        ))}
      </div>
      {commands.length === 0 && (
        <div style={{ color: "#555", fontSize: "0.72rem", marginTop: 8 }}>
          No commands available (backend may not be connected)
        </div>
      )}
    </div>

    {/* Output */}
    <div style={cardStyle}>
      <div style={cardTitleStyle}>📋 Output</div>
      <pre
        style={{
          background: "#050810",
          color: "#d1d5db",
          padding: 12,
          borderRadius: 8,
          border: "1px solid #1a1f2e",
          minHeight: 160,
          maxHeight: 400,
          overflow: "auto",
          fontSize: "0.72rem",
          lineHeight: 1.6,
          whiteSpace: "pre-wrap",
          wordBreak: "break-all",
          fontFamily: "'JetBrains Mono', monospace",
          margin: 0,
          marginTop: 8,
        }}
      >
        {runningCmd ? "Running…" : cmdOutput || "Select a command to run."}
      </pre>
    </div>
  </div>
);

/* ── Shared Components ────────────────────────────── */

const InfoCard: React.FC<{
  icon: string;
  label: string;
  value: string;
}> = ({ icon, label, value }) => (
  <div style={cardStyle}>
    <div style={{ fontSize: "0.68rem", color: "#6b7280", textTransform: "uppercase", letterSpacing: 1 }}>
      {icon} {label}
    </div>
    <div
      style={{
        fontSize: "0.85rem",
        fontWeight: 600,
        color: "#e0e0e0",
        marginTop: 4,
        wordBreak: "break-all",
      }}
    >
      {value}
    </div>
  </div>
);

/* ── Styles ────────────────────────────────────────── */

const cardStyle: React.CSSProperties = {
  background: "#111827",
  border: "1px solid #1f2937",
  borderRadius: 10,
  padding: "12px 14px",
};

const cardTitleStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "#9ca3af",
  textTransform: "uppercase",
  letterSpacing: 1,
  fontWeight: 600,
};

export default AdminPanel;
