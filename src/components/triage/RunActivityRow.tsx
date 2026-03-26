import { createMemo, For, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { AgentLogEntry, SessionAgentInfo } from '../../lib/stores/agent';
import { getAgentStore } from '../../lib/stores/agent';
import { shortPath } from '../../lib/utils';
import { AgentIndicator } from '../ui/AgentIndicator';

type RunActivityRowProps = {
  session: SessionAgentInfo;
  selected: boolean;
  expanded: boolean;
  onClick: () => void;
  tick: number;
};

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function kindLabel(kind: string): string {
  switch (kind) {
    case 'sessionStart':
      return 'Session';
    case 'toolStart':
      return 'Tool';
    case 'toolEnd':
      return 'Done';
    case 'subagentStart':
      return 'Spawn';
    case 'subagentStop':
      return 'End';
    case 'sessionStop':
      return 'Stop';
    case 'notification':
      return 'Note';
    default:
      return kind;
  }
}

function kindColor(kind: string): string {
  switch (kind) {
    case 'sessionStart':
      return 'text-[var(--color-success)]';
    case 'toolStart':
      return 'text-[var(--color-primary)]';
    case 'toolEnd':
      return 'text-text-dim';
    case 'subagentStart':
      return 'text-[var(--color-info)]';
    case 'subagentStop':
      return 'text-[var(--color-info)]';
    case 'sessionStop':
      return 'text-[var(--color-error)]';
    case 'notification':
      return 'text-[var(--color-warning)]';
    default:
      return 'text-text-dim';
  }
}

function entryDetail(entry: AgentLogEntry): string {
  if (entry.toolName) {
    const file = entry.filePath ? ` ${shortPath(entry.filePath, 2)}` : '';
    return `${entry.toolName}${file}`;
  }
  if (entry.message) return entry.message;
  if (entry.agentId) return entry.agentId;
  return '';
}

export function RunActivityRow(props: RunActivityRowProps) {
  const agentStore = getAgentStore();

  const sessionLog = createMemo(() =>
    agentStore.getLogForSession(props.session.sessionId).slice().reverse(),
  );

  const toolSummary = createMemo(() => {
    const log = agentStore.getLogForSession(props.session.sessionId);
    const toolCalls = log.filter((e) => e.kind === 'toolStart');
    const fileMods = log.filter(
      (e) => e.kind === 'toolEnd' && (e.toolName === 'Write' || e.toolName === 'Edit'),
    );
    return { toolCalls: toolCalls.length, fileMods: fileMods.length };
  });

  const elapsedText = createMemo(() => {
    void props.tick; // Track tick signal to trigger re-evaluation
    const elapsed = Date.now() - props.session.startedAt;
    const secs = Math.floor(elapsed / 1000);
    if (secs < 60) return `${secs}s`;
    const mins = Math.floor(secs / 60);
    return `${mins}m ${secs % 60}s`;
  });

  return (
    <div>
      {/* biome-ignore lint/a11y/noStaticElementInteractions: row click */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: keyboard handled by parent */}
      <div
        class={cn(
          'flex items-center h-9 px-3 gap-2 cursor-pointer border-b border-border-subtle transition-colors duration-150',
          props.selected
            ? 'bg-surface-raised border-l-2 border-l-[var(--color-primary)] pl-[10px]'
            : 'hover:bg-surface-raised/50',
        )}
        onClick={props.onClick}
      >
        {/* Status dot */}
        <span
          class={cn(
            'w-2 h-2 shrink-0',
            props.session.status === 'active' || props.session.status === 'idle'
              ? 'bg-[var(--color-warning)]'
              : props.session.status === 'stopped'
                ? 'bg-[var(--color-success)]'
                : 'bg-text-dim opacity-40',
          )}
        />

        {/* Agent indicator */}
        <AgentIndicator status={props.session.status} class="shrink-0" />

        {/* Session ID (truncated) */}
        <span class="text-[11px] text-[var(--color-info)] shrink-0 truncate max-w-24">
          {props.session.sessionId.slice(0, 12)}
        </span>

        {/* Current tool or summary */}
        <span class="text-[12px] text-text-main truncate flex-1">
          <Show
            when={props.session.currentTool}
            fallback={`${toolSummary().toolCalls} tools, ${toolSummary().fileMods} edits`}
          >
            {props.session.currentTool}
            <Show when={props.session.currentFile}>
              <span class="text-text-dim ml-1">
                {shortPath(props.session.currentFile ?? '', 2)}
              </span>
            </Show>
          </Show>
        </span>

        {/* Subagent count */}
        <Show when={props.session.subagentCount > 0}>
          <span class="text-[10px] text-[var(--color-info)]">
            {props.session.subagentCount} sub
          </span>
        </Show>

        {/* Elapsed time */}
        <span class="text-[11px] text-text-dim tabular-nums shrink-0">{elapsedText()}</span>
      </div>

      {/* Expanded detail: tool call timeline */}
      <Show when={props.expanded}>
        <div class="bg-surface-raised border-b border-border-subtle">
          <div class="overflow-y-auto max-h-48">
            <Show
              when={sessionLog().length > 0}
              fallback={
                <div class="px-3 py-2 text-[11px] text-text-dim">No activity recorded yet</div>
              }
            >
              <For each={sessionLog()}>
                {(entry) => (
                  <div class="flex items-baseline gap-2 px-3 py-0.5 text-[12px] hover:bg-bg-main/30">
                    <span class="text-text-dim shrink-0 w-16 text-[11px] tabular-nums">
                      {formatTime(entry.ts)}
                    </span>
                    <span class={`shrink-0 w-12 text-[10px] font-medium ${kindColor(entry.kind)}`}>
                      {kindLabel(entry.kind)}
                    </span>
                    <span class="text-text-main truncate">{entryDetail(entry)}</span>
                  </div>
                )}
              </For>
            </Show>
          </div>
        </div>
      </Show>
    </div>
  );
}
