import type { JSX } from 'solid-js';
import { Show } from 'solid-js';
import { cn } from '../../lib/cn';

type SectionHeaderProps = {
  label: string;
  action?: JSX.Element;
  class?: string;
  collapsed?: boolean;
  onToggle?: () => void;
  count?: number;
};

export function SectionHeader(props: SectionHeaderProps) {
  const isCollapsible = () => props.onToggle !== undefined;

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: collapsible section toggle
    <div
      class={cn(
        'flex items-center justify-between px-3 py-1.5',
        isCollapsible() &&
          'cursor-pointer select-none hover:bg-bg-main/30 transition-colors duration-150',
        props.class,
      )}
      onClick={() => props.onToggle?.()}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') props.onToggle?.();
      }}
      role={isCollapsible() ? 'button' : undefined}
      tabIndex={isCollapsible() ? 0 : undefined}
    >
      <div class="flex items-center gap-1.5">
        <Show when={isCollapsible()}>
          <svg
            aria-hidden="true"
            width="10"
            height="10"
            viewBox="0 0 10 10"
            fill="currentColor"
            class={cn(
              'text-text-dim transition-transform duration-150 shrink-0',
              props.collapsed ? '' : 'rotate-90',
            )}
          >
            <path
              d="M3 1.5 L7 5 L3 8.5"
              stroke="currentColor"
              stroke-width="1.5"
              fill="none"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </Show>
        <span class="text-[10px] font-bold uppercase tracking-widest text-text-dim">
          {props.label}
        </span>
        <Show when={props.count !== undefined}>
          <span class="text-[10px] text-text-dim">{props.count}</span>
        </Show>
      </div>
      <Show when={props.action}>{props.action}</Show>
    </div>
  );
}
