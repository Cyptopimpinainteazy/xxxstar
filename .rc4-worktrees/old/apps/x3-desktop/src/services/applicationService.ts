/**
 * Application service — launch, stop, and monitor application processes.
 *
 * Bridges the Zustand application store with the IPC service layer.
 */

import type { Application } from "@/types/application";
import type { LaunchResult } from "@/types/ipc";
import { ipcInvoke, AppError } from "./ipcService";
import { useApplicationStore } from "@/stores/applicationStore";

/* ── Default application registry ──────────────────────────── */

/**
 * Hard-coded registry of known applications from the monorepo.
 * This is the fallback when the backend doesn't provide a registry.
 */
export const DEFAULT_APPLICATIONS: Application[] = [
  /* ── Tier-1: Core X3 Panels ──────────────────────────────── */
  {
    id: "swarm-health",
    name: "Swarm Health",
    description: "GPU provider dashboard — live VRAM, compute, temps, SLA proofs",
    category: "service",
    icon: { type: "file", path: "/assets/icons/swarm-health.svg", color: "#ff6b35" },
    launchCommand: { type: "tauri", target: "launch_swarm_health" },
  },
  {
    id: "network-control",
    name: "Network Control",
    description: "RPC connections, peer nodes, agent dispatch, gossip channels",
    category: "service",
    icon: { type: "file", path: "/assets/icons/network.svg", color: "#00b4ff" },
    launchCommand: { type: "tauri", target: "launch_network_control" },
  },
  {
    id: "storage-manager",
    name: "Storage Manager",
    description: "CID pinning, content-addressed storage, proof submissions",
    category: "blockchain",
    icon: { type: "file", path: "/assets/icons/storage-manager.svg", color: "#8b5cf6" },
    launchCommand: { type: "tauri", target: "launch_storage_manager" },
  },
  {
    id: "dev-tools",
    name: "Dev Tools",
    description: "Build status, contract deploy, replay traces, X3-lang compile",
    category: "development",
    icon: { type: "file", path: "/assets/icons/devtools.svg", color: "#ff4488" },
    launchCommand: { type: "tauri", target: "launch_dev_tools" },
  },
  {
    id: "security-vault",
    name: "Security Vault",
    description: "Key custody, hardware attestation, governance signing",
    category: "security",
    icon: { type: "file", path: "/assets/icons/security.svg", color: "#ef5350" },
    launchCommand: { type: "tauri", target: "launch_security_vault" },
  },
  {
    id: "live-telemetry",
    name: "Live Telemetry",
    description: "Streaming GPU swarm heatmap + storage utilization graph",
    category: "analysis",
    icon: { type: "file", path: "/assets/icons/telemetry.svg", color: "#ff8c42" },
    launchCommand: { type: "tauri", target: "launch_swarm_health" },
  },
  {
    id: "blockchain-connector",
    name: "Blockchain Connector",
    description: "Enterprise multi-chain connector — 40+ networks, benchmarks, GPU testing, billing",
    category: "blockchain",
    icon: { type: "placeholder", category: "blockchain", color: "#ff6b35" },
    launchCommand: { type: "internal", target: "blockchain-connector" },
  },
  /* ── Existing URL/Tauri Applications ─────────────────────── */
  {
    id: "admin-command-center",
    name: "Command Center",
    description: "Administrative dashboard for node and network management",
    category: "utility",
    icon: { type: "file", path: "/assets/icons/warroom.svg", color: "#ff1744" },
    launchCommand: { type: "url", target: "http://localhost:3006" },
  },
  {
    id: "funding-automator",
    name: "Funding Automator",
    description: "Automated token distribution and funding workflows",
    category: "defi",
    icon: { type: "placeholder", category: "defi", color: "#ff8c42" },
    launchCommand: { type: "tauri", target: "launch_funding_automator" },
  },
  {
    id: "dev-dashboard",
    name: "Dev Dashboard",
    description: "Developer tools, contract deployment, and debugging",
    category: "development",
    icon: { type: "file", path: "/assets/icons/plugin.svg", color: "#ffd700" },
    launchCommand: { type: "url", target: "http://localhost:3008" },
  },
  {
    id: "3ai",
    name: "3AI Assistant",
    description: "AI assistant for blockchain operations and queries",
    category: "utility",
    icon: { type: "placeholder", category: "utility", color: "#66bb6a" },
    launchCommand: { type: "tauri", target: "launch_3ai" },
  },
  {
    id: "governance",
    name: "Governance",
    description: "On-chain governance proposals, voting, and delegation",
    category: "blockchain",
    icon: { type: "file", path: "/assets/icons/governance.svg", color: "#ff3366" },
    launchCommand: { type: "url", target: "http://localhost:3009" },
  },
  {
    id: "launchpad",
    name: "Launchpad",
    description: "Token and project launchpad with IDO support",
    category: "defi",
    icon: { type: "placeholder", category: "defi", color: "#ffa726" },
    launchCommand: { type: "url", target: "http://localhost:3010" },
  },
  {
    id: "unified-dashboard",
    name: "Unified Dashboard",
    description: "Single-pane overview of all ecosystem metrics",
    category: "analysis",
    icon: { type: "placeholder", category: "analysis", color: "#42a5f5" },
    launchCommand: { type: "url", target: "http://localhost:3011" },
  },
  {
    id: "quantum-voyager",
    name: "Quantum Voyager",
    description: "3D blockchain visualiser and system explorer",
    category: "utility",
    icon: { type: "file", path: "/assets/icons/quantum.svg", color: "#9d4edd" },
    launchCommand: { type: "tauri", target: "launch_quantum_voyager" },
  },

  {
    id: "phase5-panel",
    name: "Phase 5 Panel",
    description: "Phase 5 deployment control panel",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#ab47bc" },
    launchCommand: { type: "url", target: "http://localhost:3012" },
  },
  {
    id: "htlc-manager",
    name: "HTLC Manager",
    description: "Hash Time-Lock Contract management and monitoring",
    category: "blockchain",
    icon: { type: "file", path: "/assets/icons/storage.svg", color: "#26c6da" },
    launchCommand: { type: "tauri", target: "launch_htlc_manager" },
  },
  {
    id: "system-monitoring",
    name: "System Monitor",
    description: "Real-time CPU, memory, disk, and IPFS storage metrics",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#64b5f6" },
    launchCommand: { type: "internal", target: "system-monitoring" },
  },
  {
    id: "documentation",
    name: "Documentation",
    description: "GPU Swarm Dashboard & CI/CD documentation and guides",
    category: "utility",
    icon: { type: "placeholder", category: "utility", color: "#4fc3f7" },
    launchCommand: { type: "internal", target: "documentation" },
  },
  /* ── Explorer Sub-Apps ───────────────────────────────────── */
  {
    id: "ai-swarm",
    name: "AI Swarm",
    description: "GPU AI Swarm node management — VRAM, compute, temps, SLA proofs",
    category: "service",
    icon: { type: "file", path: "/assets/icons/ai-swarm.svg", color: "#00e5ff" },
    launchCommand: { type: "internal", target: "ai-swarm" },
  },
  {
    id: "blog",
    name: "Blog",
    description: "Ecosystem updates, blockchain innovation, and adoption stories",
    category: "utility",
    icon: { type: "file", path: "/assets/icons/blog.svg", color: "#ff7043" },
    launchCommand: { type: "internal", target: "blog" },
  },
  {
    id: "bridge",
    name: "Bridge",
    description: "Cross-chain bridging — Ethereum, Polygon, Arbitrum, Optimism, Base",
    category: "defi",
    icon: { type: "file", path: "/assets/icons/bridge.svg", color: "#7c4dff" },
    launchCommand: { type: "internal", target: "bridge" },
  },
  {
    id: "community",
    name: "Community",
    description: "Community hub — Discord, GitHub, forum, events, grants",
    category: "utility",
    icon: { type: "file", path: "/assets/icons/community.svg", color: "#26c6da" },
    launchCommand: { type: "internal", target: "community" },
  },
  {
    id: "developers-portal",
    name: "Developers",
    description: "Developer portal — docs, API reference, SDKs, cookbook, tutorials",
    category: "development",
    icon: { type: "file", path: "/assets/icons/developers.svg", color: "#00e5ff" },
    launchCommand: { type: "internal", target: "developers-portal" },
  },
  {
    id: "earn",
    name: "Earn",
    description: "Points & rewards — bridge, swap, stake to earn X3 Points",
    category: "defi",
    icon: { type: "file", path: "/assets/icons/earn.svg", color: "#ffd740" },
    launchCommand: { type: "internal", target: "earn" },
  },
  {
    id: "ecosystem",
    name: "Ecosystem",
    description: "Ecosystem overview — TVL, protocols, AI Swarm, DeFi stats",
    category: "service",
    icon: { type: "file", path: "/assets/icons/ecosystem.svg", color: "#00e676" },
    launchCommand: { type: "internal", target: "ecosystem" },
  },
  {
    id: "block-explorer",
    name: "Block Explorer",
    description: "Search blocks, transactions, accounts — live chain data via RPC",
    category: "blockchain",
    icon: { type: "placeholder", category: "blockchain", color: "#ff6b35" },
    launchCommand: { type: "internal", target: "block-explorer" },
  },
  {
    id: "learn",
    name: "Learn",
    description: "Learning center — onboarding, architecture, tokenomics, tutorials",
    category: "utility",
    icon: { type: "file", path: "/assets/icons/learn.svg", color: "#7c4dff" },
    launchCommand: { type: "internal", target: "learn" },
  },
  {
    id: "defi-metrics",
    name: "DeFi Metrics",
    description: "Live DeFi metrics — 24h volume, active protocols, cross-chain swaps",
    category: "analysis",
    icon: { type: "file", path: "/assets/icons/metrics.svg", color: "#42a5f5" },
    launchCommand: { type: "internal", target: "defi-metrics" },
  },
  {
    id: "network-status",
    name: "Network",
    description: "Network status — validators, RPC providers, GPU swarm, on/off ramps",
    category: "service",
    icon: { type: "file", path: "/assets/icons/network.svg", color: "#00b4ff" },
    launchCommand: { type: "internal", target: "network-status" },
  },
  {
    id: "portfolio",
    name: "Portfolio",
    description: "Cross-chain portfolio tracker — positions, yields, risk levels",
    category: "defi",
    icon: { type: "file", path: "/assets/icons/portfolio.svg", color: "#42a5f5" },
    launchCommand: { type: "internal", target: "portfolio" },
  },
  {
    id: "prometheus-metrics",
    name: "Prometheus",
    description: "Node metrics viewer — raw chain metrics from Prometheus endpoints",
    category: "analysis",
    icon: { type: "file", path: "/assets/icons/prometheus.svg", color: "#ff5722" },
    launchCommand: { type: "internal", target: "prometheus-metrics" },
  },
  {
    id: "quantum-landing",
    name: "Neural Validator",
    description: "Real-time chain telemetry, validator globe, quantum orderbook",
    category: "blockchain",
    icon: { type: "file", path: "/assets/icons/quantum.svg", color: "#9d4edd" },
    launchCommand: { type: "internal", target: "quantum-landing" },
  },
  {
    id: "security-page",
    name: "Security",
    description: "Security practices, bug bounty, audit status, vulnerability disclosure",
    category: "security",
    icon: { type: "file", path: "/assets/icons/security.svg", color: "#ef5350" },
    launchCommand: { type: "internal", target: "security-page" },
  },
  {
    id: "solutions",
    name: "Solutions",
    description: "Solutions marketplace — DeFi, Games, Payments, AI, Commerce, RWA",
    category: "development",
    icon: { type: "file", path: "/assets/icons/solutions.svg", color: "#00bfa5" },
    launchCommand: { type: "internal", target: "solutions" },
  },
  {
    id: "stake",
    name: "Stake",
    description: "Staking interface — validator delegation, lock tiers, and reward tracking",
    category: "defi",
    icon: { type: "file", path: "/assets/icons/stake.svg", color: "#ff6d00" },
    launchCommand: { type: "internal", target: "stake" },
  },
  {
    id: "atomic-swap",
    name: "Atomic Swap",
    description: "Cross-chain swap console — guarded by current router/gateway feature flags",
    category: "defi",
    icon: { type: "file", path: "/assets/icons/swap.svg", color: "#00e5ff" },
    launchCommand: { type: "internal", target: "atomic-swap" },
  },
  {
    id: "treasury",
    name: "Treasury",
    description: "DAO treasury dashboard — balances, distribution, revenue, burn tracking",
    category: "blockchain",
    icon: { type: "file", path: "/assets/icons/treasury.svg", color: "#ffd700" },
    launchCommand: { type: "internal", target: "treasury" },
  },
  {
    id: "x3-chain",
    name: "X3 Chain",
    description: "X3 Adaptive Intelligence — self-evolving blockchain, mutations, swarm",
    category: "blockchain",
    icon: { type: "file", path: "/assets/icons/x3-chain.svg", color: "#76ff03" },
    launchCommand: { type: "internal", target: "x3-chain" },
  },
  {
    id: "x3os",
    name: "x3OS",
    description: "x3Star OS — multi-VM window manager, terminal, execution console",
    category: "service",
    icon: { type: "file", path: "/assets/icons/x3os.svg", color: "#18ffff" },
    launchCommand: { type: "internal", target: "x3os" },
  },
  {
    id: "world-monitor",
    name: "World Monitor",
    description: "Global Node Telemetry and Real-time Visualization",
    category: "service",
    icon: { type: "file", path: "/assets/icons/network.svg", color: "#26c6da" },
    launchCommand: { type: "internal", target: "world-monitor" },
  },
  {
    id: "x3star",
    name: "x3Star Terminal",
    description: "Bloomberg-grade execution console — production chain management",
    category: "service",
    icon: { type: "file", path: "/assets/icons/x3star.svg", color: "#ffd740" },
    launchCommand: { type: "internal", target: "x3star" },
  },
  {
    id: "privacy-policy",
    name: "Privacy Policy",
    description: "Data collection, usage, disclosure, and safeguards",
    category: "other",
    icon: { type: "placeholder", category: "other", color: "#78909c" },
    launchCommand: { type: "internal", target: "privacy-policy" },
  },
  {
    id: "terms-of-service",
    name: "Terms of Service",
    description: "Platform terms, user obligations, and legal provisions",
    category: "other",
    icon: { type: "placeholder", category: "other", color: "#78909c" },
    launchCommand: { type: "internal", target: "terms-of-service" },
  },

  /* ── Explorer Sub-Pages (deeper routes) ── */
  {
    id: "dev-docs",
    name: "Developer Docs",
    description: "Full developer documentation — SDKs, APIs, architecture, tutorials",
    category: "development",
    icon: { type: "file", path: "/assets/icons/developers.svg", color: "#00e5ff" },
    launchCommand: { type: "internal", target: "dev-docs" },
  },
  {
    id: "solutions-detail",
    name: "Solutions Marketplace",
    description: "Browse DeFi, Games, AI, Payments, Commerce, RWA solutions",
    category: "development",
    icon: { type: "file", path: "/assets/icons/solutions.svg", color: "#00bfa5" },
    launchCommand: { type: "internal", target: "solutions-detail" },
  },
  {
    id: "network-validators",
    name: "Validators & RPC",
    description: "Validator management, RPC providers, on/off ramps",
    category: "service",
    icon: { type: "file", path: "/assets/icons/network.svg", color: "#00b4ff" },
    launchCommand: { type: "internal", target: "network-validators" },
  },
  {
    id: "learn-architecture",
    name: "Learn Architecture",
    description: "Architecture deep-dive, core concepts, tokenomics, tutorials",
    category: "utility",
    icon: { type: "file", path: "/assets/icons/learn.svg", color: "#7c4dff" },
    launchCommand: { type: "internal", target: "learn-architecture" },
  },
  {
    id: "x3-sub-pages",
    name: "X3 Deep Dive",
    description: "X3 evolution, scripts, verifier, swarm auctions & predictions",
    category: "blockchain",
    icon: { type: "file", path: "/assets/icons/x3-chain.svg", color: "#76ff03" },
    launchCommand: { type: "internal", target: "x3-sub-pages" },
  },
  {
    id: "community-hub",
    name: "Community Hub",
    description: "Ecosystem projects, events, forum discussions, grant programs",
    category: "utility",
    icon: { type: "file", path: "/assets/icons/community.svg", color: "#26c6da" },
    launchCommand: { type: "internal", target: "community-hub" },
  },
  {
    id: "quantum-enhanced",
    name: "Quantum Terminal",
    description: "Enhanced quantum visualization — orderbook, telemetry, validator globe",
    category: "blockchain",
    icon: { type: "file", path: "/assets/icons/quantum.svg", color: "#9d4edd" },
    launchCommand: { type: "internal", target: "quantum-enhanced" },
  },
  {
    id: "explorer-home",
    name: "Explorer Home",
    description: "Block explorer home — latest blocks, extrinsics, network activity",
    category: "blockchain",
    icon: { type: "placeholder", category: "blockchain", color: "#ff6b35" },
    launchCommand: { type: "internal", target: "explorer-home" },
  },
  {
    id: "explorer-detail",
    name: "Explorer Detail",
    description: "Look up blocks, transactions, and accounts by hash or number",
    category: "blockchain",
    icon: { type: "placeholder", category: "blockchain", color: "#ff6b35" },
    launchCommand: { type: "internal", target: "explorer-detail" },
  },

  /* ── Wallet (ported from apps/wallet) ── */
  {
    id: "wallet",
    name: "X3 Wallet",
    description: "Multi-chain wallet — send, receive, swap, stake, comit transactions",
    category: "defi",
    icon: { type: "placeholder", category: "defi", color: "#ff6d00" },
    launchCommand: { type: "internal", target: "wallet" },
  },
  {
    id: "validators",
    name: "Validators",
    description: "Real-time 3D validator globe with node status, health, and network links",
    category: "blockchain",
    icon: { type: "placeholder", category: "blockchain", color: "#00d2ff" },
    launchCommand: { type: "internal", target: "validators" },
  },

  /* ── X3 Intelligence (ported from apps/x3-intelligence) ── */
  {
    id: "x3-floor-dashboard",
    name: "X3 Trading Floor",
    description: "Arbitrage jurisdiction control — agents, intents, volume, slashing",
    category: "analysis",
    icon: { type: "placeholder", category: "analysis", color: "#00d4aa" },
    launchCommand: { type: "internal", target: "x3-floor-dashboard" },
  },

  /* ── X3 Protocol-Native CRM ────────────────────────────── */
  {
    id: "x3-crm",
    name: "X3 CRM",
    description: "Enterprise validator & partnership CRM — capacity mapping, deal tracking, revenue OS",
    category: "service",
    icon: { type: "file", path: "/assets/icons/crm.svg", color: "#00e676" },
    launchCommand: { type: "internal", target: "x3-crm-dashboard" },
  },

  /* ── DEX (ported from apps/dex) ── */
  {
    id: "dex",
    name: "X3 DEX",
    description: "Decentralized exchange — swap, market overview, recent trades",
    category: "defi",
    icon: { type: "placeholder", category: "defi", color: "#00e5ff" },
    launchCommand: { type: "internal", target: "dex" },
  },
  {
    id: "dex-orderbook",
    name: "DEX Orderbook",
    description: "Live orderbook with depth, spread, and active order flow",
    category: "defi",
    icon: { type: "placeholder", category: "defi", color: "#22d3ee" },
    launchCommand: { type: "internal", target: "dex-orderbook" },
  },


  /* ── Swarm Dashboard (ported from apps/swarm-dashboard) ── */
  {
    id: "gpu-swarm-dashboard",
    name: "GPU Swarm Monitor",
    description: "Real-time GPU swarm — utilization, tasks, alerts, health status",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#00e676" },
    launchCommand: { type: "internal", target: "gpu-swarm-dashboard" },
  },

  /* ── Infrastructure Dashboard (ported from apps/inferstructor-dashboard) ── */
  {
    id: "infrastructure",
    name: "Infrastructure Monitor",
    description: "Bridge status, GPU lanes, RPC proxy, TPS metrics — live infra telemetry",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#3b82f6" },
    launchCommand: { type: "internal", target: "infrastructure" },
  },

  /* ── External URL Apps ──────────────────────────────────── */
  {
    id: "ollama-code-reviewer",
    name: "Ollama Code Reviewer",
    description: "Local AI code-review UI powered by Ollama",
    category: "development",
    icon: { type: "placeholder", category: "development", color: "#10b981" },
    launchCommand: { type: "url", target: "http://localhost:5175" },
  },
  {
    id: "3aixchange-dex",
    name: "3aiXchange DEX",
    description: "3aiXchange Chakra UI DEX with Abby AI assistant",
    category: "defi",
    icon: { type: "placeholder", category: "defi", color: "#6366f1" },
    launchCommand: { type: "url", target: "http://localhost:5176" },
  },
  {
    id: "x3-app-store",
    name: "X3 App Store",
    description: "App marketplace frontend",
    category: "utility",
    icon: { type: "placeholder", category: "utility", color: "#f59e0b" },
    launchCommand: { type: "url", target: "http://localhost:3001" },
  },
  {
    id: "gpu-validator-dashboard",
    name: "GPU Validator Dashboard",
    description: "Cross-chain GPU validator benchmark and metrics dashboard",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#22d3ee" },
    launchCommand: { type: "url", target: "http://localhost:8080" },
  },
  {
    id: "autonomic-control-plane",
    name: "Autonomic Control Plane",
    description: "Autonomic operator dashboard and control surface",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#f97316" },
    launchCommand: { type: "url", target: "http://localhost:8080/dashboard.html" },
  },
  {
    id: "blockchain-tps-tester",
    name: "Blockchain TPS Tester",
    description: "EVM TPS benchmark runner and reporting console",
    category: "development",
    icon: { type: "placeholder", category: "development", color: "#38bdf8" },
    launchCommand: { type: "url", target: "http://localhost:3020" },
  },
  {
    id: "foundry-hardhat-gui",
    name: "Foundry/Hardhat GUI",
    description: "Local Foundry + Hardhat orchestration dashboard",
    category: "development",
    icon: { type: "placeholder", category: "development", color: "#fb7185" },
    launchCommand: { type: "url", target: "http://localhost:8787" },
  },
  {
    id: "gpu-swarm-node-admin",
    name: "GPU Swarm Node Admin",
    description: "Rust-native GPU swarm node admin console",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#a78bfa" },
    launchCommand: { type: "url", target: "http://localhost:9101" },
  },

  /* ── RPC Pool Stats (deep-dive from Infrastructure Monitor) ── */
  {
    id: "rpc-stats",
    name: "RPC Pool Stats",
    description: "Full RPC pool analytics — provider breakdown, gas savings, latency, fastest endpoints",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#f59e0b" },
    launchCommand: { type: "internal", target: "rpc-stats" },
  },

  /* ── Airdrops & Faucets ── */
  {
    id: "airdrops",
    name: "Airdrops & Faucets",
    description: "Discovered airdrops, testnet faucets, auto-claim tracking, wallet balances",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#ec4899" },
    launchCommand: { type: "internal", target: "airdrops" },
  },

  /* ── Health Dashboard (ported from apps/health-dashboard) ── */
  {
    id: "health-dashboard",
    name: "Health Dashboard",
    description: "System health — service status, metrics, response times, uptime",
    category: "service",
    icon: { type: "placeholder", category: "service", color: "#42a5f5" },
    launchCommand: { type: "internal", target: "health-dashboard" },
  },

  /* ── Admin Dashboard ── */
  {
    id: "admin-dashboard",
    name: "Admin Dashboard",
    description: "System administration — service health, allowlisted commands, diagnostics",
    category: "security",
    icon: { type: "placeholder", category: "security", color: "#ef4444" },
    launchCommand: { type: "internal", target: "admin-dashboard" },
  },
];

