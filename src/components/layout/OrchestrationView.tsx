import { listen } from '@tauri-apps/api/event';
import { ArrowLeft, Clock, ExternalLink, Square } from 'lucide-solid';
import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import {
  cancelQueue,
  getQueueStatus,
  getRunStatus,
  respondToPermission,
} from '../../lib/commands/run';
import { listTasks } from '../../lib/commands/task';
import { getLayoutStore } from '../../lib/stores/layout';
import { getLifecycleStore } from '../../lib/stores/lifecycle';
import { getRepoStore } from '../../lib/stores/repo';
import { getTaskStore, worktreePathFromTaskPath } from '../../lib/stores/task';
import type { QueueStatus } from '../../types/github';
import type { LifecycleStatus } from '../../types/lifecycle';
import type { RunInfo, RunStepEvent } from '../../types/run';
import type { TaskInfo, TaskStatus } from '../../types/task';
import { AnalysisCard } from '../pr/AnalysisCard';
import { TaskBadge } from '../task/TaskBadge';
import { Button } from '../ui/Button';

type RunCardStatus = 'running' | 'succeeded' | 'failed' | 'queued' | 'cancelled';

function _taskStatusToCardStatus(status: TaskStatus): RunCardStatus {
  switch (status) {
    case 'running':
    case 'blocked':
      return 'running';
    case 'succeeded':
      return 'succeeded';
    case 'failed':
      return 'failed';
    case 'cancelled':
      return 'cancelled';
    case 'created':
      return 'queued';
  }
}

