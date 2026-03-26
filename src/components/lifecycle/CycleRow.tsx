import { createSignal, For, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import { LIFECYCLE_STATUS_COLORS, LIFECYCLE_STATUS_LABELS } from '../../lib/constants/lifecycle';
import type { LifecycleTimelineEntry, WorkflowCycle } from '../../types/lifecycle';

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

function formatFullTimestamp(epochMs: number): string {
  const d = new Date(epochMs);
  const hh = d.getHours().toString().padStart(2, '0');
  const mm = d.getMinutes().toString().padStart(2, '0');
  const ss = d.getSeconds().toString().padStart(2, '0');
  return `${hh}:${mm}:${ss}`;
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

/** Resolve the status label: custom displayStatus from workflow def takes precedence */
function resolveStatusLabel(cycle: WorkflowCycle): string {
  if (cycle.displayStatus) return cycle.displayStatus;
  return LIFECYCLE_STATUS_LABELS[cycle.status] ?? cycle.status;
}

/** Color class for a timeline entry status */
function timelineStatusColor(status: string): string {
  if (status === 'running' || status === 'fixing' || status === 'retrying')
    return 'text-[var(--color-warning)]';
  if (status === 'completed') return 'text-[var(--color-success)]';
  if (status === 'failed') return 'text-[var(--color-error)]';
  if (status === 'reviewReady') return 'text-accent-primary';
  if (status === 'approved') return 'text-[var(--color-info)]';
  // Custom statuses: use primary color as default
  return 'text-accent-primary';
}

function TimelineEntryRow(props: { entry: LifecycleTimelineEntry }) {
  return (
    <div class="flex items-center gap-3 py-0.5">
      <span class="text-[10px] text-text-dim font-mono shrink-0 w-16">
        {formatFullTimestamp(props.entry.timestamp)}
      </span>
      <span class="inline-block w-1.5 h-1.5 shrink-0 bg-border-subtle" />
      <span class={cn('text-[11px] font-medium', timelineStatusColor(props.entry.status))}>
        {props.entry.displayStatus}
      </span>
      <span class="text-[11px] text-text-dim truncate">{props.entry.detail}</span>
    </div>
  );
}

export function CycleRow(props: CycleRowProps) {
  const [expanded, setExpanded] = createSignal(false);

  const status = () => props.cycle.status;
  const label = () => resolveStatusLabel(props.cycle);
  const statusColor = () => LIFECYCLE_STATUS_COLORS[status()] ?? 'text-accent-primary';
  const workflowLabel = () =>
    WORKFLOW_TYPE_LABELS[props.cycle.workflowType] ?? props.cycle.workflowType;
  const triggerLabel = () =>
    TRIGGER_SOURCE_LABELS[props.cycle.triggerSource] ?? props.cycle.triggerSource;

  const elapsed = () => {
    if (!props.cycle.startedAt || props.cycle.startedAt === 0) return '';
    // Show final elapsed for completed cycles if completedAt is available
    if (props.cycle.completedAt) {
      return formatElapsed(props.cycle.completedAt - props.cycle.startedAt);
    }
    // Only show live elapsed for active cycles
    if (status() === 'completed' || status() === 'failed') return '';
    return formatElapsed(props.nowMs - props.cycle.startedAt);
  };

  const isActive = () =>
    status() === 'running' ||
    status() === 'fixing' ||
    status() === 'retrying' ||
    status() === 'approved';

  const hasTimeline = () => props.cycle.timeline.length > 0;

  return (
    <div>
      {/* Collapsed row: 36px height, UX-DR1 Inbox Row pattern */}
      <button
        type="button"
        class={cn(
          'w-full h-9 px-3 flex items-center gap-2 text-base border-b border-border-subtle/50 border-l-2 cursor-pointer transition-colors duration-150 text-left',
          workflowTypeBorderColor(props.cycle.workflowType),
          expanded()
            ? 'bg-[var(--color-surface-raised)]'
            : 'hover:bg-[var(--color-surface-raised)]',
        )}
        onClick={() => setExpanded(!expanded())}
      >
        {/* Status dot (8x8px square, no border-radius per DESIGN.md) */}
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
            <Show when={props.cycle.workflowName}>
              <span class="text-[10px] text-text-dim shrink-0">{props.cycle.workflowName}</span>
            </Show>
          </div>
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
      </button>

      {/* Expanded detail: timeline + metadata (200ms expand per DESIGN.md) */}
      <Show when={expanded()}>
        <div class="bg-[var(--color-surface-raised)] border-b border-border-subtle/50 border-l-2 border-l-transparent px-3 py-2 transition-all duration-200">
          {/* Trigger + attempt metadata */}
          <div class="text-[11px] text-text-dim mb-2 flex items-center gap-1">
            <span>Triggered by</span>
            <span class="text-text-main">{triggerLabel()}</span>
            <Show when={props.cycle.attempt > 1}>
              <span class="mx-1">&middot;</span>
              <span>attempt {props.cycle.attempt}</span>
            </Show>
            <Show when={props.cycle.startedAt}>
              <span class="mx-1">&middot;</span>
              <span>dispatched {formatTimestamp(props.cycle.startedAt)}</span>
            </Show>
            <Show when={props.cycle.completedAt}>
              <span class="mx-1">&middot;</span>
              <span>completed {formatTimestamp(props.cycle.completedAt as number)}</span>
            </Show>
          </div>

          {/* Lifecycle timeline */}
          <Show when={hasTimeline()}>
            <div class="mb-1">
              <div class="text-[10px] font-medium text-text-dim uppercase tracking-[0.06em] mb-1">
                LIFECYCLE ({props.cycle.timeline.length})
              </div>
              <div class="pl-1">
                <For each={props.cycle.timeline}>
                  {(entry) => <TimelineEntryRow entry={entry} />}
                </For>
              </div>
            </div>
          </Show>

          {/* Empty timeline fallback */}
          <Show when={!hasTimeline()}>
            <div class="text-[11px] text-text-dim">No lifecycle transitions recorded yet.</div>
          </Show>
        </div>
      </Show>
    </div>
  );
}
