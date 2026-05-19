export type FeatureMode =
  | "LIVE_TESTNET"
  | "GUARDED_TESTNET"
  | "SIM_TESTNET"
  | "DISABLED_BLOCKED";

export interface FeatureStatus {
  id: string;
  name: string;
  mode: FeatureMode;
  tauriApp: string;
  proofReport: string;
  requiredTests: string[];
  risk: string;
}

export interface DesktopReadinessStatus {
  currentVersion: string;
  lastReviewed: string;
  sourceOfTruth: string;
  status: "guarded" | "live" | "blocked";
  summary: string;
  gaps: string[];
}

export const DESKTOP_READINESS_STATUS: DesktopReadinessStatus = {
  currentVersion: "0.1.0",
  lastReviewed: "2026-05-05",
  sourceOfTruth: "FEATURE_REGISTRY.toml + TESTNET_FEATURE_FLAGS.toml",
  status: "guarded",
  summary:
    "Desktop shell is wired for local Tauri panels and selected internal apps, but release status remains guarded until builds, app registry coverage, health checks, and e2e tests are proven in CI.",
  gaps: [
    "Desktop build/test scripts must run real checks in CI.",
    "Backend app registry currently returns an empty list, so the frontend falls back to a static registry.",
    "Several external app launch targets depend on local services that are not health-gated before launch.",
    "Update/install flow is not wired to a signed release channel yet.",
  ],
};

export const FEATURE_STATUSES: FeatureStatus[] = [
  {
    id: "atomic_kernel",
    name: "Atomic Kernel",
    mode: "LIVE_TESTNET",
    tauriApp: "AtomicLock",
    proofReport: "reports/six_route_invariants.md",
    requiredTests: [
      "canonical_supply_invariant_holds",
      "represented_total_equals_canonical_supply",
      "pending_supply_returns_to_zero",
    ],
    risk: "Runtime/token accounting path; keep claims tied to invariant tests.",
  },
  {
    id: "atomic_router",
    name: "Atomic Router",
    mode: "LIVE_TESTNET",
    tauriApp: "Gateway",
    proofReport: "reports/six_route_invariants.md",
    requiredTests: [
      "route_canonical_supply_invariant",
      "duplicate_completion_rejected",
      "refund_after_completion_rejected",
    ],
    risk: "Cross-VM settlement path; needs replay and refund failure coverage.",
  },
  {
    id: "atomic_gateway",
    name: "Atomic Gateway",
    mode: "GUARDED_TESTNET",
    tauriApp: "Gateway",
    proofReport: "reports/btc_gateway_report.md",
    requiredTests: ["audit_gate_enabled", "revoke_disables_gateway"],
    risk: "External gateway enablement remains guarded behind audit/revoke gates.",
  },
  {
    id: "btc_fortress_gateway",
    name: "BTC Fortress Gateway",
    mode: "SIM_TESTNET",
    tauriApp: "BTCGateway",
    proofReport: "reports/btc_gateway_report.md",
    requiredTests: [
      "btc_regtest_deposit_detected",
      "btc_requires_confirmations",
      "btc_mainnet_disabled_by_feature_flag",
    ],
    risk: "BTC mainnet remains disabled; desktop must present this as simulation/testnet only.",
  },
  {
    id: "axe",
    name: "AXE DEX",
    mode: "GUARDED_TESTNET",
    tauriApp: "AXEForge",
    proofReport: "reports/testnet_readiness_report.md",
    requiredTests: ["axe_create_pool", "axe_swap", "axe_fee_accounting"],
    risk: "DEX accounting path; launch UI should stay guarded until pool/swap proofs are refreshed.",
  },
  {
    id: "tauri_os",
    name: "Tauri OS",
    mode: "GUARDED_TESTNET",
    tauriApp: "AtomicConsole",
    proofReport: "reports/tauri_e2e_report.md",
    requiredTests: ["dead_buttons_report", "tauri_wiring_report"],
    risk: "Desktop release blocker until dead buttons and Tauri wiring are proven.",
  },
];

export function summarizeFeatureModes(features: FeatureStatus[] = FEATURE_STATUSES) {
  return features.reduce(
    (counts, feature) => {
      counts[feature.mode] += 1;
      return counts;
    },
    {
      LIVE_TESTNET: 0,
      GUARDED_TESTNET: 0,
      SIM_TESTNET: 0,
      DISABLED_BLOCKED: 0,
    } satisfies Record<FeatureMode, number>,
  );
}