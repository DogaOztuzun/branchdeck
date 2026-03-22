import { createEffect, createSignal, Show } from 'solid-js';
import { Button } from '../ui/Button';
import { Checkbox, CheckboxLabel } from '../ui/Checkbox';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/Dialog';

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
    <Dialog
      open={props.open}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
    >
      <DialogContent class="max-w-sm" showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>
            <Show when={props.open}>Remove worktree "{props.worktreeName}"?</Show>
          </DialogTitle>
        </DialogHeader>

        <p class="mt-2 text-base text-text-dim">
          Deleting will permanently remove the worktree directory from disk.
        </p>

        <div class="flex items-center gap-2 mt-4">
          <Checkbox checked={deleteBranch()} onChange={(checked) => setDeleteBranch(checked)} />
          <CheckboxLabel class="text-base text-text-main cursor-pointer">
            Also delete local branch
          </CheckboxLabel>
        </div>

        <div class="mt-5 flex justify-end gap-2">
          <Button variant="ghost" size="compact" onClick={() => props.onClose()}>
            Cancel
          </Button>
          <Button variant="danger" size="compact" onClick={() => props.onConfirm(deleteBranch())}>
            Delete
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
