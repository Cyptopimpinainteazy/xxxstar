import { describe, expect, it, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { render, screen, waitFor } from '../test/test-utils';
import { OrchestraOperationsPanel } from './OrchestraOperationsPanel';

function buildProps() {
  return {
    services: {
      'x3-gateway': 'up',
      'x3-sidecar': 'up',
      'x3-orchestra-control-plane': 'up',
    },
    aggregated: {
      current_tps: 100,
      peak_tps: 120,
      services_up: 3,
      services_total: 3,
      total_gpu_txns: 10,
      total_gpu_success: 10,
      total_gpu_failed: 0,
      success_rate: 100,
      avg_gpu_utilization: 50,
      avg_gpu_memory_mb: 1024,
      avg_gpu_temp_c: 60,
      bridge_received: 10,
      bridge_forwarded: 10,
      bridge_failed: 0,
      dropped_tx_pct: 0,
      throughput_utilization: 10,
      rpc_total_requests: 20,
      rpc_cache_hit_rate: '50%',
      rpc_cached_responses: 10,
      rpc_gpu_verified: 5,
      rpc_errors: 0,
      uptime_seconds: 100,
      cost_per_tx_usd: 0.01,
      cost_per_million_tx_usd: 10,
      gpu_power_watts: 300,
      gpu_cost_per_hour_usd: 1,
    },
    intents: [
      {
        intent_id: 'intent-1',
        tenant_id: 'tenant-1',
        kind: 'publication',
        status: 'pending_approval',
        risk_class: 'high',
        submitter: 'ops',
        requires_approval: true,
        payload: {},
        created_at: '2026-04-07T00:00:00Z',
        updated_at: '2026-04-07T00:00:00Z',
      },
    ],
    approvalCases: [
      {
        case_id: 'case-1',
        intent_id: 'intent-1',
        status: 'open',
        review_kind: 'treasury-board',
        requested_by: 'ops',
        summary: 'Treasury transfer',
        metadata: {},
        created_at: '2026-04-07T00:00:00Z',
        updated_at: '2026-04-07T00:00:00Z',
      },
      {
        case_id: 'case-2',
        intent_id: 'intent-2',
        status: 'open',
        review_kind: 'content-review',
        requested_by: 'marketing',
        summary: 'Publication launch',
        metadata: {},
        created_at: '2026-04-07T01:00:00Z',
        updated_at: '2026-04-07T01:00:00Z',
      },
    ],
    voteWindows: [
      {
        window_id: 'window-1',
        approval_case_id: 'case-1',
        title: 'Treasury board vote',
        status: 'open',
        opens_at_unix: 1,
        closes_at_unix: 2,
        electorate: ['alice', 'bob'],
        tally: { approvals: 1, rejections: 0, abstentions: 0 },
        created_at: '2026-04-07T00:00:00Z',
        updated_at: '2026-04-07T00:00:00Z',
      },
    ],
    evidenceBundles: [
      {
        bundle_id: 'bundle-1',
        intent_id: 'intent-1',
        approval_case_id: 'case-1',
        vote_window_id: 'window-1',
        artifact_uri: 'orchestra://bundle-1',
        digest: 'digest-1',
        summary: { action: 'vote_window_closed', detail: {} },
        created_at: '2026-04-07T00:00:00Z',
        updated_at: '2026-04-07T00:00:00Z',
      },
      {
        bundle_id: 'bundle-2',
        intent_id: 'intent-2',
        approval_case_id: 'case-2',
        vote_window_id: null,
        artifact_uri: 'orchestra://bundle-2',
        digest: 'digest-2',
        summary: { action: 'approval_created', detail: {} },
        created_at: '2026-04-07T01:00:00Z',
        updated_at: '2026-04-07T01:00:00Z',
      },
    ],
    benchmarkStatus: {
      measured: 2,
      total: 10,
      progress_pct: 20,
      last_updated: '2026-04-07T01:00:00Z',
      top5: [],
    },
    benchmarkReports: [
      {
        report_id: 'report-1',
        generated_at_unix: 1,
        profile: 'provider_onboarding',
        chain_name: 'PartnerChain',
        recommendation: 'TurboLaneMode',
        signer: 'x3-sidecar',
        workload_profile: { total_transactions: 100 },
        artifacts: [],
      },
    ],
    jobs: [],
    loading: false,
    error: null,
    onRefresh: vi.fn().mockResolvedValue(undefined),
    onCloseVoteWindow: vi.fn().mockResolvedValue(undefined),
    onImportVoteTally: vi.fn().mockResolvedValue({ approvals: 2, rejections: 1, abstentions: 0 }),
  };
}

describe('OrchestraOperationsPanel UI', () => {
  it('filters evidence by selected approval case', async () => {
    const user = userEvent.setup();
    const props = buildProps();

    render(<OrchestraOperationsPanel {...props} />);

    expect(screen.getByText('orchestra://bundle-1')).toBeInTheDocument();
    expect(screen.queryByText('orchestra://bundle-2')).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /publication launch/i }));

    expect(screen.getByText('orchestra://bundle-2')).toBeInTheDocument();
    expect(screen.queryByText('orchestra://bundle-1')).not.toBeInTheDocument();
  });

  it('runs tally import and overdue vote closure actions', async () => {
    const user = userEvent.setup();
    const props = buildProps();

    render(<OrchestraOperationsPanel {...props} />);

    await user.click(screen.getByRole('button', { name: /import tally/i }));
    await waitFor(() => {
      expect(props.onImportVoteTally).toHaveBeenCalledWith('window-1');
    });

    expect(await screen.findByText(/imported 2\/1\/0/i)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: /close overdue window/i }));
    await waitFor(() => {
      expect(props.onCloseVoteWindow).toHaveBeenCalledWith('window-1');
    });
    await waitFor(() => {
      expect(props.onRefresh).toHaveBeenCalled();
    });
  });
});