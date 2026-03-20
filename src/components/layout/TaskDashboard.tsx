import { listen } from '@tauri-apps/api/event';
import { createMemo, createSignal, For, Match, onCleanup, onMount, Show, Switch } from 'solid-js';
import { enrichPrSummary, listAllOpenPrs } from '../../lib/commands/github';
import { batchLaunch, shepherdPr } from '../../lib/commands/run';
import { listTasks } from '../../lib/commands/task';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import { getTaskStore, worktreePathFromTaskPath } from '../../lib/stores/task';
import { parseArtifactSummary } from '../../lib/utils';
import type { PrSummary } from '../../types/github';
import type { TaskInfo, TaskStatus } from '../../types/task';
import { TaskBadge } from '../task/TaskBadge';

type DashboardTab = 'prs' | 'tasks';

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

function ciSortWeight(ciStatus: string | null): number {
  if (ciStatus === 'failing') return 0;
  if (ciStatus === 'pending') return 1;
  if (!ciStatus) return 2;
  return 3;
}

function formatAge(createdAt: string | null): string {
  if (!createdAt) return '';
  const ms = Date.now() - new Date(createdAt).getTime();
  const hours = Math.floor(ms / 3_600_000);
  if (hours < 1) return '<1h';
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  return `${days}d`;
}

function CiBadge(props: { status: string | null }) {
  return (
    <Switch>
      <Match when={props.status === 'failing'}>
        <span class="text-[10px] font-medium text-red-400">failing</span>
      </Match>
      <Match when={props.status === 'pending'}>
        <span class="text-[10px] font-medium text-yellow-400">pending</span>
      </Match>
      <Match when={props.status === 'passing'}>
        <span class="text-[10px] font-medium text-green-400">passing</span>
      </Match>
      <Match when={!props.status}>
        <span class="text-[10px] text-text-muted">no CI</span>
      </Match>
    </Switch>
  );
}

function ReviewBadge(props: { decision: string | null }) {
  return (
    <Show when={props.decision}>
      {(d) => (
        <span
          class={`text-[10px] font-medium ${d() === 'changes_requested' ? 'text-orange-400' : d() === 'approved' ? 'text-green-400' : 'text-text-muted'}`}
        >
          {d() === 'changes_requested' ? 'changes req.' : d()}
        </span>
      )}
    </Show>
  );
}

