import { Show } from 'solid-js';
import { cn } from '../../lib/cn';
import { LIFECYCLE_STATUS_COLORS, LIFECYCLE_STATUS_LABELS } from '../../lib/constants/lifecycle';
import type { WorkflowCycle } from '../../types/lifecycle';

type CycleRowProps = {
  cycle: WorkflowCycle;
  nowMs: number;
};

const WORKFLOW_TYPE_LABELS: Record<string, string> = {
  'issue-resolution': 'ISSUE FIX',
  'sat-scoring': 'SAT SCORE',
  verification: 'VERIFY',
  manual: 'MANUAL',
};

const TRIGGER_SOURCE_LABELS: Record<string, string> = {
  'pr-poll': 'PR poll',
  'post-merge': 'Post-merge',
  'issue-detected': 'Issue detected',
  retry: 'Retry',
  manual: 'Manual',
};

function formatElapsed(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  return `${Math.floor(m / 60)}h ${m % 60}m`;
}

function formatTimestamp(epochMs: number): string {
  const d = new Date(epochMs);
  const hh = d.getHours().toString().padStart(2, '0');
  const mm = d.getMinutes().toString().padStart(2, '0');
  return `${hh}:${mm}`;
}

function statusDotColor(status: string): string {
  if (status === 'running' || status === 'fixing' || status === 'retrying')
    return 'bg-accent-warning';
  if (status === 'completed') return 'bg-accent-success';
  if (status === 'failed') return 'bg-accent-error';
  if (status === 'reviewReady') return 'bg-accent-primary';
  if (status === 'approved') return 'bg-accent-info';
  return 'bg-text-dim/40';
}

function workflowTypeBorderColor(type: string): string {
  if (type === 'issue-resolution') return 'border-l-accent-error';
  if (type === 'sat-scoring') return 'border-l-accent-primary';
  if (type === 'verification') return 'border-l-accent-success';
  return 'border-l-text-dim/40';
}

export function CycleRow(props: CycleRowProps) {
  const status = () => props.cycle.status;
  const label = () => LIFECYCLE_STATUS_LABELS[status()];
  const statusColor = () => LIFECYCLE_STATUS_COLORS[status()];
  const workflowLabel = () =>
    WORKFLOW_TYPE_LABELS[props.cycle.workflowType] ?? props.cycle.workflowType;
  const triggerLabel = () =>
    TRIGGER_SOURCE_LABELS[props.cycle.triggerSource] ?? props.cycle.triggerSource;

  const elapsed = () => {
    if (!props.cycle.startedAt || props.cycle.startedAt === 0) return '';
    // Only show live elapsed for active cycles; completed cycles lack a backend completedAt
    if (status() === 'completed' || status() === 'failed') return '';
    return formatElapsed(props.nowMs - props.cycle.startedAt);
  };

  const isActive = () =>
    status() === 'running' ||
    status() === 'fixing' ||
    status() === 'retrying' ||
    status() === 'approved';

  return (
    <div
      class={cn(
        'px-3 py-1.5 flex items-center gap-2 text-base border-b border-border-subtle/50 border-l-2 hover:bg-[var(--color-surface-raised)] transition-colors duration-150',
        workflowTypeBorderColor(props.cycle.workflowType),
      )}
    >
      {/* Status dot */}
      <span
        class={cn(
          'inline-block w-2 h-2 shrink-0',
          statusDotColor(status()),
          isActive() ? 'animate-pulse' : '',
        )}
      />

      {/* Workflow type badge */}
      <span class="text-[10px] font-medium uppercase tracking-wider text-text-dim border border-border-subtle px-1.5 py-0 shrink-0">
        {workflowLabel()}
      </span>

      {/* PR key + description */}
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <span class="text-sm text-accent-info shrink-0">{props.cycle.prKey}</span>
          <span class="text-text-main truncate">{props.cycle.description}</span>
        </div>
        <Show when={props.cycle.triggerSource}>
          <div class="text-[11px] text-text-dim mt-0.5">
            Triggered by {triggerLabel()}
            <Show when={props.cycle.attempt > 1}> &middot; attempt {props.cycle.attempt}</Show>
          </div>
        </Show>
      </div>

      {/* Right: timestamps + status */}
      <div class="flex items-center gap-3 shrink-0">
        <Show when={elapsed()}>
          <span class="text-[11px] text-text-dim font-mono">{elapsed()}</span>
        </Show>
        <Show when={props.cycle.startedAt}>
          <span class="text-[11px] text-text-dim">{formatTimestamp(props.cycle.startedAt)}</span>
        </Show>
        <span class={cn('text-xs font-medium uppercase', statusColor())}>{label()}</span>
      </div>
    </div>
  );
}
