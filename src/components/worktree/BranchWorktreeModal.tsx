import { Dialog } from '@kobalte/core';
import { createEffect, createMemo, createSignal, For, Show } from 'solid-js';
import { listBranches } from '../../lib/commands/git';
import { getRepoStore } from '../../lib/stores/repo';
import type { BranchInfo, WorktreeInfo } from '../../types/git';

type BranchWorktreeModalProps = {
  open: boolean;
  repoPath: string;
  onClose: () => void;
  onCreate: (wt: WorktreeInfo) => void;
};

export function BranchWorktreeModal(props: BranchWorktreeModalProps) {
  const repoStore = getRepoStore();
  const [branches, setBranches] = createSignal<BranchInfo[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [search, setSearch] = createSignal('');
  const [selected, setSelected] = createSignal<string | null>(null);
  const [creating, setCreating] = createSignal(false);

  // Reset state and load branches when modal opens
  createEffect(() => {
    if (props.open) {
      setBranches([]);
      setSearch('');
      setSelected(null);
      setError(null);
      setCreating(false);
      setLoading(true);
      listBranches(props.repoPath)
        .then((result) => {
          setBranches(result);
        })
        .catch((e) => {
          setError(String(e));
        })
        .finally(() => {
          setLoading(false);
        });
    }
  });

  const filteredBranches = createMemo(() => {
    const query = search().toLowerCase();
    if (!query) return branches();
    return branches().filter((b) => b.name.toLowerCase().includes(query));
  });

  const selectedBranch = createMemo(() => {
    const name = selected();
    if (!name) return null;
    return branches().find((b) => b.name === name) ?? null;
  });

  const worktreeName = createMemo(() => {
    const branch = selectedBranch();
    if (!branch) return null;
    if (branch.isRemote) {
      return branch.name.replace(/^origin\//, '');
    }
    return branch.name;
  });

  const worktreePath = createMemo(() => {
    const name = worktreeName();
    if (!name) return null;
    const sanitized = name
      .toLowerCase()
      .replace(/[^a-z0-9_-]/g, '-')
      .replace(/-{2,}/g, '-')
      .replace(/^[-_]+|[-_]+$/g, '');
    return sanitized ? `../worktrees/${sanitized}` : null;
  });

  const isCreateDisabled = createMemo(() => {
    if (creating()) return true;
    const branch = selectedBranch();
    if (!branch) return true;
    return branch.hasWorktree;
  });

  async function handleCreate() {
    const branch = selectedBranch();
    const name = worktreeName();
    if (!branch || !name) return;

    setCreating(true);
    setError(null);
    try {
      const wt = await repoStore.createWorktree(props.repoPath, name, name);
      props.onCreate(wt);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }

  return (
    <Dialog.Root
      open={props.open}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-40 bg-black/50" />
        <Dialog.Content class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-96 bg-bg-sidebar border border-border-subtle rounded-lg shadow-lg p-4">
          <Dialog.Title class="text-sm font-semibold text-text-main mb-3">
            Checkout Branch
          </Dialog.Title>

          <input
            type="text"
            placeholder="Search branches..."
            value={search()}
            onInput={(e) => setSearch(e.currentTarget.value)}
            autofocus
            class="w-full px-3 py-1.5 text-xs bg-bg-main border border-border-subtle rounded text-text-main placeholder:text-text-dim focus:outline-none focus:border-accent-primary"
          />

          <Show when={loading()}>
            <div class="mt-3 text-xs text-text-dim">Loading branches...</div>
          </Show>

          <Show
            when={
              !loading() && !error() && filteredBranches().length === 0 && branches().length > 0
            }
          >
            <div class="mt-3 text-xs text-text-dim">No branches match your search.</div>
          </Show>

          <Show when={!loading() && !error() && branches().length === 0}>
            <div class="mt-3 text-xs text-text-dim">No branches found.</div>
          </Show>

          <Show when={!loading() && filteredBranches().length > 0}>
            <div class="mt-3 max-h-64 overflow-y-auto border border-border-subtle rounded">
              <For each={filteredBranches()}>
                {(branch) => (
                  <button
                    type="button"
                    disabled={branch.hasWorktree}
                    class={`flex items-center gap-2 w-full px-3 py-1.5 text-xs text-left cursor-pointer ${
                      branch.hasWorktree
                        ? 'opacity-50 cursor-not-allowed'
                        : selected() === branch.name
                          ? 'bg-accent-primary/20 text-accent-primary'
                          : 'text-text-main hover:bg-bg-main/50'
                    }`}
                    onClick={() => {
                      if (!branch.hasWorktree) {
                        setSelected(branch.name);
                      }
                    }}
                  >
                    <span class="truncate">{branch.name}</span>
                    <span class="ml-auto flex gap-1 shrink-0">
                      <Show when={branch.isRemote}>
                        <span class="px-1.5 py-0.5 text-[10px] text-text-dim bg-bg-main rounded">
                          remote
                        </span>
                      </Show>
                      <Show when={branch.hasWorktree}>
                        <span class="px-1.5 py-0.5 text-[10px] text-accent-info bg-bg-main rounded">
                          in use
                        </span>
                      </Show>
                    </span>
                  </button>
                )}
              </For>
            </div>
          </Show>

          <Show when={selectedBranch()}>
            <div class="mt-3 space-y-1.5 text-xs">
              <div class="flex gap-2">
                <span class="text-text-dim">Branch:</span>
                <span class="text-text-main">{selectedBranch()?.name}</span>
              </div>
              <div class="flex gap-2">
                <span class="text-text-dim">Path:</span>
                <span class="text-text-main truncate">{worktreePath()}</span>
              </div>
            </div>
          </Show>

          <Show when={error()}>
            <p class="mt-2 text-xs text-accent-error">{error()}</p>
          </Show>

          <div class="mt-4 flex justify-end gap-2">
            <button
              type="button"
              class="px-3 py-1.5 text-xs text-text-dim hover:text-text-main cursor-pointer rounded hover:bg-bg-main/50"
              onClick={() => props.onClose()}
            >
              Cancel
            </button>
            <button
              type="button"
              disabled={isCreateDisabled()}
              class="px-3 py-1.5 text-xs bg-accent-primary text-bg rounded cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90"
              onClick={handleCreate}
            >
              {creating() ? 'Creating...' : 'Create Worktree'}
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
