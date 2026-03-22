import { ChevronDown, ChevronRight, FolderGit2, GitBranch, Plus } from 'lucide-solid';
import { createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { cn } from '../../lib/cn';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import { getTaskStore } from '../../lib/stores/task';
import type { RepoInfo, WorktreeInfo } from '../../types/git';
import type { PrInfo } from '../../types/github';
import { PrTooltip } from '../pr/PrTooltip';
import { Button } from '../ui/Button';
import { ContextMenu } from '../ui/ContextMenu';
import { SectionHeader } from '../ui/SectionHeader';
import { AddWorktreeModal } from '../worktree/AddWorktreeModal';
import { BranchWorktreeModal } from '../worktree/BranchWorktreeModal';
import { DeleteWorktreeDialog } from '../worktree/DeleteWorktreeDialog';

const PR_COLORS: Record<string, string> = {
  open: '#7aa2f7',
  draft: '#565f89',
  merged: '#bb9af7',
  closed: '#f7768e',
};

function PrBadge(props: {
  repoPath: string;
  branch: string;
  onHoverStart: (pr: PrInfo, el: HTMLElement) => void;
  onHoverEnd: () => void;
}) {
  const repoStore = getRepoStore();

  const pr = () => repoStore.getPrForBranch(props.repoPath, props.branch);

  const prColor = () => {
    const p = pr();
    if (!p) return '#565f89';
    return p.isDraft ? PR_COLORS.draft : (PR_COLORS[p.state] ?? '#565f89');
  };

  const reviewIcon = () => {
    const p = pr();
    if (!p) return null;
    if (p.reviewDecision === 'approved') return { char: '\u2713', color: '#9ece6a' };
    if (p.reviewDecision === 'changes_requested') return { char: '!', color: '#f7768e' };
    if (p.reviews.length > 0) return { char: '\u25CF', color: '#e0af68' };
    return null;
  };

  const checksIcon = () => {
    const p = pr();
    if (!p || p.checks.length === 0) return null;
    const nonBlocking = new Set(['success', 'skipped', 'neutral', 'cancelled']);
    const allPassed = p.checks.every(
      (c) => c.status === 'completed' && nonBlocking.has(c.conclusion ?? ''),
    );
    const anyFailed = p.checks.some(
      (c) => c.status === 'completed' && !nonBlocking.has(c.conclusion ?? ''),
    );
    const anyRunning = p.checks.some((c) => c.status === 'in_progress');
    if (allPassed) return { char: '\u2713', color: '#9ece6a' };
    if (anyFailed) return { char: '\u2715', color: '#f7768e' };
    if (anyRunning) return { char: '\u2022', color: '#e0af68' };
    return { char: '\u25CB', color: '#565f89' };
  };

  return (
    <Show when={pr()}>
      {/* biome-ignore lint/a11y/noStaticElementInteractions: tooltip hover trigger */}
      <span
        class="ml-1 flex items-center gap-0.5 text-[10px] shrink-0"
        onMouseEnter={(e) => {
          const p = pr();
          if (p) props.onHoverStart(p, e.currentTarget as HTMLElement);
        }}
        onMouseLeave={() => props.onHoverEnd()}
      >
        <span style={{ color: prColor() }}>{'\u25CF'}</span>
        <Show when={reviewIcon()}>
          {(ri) => <span style={{ color: ri().color }}>{ri().char}</span>}
        </Show>
        <Show when={checksIcon()}>
          {(ci) => <span style={{ color: ci().color }}>{ci().char}</span>}
        </Show>
        <span style={{ color: prColor() }}>#{pr()?.number}</span>
      </span>
    </Show>
  );
}

export function RepoSidebar() {
  const repoStore = getRepoStore();
  const [expandedRepos, setExpandedRepos] = createSignal<Set<string>>(new Set());
  const [addWorktreeRepo, setAddWorktreeRepo] = createSignal<string | null>(null);
  const [branchWorktreeRepo, setBranchWorktreeRepo] = createSignal<string | null>(null);
  const [contextMenu, setContextMenu] = createSignal<{
    x: number;
    y: number;
    repo: RepoInfo;
  } | null>(null);
  const [wtContextMenu, setWtContextMenu] = createSignal<{
    x: number;
    y: number;
    repoPath: string;
    wt: WorktreeInfo;
  } | null>(null);
  const [hoveredPr, setHoveredPr] = createSignal<{
    pr: PrInfo;
    anchorEl: HTMLElement;
  } | null>(null);

  let sidebarScrollRef: HTMLDivElement | undefined;
  let prLeaveTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    await repoStore.restoreLastSession();
    if (repoStore.state.activeRepoPath) {
      setExpandedRepos(new Set([repoStore.state.activeRepoPath]));
    }

    const scrollEl = sidebarScrollRef;
    if (scrollEl) {
      const handleScroll = () => {
        if (prLeaveTimer !== undefined) {
          clearTimeout(prLeaveTimer);
          prLeaveTimer = undefined;
        }
        setHoveredPr(null);
      };
      scrollEl.addEventListener('scroll', handleScroll);
      onCleanup(() => scrollEl.removeEventListener('scroll', handleScroll));
    }
  });

  const trackingInterval = setInterval(() => {
    repoStore.refreshTracking();
  }, 60_000);
  onCleanup(() => clearInterval(trackingInterval));

  const activePrInterval = setInterval(() => {
    repoStore.refreshPrStatus();
  }, 15_000);
  onCleanup(() => clearInterval(activePrInterval));

  const inactivePrInterval = setInterval(() => {
    const expanded = expandedRepos();
    for (const repoPath of Object.keys(repoStore.state.worktreesByRepo)) {
      if (repoPath === repoStore.state.activeRepoPath) continue;
      if (!expanded.has(repoPath)) continue;
      repoStore.loadPrStatus(repoPath);
    }
  }, 60_000);
  onCleanup(() => clearInterval(inactivePrInterval));

  onCleanup(() => {
    if (prLeaveTimer !== undefined) {
      clearTimeout(prLeaveTimer);
      prLeaveTimer = undefined;
    }
  });

  function toggleExpanded(repoPath: string) {
    setExpandedRepos((prev) => {
      const next = new Set(prev);
      if (next.has(repoPath)) {
        next.delete(repoPath);
      } else {
        next.add(repoPath);
      }
      return next;
    });
  }

  const [deleteError, setDeleteError] = createSignal<string | null>(null);
  const [deleteTarget, setDeleteTarget] = createSignal<{
    repoPath: string;
    wt: WorktreeInfo;
  } | null>(null);

  async function handleDeleteWorktree(repoPath: string, wtName: string, deleteBranch: boolean) {
    setDeleteError(null);
    setDeleteTarget(null);
    try {
      await repoStore.removeWorktree(repoPath, wtName, deleteBranch);
    } catch (e) {
      setDeleteError(String(e));
    }
  }

  function handleContextMenu(e: MouseEvent, repo: RepoInfo) {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, repo });
  }

  return (
    <div class="flex flex-col h-full bg-bg-sidebar">
      <SectionHeader
        label="Repositories"
        class="px-3 py-2 mt-0 mb-0 border-b border-border-subtle"
      />
      <div class="flex-1 overflow-y-auto py-1" ref={sidebarScrollRef}>
        <For each={repoStore.state.repos}>
          {(repo) => {
            const worktrees = () => repoStore.state.worktreesByRepo[repo.path] ?? [];
            const isActive = () => repoStore.state.activeRepoPath === repo.path;
            const isExpanded = () => expandedRepos().has(repo.path);

            return (
              <div>
                <button
                  type="button"
                  class={cn(
                    'flex items-center w-full px-3 py-1 text-base cursor-pointer hover:bg-bg-main/50 gap-1.5 transition-colors duration-150',
                    isActive() ? 'text-accent-primary' : 'text-text-main',
                  )}
                  onClick={async () => {
                    if (!isExpanded()) {
                      await repoStore.ensureWorktreesLoaded(repo.path);
                    }
                    toggleExpanded(repo.path);
                  }}
                  onContextMenu={(e) => handleContextMenu(e, repo)}
                >
                  {isExpanded() ? (
                    <ChevronDown size={12} class="text-text-dim shrink-0" />
                  ) : (
                    <ChevronRight size={12} class="text-text-dim shrink-0" />
                  )}
                  <FolderGit2
                    size={14}
                    class={cn(isActive() ? 'text-accent-primary' : 'text-text-dim', 'shrink-0')}
                  />
                  <span class="truncate font-medium">{repo.name}</span>
                  <span class="ml-auto text-base text-accent-info shrink-0 max-w-[50%] truncate text-right">
                    {repo.currentBranch}
                  </span>
                </button>
                <Show when={isExpanded()}>
                  <div class="ml-4">
                    <For each={worktrees()}>
                      {(wt) => (
                        <button
                          type="button"
                          class={cn(
                            'flex items-center w-full px-3 py-1 text-base cursor-pointer hover:bg-bg-main/50 group gap-1.5 transition-colors duration-150',
                            repoStore.state.activeWorktreePath === wt.path
                              ? 'text-accent-info'
                              : 'text-text-dim',
                          )}
                          onClick={() => {
                            repoStore.selectRepoAndWorktree(repo.path, wt.path);
                            const layoutStore = getLayoutStore();
                            const taskStore = getTaskStore();
                            if (taskStore.hasTaskForWorktree(wt.path)) {
                              layoutStore.autoContext({ kind: 'task', worktreePath: wt.path });
                            } else {
                              layoutStore.autoContext({ kind: 'agents' });
                            }
                          }}
                          onContextMenu={(e) => {
                            if (!wt.isMain) {
                              e.preventDefault();
                              e.stopPropagation();
                              setWtContextMenu({
                                x: e.clientX,
                                y: e.clientY,
                                repoPath: repo.path,
                                wt,
                              });
                            }
                          }}
                        >
                          <GitBranch
                            size={12}
                            class={cn(
                              wt.isMain ? 'text-accent-success' : 'text-text-dim/50',
                              'shrink-0',
                            )}
                          />
                          <span class="truncate">{wt.branch || wt.name}</span>
                          {(() => {
                            const tracking = repoStore.state.trackingByBranch[wt.branch];
                            if (!tracking || (tracking.ahead === 0 && tracking.behind === 0))
                              return null;
                            return (
                              <span class="ml-auto flex gap-1 text-base shrink-0">
                                {tracking.ahead > 0 && (
                                  <span class="text-accent-success">
                                    {'\u2191'}
                                    {tracking.ahead}
                                  </span>
                                )}
                                {tracking.behind > 0 && (
                                  <span class="text-accent-error">
                                    {'\u2193'}
                                    {tracking.behind}
                                  </span>
                                )}
                              </span>
                            );
                          })()}
                          <PrBadge
                            repoPath={repo.path}
                            branch={wt.branch}
                            onHoverStart={(pr, el) => {
                              if (prLeaveTimer !== undefined) {
                                clearTimeout(prLeaveTimer);
                                prLeaveTimer = undefined;
                              }
                              setHoveredPr({ pr, anchorEl: el });
                            }}
                            onHoverEnd={() => {
                              prLeaveTimer = setTimeout(() => setHoveredPr(null), 200);
                            }}
                          />
                        </button>
                      )}
                    </For>
                    <button
                      type="button"
                      class="w-full flex items-center gap-1.5 px-3 py-1 text-base text-text-dim hover:text-text-main cursor-pointer text-left hover:bg-bg-main/50 transition-colors duration-150"
                      onClick={() => setAddWorktreeRepo(repo.path)}
                    >
                      <Plus size={12} />
                      New Worktree
                    </button>
                  </div>
                </Show>
              </div>
            );
          }}
        </For>
      </div>
      <Show when={deleteError()}>
        <div class="px-3 py-2 text-base text-accent-error border-t border-border-subtle">
          {deleteError()}
          <button
            type="button"
            class="ml-2 text-text-dim hover:text-text-main cursor-pointer"
            onClick={() => setDeleteError(null)}
          >
            dismiss
          </button>
        </div>
      </Show>
      <div class="p-2 border-t border-border-subtle">
        <Button
          variant="ghost"
          size="compact"
          class="w-full justify-start gap-1.5"
          onClick={() => repoStore.addRepo()}
        >
          <Plus size={12} />
          Add Repository
        </Button>
      </div>
      <DeleteWorktreeDialog
        open={deleteTarget() !== null}
        worktreeName={deleteTarget()?.wt.name ?? ''}
        onClose={() => setDeleteTarget(null)}
        onConfirm={(deleteBranch) => {
          const target = deleteTarget();
          if (target) {
            handleDeleteWorktree(target.repoPath, target.wt.name, deleteBranch);
          }
        }}
      />
      <AddWorktreeModal
        open={addWorktreeRepo() !== null}
        repoPath={addWorktreeRepo() ?? ''}
        onClose={() => setAddWorktreeRepo(null)}
        onCreate={(wt) => {
          const repoPath = addWorktreeRepo();
          setAddWorktreeRepo(null);
          if (repoPath) {
            repoStore.selectRepoAndWorktree(repoPath, wt.path);
          }
        }}
      />
      <BranchWorktreeModal
        open={branchWorktreeRepo() !== null}
        repoPath={branchWorktreeRepo() ?? ''}
        onClose={() => setBranchWorktreeRepo(null)}
        onCreate={(wt) => {
          const repoPath = branchWorktreeRepo();
          setBranchWorktreeRepo(null);
          if (repoPath) {
            repoStore.selectRepoAndWorktree(repoPath, wt.path);
          }
        }}
      />
      <Show when={contextMenu()}>
        {(menu) => (
          <Portal>
            <ContextMenu
              x={menu().x}
              y={menu().y}
              items={[
                {
                  label: 'Checkout Branch',
                  onClick: () => setBranchWorktreeRepo(menu().repo.path),
                },
                {
                  label: 'Close Project',
                  variant: 'danger',
                  onClick: () => repoStore.removeRepo(menu().repo.path),
                },
              ]}
              onClose={() => setContextMenu(null)}
            />
          </Portal>
        )}
      </Show>
      <Show when={wtContextMenu()}>
        {(menu) => (
          <Portal>
            <ContextMenu
              x={menu().x}
              y={menu().y}
              items={[
                {
                  label: 'Delete Worktree',
                  variant: 'danger',
                  onClick: () => {
                    const target = { repoPath: menu().repoPath, wt: menu().wt };
                    setTimeout(() => setDeleteTarget(target), 50);
                  },
                },
              ]}
              onClose={() => setWtContextMenu(null)}
            />
          </Portal>
        )}
      </Show>
      {(() => {
        const hovered = hoveredPr();
        if (!hovered) return null;
        return (
          <PrTooltip
            pr={hovered.pr}
            anchorEl={hovered.anchorEl}
            onClose={() => setHoveredPr(null)}
            onHover={() => {
              if (prLeaveTimer !== undefined) {
                clearTimeout(prLeaveTimer);
                prLeaveTimer = undefined;
              }
            }}
          />
        );
      })()}
    </div>
  );
}
