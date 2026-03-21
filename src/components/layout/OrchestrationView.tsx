import { listen } from '@tauri-apps/api/event';
import { ArrowLeft, Clock, GitBranch, Square } from 'lucide-solid';
import { createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { cancelQueue, getQueueStatus } from '../../lib/commands/run';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import type { QueuedRun, QueueStatus } from '../../types/github';
import type { RunInfo, RunStatusEvent, RunStepEvent } from '../../types/run';
import { Badge } from '../ui/Badge';
import { Button } from '../ui/Button';

type RunCardStatus = 'running' | 'succeeded' | 'failed' | 'queued' | 'cancelled';

function statusVariant(status: RunCardStatus) {
  switch (status) {
    case 'succeeded':
      return 'success' as const;
    case 'running':
      return 'warning' as const;
    case 'failed':
      return 'error' as const;
    case 'queued':
      return 'info' as const;
    case 'cancelled':
      return 'neutral' as const;
  }
}

function RunCard(props: {
  name: string;
  status: RunCardStatus;
  branch?: string;
  elapsed?: string;
  lastStep?: string;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      class="w-full text-left p-3 bg-bg-main border border-border-subtle hover:border-accent-primary/50 transition-colors duration-150 cursor-pointer group"
      onClick={props.onClick}
    >
      <div class="flex items-start justify-between mb-2">
        <span class="text-xs font-medium group-hover:text-accent-primary transition-colors duration-150">
          {props.name}
        </span>
        <Badge variant={statusVariant(props.status)}>{props.status.toUpperCase()}</Badge>
      </div>
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px] text-text-dim font-mono">
        <Show when={props.branch}>
          <div class="flex items-center gap-1">
            <GitBranch size={10} />
            {props.branch}
          </div>
        </Show>
        <Show when={props.elapsed}>
          <div class="flex items-center gap-1">
            <Clock size={10} />
            {props.elapsed}
          </div>
        </Show>
      </div>
      <Show when={props.lastStep && props.status === 'running'}>
        <div class="mt-2 p-1.5 bg-bg-sidebar border border-border-subtle text-[10px] text-text-dim leading-relaxed">
          {props.lastStep}
        </div>
      </Show>
    </button>
  );
}

function formatElapsed(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return m > 0 ? `${m}m ${s.toFixed(0)}s` : `${s.toFixed(0)}s`;
}

function worktreeLabel(path: string): string {
  const parts = path.replace(/\/$/, '').split('/');
  return parts[parts.length - 1] ?? path;
}

