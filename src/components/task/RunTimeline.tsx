import { createMemo, createSignal, For, onCleanup, Show } from 'solid-js';
import type { RunLogEntry } from '../../lib/stores/task';
import type { RunInfo } from '../../types/run';

type RunTimelineProps = {
  entries: RunLogEntry[];
  visible: boolean;
  activeRun?: RunInfo | null;
};

function typeLabel(type: RunLogEntry['type']): string {
  switch (type) {
    case 'run_step':
      return 'Step';
    case 'assistant_text':
      return 'Text';
    case 'tool_call':
      return 'Tool';
    case 'status_change':
      return 'Status';
  }
}

function typeColor(type: RunLogEntry['type']): string {
  switch (type) {
    case 'run_step':
      return 'text-accent-primary';
    case 'assistant_text':
      return 'text-text-dim';
    case 'tool_call':
      return 'text-accent-info';
    case 'status_change':
      return 'text-accent-warning';
  }
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function formatDuration(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return m > 0 ? `${m}m ${s}s` : `${s}s`;
}

function formatCost(usd: number): string {
  return `$${usd.toFixed(4)}`;
}

function statusText(status: string): string {
  switch (status) {
    case 'running':
      return 'Running';
    case 'blocked':
      return 'Blocked';
    case 'succeeded':
      return 'Succeeded';
    case 'failed':
      return 'Failed';
    case 'starting':
      return 'Starting';
    case 'cancelled':
      return 'Cancelled';
    default:
      return status;
  }
}

function EntryDetail(props: { entry: RunLogEntry }) {
  return (
    <Show
      when={props.entry.type === 'tool_call'}
      fallback={
        <Show
          when={props.entry.type === 'assistant_text'}
          fallback={<span class="text-text-main truncate">{props.entry.detail}</span>}
        >
          <span class="text-text-main truncate italic">{props.entry.detail}</span>
        </Show>
      }
    >
      <ToolCallDetail detail={props.entry.detail} />
    </Show>
  );
}

function ToolCallDetail(props: { detail: string }) {
  const parts = createMemo(() => {
    const spaceIdx = props.detail.indexOf(' ');
    if (spaceIdx === -1) return { tool: props.detail, path: null };
    return {
      tool: props.detail.slice(0, spaceIdx),
      path: props.detail.slice(spaceIdx + 1),
    };
  });

  return (
    <span class="truncate">
      <span class="text-accent-info font-semibold">{parts().tool}</span>
      <Show when={parts().path}>
        <span class="text-text-dim ml-1">{parts().path}</span>
      </Show>
    </span>
  );
}

function CostBadge(props: { detail: string }) {
  const cost = createMemo(() => {
    const match = props.detail.match(/\(\$[\d.]+\)/);
    return match ? match[0] : null;
  });

  return (
    <Show when={cost()} fallback={<span class="text-text-main truncate">{props.detail}</span>}>
      {(c) => (
        <span class="text-text-main truncate">
          {props.detail.replace(c(), '').trim()}{' '}
          <span class="text-accent-primary font-medium">{c()}</span>
        </span>
      )}
    </Show>
  );
}

function RunHeader(props: { activeRun: RunInfo }) {
  const initialSecs = () => {
    const startMs = new Date(props.activeRun.startedAt).getTime();
    if (Number.isNaN(startMs)) return 0;
    return Math.max(0, Math.floor((Date.now() - startMs) / 1000));
  };

  const [elapsed, setElapsed] = createSignal(initialSecs());

  const isActive = () => {
    const s = props.activeRun.status;
    return s === 'running' || s === 'starting' || s === 'blocked';
  };

  const interval = setInterval(() => {
    if (isActive()) {
      setElapsed((prev) => prev + 1);
    }
  }, 1000);

  onCleanup(() => clearInterval(interval));

  return (
    <div class="flex items-center justify-between px-3 py-1.5 bg-bg-sidebar-alt border-b border-border-subtle">
      <span class="text-xs text-text-main">
        {statusText(props.activeRun.status)}{' '}
        <span class="text-text-dim">&mdash; {formatDuration(elapsed())}</span>
      </span>
      <span class="text-xs text-accent-primary font-medium">
        {formatCost(props.activeRun.costUsd)}
      </span>
    </div>
  );
}

export function RunTimeline(props: RunTimelineProps) {
  const reversed = createMemo(() => [...props.entries].reverse());

  return (
    <Show when={props.visible}>
      <div class="border-t border-border-subtle bg-bg-sidebar">
        <Show when={props.activeRun}>{(run) => <RunHeader activeRun={run()} />}</Show>
        <div class="flex items-center justify-between px-3 py-1 border-b border-border-subtle">
          <span class="text-[10px] text-text-dim uppercase tracking-wider">Run Timeline</span>
          <span class="text-xs text-text-dim">{props.entries.length} events</span>
        </div>
        <div class="overflow-y-auto max-h-32">
          <Show
            when={reversed().length > 0}
            fallback={<div class="px-3 py-2 text-xs text-text-dim">No run activity yet.</div>}
          >
            <For each={reversed()}>
              {(entry) => (
                <div class="flex items-baseline gap-2 px-3 py-0.5 text-xs hover:bg-bg-main/30">
                  <span class="text-text-dim shrink-0 w-16">{formatTime(entry.ts)}</span>
                  <span class={`shrink-0 w-10 ${typeColor(entry.type)}`}>
                    {typeLabel(entry.type)}
                  </span>
                  <Show
                    when={entry.type === 'status_change'}
                    fallback={<EntryDetail entry={entry} />}
                  >
                    <CostBadge detail={entry.detail} />
                  </Show>
                </div>
              )}
            </For>
          </Show>
        </div>
      </div>
    </Show>
  );
}
