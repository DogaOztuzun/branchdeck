import { createEffect, createMemo, createSignal, onCleanup, Show } from 'solid-js';
import { listBranches, previewWorktree } from '../../lib/commands/git';
import { getRepoStore } from '../../lib/stores/repo';
import type { BranchInfo, WorktreeInfo, WorktreePreview } from '../../types/git';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/Dialog';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/Select';

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
  const [branches, setBranches] = createSignal<BranchInfo[]>([]);
  const [baseBranch, setBaseBranch] = createSignal<string>('');

  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let requestId = 0;

  // Reset state when modal opens/closes
  createEffect(() => {
    if (props.open) {
      setName('');
      setPreview(null);
      setError(null);
      setCreating(false);
      setBranches([]);
      setBaseBranch('');
      requestId = 0;
      listBranches(props.repoPath)
        .then((result) => {
          setBranches(result);
          const head = result.find((b) => b.isHead);
          if (head) setBaseBranch(head.name);
        })
        .catch(() => {});
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

  const localBranches = createMemo(() => branches().filter((b) => !b.isRemote));

  async function handleCreate(e: SubmitEvent) {
    e.preventDefault();
    if (isCreateDisabled()) return;

    const p = preview();
    if (!p) return;

    setCreating(true);
    setError(null);
    try {
      const base = baseBranch() || undefined;
      const wt = await repoStore.createWorktree(props.repoPath, p.sanitizedName, undefined, base);
      props.onCreate(wt);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }

  return (
    <Dialog
      open={props.open}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
    >
      <DialogContent class="max-w-sm" showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>New Worktree</DialogTitle>
        </DialogHeader>

        <form onSubmit={handleCreate}>
          <input
            type="text"
            placeholder="Worktree name"
            value={name()}
            onInput={(e) => handleNameInput(e.currentTarget.value)}
            autofocus
            class="w-full px-3 py-1.5 text-xs bg-bg-main border border-border-subtle text-text-main placeholder:text-text-dim focus:outline-none focus:border-accent-primary"
          />

          <Show when={localBranches().length > 0}>
            <div class="mt-2">
              <span class="text-xs text-text-dim">Base branch</span>
              <div class="mt-1">
                <Select
                  options={localBranches().map((b) => b.name)}
                  value={baseBranch()}
                  onChange={(val) => setBaseBranch(val ?? '')}
                  placeholder="Select base branch"
                  itemComponent={(props) => (
                    <SelectItem item={props.item}>{props.item.rawValue}</SelectItem>
                  )}
                >
                  <SelectTrigger>
                    <SelectValue<string>>{(state) => state.selectedOption()}</SelectValue>
                  </SelectTrigger>
                  <SelectContent />
                </Select>
              </div>
            </div>
          </Show>

          <div class="mt-3 space-y-1.5 text-xs">
            <div class="flex gap-2">
              <span class="text-text-dim">Base:</span>
              <span class="text-text-main">{baseBranch() || preview()?.baseBranch || '—'}</span>
            </div>
            <div class="flex gap-2">
              <span class="text-text-dim">Branch:</span>
              <span class="text-text-main">{preview()?.branchName ?? '—'}</span>
            </div>
            <div class="flex gap-2">
              <span class="text-text-dim">Path:</span>
              <span class="text-text-main truncate">{preview()?.worktreePath ?? '—'}</span>
            </div>
            <Show when={preview()?.sanitizedName === ''}>
              <p class="text-accent-error">Name must contain at least one letter or number</p>
            </Show>
            <Show when={preview()?.branchExists}>
              <p class="text-accent-info">Will use existing branch</p>
            </Show>
            <Show when={preview()?.pathExists}>
              <p class="text-accent-error">Directory already exists</p>
            </Show>
            <Show when={preview()?.worktreeExists}>
              <p class="text-accent-error">Worktree with this name already exists</p>
            </Show>
          </div>

          <Show when={error()}>
            <p class="mt-2 text-xs text-accent-error">{error()}</p>
          </Show>

          <div class="mt-4 flex justify-end gap-2">
            <button
              type="button"
              class="px-3 py-1.5 text-xs text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50"
              onClick={() => props.onClose()}
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isCreateDisabled()}
              class="px-3 py-1.5 text-xs bg-accent-primary text-bg cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed hover:opacity-90"
            >
              {creating() ? 'Creating...' : 'Create Worktree'}
            </button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