export function OrchestrationView() {
  const layout = getLayoutStore();
  const repoStore = getRepoStore();
  const [queue, setQueue] = createSignal<QueueStatus | null>(null);
  const [activeRun, setActiveRun] = createSignal<RunInfo | null>(null);
  const [lastSteps, setLastSteps] = createSignal<Record<string, string>>({});
  const [completedRuns, setCompletedRuns] = createSignal<
    { name: string; status: RunCardStatus; elapsed: string; worktreePath: string }[]
  >([]);

  onMount(async () => {
    const qs = await getQueueStatus();
    if (qs.queued.length > 0 || qs.active) {
      setQueue(qs);
    }
  });

  const unlisteners: (() => void)[] = [];

  onMount(() => {
    listen<QueueStatus>('run:queue_status', (e) => {
      const qs = e.payload;
      if (qs.queued.length === 0 && !qs.active) {
        setQueue(null);
      } else {
        setQueue(qs);
      }
    }).then((fn) => unlisteners.push(fn));

    listen<RunInfo>('run:status_changed', (e) => {
      setActiveRun(e.payload);
    }).then((fn) => unlisteners.push(fn));

    listen<RunStepEvent>('run:step', (e) => {
      const sid = e.payload.sessionId ?? 'unknown';
      setLastSteps((prev) => ({ ...prev, [sid]: e.payload.detail }));
    }).then((fn) => unlisteners.push(fn));

    listen<RunStatusEvent>('run:complete', (e) => {
      const sid = e.payload.sessionId ?? 'unknown';
      setCompletedRuns((prev) => [
        ...prev,
        {
          name: sid,
          status: e.payload.type === 'run_complete' ? 'succeeded' : 'failed',
          elapsed: '',
          worktreePath: '',
        },
      ]);
    }).then((fn) => unlisteners.push(fn));
  });

  onCleanup(() => {
    for (const fn of unlisteners) fn();
  });

  const totalCompleted = () => (queue()?.completed ?? 0) + completedRuns().length;
  const totalFailed = () => queue()?.failed ?? 0;
  const totalQueued = () => queue()?.queued.length ?? 0;
  const hasActive = () => !!queue()?.active;
  const isIdle = () => !queue();

  function handleCardClick(worktreePath: string) {
    if (!worktreePath) return;
    const activeRepo = repoStore.state.activeRepoPath;
    if (activeRepo) {
      repoStore.selectRepoAndWorktree(activeRepo, worktreePath);
    }
    layout.navigateToTask(worktreePath);
  }

  return (
    <div class="flex-1 flex flex-col overflow-hidden bg-bg-main">
      <Show
        when={!isIdle()}
        fallback={
          <div class="flex-1 flex items-center justify-center">
            <div class="text-center">
              <div class="text-sm text-text-dim mb-2">No active orchestrations</div>
              <div class="text-[10px] text-text-dim">
                Use the PRs panel to batch shepherd and start orchestrated runs.
              </div>
              <button
                type="button"
                class="mt-3 text-[10px] text-accent-primary hover:text-accent-primary/80 cursor-pointer"
                onClick={() => {
                  layout.setActiveView('workspace');
                  layout.showRightPanel({ kind: 'prs' });
                }}
              >
                Open PRs panel
              </button>
            </div>
          </div>
        }
      >
        {/* Header */}
        <div class="p-4 border-b border-border-subtle flex items-center justify-between bg-bg-sidebar/30">
          <div class="flex items-center gap-3">
            <button
              type="button"
              class="text-text-dim hover:text-text-main cursor-pointer p-1"
              title="Back to Workspace"
              onClick={() => layout.setActiveView('workspace')}
            >
              <ArrowLeft size={16} />
            </button>
            <div>
              <h2 class="text-sm font-bold">Batch Queue</h2>
              <p class="text-[10px] text-text-dim mt-1">
                {hasActive() ? '1 running' : ''}
                {totalQueued() > 0 ? ` · ${totalQueued()} queued` : ''}
                {totalCompleted() > 0 ? ` · ${totalCompleted()} done` : ''}
                {totalFailed() > 0 ? ` · ${totalFailed()} failed` : ''}
              </p>
            </div>
          </div>
          <Button variant="danger" size="compact" onClick={() => cancelQueue().catch(() => {})}>
            <Square size={10} class="mr-1.5" />
            Cancel All
          </Button>
        </div>

        {/* Run grid */}
        <div class="flex-1 overflow-y-auto p-4">
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {/* Active run */}
            <Show when={activeRun()}>
              {(run) => (
                <RunCard
                  name={worktreeLabel(run().taskPath)}
                  status="running"
                  elapsed={formatElapsed(run().elapsedSecs)}
                  lastStep={lastSteps()[run().sessionId ?? ''] ?? undefined}
                  onClick={() => handleCardClick(run().taskPath)}
                />
              )}
            </Show>

            {/* Queued runs */}
            <For each={queue()?.queued ?? []}>
              {(qr: QueuedRun) => (
                <RunCard
                  name={worktreeLabel(qr.worktreePath)}
                  status="queued"
                  branch={worktreeLabel(qr.worktreePath)}
                  onClick={() => handleCardClick(qr.worktreePath)}
                />
              )}
            </For>

            {/* Completed runs */}
            <For each={completedRuns()}>
              {(r) => (
                <RunCard
                  name={r.name}
                  status={r.status}
                  elapsed={r.elapsed}
                  onClick={() => handleCardClick(r.worktreePath)}
                />
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  );
}
