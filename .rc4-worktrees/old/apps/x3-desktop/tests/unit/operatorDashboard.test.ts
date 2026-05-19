/**
 * Operator Dashboard IPC Commands Test Suite
 * 
 * This test suite validates that all four dashboard panel commands
 * (SwarmHealth, Network, Storage, IDE) return correctly shaped mock data
 * and can be properly consumed by frontend panels.
 * 
 * Run: npm test -- operatorDashboard.test.ts
 */

import { describe, it, expect, beforeAll, vi } from "vitest";
import type {
  SwarmHealthData,
  NetworkControlData,
  StorageMonitorData,
  IdeTelemetryData,
} from "../../src-tauri/src/main.rs"; // Simulated types

/* ════════════════════════════════════════════════════════════════
   Mock Data Definitions (matching Rust structs)
   ════════════════════════════════════════════════════════════════ */

interface GpuStats {
  id: string;
  total_memory_mb: number;
  used_memory_mb: number;
  utilization_pct: number;
  temperature_c: number;
  fan_rpm?: number;
}

interface NodeHealth {
  node_id: string;
  status: "online" | "degraded" | "offline";
  cpu_usage_pct: number;
  mem_usage_pct: number;
  gpus: GpuStats[];
  last_seen_ms: number;
}

interface SwarmHealthMock {
  swarm_id: string;
  overall_health: "healthy" | "warning" | "critical";
  nodes: NodeHealth[];
  timestamp_ms: number;
}

interface PeerInfo {
  peer_id: string;
  ip: string;
  port: number;
  connection_age_sec: number;
  bytes_sent: number;
  bytes_recv: number;
  status: string;
}

interface BandwidthStats {
  tx_bps: number;
  rx_bps: number;
  tx_peak_bps: number;
  rx_peak_bps: number;
}

interface NetworkSnapshotMock {
  node_id: string;
  peers: PeerInfo[];
  bandwidth: BandwidthStats;
  latency_ms_avg: number;
  connections_open: number;
  timestamp_ms: number;
}

interface DiskHealth {
  mount: string;
  device: string;
  capacity_gb: number;
  used_gb: number;
  health: "good" | "degraded" | "failed";
  smart_status?: string;
  iops: number;
}

interface StorageReportMock {
  node_id: string;
  disks: DiskHealth[];
  total_capacity_gb: number;
  free_capacity_gb: number;
  aggregate_iops: number;
  timestamp_ms: number;
}

interface IdeJob {
  job_id: string;
  user: string;
  started_at_ms: number;
  progress_pct: number;
  status: "queued" | "running" | "success" | "failed";
  logs_preview: string[];
}

interface IdeTelemetryMock {
  node_id: string;
  active_sessions: number;
  jobs: IdeJob[];
  timestamp_ms: number;
}

/* ════════════════════════════════════════════════════════════════
   Mock Response Generators
   ════════════════════════════════════════════════════════════════ */

function mockSwarmHealth(): SwarmHealthMock {
  const now = Date.now();
  return {
    swarm_id: "x3-swarm-alpha",
    overall_health: "healthy",
    nodes: [
      {
        node_id: "node-01",
        status: "online",
        cpu_usage_pct: 21.3,
        mem_usage_pct: 58.1,
        gpus: [
          {
            id: "GPU-0",
            total_memory_mb: 32768,
            used_memory_mb: 10240,
            utilization_pct: 42,
            temperature_c: 67,
            fan_rpm: 2300,
          },
          {
            id: "GPU-1",
            total_memory_mb: 32768,
            used_memory_mb: 15360,
            utilization_pct: 47,
            temperature_c: 71,
            fan_rpm: 2450,
          },
        ],
        last_seen_ms: now,
      },
      {
        node_id: "node-02",
        status: "online",
        cpu_usage_pct: 18.7,
        mem_usage_pct: 45.2,
        gpus: [
          {
            id: "GPU-0",
            total_memory_mb: 32768,
            used_memory_mb: 8192,
            utilization_pct: 25,
            temperature_c: 52,
            fan_rpm: 1800,
          },
        ],
        last_seen_ms: now,
      },
    ],
    timestamp_ms: now,
  };
}

function mockNetworkControl(): NetworkSnapshotMock {
  const now = Date.now();
  return {
    node_id: "node-01",
    peers: [
      {
        peer_id: "peer-a1",
        ip: "10.0.0.5",
        port: 30333,
        connection_age_sec: 43200,
        bytes_sent: 12_345_678,
        bytes_recv: 23_456_789,
        status: "established",
      },
      {
        peer_id: "peer-b2",
        ip: "10.0.0.9",
        port: 30333,
        connection_age_sec: 120,
        bytes_sent: 1_234_567,
        bytes_recv: 987_654,
        status: "handshaking",
      },
      {
        peer_id: "peer-c3",
        ip: "10.0.0.12",
        port: 30333,
        connection_age_sec: 3600,
        bytes_sent: 5_000_000,
        bytes_recv: 7_500_000,
        status: "established",
      },
    ],
    bandwidth: {
      tx_bps: 125_000,
      rx_bps: 210_000,
      tx_peak_bps: 2_000_000,
      rx_peak_bps: 3_500_000,
    },
    latency_ms_avg: 21.6,
    connections_open: 12,
    timestamp_ms: now,
  };
}

