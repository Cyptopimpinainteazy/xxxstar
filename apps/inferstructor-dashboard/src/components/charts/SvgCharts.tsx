interface SvgLineSeries {
  key: string;
  color: string;
  label: string;
  dashed?: boolean;
  area?: boolean;
  fillOpacity?: number;
}

interface SvgLineChartProps<T extends object> {
  data: T[];
  labelKey: keyof T & string;
  series: SvgLineSeries[];
  ariaLabel: string;
  showXAxis?: boolean;
  showYAxis?: boolean;
  showGrid?: boolean;
  heightClassName?: string;
  valueFormatter?: (value: number) => string;
}

interface SvgBarDatum {
  label: string;
  value: number;
  title?: string;
}

interface SvgBarChartProps {
  data: SvgBarDatum[];
  ariaLabel: string;
  color?: string;
  heightClassName?: string;
  valueFormatter?: (value: number) => string;
}

const SVG_WIDTH = 760;
const SVG_HEIGHT = 220;
const GRID_LINES = 4;

function formatAxisValue(value: number): string {
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }

  if (value >= 1_000) {
    return `${(value / 1_000).toFixed(0)}K`;
  }

  return `${Math.round(value)}`;
}

function buildLinePath(points: Array<{ x: number; y: number }>): string {
  if (points.length === 0) {
    return '';
  }

  return points.map((point, index) => `${index === 0 ? 'M' : 'L'} ${point.x} ${point.y}`).join(' ');
}

function buildAreaPath(points: Array<{ x: number; y: number }>, baselineY: number): string {
  if (points.length === 0) {
    return '';
  }

  const linePath = buildLinePath(points);
  const lastPoint = points[points.length - 1];
  const firstPoint = points[0];

  return `${linePath} L ${lastPoint.x} ${baselineY} L ${firstPoint.x} ${baselineY} Z`;
}

