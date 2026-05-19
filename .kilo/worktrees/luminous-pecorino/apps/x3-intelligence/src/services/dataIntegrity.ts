/**
 * Data Integrity Enforcement — X3 Intelligence Dashboard
 *
 * RULE: If real chain data is unavailable, the UI must NEVER silently fall back
 * to demo/mock data. Instead, it raises a visible alert and logs telemetry.
 *
 * This module implements the DataSourceState machine described in the pre-mainnet
 * architecture. All pages must call `reportDemoFallback()` whenever they use
 * demo/static data instead of live chain data.
 */

// ── State Machine ──────────────────────────────────────────────────────────────

export const enum DataSourceState {
  /** ≥2/3 RPC quorum agreed on current state root. */
  LIVE_VERIFIED = "LIVE_VERIFIED",
  /** Connected to at least one node but quorum not yet established. */
  LIVE_UNVERIFIED = "LIVE_UNVERIFIED",
  /** RPC reachable but returning stale / inconsistent data. */
  DEGRADED = "DEGRADED",
  /** Reorg detected — UI rolling back. */
  REORG_DETECTED = "REORG_DETECTED",
  /** Multiple nodes disagree on state root. */
  INCONSISTENT = "INCONSISTENT",
  /** No RPC reachable — silently showing demo/static data. */
  DEMO_FALLBACK = "DEMO_FALLBACK",
  /** All RPC nodes unreachable — interactions disabled. */
  UNAVAILABLE = "UNAVAILABLE",
}

// ── Telemetry Event ────────────────────────────────────────────────────────────

interface IntegrityEvent {
  timestamp: number;
  state: DataSourceState;
  page: string;
  reason: string;
}

// ── Manager ───────────────────────────────────────────────────────────────────

class DataIntegrityManager {
  private _state: DataSourceState = DataSourceState.LIVE_UNVERIFIED;
  private _listeners: Set<() => void> = new Set();
  private _eventLog: IntegrityEvent[] = [];
  private _demoPages: Set<string> = new Set();
  private _suppressInDev: boolean = import.meta.env.DEV === true;

  get state(): DataSourceState {
    return this._state;
  }

  get isDemoActive(): boolean {
    return this._demoPages.size > 0;
  }

  get demoPages(): string[] {
    return Array.from(this._demoPages);
  }

  get eventLog(): IntegrityEvent[] {
    return [...this._eventLog];
  }

  /**
   * Call this in every catch block that falls back to demo/static data.
   *
   * @param page    - Component or page name ("FloorDashboard", "IntentsPage", …)
   * @param reason  - Human-readable explanation ("API error: ECONNREFUSED")
   */
  reportDemoFallback(page: string, reason: string): void {
    this._demoPages.add(page);
    this._transition(DataSourceState.DEMO_FALLBACK, page, reason);

    // In production, escalate to telemetry/monitoring.
    if (!this._suppressInDev) {
      this._emitTelemetry({ state: DataSourceState.DEMO_FALLBACK, page, reason });
    } else {
      // Dev: log to console so developers see it clearly.
      console.warn(
        `[DataIntegrity] ⚠ DEMO FALLBACK — ${page}: ${reason}. ` +
          "This must be a production incident if it fires in mainnet.",
      );
    }
  }

  /**
   * Call when a page successfully loads live chain data, clearing its demo flag.
   */
  reportLive(page: string): void {
    this._demoPages.delete(page);
    if (this._demoPages.size === 0) {
      this._transition(DataSourceState.LIVE_VERIFIED, page, "all pages live");
    }
  }

  /**
   * Call when a reorg is detected.
   */
  reportReorg(page: string, blockHeight: number): void {
    this._transition(
      DataSourceState.REORG_DETECTED,
      page,
      `reorg at block ${blockHeight}`,
    );
  }

  /**
   * Call when RPC quorum disagrees.
   */
  reportInconsistency(page: string, detail: string): void {
    this._transition(DataSourceState.INCONSISTENT, page, detail);
  }

  subscribe(listener: () => void): () => void {
    this._listeners.add(listener);
    return () => this._listeners.delete(listener);
  }

  private _transition(
    next: DataSourceState,
    page: string,
    reason: string,
  ): void {
    if (this._state === next) return;
    this._state = next;
    const event: IntegrityEvent = {
      timestamp: Date.now(),
      state: next,
      page,
      reason,
    };
    this._eventLog.push(event);
    // Keep log bounded
    if (this._eventLog.length > 200) this._eventLog.shift();
    this._listeners.forEach((l) => l());
  }

  private _emitTelemetry(event: Omit<IntegrityEvent, "timestamp">): void {
    // Production: POST to monitoring endpoint.
    // Increment counter: frontend_demo_fallback_total
    const payload = { ...event, timestamp: Date.now() };
    try {
      navigator.sendBeacon?.(
        "/api/v1/telemetry/integrity",
        JSON.stringify(payload),
      );
    } catch {
      // Best-effort — never let telemetry throw.
    }
  }
}

// Singleton
export const dataIntegrity = new DataIntegrityManager();

// ── React hook ────────────────────────────────────────────────────────────────

import { useEffect, useState } from "react";

export function useDataIntegrity() {
  const [state, setState] = useState<DataSourceState>(dataIntegrity.state);
  const [demoPages, setDemoPages] = useState<string[]>(
    dataIntegrity.demoPages,
  );

  useEffect(() => {
    const unsub = dataIntegrity.subscribe(() => {
      setState(dataIntegrity.state);
      setDemoPages(dataIntegrity.demoPages);
    });
    return unsub;
  }, []);

  return { state, demoPages, isDemoActive: dataIntegrity.isDemoActive };
}

// ── Banner helpers ─────────────────────────────────────────────────────────────

export function isAlertState(state: DataSourceState): boolean {
  return (
    state === DataSourceState.DEMO_FALLBACK ||
    state === DataSourceState.INCONSISTENT ||
    state === DataSourceState.REORG_DETECTED ||
    state === DataSourceState.UNAVAILABLE
  );
}

export function stateBadgeLabel(state: DataSourceState): string {
  switch (state) {
    case DataSourceState.LIVE_VERIFIED:
      return "● LIVE";
    case DataSourceState.LIVE_UNVERIFIED:
      return "○ CONNECTING";
    case DataSourceState.DEGRADED:
      return "⚠ DEGRADED";
    case DataSourceState.REORG_DETECTED:
      return "↺ REORG";
    case DataSourceState.INCONSISTENT:
      return "✗ INCONSISTENT";
    case DataSourceState.DEMO_FALLBACK:
      return "⚠ DEMO DATA";
    case DataSourceState.UNAVAILABLE:
      return "✗ UNAVAILABLE";
  }
}