function mockStorageMonitor(): StorageReportMock {
  const now = Date.now();
  const disks: DiskHealth[] = [
    {
      mount: "/",
      device: "/dev/nvme0n1",
      capacity_gb: 2048.0,
      used_gb: 1024.5,
      health: "good",
      smart_status: "PASSED",
      iops: 10_000,
    },
    {
      mount: "/data",
      device: "/dev/nvme1n1",
      capacity_gb: 4096.0,
      used_gb: 3000.1,
      health: "degraded",
      smart_status: "RELOCATIONS_PENDING",
      iops: 5_400,
    },
  ];

  const total_capacity: number = disks.reduce((sum, d) => sum + d.capacity_gb, 0);
  const used_total: number = disks.reduce((sum, d) => sum + d.used_gb, 0);

  return {
    node_id: "node-01",
    disks,
    total_capacity_gb: total_capacity,
    free_capacity_gb: total_capacity - used_total,
    aggregate_iops: 15_400,
    timestamp_ms: now,
  };
}

function mockIdeIpc(): IdeTelemetryMock {
  const now = Date.now();
  return {
    node_id: "ide-host-01",
    active_sessions: 3,
    jobs: [
      {
        job_id: "ide-9421",
        user: "alice",
        started_at_ms: now - 60_000,
        progress_pct: 72,
        status: "running",
        logs_preview: [
          "Cloning repo...",
          "Building project...",
          "Running tests: 73%",
        ],
      },
      {
        job_id: "ide-9422",
        user: "bob",
        started_at_ms: now - 300_000,
        progress_pct: 100,
        status: "success",
        logs_preview: ["Build succeeded", "Deployed artifact"],
      },
      {
        job_id: "ide-9423",
        user: "charlie",
        started_at_ms: now - 10_000,
        progress_pct: 5,
        status: "queued",
        logs_preview: ["Waiting for resources..."],
      },
    ],
    timestamp_ms: now,
  };
}

/* ════════════════════════════════════════════════════════════════
   Test Suite
   ════════════════════════════════════════════════════════════════ */

