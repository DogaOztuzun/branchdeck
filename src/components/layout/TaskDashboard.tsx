import { listen } from '@tauri-apps/api/event';
import { createMemo, createSignal, For, Match, onCleanup, onMount, Show, Switch } from 'solid-js';
import { listTasks } from '../../lib/commands/task';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import { worktreePathFromTaskPath } from '../../lib/stores/task';
import { parseArtifactSummary } from '../../lib/utils';
import type { TaskInfo, TaskStatus } from '../../types/task';
import { TaskBadge } from '../task/TaskBadge';

type DashboardItem = {
  task: TaskInfo;
  repoName: string;
  repoPath: string;
  worktreePath: string;
  branch: string;
};

const STATUS_WEIGHT: Record<TaskStatus, number> = {
  blocked: 0,
  running: 1,
  failed: 2,
  cancelled: 3,
  created: 4,
  succeeded: 5,
};

export function TaskDashboard() {
  const repoStore = getRepoStore();
  const layout = getLayoutStore();
  const [items, setItems] = createSignal<DashboardItem[]>([]);
  const [loading, setLoading] = createSignal(true);

  const sortedItems = createMemo(() =>
    [...items()].sort(
      (a, b) => STATUS_WEIGHT[a.task.frontmatter.status] - STATUS_WEIGHT[b.task.frontmatter.status],
    ),
  );

  const hasActiveItems = createMemo(() =>
    items().some(
      (i) =>
        i.task.frontmatter.status === 'blocked' ||
        i.task.frontmatter.status === 'running' ||
        i.task.frontmatter.status === 'failed',
    ),
  );

  async function loadAllTasks() {
    setLoading(true);
    try {
      const repos = repoStore.state.repos;
      for (const repo of repos) {
        try {
          await repoStore.ensureWorktreesLoaded(repo.path);
        } catch {
          // Skip unreachable repos — load whatever we can
        }
      }

      const allWorktreePaths: string[] = [];
      for (const repoPath of Object.keys(repoStore.state.worktreesByRepo)) {
        const wts = repoStore.state.worktreesByRepo[repoPath] ?? [];
        for (const wt of wts) {
          allWorktreePaths.push(wt.path);
        }
      }

      const tasks = await listTasks(allWorktreePaths);
      const result: DashboardItem[] = [];

      for (const task of tasks) {
        const wtPath = worktreePathFromTaskPath(task.path);
        let foundRepo: string | null = null;
        let foundRepoName = '';
        let foundBranch = '';

        for (const repoPath of Object.keys(repoStore.state.worktreesByRepo)) {
          const wts = repoStore.state.worktreesByRepo[repoPath] ?? [];
          const normalized = wtPath.endsWith('/') ? wtPath : `${wtPath}/`;
          const match = wts.find((w) => {
            const wp = w.path.endsWith('/') ? w.path : `${w.path}/`;
            return wp === normalized;
          });
          if (match) {
            foundRepo = repoPath;
            foundBranch = match.branch;
            const repo = repoStore.state.repos.find((r) => r.path === repoPath);
            foundRepoName = repo?.name ?? repoPath.split('/').pop() ?? repoPath;
            break;
          }
        }

        if (foundRepo) {
          result.push({
            task,
            repoName: foundRepoName,
            repoPath: foundRepo,
            worktreePath: wtPath,
            branch: foundBranch,
          });
        }
      }

      setItems(result);
    } catch (e) {
      console.error('Dashboard: failed to load tasks', e);
    } finally {
      setLoading(false);
    }
  }

  let unlisten: (() => void) | null = null;

  onMount(() => {
    loadAllTasks();
    listen<TaskInfo>('task:updated', (e) => {
      const updated = e.payload;
      setItems((prev) => {
        const idx = prev.findIndex((i) => i.task.path === updated.path);
        if (idx !== -1) {
          const existing = prev[idx];
          if (!existing) return prev;
          const copy = [...prev];
          copy[idx] = { ...existing, task: updated };
          return copy;
        }
        return prev;
      });
    }).then((fn) => {
      unlisten = fn;
    });
  });

  onCleanup(() => {
    unlisten?.();
  });

  function handleCardClick(item: DashboardItem) {
    repoStore.selectRepoAndWorktree(item.repoPath, item.worktreePath);
    layout.toggleTeamSidebar();
  }

  return (
    <div class="h-full flex flex-col bg-surface overflow-hidden">
      <div class="px-3 py-2 border-b border-border flex items-center justify-between">
        <span class="text-xs font-bold uppercase text-text-muted tracking-wider">Dashboard</span>
        <button
          type="button"
          class="text-text-muted hover:text-text cursor-pointer"
          title="Refresh all tasks"
          onClick={() => loadAllTasks()}
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
      </div>

      <div class="flex-1 overflow-y-auto">
        <Switch>
          <Match when={loading()}>
            <div class="space-y-1 p-2">
              <div class="animate-pulse bg-bg/50 rounded h-10" />
              <div class="animate-pulse bg-bg/50 rounded h-10" />
              <div class="animate-pulse bg-bg/50 rounded h-10" />
            </div>
          </Match>
          <Match when={items().length === 0}>
            <div class="text-xs text-text-muted text-center px-3 py-4">
              <p>No tasks yet</p>
              <p class="mt-1 opacity-70">Create a task from the Team sidebar</p>
            </div>
          </Match>
          <Match when={items().length > 0}>
            <Show when={!hasActiveItems()}>
              <div class="text-xs text-text-muted text-center px-3 py-3">All quiet</div>
            </Show>
            <div class="p-1">
              <For each={sortedItems()}>
                {(item) => (
                  <button
                    type="button"
                    class={`w-full text-left px-2 py-1.5 rounded text-xs hover:bg-bg/50 cursor-pointer ${item.task.frontmatter.status === 'blocked' ? 'border-l-2 border-yellow-400' : ''}`}
                    title={`${item.task.frontmatter.type} · ${item.task.frontmatter['run-count']} runs`}
                    onClick={() => handleCardClick(item)}
                  >
                    <div class="flex items-center justify-between gap-1">
                      <span class="truncate">
                        <span class="text-text-muted">{item.repoName}</span>
                        <span class="text-text-muted">/</span>
                        <span class="text-text">{item.branch}</span>
                      </span>
                      <TaskBadge status={item.task.frontmatter.status} />
                    </div>
                    {(() => {
                      const a = parseArtifactSummary(item.task.body);
                      if (!a) return null;
                      return (
                        <div class="flex items-center gap-1.5 mt-0.5 text-[10px] text-text-muted">
                          <Show when={a.totalCommits > 0}>
                            <span>
                              {a.totalCommits} commit{a.totalCommits === 1 ? '' : 's'}
                            </span>
                          </Show>
                          <Show when={a.pr}>
                            <span class="text-info">PR #{a.pr}</span>
                          </Show>
                        </div>
                      );
                    })()}
                  </button>
                )}
              </For>
            </div>
          </Match>
        </Switch>
      </div>
    </div>
  );
}
