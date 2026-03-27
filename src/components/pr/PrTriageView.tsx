import { listen } from '@tauri-apps/api/event';
import { ArrowLeft, RefreshCw } from 'lucide-solid';
import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { getActivityStore } from '../../lib/stores/activity';
import { getLayoutStore } from '../../lib/stores/layout';
import { getLifecycleStore, groupTriagePrs } from '../../lib/stores/lifecycle';
import { createOvernightSummary } from '../../lib/stores/overnight';
import { getSATStore } from '../../lib/stores/sat';
import type { TriagePr } from '../../types/lifecycle';
import { SummaryStatsBar } from '../ui/SummaryStatsBar';
import { AnalysisCard } from './AnalysisCard';
import { TriageCard } from './TriageCard';
import { TriageNewRow } from './TriageNewRow';

type RepoGroup = { repo: string; items: TriagePr[] };

function groupByRepo(items: TriagePr[]): RepoGroup[] {
  const map = new Map<string, TriagePr[]>();
  for (const item of items) {
    const repo = item.pr?.repoName?.split('/').pop() ?? 'unknown';
    if (!map.has(repo)) map.set(repo, []);
    map.get(repo)?.push(item);
  }
  return Array.from(map.entries()).map(([repo, items]) => ({ repo, items }));
}

type RunStepEvent = {
  sessionId: string;
  description: string;
};

export function PrTriageView() {
  const layout = getLayoutStore();
  const lifecycleStore = getLifecycleStore();
  const activityStore = getActivityStore();
  const satStore = getSATStore();

  const overnight = createOvernightSummary({
    getEvents: () => activityStore.events(),
    getTriagePrs: () => lifecycleStore.getTriagePrs(),
    getCurrentScore: () => satStore.currentScore(),
    getPreviousScore: () => {
      const c = satStore.cycles();
      if (c.length < 2) return satStore.currentScore();
      return c[c.length - 2].score;
    },
  });

  const [lastSteps, setLastSteps] = createSignal<Record<string, RunStepEvent>>({});
  const [tickMs, setTickMs] = createSignal(Date.now());
  const [showFailingOnly, setShowFailingOnly] = createSignal(false);

  let unlistenStep: (() => void) | null = null;
  let tickInterval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    lifecycleStore.startListening();
    lifecycleStore.loadInitial();
    overnight.recordSessionStart();

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
    if (!showFailingOnly()) return groups().newPrs;
    return groups().newPrs.filter(
      (item) => item.pr?.ciStatus === 'FAILURE' || item.pr?.ciStatus === 'ERROR',
    );
  });

  const hasFailingPrs = createMemo(() =>
    groups().newPrs.some(
      (item) => item.pr?.ciStatus === 'FAILURE' || item.pr?.ciStatus === 'ERROR',
    ),
  );

  const totalPrs = createMemo(
    () =>
      groups().needsAttention.length +
      groups().inProgress.length +
      groups().watching.length +
      groups().newPrs.length +
      groups().done.length,
  );

  function hasDiscoveredPr(item: TriagePr): item is TriagePr & { pr: NonNullable<TriagePr['pr']> } {
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
        <span class="text-lg font-semibold text-text-main">PR TRIAGE</span>
        <button
          type="button"
          class="p-1 text-text-dim hover:text-text-main cursor-pointer"
          onClick={() => lifecycleStore.loadInitial()}
          title="Force refresh"
        >
          <RefreshCw size={14} />
        </button>
      </div>

      {/* Overnight summary bar */}
      <Show when={overnight.hasActivity()}>
        <SummaryStatsBar
          stats={overnight.summaryItems()}
          class="mb-3 bg-bg-surface border border-border-subtle"
        />
      </Show>

      {/* Summary counts */}
      <div class="text-sm text-text-dim mb-4">
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
                    // biome-ignore lint/style/noNonNullAssertion: guarded by Show when={item.lifecycle && item.analysis}
                    worktreePath={item.lifecycle!.worktreePath}
                    // biome-ignore lint/style/noNonNullAssertion: guarded by Show when={item.lifecycle && item.analysis}
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
        <div class="mb-6 pt-4 border-t border-border-subtle/30">
          <h3 class="text-xs text-text-dim uppercase tracking-wider mb-2">
            IN PROGRESS ({groups().inProgress.length})
          </h3>
          <div>
            <For each={groupByRepo(groups().inProgress)}>
              {(group) => (
                <>
                  <div class="px-3 py-1 text-xs text-text-dim uppercase tracking-wider bg-bg-main/30 border-b border-border-subtle/50">
                    {group.repo}
                  </div>
                  <For each={group.items}>
                    {(item) => (
                      <TriageCard
                        item={item}
                        lastStep={
                          item.currentSessionId ? lastSteps()[item.currentSessionId] : undefined
                        }
                        tickMs={tickMs()}
                      />
                    )}
                  </For>
                </>
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
          <div>
            <For each={groupByRepo(groups().watching)}>
              {(group) => (
                <>
                  <div class="px-3 py-1 text-xs text-text-dim uppercase tracking-wider bg-bg-main/30 border-b border-border-subtle/50">
                    {group.repo}
                  </div>
                  <For each={group.items}>
                    {(item) => <TriageCard item={item} lastStep={undefined} tickMs={tickMs()} />}
                  </For>
                </>
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* New */}
      <Show when={groups().newPrs.length > 0 || totalPrs() === 0}>
        <div class="mb-6 pt-4 border-t border-border-subtle/30">
          <div class="flex items-center gap-2 mb-2">
            <h3 class="text-xs text-text-dim uppercase tracking-wider">
              NEW ({filteredNew().length})
            </h3>
            <Show when={hasFailingPrs()}>
              <button
                type="button"
                class="text-xs text-text-dim cursor-pointer hover:text-text-main"
                onClick={() => setShowFailingOnly(!showFailingOnly())}
              >
                {showFailingOnly() ? `Show all (${groups().newPrs.length})` : 'Failing only'}
              </button>
            </Show>
          </div>
          <div>
            <For each={groupByRepo(filteredNew())}>
              {(group) => (
                <>
                  <div class="px-3 py-1 text-xs text-text-dim uppercase tracking-wider bg-bg-main/30 border-b border-border-subtle/50">
                    {group.repo}
                  </div>
                  <For each={group.items}>
                    {(item) => (
                      <Show when={hasDiscoveredPr(item)}>
                        <TriageNewRow
                          item={item as TriagePr & { pr: NonNullable<TriagePr['pr']> }}
                        />
                      </Show>
                    )}
                  </For>
                </>
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
        <div class="mb-6 pt-4 border-t border-border-subtle/30">
          <h3 class="text-xs text-text-dim uppercase tracking-wider mb-2">
            DONE ({groups().done.length})
          </h3>
          <div>
            <For each={groups().done}>
              {(item) => (
                <div class="px-3 py-1.5 flex items-center gap-2 text-base border-b border-border-subtle/50">
                  <span class="inline-block w-2 h-2 shrink-0 bg-accent-success" />
                  <Show when={item.pr}>
                    <span class="text-sm text-accent-info shrink-0">#{item.pr?.number}</span>
                  </Show>
                  <span class="text-text-dim truncate">{item.pr?.branch ?? item.prKey}</span>
                  <Show when={item.pr}>
                    <span class="text-xs text-text-dim shrink-0">
                      {item.pr?.repoName.split('/').pop()}
                    </span>
                  </Show>
                  <span class="ml-auto text-xs text-accent-success uppercase">Done</span>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  );
}
