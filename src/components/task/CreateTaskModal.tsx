import { Dialog } from '@kobalte/core';
import { createEffect, createMemo, createSignal, Show } from 'solid-js';
import { createTask, watchTaskPath } from '../../lib/commands/task';
import type { TaskType } from '../../types/task';

type CreateTaskModalProps = {
  open: boolean;
  worktreePath: string;
  repo: string;
  branch: string;
  onClose: () => void;
};

export function CreateTaskModal(props: CreateTaskModalProps) {
  const [taskType, setTaskType] = createSignal<TaskType>('issue-fix');
  const [prNumber, setPrNumber] = createSignal('');
  const [description, setDescription] = createSignal('');
  const [error, setError] = createSignal<string | null>(null);
  const [creating, setCreating] = createSignal(false);

  createEffect(() => {
    if (props.open) {
      setTaskType('issue-fix');
      setPrNumber('');
      setDescription('');
      setError(null);
      setCreating(false);
    }
  });

  const isPrShepherd = createMemo(() => taskType() === 'pr-shepherd');

  const isCreateDisabled = createMemo(() => {
    if (creating()) return true;
    if (isPrShepherd() && !prNumber().trim()) return true;
    return false;
  });

  async function handleCreate(e: SubmitEvent) {
    e.preventDefault();
    if (isCreateDisabled()) return;

    setCreating(true);
    setError(null);
    try {
      const prRaw = prNumber().trim();
      const pr = isPrShepherd() ? Number.parseInt(prRaw, 10) : undefined;
      if (isPrShepherd() && (Number.isNaN(pr) || !pr || pr <= 0 || String(pr) !== prRaw)) {
        setError('PR number must be a positive integer');
        setCreating(false);
        return;
      }
      const desc = description().trim() || undefined;
      await createTask(props.worktreePath, taskType(), props.repo, props.branch, pr, desc);
      await watchTaskPath(props.worktreePath).catch(() => {});
      props.onClose();
    } catch (e) {
      const msg = String(e);
      if (msg.includes('AlreadyExists') || msg.includes('already exists')) {
        setError('Task already exists for this worktree');
      } else {
        setError(msg);
      }
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
          <Dialog.Title class="text-sm font-semibold text-text mb-3">New Task</Dialog.Title>

          <form onSubmit={handleCreate}>
            <div>
              <label class="text-xs text-text-muted" for="task-type-select">
                Task type
              </label>
              <select
                id="task-type-select"
                value={taskType()}
                onChange={(e) => setTaskType(e.currentTarget.value as TaskType)}
                style={{ 'background-color': 'var(--color-bg)', color: 'var(--color-text)' }}
                class="w-full mt-1 px-3 py-1.5 text-xs border border-border rounded focus:outline-none focus:border-primary appearance-none [&>option]:bg-bg [&>option]:text-text"
              >
                <option value="issue-fix">Issue Fix</option>
                <option value="pr-shepherd">PR Shepherd</option>
              </select>
            </div>

            <Show when={isPrShepherd()}>
              <div class="mt-2">
                <label class="text-xs text-text-muted" for="pr-number-input">
                  PR number
                </label>
                <input
                  id="pr-number-input"
                  type="number"
                  min="1"
                  placeholder="#123"
                  value={prNumber()}
                  onInput={(e) => setPrNumber(e.currentTarget.value)}
                  class="w-full mt-1 px-3 py-1.5 text-xs bg-bg border border-border rounded text-text placeholder:text-text-muted focus:outline-none focus:border-primary"
                />
              </div>
            </Show>

            <div class="mt-2">
              <label class="text-xs text-text-muted" for="task-description">
                Description
              </label>
              <textarea
                id="task-description"
                placeholder="What should the agent do?"
                value={description()}
                onInput={(e) => setDescription(e.currentTarget.value)}
                rows={3}
                class="w-full mt-1 px-3 py-1.5 text-xs bg-bg border border-border rounded text-text placeholder:text-text-muted focus:outline-none focus:border-primary resize-y"
              />
            </div>

            <div class="mt-3 space-y-1.5 text-xs">
              <div class="flex gap-2">
                <span class="text-text-muted">Repo:</span>
                <span class="text-text truncate">{props.repo}</span>
              </div>
              <div class="flex gap-2">
                <span class="text-text-muted">Branch:</span>
                <span class="text-text truncate">{props.branch}</span>
              </div>
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
                {creating() ? 'Creating...' : 'Create Task'}
              </button>
            </div>
          </form>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
