import { Dialog } from '@kobalte/core';
import { createEffect, createMemo, createSignal, onCleanup, Show } from 'solid-js';
import { previewWorktree } from '../../lib/commands/git';
import { getRepoStore } from '../../lib/stores/repo';
import type { WorktreeInfo, WorktreePreview } from '../../types/git';

type AddWorktreeModalProps = {
  open: boolean;
  repoPath: string;
  onClose: () => void;
  onCreate: (wt: WorktreeInfo) => void;
};

export function AddWorktreeModal(props: AddWorktreeModalProps) {
  const repoStore = getRepoStore();
  const [name, setName] = createSignal('');
  const [preview, setPreview] = createSignal<WorktreePreview | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [creating, setCreating] = createSignal(false);

  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let requestId = 0;

  // Reset state when modal opens/closes
  createEffect(() => {
    if (props.open) {
      setName('');
      setPreview(null);
      setError(null);
      setCreating(false);
      requestId = 0;
    } else {
      clearTimeout(debounceTimer);
    }
  });

  onCleanup(() => {
    clearTimeout(debounceTimer);
  });

  function handleNameInput(value: string) {
    setName(value);
    setError(null);
    clearTimeout(debounceTimer);

    if (!value.trim()) {
      setPreview(null);
      return;
    }

    debounceTimer = setTimeout(async () => {
      const thisRequest = ++requestId;
      try {
        const result = await previewWorktree(props.repoPath, value);
        if (thisRequest === requestId) {
          setPreview(result);
        }
      } catch (e) {
        if (thisRequest === requestId) {
          setError(String(e));
        }
      }
    }, 200);
  }

  const hasConflict = createMemo(() => {
    const p = preview();
    if (!p) return false;
    return p.pathExists || p.worktreeExists;
  });

  const isCreateDisabled = createMemo(() => {
    if (creating()) return true;
    if (!name().trim()) return true;
    const p = preview();
    if (!p) return true;
    if (!p.sanitizedName) return true;
    return hasConflict();
  });

  async function handleCreate(e: SubmitEvent) {
    e.preventDefault();
    if (isCreateDisabled()) return;

    const p = preview();
    if (!p) return;

    setCreating(true);
    setError(null);
    try {
      const wt = await repoStore.createWorktree(props.repoPath, p.sanitizedName);
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
        <Dialog.Content class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-96 bg-surface border border-border rounded-lg shadow-lg p-4">
          <Dialog.Title class="text-sm font-semibold text-text mb-3">New Worktree</Dialog.Title>

          <form onSubmit={handleCreate}>
            <input
              type="text"
              placeholder="Worktree name"
              value={name()}
              onInput={(e) => handleNameInput(e.currentTarget.value)}
              autofocus
              class="w-full px-3 py-1.5 text-xs bg-bg border border-border rounded text-text placeholder:text-text-muted focus:outline-none focus:border-primary"
            />

            <div class="mt-3 space-y-1.5 text-xs">
              <div class="flex gap-2">
                <span class="text-text-muted">Base:</span>
                <span class="text-text">{preview()?.baseBranch ?? '—'}</span>
              </div>
              <div class="flex gap-2">
                <span class="text-text-muted">Branch:</span>
                <span class="text-text">{preview()?.branchName ?? '—'}</span>
              </div>
              <div class="flex gap-2">
                <span class="text-text-muted">Path:</span>
                <span class="text-text truncate">{preview()?.worktreePath ?? '—'}</span>
              </div>
              <Show when={preview()?.sanitizedName === ''}>
                <p class="text-error">Name must contain at least one letter or number</p>
              </Show>
              <Show when={preview()?.branchExists}>
                <p class="text-info">Will use existing branch</p>
              </Show>
              <Show when={preview()?.pathExists}>
                <p class="text-error">Directory already exists</p>
              </Show>
              <Show when={preview()?.worktreeExists}>
                <p class="text-error">Worktree with this name already exists</p>
              </Show>
            </div>

            <Show when={error()}>
              <p class="mt-2 text-xs text-error">{error()}</p>
            </Show>

            <div class="mt-4 flex justify-end gap-2">
              <button
                type="button"
                class="px-3 py-1.5 text-xs text-text-muted hover:text-text cursor-pointer rounded hover:bg-bg/50"
                onClick={() => props.onClose()}
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={isCreateDisabled()}
                class="px-3 py-1.5 text-xs bg-primary text-bg rounded cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90"
              >
                {creating() ? 'Creating...' : 'Create Worktree'}
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
