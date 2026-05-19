import React from "react";
import { useDataIntegrity, DataSourceState, isAlertState, stateBadgeLabel } from "../services/dataIntegrity";

/**
 * DemoDataBanner
 *
 * Renders a prominent alert bar whenever any page has fallen back to demo/mock
 * data instead of live chain state.  This is a REQUIRED safety control — not a
 * cosmetic feature.  It must never be hidden, dismissed without acknowledgment,
 * or removed in production builds.
 *
 * Invariant referenced: FRONTEND-TELEMETRY-001, FRONTEND-TELEMETRY-002
 */
export function DemoDataBanner() {
  const { state, demoPages } = useDataIntegrity();

  if (!isAlertState(state)) return null;

  const isDemo = state === DataSourceState.DEMO_FALLBACK;
  const isReorg = state === DataSourceState.REORG_DETECTED;
  const isInconsistent = state === DataSourceState.INCONSISTENT;

  const colors = {
    bg: isDemo ? "#7c1c1c" : isReorg ? "#7c4a1c" : "#1c3a7c",
    border: isDemo ? "#ef4444" : isReorg ? "#f97316" : "#3b82f6",
    text: "#fff",
  } as const;

  return (
    <div
      role="alert"
      aria-live="assertive"
      data-testid="demo-data-banner"
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        zIndex: 9999,
        background: colors.bg,
        borderBottom: `2px solid ${colors.border}`,
        color: colors.text,
        padding: "8px 20px",
        display: "flex",
        alignItems: "center",
        gap: "12px",
        fontSize: "13px",
        fontFamily: "monospace",
      }}
    >
      <span style={{ fontWeight: 700, letterSpacing: "0.05em" }}>
        {stateBadgeLabel(state)}
      </span>

      {isDemo && (
        <span>
          Chain data unavailable — displaying static demo data.{" "}
          {demoPages.length > 0 && (
            <span style={{ opacity: 0.8 }}>
              Affected: {demoPages.join(", ")}.
            </span>
          )}{" "}
          <strong>Do not use for trading decisions.</strong>
        </span>
      )}

      {isReorg && (
        <span>
          Chain reorg detected — rolling back displayed state. Do not submit
          transactions until this clears.
        </span>
      )}

      {isInconsistent && (
        <span>
          RPC nodes disagree on chain state. Interactions disabled until quorum
          is restored.
        </span>
      )}

      {state === DataSourceState.UNAVAILABLE && (
        <span>
          All RPC endpoints unreachable. Chain data unavailable.
        </span>
      )}

      <span style={{ marginLeft: "auto", opacity: 0.6, fontSize: "11px" }}>
        {new Date().toISOString()}
      </span>
    </div>
  );
}
