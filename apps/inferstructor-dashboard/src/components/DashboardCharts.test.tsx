import { describe, expect, it } from 'vitest';
import { render, screen } from '../test/test-utils';
import { TpsChart } from './dashboard/TpsChart';
import { GPULanesSection } from './dashboard/GPULanesSection';

describe('Dashboard charts', () => {
  it('renders the dashboard TPS SVG chart', () => {
    render(
      <TpsChart
        tpsHistory={[
          { time: '10:00:00', ts: Date.now() - 2_000, tps: 120_000, forwarded: 100, received: 105 },
          { time: '10:00:02', ts: Date.now(), tps: 140_000, forwarded: 110, received: 115 },
        ]}
        timeRange="5m"
        onTimeRangeChange={() => undefined}
      />,
    );

    expect(screen.getByText('Real-Time TPS Performance')).toBeInTheDocument();
    expect(screen.getByLabelText('Dashboard TPS history chart')).toBeInTheDocument();
  });

  it('renders the dashboard GPU SVG chart', () => {
    render(
      <GPULanesSection
        gpuLanes={[
          {
            service: 'gpu-lane-a',
            gpu: { id: 0, available: true, utilization: 72, memory_used_mb: 4096, temperature_c: 65 },
            stats: { total_txns: 120_000, success_rate: 0.99, txns_per_second: 18_500 },
          },
        ] as any}
      />,
    );

    expect(screen.getByText('GPU Lanes (1 Active)')).toBeInTheDocument();
    expect(screen.getByLabelText('Dashboard GPU lane transaction chart')).toBeInTheDocument();
  });
});