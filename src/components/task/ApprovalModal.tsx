import { Dialog } from '@kobalte/core/dialog';
import { Show } from 'solid-js';
import { respondToPermission } from '../../lib/commands/run';
import { getTaskStore } from '../../lib/stores/task';

export function ApprovalModal() {
  const taskStore = getTaskStore();

  const currentPerm = () => taskStore.state.pendingPermissions[0] ?? null;
  const totalPending = () => taskStore.state.pendingPermissions.length;

  function handleRespond(decision: 'approve' | 'deny') {
    const perm = currentPerm();
    if (!perm) return;
    taskStore.removePermission(perm.toolUseId);
    respondToPermission(perm.toolUseId, decision).catch(() => {});
  }

  return (
    <Dialog open={!!currentPerm()} onOpenChange={() => {}}>
      <Dialog.Portal>
        <Dialog.Overlay class="fixed inset-0 z-50 bg-black/60" />
        <Dialog.Content class="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[420px] bg-bg-sidebar border border-accent-warning/40 shadow-2xl focus:outline-none">
          <Show when={currentPerm()}>
            {(perm) => (
              <>
                <div class="px-4 pt-4 pb-2">
                  <div class="flex items-center justify-between mb-3">
                    <Dialog.Title class="text-xs font-bold uppercase text-accent-warning tracking-wider">
                      Permission Required
                    </Dialog.Title>
                    <Show when={totalPending() > 1}>
                      <span class="text-[10px] text-text-dim">1 of {totalPending()}</span>
                    </Show>
                  </div>

                  <div class="text-sm text-text-main mb-2">
                    Tool: <span class="font-mono text-accent-info">{perm().tool ?? 'unknown'}</span>
                  </div>

                  <Show when={perm().command}>
                    <div class="text-xs text-text-dim font-mono bg-bg-main px-3 py-2 mb-3 break-all max-h-32 overflow-y-auto border border-border-subtle">
                      {perm().command}
                    </div>
                  </Show>
                </div>

                <div class="flex border-t border-border-subtle">
                  <button
                    type="button"
                    class="flex-1 px-4 py-3 text-sm font-medium text-green-400 hover:bg-green-400/10 cursor-pointer transition-colors border-r border-border-subtle"
                    onClick={() => handleRespond('approve')}
                    autofocus
                  >
                    Approve
                  </button>
                  <button
                    type="button"
                    class="flex-1 px-4 py-3 text-sm font-medium text-red-400 hover:bg-red-400/10 cursor-pointer transition-colors"
                    onClick={() => handleRespond('deny')}
                  >
                    Deny
                  </button>
                </div>
              </>
            )}
          </Show>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog>
  );
}
