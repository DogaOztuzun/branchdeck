import { Show } from 'solid-js';
import { cn } from '../../lib/cn';

type EmptyStateProps = {
  message: string;
  detail?: string;
  showIcon?: boolean;
  class?: string;
};

export function EmptyState(props: EmptyStateProps) {
  return (
    <div class={cn('flex flex-col items-center justify-center gap-2 py-16', props.class)}>
      <Show when={props.showIcon !== false}>
        <svg
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="text-accent-success"
          aria-hidden="true"
        >
          <path d="M20 6 9 17l-5-5" />
        </svg>
      </Show>
      <span class="text-lg font-semibold text-text-main">{props.message}</span>
      <Show when={props.detail}>
        <span class="text-sm font-normal text-text-dim">{props.detail}</span>
      </Show>
    </div>
  );
}
