import { createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { getRepoStore } from '../../lib/stores/repo';
import type { RepoInfo, WorktreeInfo } from '../../types/git';
import type { PrInfo } from '../../types/github';
import { PrTooltip } from '../pr/PrTooltip';
import { ContextMenu } from '../ui/ContextMenu';
import { AddWorktreeModal } from '../worktree/AddWorktreeModal';
import { BranchWorktreeModal } from '../worktree/BranchWorktreeModal';
import { DeleteWorktreeDialog } from '../worktree/DeleteWorktreeDialog';

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

    // Close tooltip on sidebar scroll (also cancel leave timer to prevent re-fire)
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

  // Refresh branch tracking every 60s
  const trackingInterval = setInterval(() => {
    repoStore.refreshTracking();
  }, 60_000);
  onCleanup(() => clearInterval(trackingInterval));

  // Refresh PR status: 15s for active repo, 60s for expanded inactive repos
  const activePrInterval = setInterval(() => {
    repoStore.refreshPrStatus();
  }, 15_000);
  onCleanup(() => clearInterval(activePrInterval));

  const inactivePrInterval = setInterval(() => {
    // Only refresh expanded repos (excluding active), skip collapsed
    const expanded = expandedRepos();
    for (const repoPath of Object.keys(repoStore.state.worktreesByRepo)) {
      if (repoPath === repoStore.state.activeRepoPath) continue;
      if (!expanded.has(repoPath)) continue;
      repoStore.loadPrStatus(repoPath);
    }
  }, 60_000);
  onCleanup(() => clearInterval(inactivePrInterval));

  // Clean up leave timer on unmount
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
    <div class="flex flex-col h-full bg-surface">
      <div class="px-3 py-2 text-xs font-bold text-text-muted uppercase tracking-wider border-b border-border">
        Repositories
      </div>
      <div class="flex-1 overflow-y-auto" ref={sidebarScrollRef}>
        <For each={repoStore.state.repos}>
          {(repo) => {
            const worktrees = () => repoStore.state.worktreesByRepo[repo.path] ?? [];

            return (
              <div>
                <button
                  type="button"
                  class={`flex items-center w-full px-3 py-1.5 text-xs cursor-pointer hover:bg-bg/50 ${
                    repoStore.state.activeRepoPath === repo.path ? 'text-primary' : 'text-text'
                  }`}
                  onClick={async () => {
                    const isExpanded = expandedRepos().has(repo.path);
                    if (!isExpanded) {
                      await repoStore.ensureWorktreesLoaded(repo.path);
                    }
                    toggleExpanded(repo.path);
                  }}
                  onContextMenu={(e) => handleContextMenu(e, repo)}
                >
                  <span class="mr-1.5 text-text-muted">
                    {expandedRepos().has(repo.path) ? '\u25BE' : '\u25B8'}
                  </span>
                  <span class="truncate">{repo.name}</span>
                  <span class="ml-auto text-text-muted">{repo.currentBranch}</span>
                </button>
                <Show when={expandedRepos().has(repo.path)}>
                  <div class="ml-4">
                    <For each={worktrees()}>
                      {(wt) => (
                        <button
                          type="button"
                          class={`flex items-center w-full px-3 py-1 text-xs cursor-pointer hover:bg-bg/50 group ${
                            repoStore.state.activeWorktreePath === wt.path
                              ? 'text-info'
                              : 'text-text-muted'
                          }`}
                          onClick={() => repoStore.selectRepoAndWorktree(repo.path, wt.path)}
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
                          <span
                            class={`mr-1.5 text-[10px] ${wt.isMain ? 'text-success' : 'text-text-muted/50'}`}
                          >
                            {wt.isMain ? '\u25CF' : '\u25CB'}
                          </span>
                          <span class="truncate">{wt.branch || wt.name}</span>
                          {(() => {
                            const tracking = repoStore.state.trackingByBranch[wt.branch];
                            if (!tracking || (tracking.ahead === 0 && tracking.behind === 0))
                              return null;
                            return (
                              <span class="ml-auto flex gap-1 text-[10px] shrink-0">
                                {tracking.ahead > 0 && (
                                  <span class="text-success">
                                    {'\u2191'}
                                    {tracking.ahead}
                                  </span>
                                )}
                                {tracking.behind > 0 && (
                                  <span class="text-error">
                                    {'\u2193'}
                                    {tracking.behind}
                                  </span>
                                )}
                              </span>
                            );
                          })()}
                          {(() => {
                            const pr = repoStore.getPrForBranch(repo.path, wt.branch);
                            if (!pr) return null;
                            const colors: Record<string, string> = {
                              open: '#7aa2f7',
                              draft: '#565f89',
                              merged: '#bb9af7',
                              closed: '#f7768e',
                            };
                            const prColor = pr.isDraft
                              ? colors.draft
                              : (colors[pr.state] ?? '#565f89');

                            // Review status icon
                            const reviewIcon = () => {
                              if (pr.reviewDecision === 'approved')
                                return { char: '\u2713', color: '#9ece6a' };
                              if (pr.reviewDecision === 'changes_requested')
                                return { char: '!', color: '#f7768e' };
                              if (pr.reviews.length > 0)
                                return { char: '\u25CF', color: '#e0af68' };
                              return null;
                            };

                            // Checks summary icon
                            const checksIcon = () => {
                              if (pr.checks.length === 0) return null;
                              const allPassed = pr.checks.every(
                                (c) => c.status === 'completed' && c.conclusion === 'success',
                              );
                              const anyFailed = pr.checks.some(
                                (c) => c.status === 'completed' && c.conclusion === 'failure',
                              );
                              const anyRunning = pr.checks.some((c) => c.status === 'in_progress');
                              if (allPassed) return { char: '\u2713', color: '#9ece6a' };
                              if (anyFailed) return { char: '\u2715', color: '#f7768e' };
                              if (anyRunning) return { char: '\u2022', color: '#e0af68' };
                              return { char: '\u25CB', color: '#565f89' };
                            };

                            const ri = reviewIcon();
                            const ci = checksIcon();

                            return (
                              // biome-ignore lint/a11y/noStaticElementInteractions: tooltip hover trigger
                              <span
                                class="ml-1 flex items-center gap-0.5 text-[10px] shrink-0"
                                onMouseEnter={(e) => {
                                  if (prLeaveTimer !== undefined) {
                                    clearTimeout(prLeaveTimer);
                                    prLeaveTimer = undefined;
                                  }
                                  setHoveredPr({ pr, anchorEl: e.currentTarget as HTMLElement });
                                }}
                                onMouseLeave={() => {
                                  prLeaveTimer = setTimeout(() => setHoveredPr(null), 200);
                                }}
                              >
                                {/* PR state dot */}
                                <span style={{ color: prColor }}>{'\u25CF'}</span>
                                {/* Review icon */}
                                {ri && <span style={{ color: ri.color }}>{ri.char}</span>}
                                {/* Checks icon */}
                                {ci && <span style={{ color: ci.color }}>{ci.char}</span>}
                                {/* PR number */}
                                <span style={{ color: prColor }}>#{pr.number}</span>
                              </span>
                            );
                          })()}
                        </button>
                      )}
                    </For>
                    <button
                      type="button"
                      class="w-full px-3 py-1 text-xs text-text-muted hover:text-text cursor-pointer text-left hover:bg-bg/50"
                      onClick={() => setAddWorktreeRepo(repo.path)}
                    >
                      + New Worktree
                    </button>
                  </div>
                </Show>
              </div>
            );
          }}
        </For>
      </div>
      <Show when={deleteError()}>
        <div class="px-3 py-2 text-xs text-error border-t border-border">
          {deleteError()}
          <button
            type="button"
            class="ml-2 text-text-muted hover:text-text cursor-pointer"
            onClick={() => setDeleteError(null)}
          >
            dismiss
          </button>
        </div>
      </Show>
      <div class="p-2 border-t border-border">
        <button
          type="button"
          class="w-full px-3 py-1.5 text-xs text-text-muted hover:text-text cursor-pointer text-left hover:bg-bg/50 rounded"
          onClick={() => repoStore.addRepo()}
        >
          + Add Repository
        </button>
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
                  onClick: () => setDeleteTarget({ repoPath: menu().repoPath, wt: menu().wt }),
                },
              ]}
              onClose={() => setWtContextMenu(null)}
            />
          </Portal>
        )}
      </Show>
      <Show when={hoveredPr()}>
        {(hovered) => (
          <PrTooltip
            pr={hovered().pr}
            anchorEl={hovered().anchorEl}
            onClose={() => setHoveredPr(null)}
          />
        )}
      </Show>
    </div>
  );
}
