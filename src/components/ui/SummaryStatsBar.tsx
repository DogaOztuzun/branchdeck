import { For, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { SummaryStatItem } from '../../types/ui';
import { Sparkline } from './Sparkline';

type SummaryStatsBarProps = {
  stats: SummaryStatItem[];
  class?: string;
};

const colorClasses: Record<string, string> = {
  primary: 'text-accent-primary',
  success: 'text-accent-success',
  warning: 'text-accent-warning',
  error: 'text-accent-error',
  info: 'text-accent-info',
};

export function SummaryStatsBar(props: SummaryStatsBarProps) {
  return (
    <Show when={props.stats.length > 0}>
      <div
        class={cn(
          'flex h-9 items-center gap-0 px-3 text-base font-normal text-text-main',
          props.class,
        )}
      >
        <For each={props.stats}>
          {(stat, i) => (
            <>
              <Show when={i() > 0}>
                <span class="mx-3 text-text-dim">|</span>
              </Show>
              <span class="inline-flex items-center gap-1.5">
                {stat.label}{' '}
                <span class={stat.color ? (colorClasses[stat.color] ?? '') : 'text-accent-primary'}>
                  {stat.value}
                </span>
                <Show when={stat.sparkline && stat.sparkline.data.length >= 2}>
                  <Sparkline
                    data={stat.sparkline?.data.map((score, i) => ({ cycle: i + 1, score })) ?? []}
                    color={stat.sparkline?.color}
                    class="inline-block align-middle"
                  />
                </Show>
              </span>
            </>
          )}
        </For>
      </div>
    </Show>
  );
}
