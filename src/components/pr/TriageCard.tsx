import { Show } from 'solid-js';
import { skipPr } from '../../lib/commands/lifecycle';
import { LIFECYCLE_STATUS_LABELS } from '../../lib/constants/lifecycle';
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

function StatusDot(props: { status: string | undefined }) {
  const color = () => {
    const s = props.status;
    if (s === 'running' || s === 'fixing' || s === 'retrying') return 'bg-accent-warning';
    if (s === 'stale') return 'bg-text-dim/40';
    if (s === 'approved') return 'bg-accent-info';
    return 'bg-text-dim/40';
  };
  return <span class={`inline-block w-2 h-2 shrink-0 ${color()}`} />;
}

export function TriageCard(props: TriageCardProps) {
  const status = () => props.item.lifecycle?.status;
  const label = () => (status() ? LIFECYCLE_STATUS_LABELS[status()!] : '');

  const branchName = () => props.item.pr?.branch ?? props.item.prKey;

  const elapsed = () => {
    const startedAt = props.item.lifecycle?.startedAt;
    if (!startedAt || startedAt === 0) return '';
    return formatElapsed(props.tickMs - startedAt);
  };

  const stepText = () => props.lastStep?.description ?? 'Waiting in queue...';

  const isInProgress = () =>
    status() === 'running' ||
    status() === 'fixing' ||
    status() === 'approved' ||
    status() === 'retrying';

  const repoShort = () => props.item.pr?.repoName?.split('/').pop() ?? '';

  const statusColor = () => {
    const s = status();
    if (s === 'running' || s === 'fixing' || s === 'retrying') return 'text-accent-warning';
    if (s === 'stale') return 'text-text-dim';
    if (s === 'approved') return 'text-accent-info';
    return 'text-text-dim';
  };

  return (
    <div class="px-3 py-1.5 hover:bg-[var(--color-surface-raised)] flex items-center gap-2 text-base border-b border-border-subtle/50 transition-colors duration-150">
      {/* Status dot */}
      <StatusDot status={status()} />

      {/* Left: branch + step text */}
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <Show when={props.item.pr}>
            <span class="text-sm text-accent-info shrink-0">#{props.item.pr!.number}</span>
          </Show>
          <span class="text-text-main truncate font-medium">{branchName()}</span>
        </div>
        <Show when={isInProgress()}>
          <div class="text-xs text-text-dim truncate mt-0.5">{stepText()}</div>
        </Show>
        <Show when={status() === 'stale'}>
          <div class="text-xs text-text-dim mt-0.5">CI now passing</div>
        </Show>
      </div>

      {/* Right: status + elapsed + action */}
      <div class="flex items-center gap-2 shrink-0">
        <Show when={elapsed()}>
          <span class="text-xs text-text-dim font-mono">{elapsed()}</span>
        </Show>
        <span class={`text-xs font-medium uppercase ${statusColor()}`}>{label()}</span>
        <Show when={status() === 'stale'}>
          <button
            type="button"
            class="text-xs text-text-dim hover:text-text-main border border-border-subtle px-2 py-0.5 cursor-pointer"
            onClick={() => skipPr(props.item.prKey).catch(() => {})}
          >
            Dismiss
          </button>
        </Show>
      </div>
    </div>
  );
}