export function TaskDashboard() {
  const repoStore = getRepoStore();
  const layout = getLayoutStore();
  const taskStore = getTaskStore();
  const [activeTab, setActiveTab] = createSignal<DashboardTab>('prs');
  const [items, setItems] = createSignal<DashboardItem[]>([]);
  const [prs, setPrs] = createSignal<PrSummary[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [prsLoading, setPrsLoading] = createSignal(true);
  const [authorFilter, setAuthorFilter] = createSignal<string>('');
  const [ciFilter, setCiFilter] = createSignal<string>('');
  const [shepherding, setShepherding] = createSignal<number | null>(null);
  const [selectedPrs, setSelectedPrs] = createSignal<Set<string>>(new Set());
  const [batchRunning, setBatchRunning] = createSignal(false);

  const sortedItems = createMemo(() =>
    [...items()].sort(
      (a, b) => STATUS_WEIGHT[a.task.frontmatter.status] - STATUS_WEIGHT[b.task.frontmatter.status],
    ),
  );

  const filteredPrs = createMemo(() => {
    let list = prs();
    const af = authorFilter();
    if (af) {
      list = list.filter((p) => p.author.toLowerCase().includes(af.toLowerCase()));
    }
    const cf = ciFilter();
    if (cf) {
      list = list.filter((p) => p.ciStatus === cf);
    }
    return [...list].sort((a, b) => ciSortWeight(a.ciStatus) - ciSortWeight(b.ciStatus));
  });

  const hasActiveItems = createMemo(() =>
    items().some(
      (i) =>
        i.task.frontmatter.status === 'blocked' ||
        i.task.frontmatter.status === 'running' ||
        i.task.frontmatter.status === 'failed',
    ),
  );

  const authors = createMemo(() => {
    const set = new Set(prs().map((p) => p.author));
    return [...set].sort();
  });

  function repoPathForName(repoName: string): string | undefined {
    return repoStore.state.repos.find((r) => r.name === repoName || r.path.endsWith(`/${repoName}`))
      ?.path;
  }

  function prKey(pr: PrSummary): string {
    return `${pr.repoName}:${pr.number}`;
  }

  function togglePrSelection(pr: PrSummary) {
    setSelectedPrs((prev) => {
      const next = new Set(prev);
      const key = prKey(pr);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  async function handleBatchShepherd() {
    if (batchRunning()) return;
    const selected = selectedPrs();
    if (selected.size === 0) return;
    setBatchRunning(true);
    try {
      const pairs: [string, string][] = [];
      for (const pr of filteredPrs()) {
        if (!selected.has(prKey(pr))) continue;
        const rp = repoPathForName(pr.repoName);
        if (!rp) continue;
        const result = await shepherdPr(rp, pr.number, false);
        pairs.push([result.task.path, result.worktreePath]);
      }
      if (pairs.length > 0) {
        await batchLaunch(pairs);
        setActiveTab('tasks');
        loadAllTasks();
      }
      setSelectedPrs(new Set());
    } catch (e) {
      console.error('Batch shepherd failed:', e);
    } finally {
      setBatchRunning(false);
    }
  }

  async function handleShepherd(pr: PrSummary) {
    if (shepherding() !== null) return;
    const rp = repoPathForName(pr.repoName);
    if (!rp) return;
    setShepherding(pr.number);
    try {
      await shepherdPr(rp, pr.number, true);
      setActiveTab('tasks');
      loadAllTasks();
    } catch (e) {
      console.error('Shepherd failed:', e);
    } finally {
      setShepherding(null);
    }
  }

  async function loadPrs() {
    setPrsLoading(true);
    try {
      const repos = repoStore.state.repos;
      const repoPaths = repos.map((r) => r.path);
      if (repoPaths.length === 0) {
        setPrs([]);
        return;
      }
      const result = await listAllOpenPrs(repoPaths);
      setPrs(result);

      // Enrich PRs with CI status sequentially to avoid hitting GitHub rate limits
      for (const pr of result) {
        const rp = repoPathForName(pr.repoName);
        if (!rp) continue;
        try {
          const enriched = await enrichPrSummary(rp, pr);
          setPrs((prev) =>
            prev.map((p) =>
              p.number === enriched.number && p.repoName === enriched.repoName ? enriched : p,
            ),
          );
        } catch {
          // Non-fatal — PR still shows without CI status
        }
      }
    } catch (e) {
      console.error('Dashboard: failed to load PRs', e);
    } finally {
      setPrsLoading(false);
    }
  }

  async function loadAllTasks() {
    setLoading(true);
    try {
      const repos = repoStore.state.repos;
      for (const repo of repos) {
        try {
          await repoStore.ensureWorktreesLoaded(repo.path);
        } catch {
          // Skip unreachable repos
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

  function refreshAll() {
    loadAllTasks();
    loadPrs();
  }

  let unlisten: (() => void) | null = null;

  onMount(() => {
    refreshAll();
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
        <div class="flex items-center gap-0">
          <button
            type="button"
            class={`text-xs font-bold uppercase tracking-wider px-2 py-0.5 rounded-l cursor-pointer ${activeTab() === 'prs' ? 'bg-bg text-text' : 'text-text-muted hover:text-text'}`}
            onClick={() => setActiveTab('prs')}
          >
            PRs ({prs().length})
          </button>
          <button
            type="button"
            class={`text-xs font-bold uppercase tracking-wider px-2 py-0.5 rounded-r cursor-pointer ${activeTab() === 'tasks' ? 'bg-bg text-text' : 'text-text-muted hover:text-text'}`}
            onClick={() => setActiveTab('tasks')}
          >
            Tasks ({items().length})
          </button>
        </div>
        <button
          type="button"
          class="text-text-muted hover:text-text cursor-pointer"
          title="Refresh"
          onClick={refreshAll}
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
        <Show when={activeTab() === 'prs'}>
          <div class="px-2 py-1.5 flex items-center gap-1.5 border-b border-border/50">
            <select
              class="text-[10px] bg-bg text-text rounded px-1 py-0.5 border border-border/50"
              value={authorFilter()}
              onChange={(e) => setAuthorFilter(e.currentTarget.value)}
            >
              <option value="">All authors</option>
              <For each={authors()}>{(a) => <option value={a}>{a}</option>}</For>
            </select>
            <select
              class="text-[10px] bg-bg text-text rounded px-1 py-0.5 border border-border/50"
              value={ciFilter()}
              onChange={(e) => setCiFilter(e.currentTarget.value)}
            >
              <option value="">CI: All</option>
              <option value="failing">CI: Failing</option>
              <option value="pending">CI: Pending</option>
              <option value="passing">CI: Passing</option>
            </select>
          </div>

          <Switch>
            <Match when={prsLoading()}>
              <div class="space-y-1 p-2">
                <div class="animate-pulse bg-bg/50 rounded h-12" />
                <div class="animate-pulse bg-bg/50 rounded h-12" />
              </div>
            </Match>
            <Match when={filteredPrs().length === 0}>
              <div class="text-xs text-text-muted text-center px-3 py-4">
                <p>No open PRs found</p>
                <p class="mt-1 opacity-70">Add repos to your workspace to see their PRs</p>
              </div>
            </Match>
            <Match when={filteredPrs().length > 0}>
              <div class="p-1">
                <For each={filteredPrs()}>
                  {(pr) => (
                    <div class="w-full text-left px-2 py-1.5 rounded text-xs hover:bg-bg/50">
                      <div class="flex items-center justify-between gap-1">
                        <input
                          type="checkbox"
                          class="shrink-0 accent-accent cursor-pointer"
                          checked={selectedPrs().has(prKey(pr))}
                          onChange={() => togglePrSelection(pr)}
                        />
                        <a
                          href={pr.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          class="truncate hover:underline"
                          title={`PR #${pr.number} by ${pr.author}`}
                        >
                          <span class="text-text-muted">{pr.repoName}</span>
                          <span class="text-text-muted"> · </span>
                          <span class="text-text">#{pr.number}</span>
                          <span class="text-text-muted"> </span>
                          <span class="text-text">{pr.title}</span>
                        </a>
                        <button
                          type="button"
                          class={`shrink-0 text-[10px] px-1.5 py-0.5 rounded cursor-pointer ${shepherding() === pr.number ? 'bg-accent/10 text-accent/50' : 'bg-accent/20 text-accent hover:bg-accent/30'}`}
                          title="Create worktree + task and launch shepherd"
                          disabled={shepherding() !== null}
                          onClick={() => handleShepherd(pr)}
                        >
                          {shepherding() === pr.number ? 'Starting...' : 'Shepherd'}
                        </button>
                      </div>
                      <div class="flex items-center gap-2 mt-0.5 text-[10px] text-text-muted">
                        <CiBadge status={pr.ciStatus} />
                        <ReviewBadge decision={pr.reviewDecision} />
                        <Show when={pr.additions != null || pr.deletions != null}>
                          <span>
                            <span class="text-green-400">+{pr.additions ?? 0}</span>
                            <span class="text-red-400"> -{pr.deletions ?? 0}</span>
                          </span>
                        </Show>
                        <Show when={pr.changedFiles != null}>
                          <span>{pr.changedFiles} files</span>
                        </Show>
                        <Show when={pr.createdAt}>
                          <span>{formatAge(pr.createdAt)}</span>
                        </Show>
                        <span class="ml-auto">{pr.author}</span>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </Match>
          </Switch>
          <Show when={selectedPrs().size > 0}>
            <div class="px-2 py-1.5 border-t border-border/50">
              <button
                type="button"
                class={`w-full text-xs px-2 py-1 rounded cursor-pointer ${batchRunning() ? 'bg-accent/10 text-accent/50' : 'bg-accent/20 text-accent hover:bg-accent/30'}`}
                disabled={batchRunning()}
                onClick={handleBatchShepherd}
              >
                {batchRunning() ? 'Starting batch...' : `Shepherd Selected (${selectedPrs().size})`}
              </button>
            </div>
          </Show>
        </Show>

        <Show when={activeTab() === 'tasks'}>
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
                      <div class="flex items-center gap-1.5 mt-0.5 text-[10px] text-text-muted">
                        <Show
                          when={
                            taskStore.state.activeRun?.taskPath === item.task.path &&
                            taskStore.state.activeRun?.costUsd
                          }
                        >
                          <span>${taskStore.state.activeRun?.costUsd?.toFixed(3)}</span>
                        </Show>
                        <Show when={parseArtifactSummary(item.task.body)}>
                          {(artifacts) => (
                            <>
                              <Show when={artifacts().totalCommits > 0}>
                                <span>
                                  {artifacts().totalCommits} commit
                                  {artifacts().totalCommits === 1 ? '' : 's'}
                                </span>
                              </Show>
                              <Show when={artifacts().pr}>
                                <span class="text-info">PR #{artifacts().pr}</span>
                              </Show>
                            </>
                          )}
                        </Show>
                      </div>
                    </button>
                  )}
                </For>
              </div>
            </Match>
          </Switch>
        </Show>
      </div>
    </div>
  );
}
