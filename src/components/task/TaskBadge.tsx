import { Show } from 'solid-js';
import type { TaskStatus } from '../../types/task';

type TaskBadgeProps = {
  status: TaskStatus;
};

function badgeClasses(status: TaskStatus): string {
  switch (status) {
    case 'created':
      return 'text-zinc-400';
    case 'running':
      return 'text-blue-400';
    case 'blocked':
      return 'text-yellow-400';
    case 'succeeded':
      return 'text-emerald-400';
    case 'failed':
      return 'text-red-400';
    case 'cancelled':
      return 'text-zinc-500';
  }
}

function dotBg(status: TaskStatus): string {
  switch (status) {
    case 'created':
      return 'bg-zinc-400';
    case 'running':
      return 'bg-blue-400';
    case 'blocked':
      return 'bg-yellow-400';
    case 'succeeded':
      return 'bg-emerald-400';
    case 'failed':
      return 'bg-red-400';
    case 'cancelled':
      return 'bg-zinc-500';
  }
}

export function TaskBadge(props: TaskBadgeProps) {
  return (
    <span
      class={`inline-flex items-center gap-1 shrink-0 text-[10px] font-medium ${badgeClasses(props.status)}`}
    >
      <span class="relative flex h-1.5 w-1.5">
        <Show when={props.status === 'running'}>
          <span
            class={`absolute inline-flex h-full w-full rounded-full opacity-75 animate-ping ${dotBg(props.status)}`}
          />
        </Show>
        <span class={`relative inline-flex rounded-full h-1.5 w-1.5 ${dotBg(props.status)}`} />
      </span>
      {props.status}
    </span>
  );
}
