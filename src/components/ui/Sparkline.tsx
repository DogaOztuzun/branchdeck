import type { ChartDataPoint } from '../../types/chart';

type SparklineProps = {
  data: ChartDataPoint[];
  color?: string;
  class?: string;
};

export function Sparkline(props: SparklineProps) {
  const WIDTH = 80;
  const HEIGHT = 24;
  const PAD = 2;

  const points = () => {
    const d = props.data;
    if (d.length === 0) return '';
    const minScore = Math.min(...d.map((p) => p.score));
    const maxScore = Math.max(...d.map((p) => p.score));
    const range = maxScore - minScore || 1;
    return d
      .map((p, i) => {
        const x = PAD + (i / Math.max(d.length - 1, 1)) * (WIDTH - PAD * 2);
        const y = PAD + ((maxScore - p.score) / range) * (HEIGHT - PAD * 2);
        return `${x},${y}`;
      })
      .join(' ');
  };

  return (
    <svg
      width={WIDTH}
      height={HEIGHT}
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      class={props.class}
      aria-label="Sparkline"
      role="img"
    >
      <title>Trend</title>
      <polyline
        points={points()}
        fill="none"
        stroke={props.color ?? '#7aa2f7'}
        stroke-width="1.5"
      />
    </svg>
  );
}