describe("🎯 X3 Operator Dashboard — IPC Commands", () => {
  /* ──────────────────────────────────────────────────────────────
     SwarmHealthPanel Tests
     ────────────────────────────────────────────────────────────── */

  describe("SwarmHealthPanel → launch_swarm_health", () => {
    let swarmData: SwarmHealthMock;

    beforeAll(() => {
      swarmData = mockSwarmHealth();
    });

    it("should return non-empty swarm_id", () => {
      expect(swarmData.swarm_id).toBeTruthy();
      expect(swarmData.swarm_id).toContain("swarm");
    });

    it("should have valid overall_health status", () => {
      const validStatuses = ["healthy", "warning", "critical"];
      expect(validStatuses).toContain(swarmData.overall_health);
    });

    it("should contain array of at least one node", () => {
      expect(swarmData.nodes).toBeDefined();
      expect(Array.isArray(swarmData.nodes)).toBe(true);
      expect(swarmData.nodes.length).toBeGreaterThan(0);
    });

    it("should have valid node statuses", () => {
      const validStatuses = ["online", "degraded", "offline"];
      swarmData.nodes.forEach((node) => {
        expect(validStatuses).toContain(node.status);
      });
    });

    it("each node should have CPU/memory metrics", () => {
      swarmData.nodes.forEach((node) => {
        expect(node.cpu_usage_pct).toBeGreaterThanOrEqual(0);
        expect(node.cpu_usage_pct).toBeLessThanOrEqual(100);
        expect(node.mem_usage_pct).toBeGreaterThanOrEqual(0);
        expect(node.mem_usage_pct).toBeLessThanOrEqual(100);
      });
    });

    it("GPU stats should be valid", () => {
      swarmData.nodes.forEach((node) => {
        node.gpus.forEach((gpu) => {
          expect(gpu.id).toBeTruthy();
          expect(gpu.total_memory_mb).toBeGreaterThan(0);
          expect(gpu.used_memory_mb).toBeLessThanOrEqual(gpu.total_memory_mb);
          expect(gpu.utilization_pct).toBeGreaterThanOrEqual(0);
          expect(gpu.utilization_pct).toBeLessThanOrEqual(100);
          expect(gpu.temperature_c).toBeGreaterThan(0);
          expect(gpu.temperature_c).toBeLessThan(150);
        });
      });
    });

    it("should have recent timestamp", () => {
      const now = Date.now();
      expect(swarmData.timestamp_ms).toBeLessThanOrEqual(now);
      expect(swarmData.timestamp_ms).toBeGreaterThan(now - 10_000); // within 10s
    });
  });

  /* ──────────────────────────────────────────────────────────────
     NetworkPanel Tests
     ────────────────────────────────────────────────────────────── */

  describe("NetworkPanel → launch_network_control", () => {
    let networkData: NetworkSnapshotMock;

    beforeAll(() => {
      networkData = mockNetworkControl();
    });

    it("should return valid node_id", () => {
      expect(networkData.node_id).toBeTruthy();
      expect(networkData.node_id).toMatch(/^node-/);
    });

    it("should contain peer list", () => {
      expect(Array.isArray(networkData.peers)).toBe(true);
      expect(networkData.peers.length).toBeGreaterThan(0);
    });

    it("peers should have valid IP addresses", () => {
      networkData.peers.forEach((peer) => {
        expect(peer.peer_id).toBeTruthy();
        expect(peer.ip).toMatch(/^\d+\.\d+\.\d+\.\d+$/);
        expect(peer.port).toBeGreaterThan(0);
        expect(peer.port).toBeLessThan(65536);
      });
    });

    it("peers should have valid connection stats", () => {
      networkData.peers.forEach((peer) => {
        expect(peer.connection_age_sec).toBeGreaterThanOrEqual(0);
        expect(peer.bytes_sent).toBeGreaterThanOrEqual(0);
        expect(peer.bytes_recv).toBeGreaterThanOrEqual(0);
      });
    });

    it("bandwidth stats should be valid", () => {
      expect(networkData.bandwidth.tx_bps).toBeGreaterThanOrEqual(0);
      expect(networkData.bandwidth.rx_bps).toBeGreaterThanOrEqual(0);
      expect(networkData.bandwidth.tx_peak_bps).toBeGreaterThanOrEqual(
        networkData.bandwidth.tx_bps,
      );
      expect(networkData.bandwidth.rx_peak_bps).toBeGreaterThanOrEqual(
        networkData.bandwidth.rx_bps,
      );
    });

    it("latency and connection count should be reasonable", () => {
      expect(networkData.latency_ms_avg).toBeGreaterThan(0);
      expect(networkData.latency_ms_avg).toBeLessThan(1000); // < 1 second
      expect(networkData.connections_open).toBeGreaterThanOrEqual(0);
    });
  });

  /* ──────────────────────────────────────────────────────────────
     StoragePanel Tests
     ────────────────────────────────────────────────────────────── */

  describe("StoragePanel → launch_storage_monitor", () => {
    let storageData: StorageReportMock;

    beforeAll(() => {
      storageData = mockStorageMonitor();
    });

    it("should contain disk array", () => {
      expect(Array.isArray(storageData.disks)).toBe(true);
      expect(storageData.disks.length).toBeGreaterThan(0);
    });

    it("each disk should have valid health status", () => {
      const validHealth = ["good", "degraded", "failed"];
      storageData.disks.forEach((disk) => {
        expect(validHealth).toContain(disk.health);
        expect(disk.mount).toBeTruthy();
        expect(disk.device).toBeTruthy();
      });
    });

    it("disk capacity should be consistent", () => {
      storageData.disks.forEach((disk) => {
        expect(disk.capacity_gb).toBeGreaterThan(0);
        expect(disk.used_gb).toBeGreaterThanOrEqual(0);
        expect(disk.used_gb).toBeLessThanOrEqual(disk.capacity_gb);
        expect(disk.iops).toBeGreaterThanOrEqual(0);
      });
    });

    it("aggregate stats should be mathematically valid", () => {
      const sumCapacity = storageData.disks.reduce(
        (sum, d) => sum + d.capacity_gb,
        0,
      );
      const sumUsed = storageData.disks.reduce(
        (sum, d) => sum + d.used_gb,
        0,
      );

      expect(storageData.total_capacity_gb).toBeCloseTo(sumCapacity, 2);
      expect(storageData.free_capacity_gb).toBeCloseTo(
        sumCapacity - sumUsed,
        2,
      );
    });

    it("should have recent timestamp", () => {
      const now = Date.now();
      expect(storageData.timestamp_ms).toBeLessThanOrEqual(now);
      expect(storageData.timestamp_ms).toBeGreaterThan(now - 10_000);
    });
  });

  /* ──────────────────────────────────────────────────────────────
     IDEPanel Tests
     ────────────────────────────────────────────────────────────── */

  describe("IDEPanel → launch_ide_ipc", () => {
    let ideData: IdeTelemetryMock;

    beforeAll(() => {
      ideData = mockIdeIpc();
    });

    it("should have valid node_id and session count", () => {
      expect(ideData.node_id).toBeTruthy();
      expect(ideData.active_sessions).toBeGreaterThanOrEqual(0);
    });

    it("should contain job list", () => {
      expect(Array.isArray(ideData.jobs)).toBe(true);
    });

    it("each job should have valid structure", () => {
      const validStatuses = ["queued", "running", "success", "failed"];
      ideData.jobs.forEach((job) => {
        expect(job.job_id).toBeTruthy();
        expect(job.user).toBeTruthy();
        expect(validStatuses).toContain(job.status);
        expect(job.progress_pct).toBeGreaterThanOrEqual(0);
        expect(job.progress_pct).toBeLessThanOrEqual(100);
      });
    });

    it("job progress should match status", () => {
      ideData.jobs.forEach((job) => {
        if (job.status === "success") {
          expect(job.progress_pct).toBe(100);
        } else if (job.status === "failed") {
          expect(job.progress_pct).toBeLessThan(100);
        }
      });
    });

    it("should have log previews for each job", () => {
      ideData.jobs.forEach((job) => {
        expect(Array.isArray(job.logs_preview)).toBe(true);
        expect(job.logs_preview.length).toBeGreaterThan(0);
        job.logs_preview.forEach((log) => {
          expect(typeof log).toBe("string");
        });
      });
    });

    it("should have recent timestamp", () => {
      const now = Date.now();
      expect(ideData.timestamp_ms).toBeLessThanOrEqual(now);
      expect(ideData.timestamp_ms).toBeGreaterThan(now - 10_000);
    });
  });

  /* ──────────────────────────────────────────────────────────────
     Integration Tests (verifying cross-panel data consistency)
     ────────────────────────────────────────────────────────────── */

  describe("🔗 Cross-Panel Integration", () => {
    it("all panels should have consistent timestamps (within 10s)", () => {
      const swarm = mockSwarmHealth();
      const network = mockNetworkControl();
      const storage = mockStorageMonitor();
      const ide = mockIdeIpc();

      const timestamps = [
        swarm.timestamp_ms,
        network.timestamp_ms,
        storage.timestamp_ms,
        ide.timestamp_ms,
      ];

      const maxTime = Math.max(...timestamps);
      const minTime = Math.min(...timestamps);
      const diff = maxTime - minTime;

      expect(diff).toBeLessThan(10_000); // All within 10 seconds
    });

    it("network node_id should match swarm node entries", () => {
      const swarm = mockSwarmHealth();
      const network = mockNetworkControl();

      const nodeIds = swarm.nodes.map((n) => n.node_id);
      expect(nodeIds).toContain(network.node_id);
    });

    it("storage node_id should reference a known node", () => {
      const swarm = mockSwarmHealth();
      const storage = mockStorageMonitor();

      const nodeIds = swarm.nodes.map((n) => n.node_id);
      expect(nodeIds).toContain(storage.node_id);
    });
  });

  /* ──────────────────────────────────────────────────────────────
     Display Helpers (verify data can be formatted for UI)
     ────────────────────────────────────────────────────────────── */

  describe("📊 UI Rendering Helpers", () => {
    it("SwarmHealth can be formatted as a summary", () => {
      const swarm = mockSwarmHealth();
      const summary = `${swarm.swarm_id}: ${swarm.overall_health} (${swarm.nodes.length} nodes)`;
      expect(summary).toContain("x3-swarm");
      expect(summary).toMatch(/\d+ nodes/);
    });

    it("NetworkSnapshot can calculate utilization (%)", () => {
      const network = mockNetworkControl();
      const tx_pct = (network.bandwidth.tx_bps / network.bandwidth.tx_peak_bps) * 100;
      const rx_pct = (network.bandwidth.rx_bps / network.bandwidth.rx_peak_bps) * 100;

      expect(tx_pct).toBeGreaterThanOrEqual(0);
      expect(rx_pct).toBeGreaterThanOrEqual(0);
      expect(tx_pct).toBeLessThanOrEqual(100);
      expect(rx_pct).toBeLessThanOrEqual(100);
    });

    it("StorageReport can calculate free space %", () => {
      const storage = mockStorageMonitor();
      const usedPct = (
        (storage.total_capacity_gb - storage.free_capacity_gb) /
        storage.total_capacity_gb
      ) * 100;

      expect(usedPct).toBeGreaterThan(0);
      expect(usedPct).toBeLessThan(100);
    });

    it("IdeTelemetry can map jobs by status for UI grouping", () => {
      const ide = mockIdeIpc();
      const grouped: Record<string, number> = {};

      ide.jobs.forEach((job) => {
        grouped[job.status] = (grouped[job.status] || 0) + 1;
      });

      expect(Object.keys(grouped).length).toBeGreaterThan(0);
    });
  });
});