function buildRoundedBarPath(x: number, y: number, width: number, height: number, radius: number): string {
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

export function SvgLineChart<T extends object>({
  data,
  labelKey,
  series,
  ariaLabel,
  showXAxis = true,
  showYAxis = true,
  showGrid = true,
  heightClassName = 'h-56',
  valueFormatter = formatAxisValue,
}: SvgLineChartProps<T>) {
  if (data.length === 0 || series.length === 0) {
    return <div className={`${heightClassName} flex items-center justify-center text-sm text-gray-500`}>No chart data</div>;
  }

  const marginTop = 12;
  const marginRight = 12;
  const marginBottom = showXAxis ? 42 : 12;
  const marginLeft = showYAxis ? 44 : 12;
  const innerWidth = SVG_WIDTH - marginLeft - marginRight;
  const innerHeight = SVG_HEIGHT - marginTop - marginBottom;
  const yMax = Math.max(
    1,
    ...data.flatMap(point => series.map(line => Number((point as Record<string, number | string | undefined>)[line.key] ?? 0))),
  );
  const baselineY = marginTop + innerHeight;
  const xStep = data.length > 1 ? innerWidth / (data.length - 1) : innerWidth / 2;
  const tickValues = Array.from({ length: GRID_LINES + 1 }, (_, index) => Math.round((yMax / GRID_LINES) * index));
  const xTickStep = Math.max(1, Math.ceil(data.length / 6));

  return (
    <div className={`${heightClassName} w-full overflow-hidden rounded-lg border border-gray-800 bg-gray-950/40 p-3`}>
      <svg
        viewBox={`0 0 ${SVG_WIDTH} ${SVG_HEIGHT}`}
        role="img"
        aria-label={ariaLabel}
        className="h-full w-full"
        preserveAspectRatio="none"
      >
        <defs>
          {series.filter(line => line.area).map(line => (
            <linearGradient key={line.key} id={`gradient-${line.key}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={line.color} stopOpacity={String(line.fillOpacity ?? 0.25)} />
              <stop offset="95%" stopColor={line.color} stopOpacity="0" />
            </linearGradient>
          ))}
        </defs>

        {showGrid && tickValues.map((tickValue, index) => {
          const y = marginTop + innerHeight - (index / GRID_LINES) * innerHeight;

          return (
            <g key={tickValue}>
              <line x1={marginLeft} y1={y} x2={SVG_WIDTH - marginRight} y2={y} stroke="#374151" strokeDasharray="4 4" />
              {showYAxis && (
                <text x={marginLeft - 8} y={y + 4} fill="#9CA3AF" fontSize="10" textAnchor="end">
                  {valueFormatter(tickValue)}
                </text>
              )}
            </g>
          );
        })}

        <line x1={marginLeft} y1={baselineY} x2={SVG_WIDTH - marginRight} y2={baselineY} stroke="#374151" />

        {series.filter(line => line.area).map(line => {
          const points = data.map((point, index) => {
            const value = Number((point as Record<string, number | string | undefined>)[line.key] ?? 0);

            return {
              x: marginLeft + (data.length > 1 ? index * xStep : innerWidth / 2),
              y: marginTop + innerHeight - (value / yMax) * innerHeight,
            };
          });

          return (
            <path
              key={`${line.key}-area`}
              d={buildAreaPath(points, baselineY)}
              fill={`url(#gradient-${line.key})`}
              stroke="none"
            />
          );
        })}

        {series.map(line => {
          const points = data.map((point, index) => {
            const value = Number((point as Record<string, number | string | undefined>)[line.key] ?? 0);

            return {
              x: marginLeft + (data.length > 1 ? index * xStep : innerWidth / 2),
              y: marginTop + innerHeight - (value / yMax) * innerHeight,
              value,
              label: String(point[labelKey] ?? ''),
            };
          });

          return (
            <g key={line.key}>
              <path
                d={buildLinePath(points)}
                fill="none"
                stroke={line.color}
                strokeWidth="2"
                strokeDasharray={line.dashed ? '6 4' : undefined}
                strokeLinejoin="round"
                strokeLinecap="round"
              />
              {points.map(point => (
                <circle key={`${line.key}-${point.x}`} cx={point.x} cy={point.y} r="5" fill="transparent">
                  <title>{`${line.label}: ${valueFormatter(point.value)} at ${point.label}`}</title>
                </circle>
              ))}
            </g>
          );
        })}

        {showXAxis && data.map((point, index) => {
          if (index % xTickStep !== 0 && index !== data.length - 1) {
            return null;
          }

          const x = marginLeft + (data.length > 1 ? index * xStep : innerWidth / 2);

          return (
            <text key={`${point[labelKey]}-${index}`} x={x} y={SVG_HEIGHT - 10} fill="#9CA3AF" fontSize="10" textAnchor="middle">
              {String(point[labelKey] ?? '')}
            </text>
          );
        })}
      </svg>
    </div>
  );
}

export function SvgBarChart({
  data,
  ariaLabel,
  color = '#3B82F6',
  heightClassName = 'h-48',
  valueFormatter = formatAxisValue,
}: SvgBarChartProps) {
  if (data.length === 0) {
    return <div className={`${heightClassName} flex items-center justify-center text-sm text-gray-500`}>No chart data</div>;
  }

  const marginTop = 12;
  const marginRight = 12;
  const marginBottom = 52;
  const marginLeft = 44;
  const innerWidth = SVG_WIDTH - marginLeft - marginRight;
  const innerHeight = SVG_HEIGHT - marginTop - marginBottom;
  const maxValue = Math.max(...data.map(entry => entry.value), 1);
  const segmentWidth = innerWidth / Math.max(data.length, 1);
  const barWidth = Math.max(16, segmentWidth * 0.68);
  const tickValues = Array.from({ length: GRID_LINES + 1 }, (_, index) => Math.round((maxValue / GRID_LINES) * index));

  return (
    <div className={`${heightClassName} w-full overflow-hidden rounded-lg border border-gray-800 bg-gray-950/40 p-3`}>
      <svg
        viewBox={`0 0 ${SVG_WIDTH} ${SVG_HEIGHT}`}
        role="img"
        aria-label={ariaLabel}
        className="h-full w-full"
        preserveAspectRatio="none"
      >
        {tickValues.map((tickValue, index) => {
          const y = marginTop + innerHeight - (index / GRID_LINES) * innerHeight;

          return (
            <g key={tickValue}>
              <line x1={marginLeft} y1={y} x2={SVG_WIDTH - marginRight} y2={y} stroke="#374151" strokeDasharray="4 4" />
              <text x={marginLeft - 8} y={y + 4} fill="#9CA3AF" fontSize="10" textAnchor="end">
                {valueFormatter(tickValue)}
              </text>
            </g>
          );
        })}

        <line x1={marginLeft} y1={marginTop + innerHeight} x2={SVG_WIDTH - marginRight} y2={marginTop + innerHeight} stroke="#374151" />

        {data.map((entry, index) => {
          const barHeight = Math.max(2, (entry.value / maxValue) * innerHeight);
          const x = marginLeft + index * segmentWidth + (segmentWidth - barWidth) / 2;
          const y = marginTop + innerHeight - barHeight;
          const labelX = marginLeft + index * segmentWidth + segmentWidth / 2;

          return (
            <g key={`${entry.label}-${index}`}>
              <path d={buildRoundedBarPath(x, y, barWidth, barHeight, 4)} fill={color} fillOpacity="0.82">
                <title>{entry.title ?? `${entry.label}: ${valueFormatter(entry.value)}`}</title>
              </path>
              <text
                x={labelX}
                y={SVG_HEIGHT - 10}
                fill="#9CA3AF"
                fontSize="10"
                textAnchor="end"
                transform={`rotate(-30 ${labelX} ${SVG_HEIGHT - 10})`}
              >
                {entry.label}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}