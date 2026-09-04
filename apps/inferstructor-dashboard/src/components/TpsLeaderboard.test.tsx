import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '../test/test-utils';
import { TpsLeaderboard } from './TpsLeaderboard';

const apiMocks = vi.hoisted(() => ({
  getTpsLeaderboard: vi.fn(),
  getTpsBenchmarkStatus: vi.fn(),
  getRpcStats: vi.fn(),
}));

vi.mock('../api', async () => {
  const actual = await vi.importActual<typeof import('../api')>('../api');

  return {
    ...actual,
    api: {
      ...actual.api,
      getTpsLeaderboard: apiMocks.getTpsLeaderboard,
      getTpsBenchmarkStatus: apiMocks.getTpsBenchmarkStatus,
      getRpcStats: apiMocks.getRpcStats,
    },
  };
});

vi.mock('./TpsLeaderboardChart', () => {
  return {
    TpsLeaderboardChart: () => <div>Mock TPS distribution chart</div>,
  };
});

describe('TpsLeaderboard', () => {
  beforeEach(() => {
    apiMocks.getTpsLeaderboard.mockReset();
    apiMocks.getTpsBenchmarkStatus.mockReset();
    apiMocks.getRpcStats.mockReset();

    apiMocks.getTpsLeaderboard.mockResolvedValue({
      leaderboard: [
        {
          chain_id: 'chain-1',
          chain_name: 'Chain One',
          ecosystem: 'x3',
          chain_type: 'l1',
          native_token: 'X3',
          is_testnet: 0,
          tps_current: 1200,
          tps_peak: 1500,
          tps_theoretical: 2000,
          total_txns_24h: 100000,
          finality_seconds: 1.2,
          block_height: 100,
          measured_at: '2026-04-07T00:00:00Z',
          best_latency_ms: 20,
          endpoint_count: 12,
          total_rps: 100,
        },
      ],
      total: 1,
      stats: {
        total_chains_measured: 1,
        avg_tps_all: 1200,
        max_tps_all: 1500,
        peak_tps_all: 1500,
        total_txns_24h_all: 100000,
      },
      category: 'chain',
      sort: 'tps_current',
      order: 'desc',
    });

    apiMocks.getTpsBenchmarkStatus.mockResolvedValue({
      measured: 1,
      total: 10,
      progress_pct: 10,
      last_updated: '2026-04-07T00:00:00Z',
      top5: [],
    });

    apiMocks.getRpcStats.mockResolvedValue({
      total_endpoints: 20,
      healthy_endpoints: 18,
      chains_covered: 100,
      combined_rps: 200,
      avg_latency_ms: 25,
      min_latency_ms: 10,
      by_provider: [],
      by_tier: [],
      top_fastest: [],
      gas_savings: {
        infura_growth_equiv: 0,
        alchemy_growth_equiv: 0,
        quicknode_build_equiv: 0,
        moralis_pro_equiv: 0,
        total_monthly_saved: 0,
        your_cost: 0,
      },
    });
  });

  it('renders the lazy chart fallback and then the chart content', async () => {
    render(<TpsLeaderboard onBack={() => undefined} />);

    expect(await screen.findByText('TPS Leaderboard')).toBeInTheDocument();
    expect(await screen.findByText('Loading TPS chart...')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('Mock TPS distribution chart')).toBeInTheDocument();
    });
  });
});