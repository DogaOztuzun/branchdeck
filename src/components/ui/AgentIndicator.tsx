import { Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { AgentStatus } from '../../types/agent';

type AgentIndicatorProps = {
  status: AgentStatus;
  detail?: string;
  class?: string;
};

function statusColor(status: AgentStatus): string {
  switch (status) {
    case 'active':
      return 'text-[var(--color-warning)]';
    case 'idle':
      return 'text-[var(--color-warning)]';
    case 'stopped':
      return 'text-[var(--color-success)]';
    default:
      return 'text-text-dim';
  }
}

function statusLabel(status: AgentStatus): string {
  switch (status) {
    case 'active':
      return 'Running';
    case 'idle':
      return 'Idle';
    case 'stopped':
      return 'Completed';
    default:
      return status;
  }
}

/**
 * Agent indicator following UX-DR4 pattern.
 * `>_` monospace character + status text.
 * Running state uses pulsing warning opacity (0.4 -> 1.0, 2s cycle).
 */
export function AgentIndicator(props: AgentIndicatorProps) {
  const isRunning = () => props.status === 'active' || props.status === 'idle';

  return (
    <span
      class={cn(
        'inline-flex items-center gap-1 text-[11px] font-medium',
        statusColor(props.status),
        isRunning() && 'animate-pulse-opacity',
        props.class,
      )}
    >
      <span class="font-mono">{'>_'}</span>
      <span>{statusLabel(props.status)}</span>
      <Show when={props.detail}>
        <span class="text-text-dim font-normal">{props.detail}</span>
      </Show>
    </span>
  );
}
