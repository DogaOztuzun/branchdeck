import { listen } from '@tauri-apps/api/event';
import { ArrowLeft, Clock, ExternalLink, GitBranch, Square } from 'lucide-solid';
import { createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import {
  cancelQueue,
  getQueueStatus,
  getRunStatus,
  respondToPermission,
} from '../../lib/commands/run';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import { getTaskStore, worktreePathFromTaskPath } from '../../lib/stores/task';
import type { QueuedRun, QueueStatus } from '../../types/github';
import type { RunInfo, RunStatusEvent, RunStepEvent } from '../../types/run';
import { TaskBadge } from '../task/TaskBadge';
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
  expanded: boolean;
  onToggle: () => void;
  onOpenWorkspace?: () => void;
  worktreePath?: string;
}) {
  const taskStore = getTaskStore();

  const task = () => {
    if (!props.worktreePath) return null;
    const key = props.worktreePath.endsWith('/') ? props.worktreePath : `${props.worktreePath}/`;
    return taskStore.state.tasksByWorktree[key] ?? null;
  };

  const pendingPerms = () => taskStore.state.pendingPermissions;
  const hasPending = () => pendingPerms().length > 0 && props.status === 'running';

  return (
    <div
      class={`bg-bg-main border transition-colors duration-150 ${
        hasPending()
          ? 'border-accent-warning/50'
          : props.expanded
            ? 'border-accent-primary/50 col-span-full'
            : 'border-border-subtle hover:border-accent-primary/30'
      } ${props.expanded ? 'col-span-full' : ''}`}
    >
      <button
        type="button"
        class="w-full text-left p-3 cursor-pointer group"
        onClick={props.onToggle}
      >
        <div class="flex items-start justify-between mb-2">
          <div class="flex items-center gap-1.5">
            <Show when={hasPending()}>
              <span class="relative flex h-2 w-2 shrink-0">
                <span class="absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75 animate-ping" />
                <span class="relative inline-flex rounded-full h-2 w-2 bg-red-400" />
              </span>
            </Show>
            <span class="text-xs font-medium group-hover:text-accent-primary transition-colors duration-150">
              {props.name}
            </span>
          </div>
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

      {/* Expanded detail */}
      <Show when={props.expanded}>
        <div class="border-t border-border-subtle px-3 py-2 space-y-2">
          <Show when={task()}>
            {(t) => (
              <>
                {/* Task info */}
                <div class="flex items-center justify-between">
                  <span class="text-[10px] text-text-dim capitalize">
                    {t().frontmatter.type.replace('-', ' ')}
                  </span>
                  <TaskBadge status={t().frontmatter.status} />
                </div>

                {/* PR context */}
                <Show when={t().frontmatter.pr}>
                  <div class="text-[10px] space-y-1">
                    <div class="flex items-center gap-2">
                      <span class="text-text-dim w-14 shrink-0">PR</span>
                      <span class="text-accent-primary">#{t().frontmatter.pr}</span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-text-dim w-14 shrink-0">Checks</span>
                      <Show
                        when={t().body.includes('Failing checks:')}
                        fallback={<span class="text-accent-success">passing</span>}
                      >
                        <span class="text-accent-error">failing</span>
                      </Show>
                    </div>
                    <Show when={t().frontmatter['run-count'] > 0}>
                      <div class="flex items-center gap-2">
                        <span class="text-text-dim w-14 shrink-0">Runs</span>
                        <span class="text-text-main">{t().frontmatter['run-count']}</span>
                      </div>
                    </Show>
                  </div>
                </Show>

                {/* Knowledge count */}
                <Show
                  when={t().body.includes('## Prior Knowledge') && !t().body.includes('(none yet)')}
                >
                  {(() => {
                    const count =
                      t()
                        .body.split('## Prior Knowledge')[1]
                        ?.split('\n')
                        .filter((l) => l.startsWith('- ')).length ?? 0;
                    return (
                      <Show when={count > 0}>
                        <div class="text-[10px] text-accent-info">
                          {count} knowledge pattern{count === 1 ? '' : 's'} recalled
                        </div>
                      </Show>
                    );
                  })()}
                </Show>
              </>
            )}
          </Show>

          {/* Inline permission approval */}
          <For each={pendingPerms()}>
            {(perm) => (
              <div class="border border-accent-warning/30 bg-accent-warning/5 p-2 mt-1">
                <div class="flex items-center gap-2 text-[10px] mb-1.5">
                  <span class="text-accent-warning font-medium uppercase tracking-wider">
                    Permission
                  </span>
                  <span class="font-mono text-accent-info">{perm.tool ?? 'unknown'}</span>
                </div>
                <Show when={perm.command}>
                  <div class="text-[10px] text-text-dim font-mono bg-bg-main/50 px-1.5 py-1 mb-1.5 break-all max-h-16 overflow-y-auto">
                    {perm.command}
                  </div>
                </Show>
                <div class="flex gap-1.5">
                  <button
                    type="button"
                    class="flex-1 px-2 py-1 text-[10px] font-medium text-green-400 border border-green-400/30 hover:bg-green-400/10 cursor-pointer"
                    onClick={(e) => {
                      e.stopPropagation();
                      taskStore.removePermission(perm.toolUseId);
                      respondToPermission(perm.toolUseId, 'approve').catch(() => {});
                    }}
                  >
                    Approve
                  </button>
                  <button
                    type="button"
                    class="flex-1 px-2 py-1 text-[10px] font-medium text-red-400 border border-red-400/30 hover:bg-red-400/10 cursor-pointer"
                    onClick={(e) => {
                      e.stopPropagation();
                      taskStore.removePermission(perm.toolUseId);
                      respondToPermission(perm.toolUseId, 'deny').catch(() => {});
                    }}
                  >
                    Deny
                  </button>
                </div>
              </div>
            )}
          </For>

          {/* Open in workspace button */}
          <Show when={props.onOpenWorkspace}>
            <button
              type="button"
              class="flex items-center gap-1 text-[10px] text-accent-primary hover:text-accent-primary/80 cursor-pointer mt-1"
              onClick={(e) => {
                e.stopPropagation();
                props.onOpenWorkspace?.();
              }}
            >
              <ExternalLink size={10} />
              Open in Workspace
            </button>
          </Show>
        </div>
      </Show>
    </div>
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
  const [expandedCard, setExpandedCard] = createSignal<string | null>(null);
  const [completedRuns, setCompletedRuns] = createSignal<
    { name: string; status: RunCardStatus; elapsed: string; worktreePath: string }[]
  >([]);

  onMount(async () => {
    const qs = await getQueueStatus();
    if (qs.queued.length > 0 || qs.active) {
      setQueue(qs);
    }
    // Fetch current run status (may have started before this view mounted)
    try {
      const runInfo = await getRunStatus();
      if (runInfo) {
        setActiveRun(runInfo);
      }
    } catch {
      // Best-effort
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

  function handleOpenWorkspace(worktreePath: string) {
    if (!worktreePath) return;
    const activeRepo = repoStore.state.activeRepoPath;
    if (activeRepo) {
      repoStore.selectRepoAndWorktree(activeRepo, worktreePath);
    }
    layout.navigateToTask(worktreePath);
  }

  function toggleCard(id: string) {
    setExpandedCard((prev) => (prev === id ? null : id));
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
            {/* Active run (with full RunInfo) */}
            <Show
              when={activeRun()}
              fallback={
                <Show when={queue()?.active}>
                  {(activePath) => {
                    const wtPath = () => worktreePathFromTaskPath(activePath());
                    return (
                      <RunCard
                        name={worktreeLabel(wtPath())}
                        status="running"
                        branch={worktreeLabel(wtPath())}
                        worktreePath={wtPath()}
                        expanded={expandedCard() === `active:${wtPath()}`}
                        onToggle={() => toggleCard(`active:${wtPath()}`)}
                        onOpenWorkspace={() => handleOpenWorkspace(wtPath())}
                      />
                    );
                  }}
                </Show>
              }
            >
              {(run) => {
                const wtPath = () => worktreePathFromTaskPath(run().taskPath);
                return (
                  <RunCard
                    name={worktreeLabel(wtPath())}
                    status={
                      run().status === 'blocked'
                        ? 'running'
                        : ((run().status as RunCardStatus) ?? 'running')
                    }
                    branch={worktreeLabel(wtPath())}
                    elapsed={formatElapsed(run().elapsedSecs)}
                    lastStep={lastSteps()[run().sessionId ?? ''] ?? undefined}
                    worktreePath={wtPath()}
                    expanded={expandedCard() === `active:${wtPath()}`}
                    onToggle={() => toggleCard(`active:${wtPath()}`)}
                    onOpenWorkspace={() => handleOpenWorkspace(wtPath())}
                  />
                );
              }}
            </Show>

            {/* Queued runs */}
            <For each={queue()?.queued ?? []}>
              {(qr: QueuedRun) => (
                <RunCard
                  name={worktreeLabel(qr.worktreePath)}
                  status="queued"
                  branch={worktreeLabel(qr.worktreePath)}
                  worktreePath={qr.worktreePath}
                  expanded={expandedCard() === `queued:${qr.worktreePath}`}
                  onToggle={() => toggleCard(`queued:${qr.worktreePath}`)}
                  onOpenWorkspace={() => handleOpenWorkspace(qr.worktreePath)}
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
                  worktreePath={r.worktreePath}
                  expanded={expandedCard() === `done:${r.name}`}
                  onToggle={() => toggleCard(`done:${r.name}`)}
                  onOpenWorkspace={
                    r.worktreePath ? () => handleOpenWorkspace(r.worktreePath) : undefined
                  }
                />
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  );
}
