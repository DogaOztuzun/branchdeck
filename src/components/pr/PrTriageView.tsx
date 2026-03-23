import { listen } from '@tauri-apps/api/event';
import { ArrowLeft, RefreshCw } from 'lucide-solid';
import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { getLayoutStore } from '../../lib/stores/layout';
import { getLifecycleStore, groupTriagePrs } from '../../lib/stores/lifecycle';
import type { TriagePr } from '../../types/lifecycle';
import { AnalysisCard } from './AnalysisCard';
import { TriageCard } from './TriageCard';
import { TriageNewRow } from './TriageNewRow';

type RunStepEvent = {
  sessionId: string;
  description: string;
};

export function PrTriageView() {
  const layout = getLayoutStore();
  const lifecycleStore = getLifecycleStore();

  const [lastSteps, setLastSteps] = createSignal<Record<string, RunStepEvent>>({});
  const [tickMs, setTickMs] = createSignal(Date.now());
  const [showAllNew, setShowAllNew] = createSignal(false);

  let unlistenStep: (() => void) | null = null;
  let tickInterval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    lifecycleStore.startListening();
    lifecycleStore.loadInitial();

    listen<RunStepEvent>('run:step', (e) => {
      const step = e.payload;
      setLastSteps((prev) => ({ ...prev, [step.sessionId]: step }));
    }).then((fn) => {
      unlistenStep = fn;
    });

    tickInterval = setInterval(() => setTickMs(Date.now()), 1000);
  });

  onCleanup(() => {
    unlistenStep?.();
    if (tickInterval) clearInterval(tickInterval);
  });

  const groups = createMemo(() => groupTriagePrs(lifecycleStore.getTriagePrs()));

  const filteredNew = createMemo(() => {
    if (showAllNew()) return groups().newPrs;
    return groups().newPrs.filter(
      (item) =>
        item.pr?.ciStatus === 'FAILURE' || item.pr?.ciStatus === 'ERROR',
    );
  });

  const totalPrs = createMemo(
    () =>
      groups().needsAttention.length +
      groups().inProgress.length +
      groups().watching.length +
      groups().newPrs.length +
      groups().done.length,
  );

  function hasDiscoveredPr(
    item: TriagePr,
  ): item is TriagePr & { pr: NonNullable<TriagePr['pr']> } {
    return item.pr !== undefined;
  }

  return (
    <div class="flex-1 overflow-y-auto p-4">
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
        <span class="text-[16px] font-semibold text-text-main">PR TRIAGE</span>
        <button
          type="button"
          class="p-1 text-text-dim hover:text-text-main cursor-pointer"
          onClick={() => lifecycleStore.loadInitial()}
          title="Force refresh"
        >
          <RefreshCw size={14} />
        </button>
      </div>

      {/* Summary counts */}
      <div class="text-[13px] text-text-dim mb-4">
        <Show when={groups().needsAttention.length > 0}>
          <span class="text-accent-error">{groups().needsAttention.length} need attention</span>
          <span class="mx-1">·</span>
        </Show>
        <Show when={groups().inProgress.length > 0}>
          <span>{groups().inProgress.length} in progress</span>
          <span class="mx-1">·</span>
        </Show>
        <Show when={groups().watching.length > 0}>
          <span>{groups().watching.length} watching</span>
          <span class="mx-1">·</span>
        </Show>
        <span>{groups().newPrs.length} new</span>
        <Show when={groups().done.length > 0}>
          <span class="mx-1">·</span>
          <span>{groups().done.length} done</span>
        </Show>
      </div>

      {/* Empty state */}
      <Show when={totalPrs() === 0}>
        <div class="flex items-center justify-center h-48 text-text-dim text-base">
          No pull requests. Add repositories with open PRs to start triaging.
        </div>
      </Show>

      {/* Needs Attention */}
      <Show when={groups().needsAttention.length > 0}>
        <div class="mb-6">
          <h3 class="text-xs text-accent-error uppercase tracking-wider mb-2">
            NEEDS ATTENTION ({groups().needsAttention.length})
          </h3>
          <div class="space-y-3">
            <For each={groups().needsAttention}>
              {(item) => (
                <Show when={item.lifecycle && item.analysis}>
                  <AnalysisCard
                    prKey={item.prKey}
                    worktreePath={item.lifecycle!.worktreePath}
                    analysis={item.analysis!}
                  />
                </Show>
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* In Progress */}
      <Show when={groups().inProgress.length > 0}>
        <div class="mb-6">
          <h3 class="text-xs text-text-dim uppercase tracking-wider mb-2">
            IN PROGRESS ({groups().inProgress.length})
          </h3>
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            <For each={groups().inProgress}>
              {(item) => (
                <TriageCard
                  item={item}
                  lastStep={item.currentSessionId ? lastSteps()[item.currentSessionId] : undefined}
                  tickMs={tickMs()}
                />
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* Watching */}
      <Show when={groups().watching.length > 0}>
        <div class="mb-6">
          <h3 class="text-xs text-text-dim uppercase tracking-wider mb-2">
            WATCHING ({groups().watching.length})
          </h3>
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            <For each={groups().watching}>
              {(item) => (
                <TriageCard item={item} lastStep={undefined} tickMs={tickMs()} />
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* New */}
      <Show when={groups().newPrs.length > 0 || totalPrs() === 0}>
        <div class="mb-6">
          <div class="flex items-center gap-2 mb-2">
            <h3 class="text-xs text-text-dim uppercase tracking-wider">
              NEW ({filteredNew().length})
            </h3>
            <Show when={groups().newPrs.length > filteredNew().length || showAllNew()}>
              <button
                type="button"
                class="text-xs text-text-dim cursor-pointer hover:text-text-main"
                onClick={() => setShowAllNew(!showAllNew())}
              >
                {showAllNew() ? 'Failing only' : `Show all (${groups().newPrs.length})`}
              </button>
            </Show>
          </div>
          <div class="divide-y divide-border-subtle/30">
            <For each={filteredNew()}>
              {(item) => (
                <Show when={hasDiscoveredPr(item)}>
                  <TriageNewRow item={item as TriagePr & { pr: NonNullable<TriagePr['pr']> }} />
                </Show>
              )}
            </For>
          </div>
          <Show when={filteredNew().length === 0 && groups().newPrs.length === 0}>
            <div class="px-3 py-4 text-base text-text-dim">
              No new PRs discovered yet. Waiting for poller...
            </div>
          </Show>
        </div>
      </Show>

      {/* Done */}
      <Show when={groups().done.length > 0}>
        <div class="mb-6">
          <h3 class="text-xs text-text-dim uppercase tracking-wider mb-2">
            DONE ({groups().done.length})
          </h3>
          <div class="flex flex-wrap gap-1.5">
            <For each={groups().done}>
              {(item) => (
                <span class="bg-bg-main border border-border-subtle px-1.5 py-0.5 text-xs text-text-dim inline-flex items-center gap-1">
                  {item.pr?.branch ?? item.prKey}
                  <span class="text-accent-success">&#10003;</span>
                </span>
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  );
}