function TaskCard(props: {
  task: TaskInfo;
  worktreePath: string;
  repoName: string;
  branch: string;
  activeRun: RunInfo | null;
  lastStep?: string;
  expanded: boolean;
  onToggle: () => void;
  onOpenWorkspace: () => void;
}) {
  const taskStore = getTaskStore();

  const isActiveRun = () => taskStore.state.activeRun?.taskPath === props.task.path;
  const hasPending = () => taskStore.state.pendingPermissions.length > 0 && isActiveRun();

  return (
    <div
      class={`bg-bg-main border transition-colors duration-150 ${
        hasPending()
          ? 'border-accent-warning/50'
          : props.expanded
            ? 'border-accent-primary/50'
            : 'border-border-subtle hover:border-accent-primary/30'
      }`}
    >
      <button
        type="button"
        class="w-full text-left p-3 cursor-pointer group"
        onClick={props.onToggle}
      >
        <div class="flex items-start justify-between mb-1">
          <div class="flex items-center gap-1.5">
            <Show when={hasPending()}>
              <span class="relative flex h-2 w-2 shrink-0">
                <span class="absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75 animate-ping" />
                <span class="relative inline-flex rounded-full h-2 w-2 bg-red-400" />
              </span>
            </Show>
            <span class="text-base font-medium group-hover:text-accent-primary transition-colors duration-150">
              {props.branch}
            </span>
          </div>
          <TaskBadge status={props.task.frontmatter.status} />
        </div>
        <div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-base text-text-dim">
          <span>{props.repoName}</span>
          <span>{props.task.frontmatter.type === 'pr-shepherd' ? 'PR Shepherd' : 'Issue Fix'}</span>
          <Show when={props.task.frontmatter.pr}>
            <span class="text-accent-primary">PR #{props.task.frontmatter.pr}</span>
          </Show>
          <Show when={props.activeRun}>
            <div class="flex items-center gap-1 font-mono">
              <Clock size={10} />
              {formatElapsed(props.activeRun?.elapsedSecs)}
            </div>
          </Show>
          <Show when={props.task.frontmatter['run-count'] > 0}>
            <span>{props.task.frontmatter['run-count']} runs</span>
          </Show>
        </div>
        <Show when={props.lastStep && props.task.frontmatter.status === 'running'}>
          <div class="mt-2 p-1.5 bg-bg-sidebar border border-border-subtle text-base text-text-dim leading-relaxed truncate">
            {props.lastStep}
          </div>
        </Show>
      </button>

      {/* Expanded detail */}
      <Show when={props.expanded}>
        <div class="border-t border-border-subtle px-3 py-2 space-y-2">
          {/* PR context */}
          <Show when={props.task.frontmatter.pr}>
            <div class="text-base space-y-1">
              <div class="flex items-center gap-2">
                <span class="text-text-dim w-16 shrink-0">Checks</span>
                <Show
                  when={props.task.body.includes('Failing checks:')}
                  fallback={<span class="text-accent-success">passing</span>}
                >
                  <span class="text-accent-error">failing</span>
                </Show>
              </div>
              <div class="flex items-center gap-2">
                <span class="text-text-dim w-16 shrink-0">Reviews</span>
                <span class="text-text-main truncate">
                  {props.task.body
                    .split('\n')
                    .find((l) => l.includes('Reviews:'))
                    ?.replace('- Reviews: ', '')
                    ?.replace('- ', '') ?? 'None'}
                </span>
              </div>
            </div>
          </Show>

          {/* Knowledge count */}
          <Show
            when={
              props.task.body.includes('## Prior Knowledge') &&
              !props.task.body.includes('(none yet)')
            }
          >
            {(() => {
              const count =
                props.task.body
                  .split('## Prior Knowledge')[1]
                  ?.split('\n')
                  .filter((l) => l.startsWith('- ')).length ?? 0;
              return (
                <Show when={count > 0}>
                  <div class="text-base text-accent-info">
                    {count} knowledge pattern{count === 1 ? '' : 's'} recalled
                  </div>
                </Show>
              );
            })()}
          </Show>

          {/* Inline permission approval */}
          <For each={taskStore.state.pendingPermissions}>
            {(perm) => (
              <div class="border border-accent-warning/30 bg-accent-warning/5 p-2 mt-1">
                <div class="flex items-center gap-2 text-base mb-1.5">
                  <span class="text-accent-warning font-medium uppercase tracking-wider">
                    Permission
                  </span>
                  <span class="font-mono text-accent-info">{perm.tool ?? 'unknown'}</span>
                </div>
                <Show when={perm.command}>
                  <div class="text-base text-text-dim font-mono bg-bg-main/50 px-1.5 py-1 mb-1.5 break-all max-h-16 overflow-y-auto">
                    {perm.command}
                  </div>
                </Show>
                <div class="flex gap-1.5">
                  <button
                    type="button"
                    class="flex-1 px-2 py-1 text-base font-medium text-green-400 border border-green-400/30 hover:bg-green-400/10 cursor-pointer"
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
                    class="flex-1 px-2 py-1 text-base font-medium text-red-400 border border-red-400/30 hover:bg-red-400/10 cursor-pointer"
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

          {/* Open in workspace */}
          <button
            type="button"
            class="flex items-center gap-1 text-base text-accent-primary hover:text-accent-primary/80 cursor-pointer mt-1"
            onClick={(e) => {
              e.stopPropagation();
              props.onOpenWorkspace();
            }}
          >
            <ExternalLink size={10} />
            Open in Workspace
          </button>
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

type TaskItem = {
  task: TaskInfo;
  worktreePath: string;
  repoName: string;
  branch: string;
};

const STATUS_ORDER: Record<TaskStatus, number> = {
  blocked: 0,
  running: 1,
  failed: 2,
  cancelled: 3,
  created: 4,
  succeeded: 5,
};

const LIFECYCLE_STATUS_LABELS: Record<LifecycleStatus, string> = {
  running: 'Analyzing',
  reviewReady: 'Review Ready',
  approved: 'Approved',
  fixing: 'Fixing',
  completed: 'Completed',
  retrying: 'Retrying',
};

const LIFECYCLE_STATUS_COLORS: Record<LifecycleStatus, string> = {
  running: 'text-[var(--color-warning)]',
  reviewReady: 'text-accent-primary',
  approved: 'text-[var(--color-info)]',
  fixing: 'text-[var(--color-warning)]',
  completed: 'text-[var(--color-success)]',
  retrying: 'text-[var(--color-error)]',
};

const LIFECYCLE_STATUS_ORDER: Record<LifecycleStatus, number> = {
  reviewReady: 0,
  running: 1,
  fixing: 2,
  retrying: 3,
  approved: 4,
  completed: 5,
};

export function OrchestrationView() {
  const layout = getLayoutStore();
  const repoStore = getRepoStore();
  const _taskStore = getTaskStore();
  const lifecycleStore = getLifecycleStore();
  const [queue, setQueue] = createSignal<QueueStatus | null>(null);
  const [activeRun, setActiveRun] = createSignal<RunInfo | null>(null);
  const [lastSteps, setLastSteps] = createSignal<Record<string, string>>({});
  const [expandedCard, setExpandedCard] = createSignal<string | null>(null);
  const [allTasks, setAllTasks] = createSignal<TaskItem[]>([]);

  async function loadAllTasks() {
    const repos = repoStore.state.repos;
    const items: TaskItem[] = [];

    for (const repo of repos) {
      try {
        await repoStore.ensureWorktreesLoaded(repo.path);
      } catch {
        continue;
      }
      const wts = repoStore.state.worktreesByRepo[repo.path] ?? [];
      const wtPaths = wts.map((w) => w.path);
      if (wtPaths.length === 0) continue;

      try {
        const tasks = await listTasks(wtPaths);
        for (const task of tasks) {
          const wtPath = worktreePathFromTaskPath(task.path);
          const wt = wts.find((w) => {
            const wp = w.path.endsWith('/') ? w.path : `${w.path}/`;
            const tp = wtPath.endsWith('/') ? wtPath : `${wtPath}/`;
            return wp === tp;
          });
          items.push({
            task,
            worktreePath: wtPath,
            repoName: repo.name,
            branch: wt?.branch ?? wtPath.split('/').pop() ?? '',
          });
        }
      } catch {
        // Skip repos with no tasks
      }
    }

    setAllTasks(items);
  }

  const sortedTasks = createMemo(() =>
    [...allTasks()].sort(
      (a, b) => STATUS_ORDER[a.task.frontmatter.status] - STATUS_ORDER[b.task.frontmatter.status],
    ),
  );

  const [expandedLifecycle, setExpandedLifecycle] = createSignal<string | null>(null);

  const sortedLifecycles = createMemo(() =>
    [...lifecycleStore.getAllLifecycles()].sort(
      (a, b) => LIFECYCLE_STATUS_ORDER[a.status] - LIFECYCLE_STATUS_ORDER[b.status],
    ),
  );

  onMount(async () => {
    const qs = await getQueueStatus();
    if (qs.queued.length > 0 || qs.active) {
      setQueue(qs);
    }
    try {
      const runInfo = await getRunStatus();
      if (runInfo) setActiveRun(runInfo);
    } catch {
      // Best-effort
    }
    loadAllTasks();
    lifecycleStore.startListening();
    lifecycleStore.loadInitial();
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

    listen<TaskInfo>('task:updated', () => {
      loadAllTasks();
    }).then((fn) => unlisteners.push(fn));
  });

  onCleanup(() => {
    for (const fn of unlisteners) fn();
    lifecycleStore.stopListening();
  });

  const totalQueued = () => queue()?.queued.length ?? 0;
  const hasActive = () => !!queue()?.active || !!activeRun();

  function handleOpenWorkspace(worktreePath: string, repoPath?: string) {
    if (repoPath) {
      repoStore.selectRepoAndWorktree(repoPath, worktreePath);
    }
    layout.navigateToTask(worktreePath);
  }

  function toggleCard(id: string) {
    setExpandedCard((prev) => (prev === id ? null : id));
  }

  const runningCount = () =>
    sortedTasks().filter(
      (t) => t.task.frontmatter.status === 'running' || t.task.frontmatter.status === 'blocked',
    ).length;
  const failedCount = () =>
    sortedTasks().filter((t) => t.task.frontmatter.status === 'failed').length;
  const succeededCount = () =>
    sortedTasks().filter((t) => t.task.frontmatter.status === 'succeeded').length;

  return (
    <div class="flex-1 flex flex-col overflow-hidden bg-bg-main">
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
            <h2 class="text-sm font-bold">Orchestrations</h2>
            <p class="text-base text-text-dim mt-1">
              {sortedTasks().length} task{sortedTasks().length === 1 ? '' : 's'}
              {runningCount() > 0 ? ` · ${runningCount()} running` : ''}
              {totalQueued() > 0 ? ` · ${totalQueued()} queued` : ''}
              {succeededCount() > 0 ? ` · ${succeededCount()} done` : ''}
              {failedCount() > 0 ? ` · ${failedCount()} failed` : ''}
            </p>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="text-base text-text-dim hover:text-text-main cursor-pointer"
            onClick={loadAllTasks}
            title="Refresh tasks"
          >
            <svg
              aria-hidden="true"
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M1.5 7a5.5 5.5 0 0 1 9.37-3.9M12.5 1.5v3h-3" />
              <path d="M12.5 7a5.5 5.5 0 0 1-9.37 3.9M1.5 12.5v-3h3" />
            </svg>
          </button>
          <Show when={hasActive()}>
            <Button variant="danger" size="compact" onClick={() => cancelQueue().catch(() => {})}>
              <Square size={10} class="mr-1.5" />
              Cancel
            </Button>
          </Show>
        </div>
      </div>

      {/* Lifecycle + Task grid */}
      <div class="flex-1 overflow-y-auto p-4">
        {/* Lifecycle entries */}
        <Show when={sortedLifecycles().length > 0}>
          <div class="mb-4">
            <div class="text-xs text-text-dim uppercase mb-2">PR Lifecycles</div>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              <For each={sortedLifecycles()}>
                {(entry) => {
                  const analysis = () => lifecycleStore.getAnalysisPlan(entry.prKey);
                  const isExpanded = () => expandedLifecycle() === entry.prKey;

                  return (
                    <div>
                      <Show
                        when={isExpanded() && entry.status === 'reviewReady' && analysis()}
                        fallback={
                          <button
                            type="button"
                            class="w-full text-left bg-bg-sidebar border border-border-subtle p-3 hover:border-accent-primary/30 transition-colors duration-150 cursor-pointer"
                            onClick={() => setExpandedLifecycle(isExpanded() ? null : entry.prKey)}
                          >
                            <div class="flex items-center justify-between">
                              <span class="text-base font-medium text-text-main">
                                {entry.prKey}
                              </span>
                              <span
                                class={`text-xs font-medium uppercase ${LIFECYCLE_STATUS_COLORS[entry.status]}`}
                              >
                                {LIFECYCLE_STATUS_LABELS[entry.status]}
                              </span>
                            </div>
                            <div class="text-xs text-text-dim mt-1">Attempt {entry.attempt}</div>
                          </button>
                        }
                      >
                        {(() => {
                          const a = analysis();
                          return a ? (
                            <AnalysisCard
                              prKey={entry.prKey}
                              worktreePath={entry.worktreePath}
                              analysis={a}
                            />
                          ) : null;
                        })()}
                      </Show>
                    </div>
                  );
                }}
              </For>
            </div>
          </div>
        </Show>

        {/* Task grid */}
        <Show
          when={sortedTasks().length > 0}
          fallback={
            <div class="flex-1 flex items-center justify-center h-full">
              <div class="text-center">
                <div class="text-sm text-text-dim mb-2">No tasks yet</div>
                <div class="text-base text-text-dim">
                  Use the PRs panel to shepherd PRs or create tasks from a worktree.
                </div>
                <button
                  type="button"
                  class="mt-3 text-base text-accent-primary hover:text-accent-primary/80 cursor-pointer"
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
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            <For each={sortedTasks()}>
              {(item) => {
                const isActiveTask = () => activeRun()?.taskPath === item.task.path;
                const repoPath = () =>
                  repoStore.state.repos.find((r) => r.name === item.repoName)?.path;
                return (
                  <TaskCard
                    task={item.task}
                    worktreePath={item.worktreePath}
                    repoName={item.repoName}
                    branch={item.branch}
                    activeRun={isActiveTask() ? activeRun() : null}
                    lastStep={
                      isActiveTask() ? lastSteps()[activeRun()?.sessionId ?? ''] : undefined
                    }
                    expanded={expandedCard() === item.task.path}
                    onToggle={() => toggleCard(item.task.path)}
                    onOpenWorkspace={() => handleOpenWorkspace(item.worktreePath, repoPath())}
                  />
                );
              }}
            </For>
          </div>
        </Show>
      </div>
    </div>
  );
}
