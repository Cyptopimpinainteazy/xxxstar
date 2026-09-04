interface ChartDatum {
  name: string;
  tps: number;
  color: string;
}

interface TpsLeaderboardChartProps {
  chartData: ChartDatum[];
}

const CHART_WIDTH = 760;
const CHART_HEIGHT = 256;
const MARGIN_TOP = 12;
const MARGIN_RIGHT = 12;
const MARGIN_BOTTOM = 68;
const MARGIN_LEFT = 52;
const GRID_LINES = 4;

function formatAxisValue(value: number): string {
  if (value >= 1000) {
    return `${(value / 1000).toFixed(0)}K`;
  }

  return String(Math.round(value));
}

function barPath(x: number, y: number, width: number, height: number, radius: number): string {
  const clampedRadius = Math.min(radius, width / 2, height);

  return [
    `M ${x} ${y + height}`,
    `L ${x} ${y + clampedRadius}`,
    `Q ${x} ${y} ${x + clampedRadius} ${y}`,
    `L ${x + width - clampedRadius} ${y}`,
    `Q ${x + width} ${y} ${x + width} ${y + clampedRadius}`,
    `L ${x + width} ${y + height}`,
    'Z',
  ].join(' ');
}

export function TpsLeaderboardChart({ chartData }: TpsLeaderboardChartProps) {
  const innerWidth = CHART_WIDTH - MARGIN_LEFT - MARGIN_RIGHT;
  const innerHeight = CHART_HEIGHT - MARGIN_TOP - MARGIN_BOTTOM;
  const maxValue = Math.max(...chartData.map(entry => entry.tps), 1);
  const segmentWidth = innerWidth / chartData.length;
  const barWidth = Math.max(10, segmentWidth * 0.7);
  const tickValues = Array.from({ length: GRID_LINES + 1 }, (_, index) => Math.round((maxValue / GRID_LINES) * index));

  return (
    <div className="card mb-6">
      <h3 className="text-sm font-semibold text-gray-400 mb-4 uppercase tracking-wider">
        Top {Math.min(20, chartData.length)} - TPS Distribution
      </h3>
      <div className="h-64 w-full overflow-hidden rounded-lg border border-gray-800 bg-gray-950/40 p-3">
        <svg
          viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
          role="img"
          aria-label="TPS distribution chart"
          className="h-full w-full"
          preserveAspectRatio="none"
        >
          <rect x="0" y="0" width={CHART_WIDTH} height={CHART_HEIGHT} fill="transparent" />

          {tickValues.map((tickValue, index) => {
            const y = MARGIN_TOP + innerHeight - (index / GRID_LINES) * innerHeight;

            return (
              <g key={tickValue}>
                <line
                  x1={MARGIN_LEFT}
                  y1={y}
                  x2={CHART_WIDTH - MARGIN_RIGHT}
                  y2={y}
                  stroke="#1f2937"
                  strokeDasharray="4 4"
                />
                <text
                  x={MARGIN_LEFT - 10}
                  y={y + 4}
                  fill="#6b7280"
                  fontSize="10"
                  textAnchor="end"
                >
                  {formatAxisValue(tickValue)}
                </text>
              </g>
            );
          })}

          <line
            x1={MARGIN_LEFT}
            y1={MARGIN_TOP + innerHeight}
            x2={CHART_WIDTH - MARGIN_RIGHT}
            y2={MARGIN_TOP + innerHeight}
            stroke="#374151"
          />

          {chartData.map((entry, index) => {
            const barHeight = Math.max(2, (entry.tps / maxValue) * innerHeight);
            const x = MARGIN_LEFT + index * segmentWidth + (segmentWidth - barWidth) / 2;
            const y = MARGIN_TOP + innerHeight - barHeight;
            const labelX = MARGIN_LEFT + index * segmentWidth + segmentWidth / 2;

            return (
              <g key={`${entry.name}-${index}`}>
                <path d={barPath(x, y, barWidth, barHeight, 4)} fill={entry.color} fillOpacity="0.82">
                  <title>{`${entry.name}: ${entry.tps.toLocaleString()} TPS`}</title>
                </path>
                <text
                  x={labelX}
                  y={CHART_HEIGHT - 10}
                  fill="#6b7280"
                  fontSize="10"
                  textAnchor="end"
                  transform={`rotate(-30 ${labelX} ${CHART_HEIGHT - 10})`}
                >
                  {entry.name}
                </text>
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
}