/* ── Service functions ─────────────────────────────────────── */

/**
 * Fetch the application registry from the backend.
 * Falls back to the hardcoded default list on failure.
 */
export async function fetchApplicationRegistry(): Promise<Application[]> {
  try {
    const apps = await ipcInvoke<Application[]>("get_app_registry", undefined, {
      retries: 1,
      timeout: 5000,
    });
    return apps && apps.length > 0 ? apps : DEFAULT_APPLICATIONS;
  } catch {
    console.warn("[AppService] Backend unavailable — using default registry");
    return DEFAULT_APPLICATIONS;
  }
}

/**
 * Launch an application.
 *
 * @param app - The application manifest
 * @throws {AppError} if launch fails or times out
 */
export async function launchApplication(app: Application): Promise<void> {
  const store = useApplicationStore.getState();
  const timeout = app.lifecycle?.timeout ?? 10_000;

  // Check dependencies
  if (app.systemRequirements?.dependencies) {
    for (const depId of app.systemRequirements.dependencies) {
      if (!store.isRunning(depId)) {
        throw new AppError(
          "DEPENDENCY_MISSING",
          `Required service "${depId}" is not running`,
          `Start ${depId} before launching ${app.name}`,
        );
      }
    }
  }

  store.startProcess(app.id);

  try {
    switch (app.launchCommand.type) {
      case "tauri": {
        const result = await ipcInvoke<LaunchResult>(
          "launch_app",
          {
            app_id: app.id,
            command: app.launchCommand.target,
            args: app.launchCommand.args ?? [],
            env: app.launchCommand.env ?? {},
          },
          { timeout },
        );

        if (result.status === "error") {
          throw new AppError("LAUNCH_FAILED", result.message ?? "Launch failed");
        }

        store.updateProcessStatus(app.id, "running");
        break;
      }

      case "url": {
        // URL apps are rendered in-window via IframePanel — no browser tab needed
        store.updateProcessStatus(app.id, "running");
        break;
      }

      case "internal": {
        // Internal Tauri app launched as a window
        store.updateProcessStatus(app.id, "running");
        break;
      }

      case "process": {
        const result = await ipcInvoke<LaunchResult>(
          "launch_app",
          {
            app_id: app.id,
            command: app.launchCommand.target,
            args: app.launchCommand.args ?? [],
            env: app.launchCommand.env ?? {},
          },
          { timeout },
        );

        if (result.status === "error") {
          throw new AppError("LAUNCH_FAILED", result.message ?? "Launch failed");
        }

        store.updateProcessStatus(app.id, "running");
        break;
      }
    }
  } catch (err) {
    store.updateProcessStatus(app.id, "crashed");
    throw err;
  }
}

/**
 * Stop a running application.
 */
export async function stopApplication(appId: string): Promise<void> {
  const store = useApplicationStore.getState();
  store.updateProcessStatus(appId, "stopping");

  try {
    await ipcInvoke("stop_app", { app_id: appId }, { timeout: 10_000 });
  } catch {
    // Force-remove on failure
  }

  store.removeProcess(appId);
}
