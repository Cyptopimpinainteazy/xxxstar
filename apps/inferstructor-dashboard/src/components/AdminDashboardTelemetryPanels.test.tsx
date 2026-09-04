import { describe, expect, it } from 'vitest';
import { render, screen } from '../test/test-utils';
import { IntelligencePanel, NetworkPanel, OverviewPanel, PerformancePanel } from './AdminDashboardTelemetryPanels';

const aggregated = {
  current_tps: 120,
  peak_tps: 200,
  services_up: 3,
  services_total: 4,
  total_gpu_txns: 500,
  total_gpu_success: 490,
  total_gpu_failed: 10,
  success_rate: 98,
  avg_gpu_utilization: 70,
  avg_gpu_memory_mb: 2048,
  avg_gpu_temp_c: 65,
  bridge_received: 500,
  bridge_forwarded: 490,
  bridge_failed: 10,
  dropped_tx_pct: 2,
  throughput_utilization: 12,
  rpc_total_requests: 1000,
  rpc_cache_hit_rate: '65%',
  rpc_cached_responses: 650,
  rpc_gpu_verified: 320,
  rpc_errors: 4,
  uptime_seconds: 7200,
  cost_per_tx_usd: 0.01,
  cost_per_million_tx_usd: 10,
  gpu_power_watts: 450,
  gpu_cost_per_hour_usd: 2.25,
};

const filteredHistory = [
  { ...aggregated, timestamp: 1712448000 },
  { ...aggregated, current_tps: 140, peak_tps: 220, throughput_utilization: 15, timestamp: 1712448060 },
];

describe('AdminDashboardTelemetryPanels', () => {
  it('renders overview and performance panels', () => {
    render(
      <>
        <OverviewPanel
          services={{ gateway: 'up', sidecar: 'down' }}
          aggregated={aggregated}
          gpuLanes={[]}
          filteredHistory={filteredHistory}
        />
        <PerformancePanel
          aggregated={aggregated}
          gpuLanes={[]}
          filteredHistory={filteredHistory}
          timeRange="5m"
          setTimeRange={() => undefined}
          timeRanges={[{ key: '5m', label: '5m', seconds: 300 }]}
        />
      </>,
    );

    expect(screen.getByText('Service Status')).toBeInTheDocument();
    expect(screen.getByText('Real-Time TPS')).toBeInTheDocument();
    expect(screen.getByText('Throughput Utilization')).toBeInTheDocument();
    expect(screen.getByLabelText('Overview TPS history chart')).toBeInTheDocument();
    expect(screen.getByLabelText('Performance TPS history chart')).toBeInTheDocument();
  });

  it('renders network and intelligence panels', () => {
    render(
      <>
        <NetworkPanel
          aggregated={aggregated}
          chain={{
            version: { 'solana-core': '2.0.0' },
            slot: 42,
            epoch: { epoch: 7, slotIndex: 2, slotsInEpoch: 10, transactionCount: 2000000000 },
            block_height: 99,
            latest_blockhash: 'abc123',
          }}
          upstreams={[{ name: 'rpc-a', healthy: true, latency_ms: 12, requests: 55, errors: 0 }]}
        />
        <IntelligencePanel aggregated={aggregated} filteredHistory={filteredHistory} />
      </>,
    );

    expect(screen.getByText('Solana Chain — Live')).toBeInTheDocument();
    expect(screen.getByText('Cost & Fee Intelligence')).toBeInTheDocument();
    expect(screen.getByText('Reliability & Fault Detection')).toBeInTheDocument();
    expect(screen.getByLabelText('Throughput utilization history chart')).toBeInTheDocument();
  });
});