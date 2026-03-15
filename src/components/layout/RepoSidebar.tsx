import { createSignal, For, onMount, Show } from 'solid-js';
import { Portal } from 'solid-js/web';
import { getRepoStore } from '../../lib/stores/repo';
import type { RepoInfo, WorktreeInfo } from '../../types/git';
import { ContextMenu } from '../ui/ContextMenu';
import { AddWorktreeModal } from '../worktree/AddWorktreeModal';
import { DeleteWorktreeDialog } from '../worktree/DeleteWorktreeDialog';

export function RepoSidebar() {
  const repoStore = getRepoStore();
  const [expandedRepos, setExpandedRepos] = createSignal<Set<string>>(new Set());
  const [addWorktreeRepo, setAddWorktreeRepo] = createSignal<string | null>(null);
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

  onMount(async () => {
    await repoStore.restoreLastSession();
    if (repoStore.state.activeRepoPath) {
      setExpandedRepos(new Set([repoStore.state.activeRepoPath]));
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
      <div class="flex-1 overflow-y-auto">
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
      <Show when={contextMenu()}>
        {(menu) => (
          <Portal>
            <ContextMenu
              x={menu().x}
              y={menu().y}
              items={[
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
    </div>
  );
}
