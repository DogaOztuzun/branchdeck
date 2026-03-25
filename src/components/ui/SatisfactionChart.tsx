import { createSignal, For, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { ChartDataPoint, PersonaLine } from '../../types/chart';

type SatisfactionChartProps = {
  data: ChartDataPoint[];
  personas?: PersonaLine[];
  onPointClick?: (point: ChartDataPoint) => void;
  class?: string;
};

const PADDING = { top: 8, right: 12, bottom: 24, left: 32 };
const HEIGHT = 180;

function scaleX(cycle: number, min: number, max: number, width: number): number {
  if (max === min) return PADDING.left + (width - PADDING.left - PADDING.right) / 2;
  return PADDING.left + ((cycle - min) / (max - min)) * (width - PADDING.left - PADDING.right);
}

function scaleY(score: number): number {
  return PADDING.top + ((100 - score) / 100) * (HEIGHT - PADDING.top - PADDING.bottom);
}

function pointsToPolyline(
  data: ChartDataPoint[],
  minCycle: number,
  maxCycle: number,
  width: number,
): string {
  return data
    .map((p) => `${scaleX(p.cycle, minCycle, maxCycle, width)},${scaleY(p.score)}`)
    .join(' ');
}

function pointsToAreaPolygon(
  data: ChartDataPoint[],
  minCycle: number,
  maxCycle: number,
  width: number,
): string {
  if (data.length === 0) return '';
  const baseline = HEIGHT - PADDING.bottom;
  const first = scaleX(data[0].cycle, minCycle, maxCycle, width);
  const last = scaleX(data[data.length - 1].cycle, minCycle, maxCycle, width);
  const linePoints = data
    .map((p) => `${scaleX(p.cycle, minCycle, maxCycle, width)},${scaleY(p.score)}`)
    .join(' ');
  return `${first},${baseline} ${linePoints} ${last},${baseline}`;
}

export function SatisfactionChart(props: SatisfactionChartProps) {
  const [hoveredIndex, setHoveredIndex] = createSignal<number | null>(null);
  const [hiddenPersonas, setHiddenPersonas] = createSignal<Set<string>>(new Set());
  const [containerWidth, setContainerWidth] = createSignal(600);

  const observe = (el: HTMLDivElement) => {
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });
    ro.observe(el);
  };

  const data = () => props.data;
  const width = () => containerWidth();
  const minCycle = () => Math.min(...data().map((d) => d.cycle));
  const maxCycle = () => Math.max(...data().map((d) => d.cycle));

  const togglePersona = (name: string) => {
    setHiddenPersonas((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  return (
    <div ref={observe} class={cn('w-full', props.class)}>
      <svg
        width={width()}
        height={HEIGHT}
        viewBox={`0 0 ${width()} ${HEIGHT}`}
        class="overflow-visible"
        aria-label="Satisfaction trend chart"
        role="img"
      >
        <title>Satisfaction trend</title>
        {/* Y-axis labels */}
        <text
          x={PADDING.left - 4}
          y={scaleY(100)}
          class="text-xs fill-text-dim"
          text-anchor="end"
          dominant-baseline="middle"
        >
          100
        </text>
        <text
          x={PADDING.left - 4}
          y={scaleY(0)}
          class="text-xs fill-text-dim"
          text-anchor="end"
          dominant-baseline="middle"
        >
          0
        </text>

        {/* X-axis labels */}
        <For each={data()}>
          {(point, i) => (
            <Show when={i() === 0 || i() === data().length - 1 || data().length <= 8}>
              <text
                x={scaleX(point.cycle, minCycle(), maxCycle(), width())}
                y={HEIGHT - 4}
                class="text-xs fill-text-dim"
                text-anchor="middle"
              >
                {point.date ?? point.cycle}
              </text>
            </Show>
          )}
        </For>

        {/* Area fill */}
        <polygon
          points={pointsToAreaPolygon(data(), minCycle(), maxCycle(), width())}
          fill="#7aa2f7"
          opacity="0.08"
        />

        {/* Main line */}
        <polyline
          points={pointsToPolyline(data(), minCycle(), maxCycle(), width())}
          fill="none"
          stroke="#7aa2f7"
          stroke-width="2"
        />

        {/* Persona lines */}
        <Show when={props.personas}>
          <For each={props.personas}>
            {(persona) => (
              <Show when={!hiddenPersonas().has(persona.name)}>
                <polyline
                  points={pointsToPolyline(persona.data, minCycle(), maxCycle(), width())}
                  fill="none"
                  stroke={persona.color}
                  stroke-width="1.5"
                  stroke-dasharray="4,4"
                />
              </Show>
            )}
          </For>
        </Show>

        {/* Data points */}
        <For each={data()}>
          {(point, i) => (
            // biome-ignore lint/a11y/noStaticElementInteractions: SVG circle click handler
            <circle
              cx={scaleX(point.cycle, minCycle(), maxCycle(), width())}
              cy={scaleY(point.score)}
              r={i() === data().length - 1 ? 4 : 3}
              fill="#7aa2f7"
              class="cursor-pointer"
              onMouseEnter={() => setHoveredIndex(i())}
              onMouseLeave={() => setHoveredIndex(null)}
              onClick={() => props.onPointClick?.(point)}
            />
          )}
        </For>
      </svg>

      {/* Tooltip */}
      <Show when={hoveredIndex() !== null}>
        {(_) => {
          const idx = hoveredIndex() ?? 0;
          const point = data()[idx];
          if (!point) return null;
          const x = scaleX(point.cycle, minCycle(), maxCycle(), width());
          const y = scaleY(point.score);
          return (
            <div
              class="absolute bg-surface-raised text-[11px] text-text-main px-2 py-1 pointer-events-none border border-border-subtle"
              style={{
                left: `${x}px`,
                top: `${y - 28}px`,
                transform: 'translateX(-50%)',
              }}
            >
              Cycle {point.cycle}: {point.score}
            </div>
          );
        }}
      </Show>

      {/* Legend */}
      <Show when={props.personas && props.personas.length > 0}>
        <div class="flex items-center gap-4 mt-1 px-8">
          <For each={props.personas}>
            {(persona) => (
              <button
                type="button"
                class={cn(
                  'flex items-center gap-1.5 text-[11px] cursor-pointer',
                  hiddenPersonas().has(persona.name) ? 'opacity-40' : '',
                )}
                onClick={() => togglePersona(persona.name)}
              >
                <span class="inline-block w-3 h-px" style={{ 'background-color': persona.color }} />
                <span class="text-text-dim">{persona.name}</span>
              </button>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
