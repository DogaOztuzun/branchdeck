import { ArrowLeft, RefreshCw } from 'lucide-solid';
import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { getLayoutStore } from '../../lib/stores/layout';
import { getLifecycleStore } from '../../lib/stores/lifecycle';
import type {
  LifecycleEvent,
  TriggerSource,
  WorkflowCycle,
  WorkflowType,
} from '../../types/lifecycle';
import { CycleRow } from './CycleRow';

/** Infer workflow type from lifecycle context */
function inferWorkflowType(event: LifecycleEvent): WorkflowType {
  if (event.status === 'retrying' || event.status === 'fixing') return 'issue-resolution';
  if (event.worktreePath.includes('sat')) return 'sat-scoring';
  if (event.status === 'completed') return 'verification';
  return 'issue-resolution';
}

/** Infer trigger source from lifecycle event */
function inferTriggerSource(event: LifecycleEvent): TriggerSource {
  if (event.attempt > 1) return 'regression';
  return 'pr-poll';
}

/** Convert a LifecycleEvent to a WorkflowCycle for display */
function toCycle(event: LifecycleEvent): WorkflowCycle {
  return {
    id: `${event.prKey}-${event.attempt}`,
    prKey: event.prKey,
    workflowType: inferWorkflowType(event),
    triggerSource: inferTriggerSource(event),
    status: event.status,
    attempt: event.attempt,
    startedAt: event.startedAt,
    updatedAt: event.startedAt,
    completedAt: event.status === 'completed' ? event.startedAt : null,
    worktreePath: event.worktreePath,
    description: event.worktreePath.split('/').pop() ?? event.prKey,
  };
}

export function LifecycleView() {
  const layout = getLayoutStore();
  const lifecycleStore = getLifecycleStore();

  const [tickMs, setTickMs] = createSignal(Date.now());
  let tickInterval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    lifecycleStore.startListening();
    lifecycleStore.loadInitial();
    tickInterval = setInterval(() => setTickMs(Date.now()), 1000);
  });

  onCleanup(() => {
    if (tickInterval) clearInterval(tickInterval);
  });

  const activeCycles = createMemo((): WorkflowCycle[] => {
    const events = Object.values(lifecycleStore.state.lifecycles);
    return events.map(toCycle).sort((a, b) => b.startedAt - a.startedAt);
  });

  const completedCycles = createMemo((): WorkflowCycle[] => {
    const events = Object.values(lifecycleStore.state.completedLifecycles);
    return events.map(toCycle).sort((a, b) => b.startedAt - a.startedAt);
  });

  const totalActive = createMemo(() => activeCycles().length);
  const totalCompleted = createMemo(() => completedCycles().length);
  const totalAll = createMemo(() => totalActive() + totalCompleted());

  const runningCount = createMemo(
    () => activeCycles().filter((c) => c.status === 'running' || c.status === 'fixing').length,
  );

  return (
    <div class="flex-1 overflow-y-auto p-4">
      <div class="max-w-[900px] mx-auto">
        {/* Header */}
        <div class="flex items-center gap-3 mb-4">
          <button
            type="button"
            class="p-1 text-text-dim hover:text-text-main cursor-pointer"
            onClick={() => layout.setActiveView('workspace')}
            title="Back to workspace"
          >
            <ArrowLeft size={16} />
          </button>
          <span class="text-lg font-semibold text-text-main">LIFECYCLE</span>
          <button
            type="button"
            class="p-1 text-text-dim hover:text-text-main cursor-pointer"
            onClick={() => lifecycleStore.loadInitial()}
            title="Force refresh"
          >
            <RefreshCw size={14} />
          </button>
        </div>

        {/* Summary stats bar */}
        <div class="text-[11px] text-text-dim mb-4 flex items-center gap-1">
          <span class="text-text-main">{totalAll()}</span>
          <span>total</span>
          <span class="mx-1">|</span>
          <Show when={runningCount() > 0}>
            <span class="text-[var(--color-warning)]">{runningCount()}</span>
            <span>running</span>
            <span class="mx-1">|</span>
          </Show>
          <span>{totalActive()}</span>
          <span>active</span>
          <span class="mx-1">|</span>
          <span class="text-[var(--color-success)]">{totalCompleted()}</span>
          <span>completed</span>
        </div>

        {/* Empty state */}
        <Show when={totalAll() === 0}>
          <div class="flex flex-col items-center justify-center h-48 text-text-dim">
            <div class="text-base font-semibold mb-2">No workflow cycles yet</div>
            <div class="text-[11px]">
              Cycles appear when the self-healing loop detects issues and starts fixing them.
            </div>
          </div>
        </Show>

        {/* Active cycles */}
        <Show when={totalActive() > 0}>
          <div class="mb-6">
            <h3 class="text-[10px] font-medium text-[var(--color-warning)] uppercase tracking-[0.06em] mb-2">
              ACTIVE ({totalActive()})
            </h3>
            <div>
              <For each={activeCycles()}>
                {(cycle) => <CycleRow cycle={cycle} nowMs={tickMs()} />}
              </For>
            </div>
          </div>
        </Show>

        {/* Completed cycles */}
        <Show when={totalCompleted() > 0}>
          <div class="mb-6 pt-4 border-t border-border-subtle/30">
            <h3 class="text-[10px] font-medium text-[var(--color-success)] uppercase tracking-[0.06em] mb-2">
              COMPLETED ({totalCompleted()})
            </h3>
            <div>
              <For each={completedCycles()}>
                {(cycle) => <CycleRow cycle={cycle} nowMs={tickMs()} />}
              </For>
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
}
