import { createMemo, createSignal, For, Match, onMount, Show, Switch } from 'solid-js';
import { enrichPrSummary, listOpenPrs } from '../../lib/commands/github';
import { batchLaunch, shepherdPr } from '../../lib/commands/run';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import type { PrSummary } from '../../types/github';
import { Button } from '../ui/Button';
import { Checkbox } from '../ui/Checkbox';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/Select';

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
        <span class="text-base font-medium text-red-400">failing</span>
      </Match>
      <Match when={props.status === 'pending'}>
        <span class="text-base font-medium text-yellow-400">pending</span>
      </Match>
      <Match when={props.status === 'passing'}>
        <span class="text-base font-medium text-green-400">passing</span>
      </Match>
      <Match when={!props.status}>
        <span class="text-base text-text-dim">no CI</span>
      </Match>
    </Switch>
  );
}

function ReviewBadge(props: { decision: string | null }) {
  return (
    <Show when={props.decision}>
      {(d) => (
        <span
          class={`text-base font-medium ${d() === 'changes_requested' ? 'text-orange-400' : d() === 'approved' ? 'text-green-400' : 'text-text-dim'}`}
        >
          {d() === 'changes_requested' ? 'changes req.' : d()}
        </span>
      )}
    </Show>
  );
}

export function PrList() {
  const repoStore = getRepoStore();
  const layout = getLayoutStore();
  const [prs, setPrs] = createSignal<PrSummary[]>([]);
  const [prsLoading, setPrsLoading] = createSignal(true);
  const [authorFilter, setAuthorFilter] = createSignal<string>('');
  const [ciFilter, setCiFilter] = createSignal<string>('');
  const [shepherding, setShepherding] = createSignal<number | null>(null);
  const [selectedPrs, setSelectedPrs] = createSignal<Set<string>>(new Set());
  const [batchRunning, setBatchRunning] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  // Map GitHub repo name → local repo path (built during loadPrs)
  const [repoNameToPath, setRepoNameToPath] = createSignal<Map<string, string>>(new Map());

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

  const authors = createMemo(() => {
    const set = new Set(prs().map((p) => p.author));
    return [...set].sort();
  });

  function repoPathForName(repoName: string): string | undefined {
    // Prefer the mapping built during loadPrs (GitHub name → local path)
    const mapped = repoNameToPath().get(repoName);
    if (mapped) return mapped;
    // Fallback: match by local folder name
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
    setError(null);
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
        layout.setActiveView('orchestrations');
      }
      setSelectedPrs(new Set());
    } catch (e) {
      setError(`Batch shepherd failed: ${e}`);
      setTimeout(() => setError(null), 5000);
    } finally {
      setBatchRunning(false);
    }
  }

  async function handleShepherd(pr: PrSummary) {
    if (shepherding() !== null) return;
    const rp = repoPathForName(pr.repoName);
    if (!rp) {
      setError(`No repo found for "${pr.repoName}"`);
      setTimeout(() => setError(null), 5000);
      return;
    }
    setShepherding(pr.number);
    setError(null);
    try {
      const result = await shepherdPr(rp, pr.number, true);
      repoStore.selectRepoAndWorktree(rp, result.worktreePath);
      layout.navigateToTask(result.worktreePath);
    } catch (e) {
      const errMsg = String(e);
      // If worktree already exists, navigate to it instead of erroring
      if (errMsg.includes('Worktree already exists')) {
        const branch = pr.branch;
        const wts = repoStore.state.worktreesByRepo[rp] ?? [];
        const existing = wts.find((wt) => wt.branch === branch);
        if (existing) {
          repoStore.selectRepoAndWorktree(rp, existing.path);
          layout.navigateToTask(existing.path);
          return;
        }
      }
      setError(`Shepherd failed: ${errMsg}`);
      setTimeout(() => setError(null), 5000);
    } finally {
      setShepherding(null);
    }
  }

  async function loadPrs() {
    setPrsLoading(true);
    try {
      const repos = repoStore.state.repos;
      if (repos.length === 0) {
        setPrs([]);
        return;
      }

      // Load PRs per-repo to build GitHub name → local path mapping
      const allPrs: PrSummary[] = [];
      const nameMap = new Map<string, string>();

      for (const repo of repos) {
        try {
          const repoPrs = await listOpenPrs(repo.path);
          for (const pr of repoPrs) {
            nameMap.set(pr.repoName, repo.path);
            allPrs.push(pr);
          }
        } catch {
          // Skip unreachable repos
        }
      }

      setRepoNameToPath(nameMap);
      setPrs(allPrs);

      // Enrich PRs with CI status
      for (const pr of allPrs) {
        const rp = nameMap.get(pr.repoName);
        if (!rp) continue;
        try {
          const enriched = await enrichPrSummary(rp, pr);
          setPrs((prev) =>
            prev.map((p) =>
              p.number === enriched.number && p.repoName === enriched.repoName ? enriched : p,
            ),
          );
        } catch {
          // Non-fatal
        }
      }
    } catch (e) {
      console.error('PrList: failed to load PRs', e);
    } finally {
      setPrsLoading(false);
    }
  }

  onMount(() => {
    loadPrs();
  });

  return (
    <div class="h-full flex flex-col bg-bg-sidebar overflow-hidden">
      <div class="px-3 py-2 border-b border-border-subtle flex items-center justify-between">
        <span class="text-[10px] font-bold uppercase text-text-dim tracking-wider">
          Pull Requests ({prs().length})
        </span>
        <button
          type="button"
          class="text-text-dim hover:text-text-main cursor-pointer"
          title="Refresh"
          onClick={loadPrs}
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

      {/* Filters */}
      <div class="px-2 py-1.5 flex items-center gap-1.5 border-b border-border-subtle/50">
        <Select
          options={['', ...authors()]}
          value={authorFilter()}
          onChange={(val) => setAuthorFilter(val ?? '')}
          placeholder="All authors"
          itemComponent={(props) => (
            <SelectItem item={props.item}>
              {props.item.rawValue === '' ? 'All authors' : props.item.rawValue}
            </SelectItem>
          )}
        >
          <SelectTrigger size="sm">
            <SelectValue<string>>
              {(state) => (state.selectedOption() === '' ? 'All authors' : state.selectedOption())}
            </SelectValue>
          </SelectTrigger>
          <SelectContent />
        </Select>
        <Select
          options={['', 'failing', 'pending', 'passing']}
          value={ciFilter()}
          onChange={(val) => setCiFilter(val ?? '')}
          placeholder="CI: All"
          itemComponent={(props) => (
            <SelectItem item={props.item}>
              {props.item.rawValue === ''
                ? 'CI: All'
                : `CI: ${props.item.rawValue.charAt(0).toUpperCase()}${props.item.rawValue.slice(1)}`}
            </SelectItem>
          )}
        >
          <SelectTrigger size="sm">
            <SelectValue<string>>
              {(state) =>
                state.selectedOption() === ''
                  ? 'CI: All'
                  : `CI: ${state.selectedOption()?.charAt(0).toUpperCase()}${state.selectedOption()?.slice(1)}`
              }
            </SelectValue>
          </SelectTrigger>
          <SelectContent />
        </Select>
      </div>

      {/* PR List */}
      <div class="flex-1 overflow-y-auto">
        <Switch>
          <Match when={prsLoading()}>
            <div class="space-y-1 p-2">
              <div class="animate-pulse bg-bg-main/50 h-12" />
              <div class="animate-pulse bg-bg-main/50 h-12" />
            </div>
          </Match>
          <Match when={filteredPrs().length === 0}>
            <div class="text-base text-text-dim text-center px-3 py-6">
              <p>No open PRs found</p>
              <p class="mt-2 text-base opacity-70">
                Add a repository with open pull requests to get started
              </p>
            </div>
          </Match>
          <Match when={filteredPrs().length > 0}>
            <div class="divide-y divide-border-subtle/30">
              <For each={filteredPrs()}>
                {(pr) => (
                  <div class="px-3 py-2 text-base hover:bg-bg-main/30 transition-colors duration-150">
                    <div class="flex items-start gap-1.5">
                      <Checkbox
                        class="shrink-0 mt-0.5"
                        checked={selectedPrs().has(prKey(pr))}
                        onChange={() => togglePrSelection(pr)}
                      />
                      <a
                        href={pr.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="min-w-0 hover:underline"
                        title={pr.title}
                      >
                        <span class="text-base font-medium text-accent-primary">#{pr.number}</span>{' '}
                        <span class="text-base text-text-main">{pr.title}</span>
                      </a>
                    </div>
                    <div class="flex items-center gap-2 mt-1 text-base text-text-dim pl-5">
                      <span class="truncate">{pr.repoName}</span>
                      <CiBadge status={pr.ciStatus} />
                      <ReviewBadge decision={pr.reviewDecision} />
                      <Show when={pr.additions != null || pr.deletions != null}>
                        <span>
                          <span class="text-green-400">+{pr.additions ?? 0}</span>
                          <span class="text-red-400"> -{pr.deletions ?? 0}</span>
                        </span>
                      </Show>
                      <Show when={pr.createdAt}>
                        <span>{formatAge(pr.createdAt)}</span>
                      </Show>
                    </div>
                    <div class="flex items-center justify-between mt-1 pl-5">
                      <span class="text-base text-text-dim">{pr.author}</span>
                      <button
                        type="button"
                        class={`text-base font-medium px-2 py-0.5 border cursor-pointer transition-colors duration-150 ${
                          shepherding() === pr.number
                            ? 'border-accent-primary/30 text-accent-primary/50 bg-accent-primary/5'
                            : 'border-border-subtle text-text-dim hover:text-accent-primary hover:border-accent-primary/50 hover:bg-accent-primary/5'
                        }`}
                        title="Create worktree + task and launch shepherd"
                        disabled={shepherding() !== null}
                        onClick={() => handleShepherd(pr)}
                      >
                        {shepherding() === pr.number ? '...' : 'Shepherd'}
                      </button>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </Match>
        </Switch>
      </div>

      {/* Error display */}
      <Show when={error()}>
        <div class="px-3 py-1.5 text-base text-accent-error border-t border-border-subtle/50">
          {error()}
        </div>
      </Show>

      {/* Batch action */}
      <Show when={selectedPrs().size > 0}>
        <div class="px-2 py-1.5 border-t border-border-subtle/50">
          <Button
            variant="primary"
            class="w-full"
            disabled={batchRunning()}
            onClick={handleBatchShepherd}
          >
            {batchRunning() ? 'Starting batch...' : `Shepherd Selected (${selectedPrs().size})`}
          </Button>
        </div>
      </Show>
    </div>
  );
}
