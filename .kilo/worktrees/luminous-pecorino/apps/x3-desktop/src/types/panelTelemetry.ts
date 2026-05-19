/**
 * Typed payloads for the Tier-1 operator panels.
 *
 * These shapes mirror the rust command responses and can be reused by the
 * frontend hooks + tests while the real backend is being wired.
 */

export type PanelCommandName =
  | "launch_swarm_health"
  | "launch_network_control"
  | "launch_storage_monitor"
  | "launch_ide_ipc";

export const TELEMETRY_EVENT = "telemetry_update";

/* ─── Swarm health ────────────────────────────────────────── */

export type SwarmNodeStatus = "online" | "idle" | "offline" | "slashed";

export interface SwarmNode {
  id: string;
  name: string;
  status: SwarmNodeStatus;
  gpuUtil: number; // percentage
  vramUsed: number; // bytes
  vramCapacity: number; // bytes
  temperature: number; // Celsius
  uptimeHours: number;
  sla: number; // percentage
  jobs: number;
}

export interface SwarmSummary {
  onlineNodes: number;
  totalNodes: number;
  avgGpuUtil: number;
  totalVramUsed: number;
  totalVramCapacity: number;
  queuedJobs: number;
}

export interface SwarmHealthData {
  summary: SwarmSummary;
  nodes: SwarmNode[];
  updatedAt: string;
}

export const sampleSwarmHealthData: SwarmHealthData = {
  summary: {
    onlineNodes: 5,
    totalNodes: 8,
    avgGpuUtil: 67,
    totalVramUsed: 85_000_000,
    totalVramCapacity: 196_608_000,
    queuedJobs: 12,
  },
  nodes: [
    {
      id: "node-0",
      name: "x3-gpu-0",
      status: "online",
      gpuUtil: 72,
      vramUsed: 18_000_000,
      vramCapacity: 24_576_000,
      temperature: 64,
      uptimeHours: 412,
      sla: 99,
      jobs: 6,
    },
    {
      id: "node-1",
      name: "x3-gpu-1",
      status: "online",
      gpuUtil: 59,
      vramUsed: 16_400_000,
      vramCapacity: 24_576_000,
      temperature: 61,
      uptimeHours: 208,
      sla: 98,
      jobs: 4,
    },
    {
      id: "node-2",
      name: "edge-node-a",
      status: "idle",
      gpuUtil: 18,
      vramUsed: 3_200_000,
      vramCapacity: 12_288_000,
      temperature: 48,
      uptimeHours: 36,
      sla: 95,
      jobs: 1,
    },
    {
      id: "node-3",
      name: "cloud-rtx-0",
      status: "slashed",
      gpuUtil: 33,
      vramUsed: 7_400_000,
      vramCapacity: 24_576_000,
      temperature: 71,
      uptimeHours: 120,
      sla: 84,
      jobs: 0,
    },
  ],
  updatedAt: "2026-02-08T14:38:31Z",
};

/* ─── Network control ─────────────────────────────────────── */

export type PeerProtocol = "tcp" | "udp" | "ws" | "mqtt";
export type PeerStatus = "connected" | "stale" | "disconnected";
export type EndpointStatus = "active" | "degraded" | "down";

export interface NetworkPeer {
  id: string;
  addr: string;
  protocol: PeerProtocol;
  latencyMs: number;
  status: PeerStatus;
  lastSeenSeconds: number;
  bytesSent: number;
  bytesReceived: number;
}

export interface NetworkRpcEndpoint {
  name: string;
  url: string;
  status: EndpointStatus;
  calls: number;
  avgMs: number;
}

export interface NetworkLogEntry {
  ts: string;
  level: "info" | "warn" | "error";
  message: string;
}

export interface NetworkControlData {
  peers: NetworkPeer[];
  rpcEndpoints: NetworkRpcEndpoint[];
  logs: NetworkLogEntry[];
  updatedAt: string;
}

export const sampleNetworkControlData: NetworkControlData = {
  peers: [
    {
      id: "peer-0",
      addr: "127.0.0.1:30333",
      protocol: "tcp",
      latencyMs: 12,
      status: "connected",
      lastSeenSeconds: 1,
      bytesSent: 10_482_221,
      bytesReceived: 54_842_113,
    },
    {
      id: "peer-6",
      addr: "relay.x3-chain.io:443",
      protocol: "ws",
      latencyMs: 33,
      status: "connected",
      lastSeenSeconds: 2,
      bytesSent: 2_003_112,
      bytesReceived: 6_124_900,
    },
    {
      id: "peer-4",
      addr: "10.0.0.5:9944",
      protocol: "ws",
      latencyMs: 0,
      status: "disconnected",
      lastSeenSeconds: 342,
      bytesSent: 0,
      bytesReceived: 0,
    },
  ],
  rpcEndpoints: [
    {
      name: "X3 Kernel RPC",
      url: "127.0.0.1:9944",
      status: "active",
      calls: 14_203,
      avgMs: 12,
    },
    {
      name: "Swarm Coordinator",
      url: "127.0.0.1:8080",
      status: "active",
      calls: 3_891,
      avgMs: 28,
    },
  ],
  logs: [
    { ts: "14:37:12", level: "info", message: "Peer x3-gpu-0 connected (tcp)" },
    { ts: "14:37:29", level: "warn", message: "Heartbeat latency spike → 172ms" },
    { ts: "14:37:53", level: "info", message: "RPC call trace chain_getBlockHash → 12ms" },
  ],
  updatedAt: "2026-02-08T14:38:31Z",
};

