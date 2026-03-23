import { Show } from 'solid-js';
import { skipPr } from '../../lib/commands/lifecycle';
import { LIFECYCLE_STATUS_COLORS, LIFECYCLE_STATUS_LABELS } from '../../lib/constants/lifecycle';
import type { TriagePr } from '../../types/lifecycle';

type RunStepEvent = {
  sessionId: string;
  description: string;
};

type TriageCardProps = {
  item: TriagePr;
  lastStep: RunStepEvent | undefined;
  tickMs: number;
};

function formatElapsed(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  return `${Math.floor(m / 60)}h ${m % 60}m`;
}

export function TriageCard(props: TriageCardProps) {
  const status = () => props.item.lifecycle?.status;
  const label = () => (status() ? LIFECYCLE_STATUS_LABELS[status()!] : '');
  const colorClass = () => (status() ? LIFECYCLE_STATUS_COLORS[status()!] : 'text-text-dim');

  const branchName = () =>
    props.item.pr?.branch ?? props.item.prKey;

  const elapsed = () => {
    const startedAt = props.item.lifecycle?.startedAt;
    if (!startedAt || startedAt === 0) return '';
    return formatElapsed(props.tickMs - startedAt);
  };

  const stepText = () =>
    props.lastStep?.description ?? 'Waiting in queue...';

  const isInProgress = () =>
    status() === 'running' ||
    status() === 'fixing' ||
    status() === 'approved' ||
    status() === 'retrying';

  const repoShort = () =>
    props.item.pr?.repoName?.split('/').pop() ?? '';

  return (
    <div
      class="bg-bg-sidebar border border-border-subtle p-3"
      tabIndex={0}
    >
      <div class="flex items-center justify-between gap-2 mb-1">
        <span class="text-base font-medium text-text-main truncate">
          {branchName()}
        </span>
        <span class={`text-[10px] font-medium uppercase shrink-0 ${colorClass()}`}>
          {label()}
        </span>
      </div>

      <Show when={isInProgress()}>
        <div class="text-xs text-text-dim mb-1">
          <Show when={elapsed()}>
            <span class="font-mono">{elapsed()}</span>
            <span class="mx-1">·</span>
          </Show>
          <span>{repoShort()}</span>
          <Show when={props.item.pr}>
            <span class="ml-1">#{props.item.pr!.number}</span>
          </Show>
        </div>
        <div class="bg-bg-main p-1.5 text-base text-text-dim truncate">
          {stepText()}
        </div>
      </Show>

      <Show when={status() === 'stale'}>
        <div class="text-xs text-text-dim mb-2">
          CI now passing
          <Show when={props.item.pr}>
            <span class="ml-1">· {repoShort()} #{props.item.pr!.number}</span>
          </Show>
        </div>
        <button
          type="button"
          class="text-base text-text-dim hover:text-text-main border border-border-subtle px-2 py-0.5 cursor-pointer"
          onClick={() => skipPr(props.item.prKey).catch(() => {})}
        >
          Dismiss
        </button>
      </Show>
    </div>
  );
}
