import { Dialog } from '@kobalte/core';
import { createEffect, createSignal, Show } from 'solid-js';

type DeleteWorktreeDialogProps = {
  open: boolean;
  worktreeName: string;
  onClose: () => void;
  onConfirm: (deleteBranch: boolean) => void;
};

export function DeleteWorktreeDialog(props: DeleteWorktreeDialogProps) {
  const [deleteBranch, setDeleteBranch] = createSignal(true);

  createEffect(() => {
    if (props.open) {
      setDeleteBranch(true);
    }
  });

  return (
    <Dialog.Root
      open={props.open}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-40 bg-black/50" />
        <Dialog.Content class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-96 bg-bg-sidebar border border-border-subtle rounded-lg shadow-lg p-5">
          <Dialog.Title class="text-sm font-semibold text-text-main">
            <Show when={props.open}>Remove worktree "{props.worktreeName}"?</Show>
          </Dialog.Title>

          <p class="mt-2 text-xs text-text-dim">
            Deleting will permanently remove the worktree directory from disk.
          </p>

          <label class="flex items-center gap-2 mt-4 text-xs text-text-main cursor-pointer">
            <input
              type="checkbox"
              checked={deleteBranch()}
              onChange={(e) => setDeleteBranch(e.currentTarget.checked)}
              class="accent-primary"
            />
            Also delete local branch
          </label>

          <div class="mt-5 flex justify-end gap-2">
            <button
              type="button"
              class="px-3 py-1.5 text-xs text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50"
              onClick={() => props.onClose()}
            >
              Cancel
            </button>
            <button
              type="button"
              class="px-3 py-1.5 text-xs bg-accent-error/90 text-white cursor-pointer hover:bg-accent-error"
              onClick={() => props.onConfirm(deleteBranch())}
            >
              Delete
            </button>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
