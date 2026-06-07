import { describe, expect, it } from 'vitest';
import { buildOrchestraIncidents } from './orchestra-incidents';

describe('buildOrchestraIncidents', () => {
  it('flags degraded services, overdue votes, and missing evidence', () => {
    const incidents = buildOrchestraIncidents({
      services: {
        'x3-gateway': 'up',
        'x3-sidecar': 'down',
        'x3-orchestra-control-plane': 'up',
      },
      aggregated: {
        current_tps: 0,
        peak_tps: 0,
        services_up: 2,
        services_total: 3,
        total_gpu_txns: 0,
        total_gpu_success: 0,
        total_gpu_failed: 0,
        success_rate: 100,
        avg_gpu_utilization: 0,
        avg_gpu_memory_mb: 0,
        avg_gpu_temp_c: 0,
        bridge_received: 0,
        bridge_forwarded: 0,
        bridge_failed: 2,
        dropped_tx_pct: 0,
        throughput_utilization: 0,
        rpc_total_requests: 0,
        rpc_cache_hit_rate: '0%',
        rpc_cached_responses: 0,
        rpc_gpu_verified: 0,
        rpc_errors: 3,
        uptime_seconds: 0,
        cost_per_tx_usd: 0,
        cost_per_million_tx_usd: 0,
        gpu_power_watts: 0,
        gpu_cost_per_hour_usd: 0,
      },
      approvalCases: [
        {
          case_id: 'case-1',
          intent_id: 'intent-1',
          status: 'open',
          review_kind: 'treasury-board',
          requested_by: 'ops',
          summary: 'Treasury transfer',
          metadata: {},
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ],
      voteWindows: [
        {
          window_id: 'window-1',
          approval_case_id: 'case-1',
          title: 'Treasury board vote',
          status: 'open',
          opens_at_unix: 1_700_000_000,
          closes_at_unix: 1_700_000_100,
          electorate: ['alice'],
          tally: { approvals: 1, rejections: 0, abstentions: 0 },
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ],
      intents: [
        {
          intent_id: 'intent-1',
          tenant_id: 'tenant-1',
          kind: 'publication',
          status: 'completed',
          risk_class: 'high',
          submitter: 'ops',
          requires_approval: true,
          payload: {},
          created_at: '2024-01-01T00:00:00Z',
          updated_at: '2024-01-01T00:00:00Z',
        },
      ],
      evidenceBundles: [],
      benchmarkStatus: {
        measured: 0,
        total: 10,
        progress_pct: 0,
        last_updated: null,
        top5: [],
      },
      jobs: [],
      nowUnix: 1_700_000_500,
    });

    expect(incidents[0]).toMatchObject({
      severity: 'critical',
      source: 'sidecar',
      title: 'Sidecar degraded',
    });
    expect(incidents.some(incident => incident.id === 'workflow-overdue-votes')).toBe(true);
    expect(incidents.some(incident => incident.id === 'workflow-missing-evidence')).toBe(true);
  });

  it('flags failed benchmark jobs as sidecar incidents', () => {
    const incidents = buildOrchestraIncidents({
      services: {
        'x3-gateway': 'up',
        'x3-sidecar': 'up',
        'x3-orchestra-control-plane': 'up',
      },
      aggregated: null,
      approvalCases: [],
      voteWindows: [],
      intents: [],
      evidenceBundles: [],
      benchmarkStatus: null,
      jobs: [
        {
          job_id: 'job-1',
          command: 'bench-rpc-latency',
          label: 'RPC latency benchmark',
          status: 'failed',
          started_at: 1,
          duration_seconds: 15,
        },
      ],
    });

    expect(incidents).toContainEqual(
      expect.objectContaining({
        id: 'sidecar-benchmark-failures',
        source: 'sidecar',
        severity: 'warning',
      }),
    );
  });
});