/* ─── Storage monitor ─────────────────────────────────────── */

export type StoragePinStatus = "pinned" | "pinning" | "unpinned" | "failed";
export type StorageProofResult = "valid" | "challenged" | "expired";
export type StoragePinType = "snapshot" | "artifact" | "agent-memory" | "contract" | "dataset";

export interface StoragePin {
  cid: string;
  name: string;
  size: number;
  status: StoragePinStatus;
  replicas: number;
  proofAgeMinutes: number;
  type: StoragePinType;
}

export interface StorageProof {
  cid: string;
  epoch: number;
  result: StorageProofResult;
  timestamp: string;
}

export interface StorageMonitorData {
  pins: StoragePin[];
  proofs: StorageProof[];
  capacityBytes: number;
  usedBytes: number;
  updatedAt: string;
}

export const sampleStorageMonitorData: StorageMonitorData = {
  pins: [
    {
      cid: "bafy2bza...k3f9x",
      name: "runtime-wasm-v0.8.2",
      size: 4_812_300,
      status: "pinned",
      replicas: 5,
      proofAgeMinutes: 3,
      type: "artifact",
    },
    {
      cid: "bafy2bza...m7p2q",
      name: "agent-memory-alpha.snap",
      size: 18_432_000,
      status: "pinned",
      replicas: 3,
      proofAgeMinutes: 12,
      type: "agent-memory",
    },
    {
      cid: "bafy2bza...a8c2e",
      name: "training-data-v3.tar",
      size: 1_073_741_824,
      status: "failed",
      replicas: 0,
      proofAgeMinutes: 999,
      type: "dataset",
    },
  ],
  proofs: [
    { cid: "bafy2bza...k3f9x", epoch: 1284391, result: "valid", timestamp: "14:32:01" },
    { cid: "bafy2bza...v9s1r", epoch: 1284391, result: "valid", timestamp: "14:31:58" },
    { cid: "bafy2bza...t6n3y", epoch: 1284390, result: "challenged", timestamp: "14:28:44" },
  ],
  capacityBytes: 20 * 1_073_741_824,
  usedBytes: 8_406_643_200,
  updatedAt: "2026-02-08T14:38:31Z",
};

/* ─── IDE telemetry ──────────────────────────────────────── */

export type BuildStatus = "building" | "success" | "failed" | "queued";

export interface BuildJob {
  id: string;
  target: string;
  status: BuildStatus;
  durationSeconds: number;
  timestamp: string;
}

export interface IdeContract {
  name: string;
  address: string;
  network: string;
  status: "deployed" | "pending" | "failed";
  gasUsed: number;
}

export interface TraceEntry {
  blockNum: number;
  extrinsic: string;
  result: "ok" | "err";
  gasUsed: number;
  stateRoot: string;
}

export interface IdeTelemetryData {
  builds: BuildJob[];
  contracts: IdeContract[];
  traces: TraceEntry[];
  logLines: string[];
  updatedAt: string;
}

export interface TelemetrySnapshot {
  swarm: SwarmHealthData;
  network: NetworkControlData;
  storage: StorageMonitorData;
  ide: IdeTelemetryData;
  updatedAt: string;
}

export const sampleIdeTelemetryData: IdeTelemetryData = {
  builds: [
    { id: "b-1", target: "x3-chain-runtime", status: "success", durationSeconds: 142, timestamp: "14:28:03" },
    { id: "b-2", target: "x3-lang-stdlib v0.3.0", status: "building", durationSeconds: 0, timestamp: "14:32:18" },
  ],
  contracts: [
    { name: "HTLC_v2", address: "5GrwvaEF...43jS", network: "x3-testnet", status: "deployed", gasUsed: 2_480_000 },
    { name: "GovernanceProxy", address: "5FHneW46...8qPm", network: "x3-testnet", status: "deployed", gasUsed: 1_120_000 },
  ],
  traces: [
    { blockNum: 1284391, extrinsic: "Balances::transfer", result: "ok", gasUsed: 125_000, stateRoot: "0xa3f2...d891" },
    { blockNum: 1284390, extrinsic: "HTLC::claim", result: "ok", gasUsed: 210_000, stateRoot: "0xc9d3...f103" },
  ],
  logLines: [
    "Compiling x3-chain-runtime v0.8.2",
    "Compiling pallet-swarm v0.4.1",
    "Building [===========>        ] 58%",
  ],
  updatedAt: "2026-02-08T14:38:31Z",
};
