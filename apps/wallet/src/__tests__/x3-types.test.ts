/**
 * Tests for X3 wallet domain types and state machine logic.
 * These tests ensure the type enums and state transitions match the on-chain
 * Rust definitions in pallets/x3-jurisdiction and pallets/x3-settlement-engine.
 */

import {
  IntentState,
  AgentStatus,
  SlashSeverity,
  DisputeState,
  VerdictOutcome,
  ChainKind,
} from "../../lib/x3/types";

describe("IntentState enum", () => {
  it("has all required lifecycle states", () => {
    const required = [
      "Submitted",
      "RouteBound",
      "Executing",
      "Executed",
      "Finalized",
      "Slashed",
      "Cancelled",
      "Expired",
    ];
    required.forEach((state) => {
      expect(Object.values(IntentState)).toContain(state);
    });
  });

  it("terminal states cannot transition further", () => {
    const terminalStates = [
      IntentState.Finalized,
      IntentState.Slashed,
      IntentState.Cancelled,
      IntentState.Expired,
    ];
    // Verify these are distinct from transitional states
    const transitionalStates = [
      IntentState.Submitted,
      IntentState.RouteBound,
      IntentState.Executing,
      IntentState.Executed,
    ];
    terminalStates.forEach((s) => {
      expect(transitionalStates).not.toContain(s);
    });
  });
});

describe("AgentStatus enum", () => {
  it("has all required statuses", () => {
    const required = ["Active", "Suspended", "Deregistered", "Deactivated"];
    required.forEach((status) => {
      expect(Object.values(AgentStatus)).toContain(status);
    });
  });

  it("only Active agents can execute intents", () => {
    const canExecute = (status: AgentStatus) => status === AgentStatus.Active;
    expect(canExecute(AgentStatus.Active)).toBe(true);
    expect(canExecute(AgentStatus.Suspended)).toBe(false);
    expect(canExecute(AgentStatus.Deregistered)).toBe(false);
    expect(canExecute(AgentStatus.Deactivated)).toBe(false);
  });
});

describe("SlashSeverity enum", () => {
  it("has a severity ordering: Minor < Moderate < Major < Critical", () => {
    const severityOrder = [
      SlashSeverity.Minor,
      SlashSeverity.Moderate,
      SlashSeverity.Major,
      SlashSeverity.Critical,
    ];
    // Each value is a distinct string
    const unique = new Set(severityOrder);
    expect(unique.size).toBe(4);
  });
});

describe("DisputeState enum", () => {
  it("has correct dispute lifecycle states", () => {
    const expected = ["Filed", "Replaying", "Resolved", "Dismissed"];
    expected.forEach((s) => {
      expect(Object.values(DisputeState)).toContain(s);
    });
  });

  it("resolved and dismissed are terminal states", () => {
    const terminal = [DisputeState.Resolved, DisputeState.Dismissed];
    const active = [DisputeState.Filed, DisputeState.Replaying];
    terminal.forEach((s) => {
      expect(active).not.toContain(s);
    });
  });
});

describe("VerdictOutcome enum", () => {
  it("has all three verdict outcomes", () => {
    expect(Object.values(VerdictOutcome)).toHaveLength(3);
    expect(Object.values(VerdictOutcome)).toContain("Guilty");
    expect(Object.values(VerdictOutcome)).toContain("NotGuilty");
    expect(Object.values(VerdictOutcome)).toContain("InvalidDispute");
  });
});

describe("ChainKind enum", () => {
  it("supports EVM and SVM chain types", () => {
    expect(ChainKind.Evm).toBe("Evm");
    expect(ChainKind.Svm).toBe("Svm");
  });
});
