import { Show } from 'solid-js';
import { respondToPermission } from '../../lib/commands/run';
import { getTaskStore } from '../../lib/stores/task';

export function ApprovalBanner() {
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
    <Show when={currentPerm()}>
      {(perm) => (
        <div class="border-t border-accent-warning/30 bg-bg-sidebar/95 px-3 py-1.5">
          <div class="flex items-center gap-2">
            <span class="w-1.5 h-1.5 rounded-full bg-accent-warning animate-pulse shrink-0" />
            <span class="text-[10px] text-accent-warning font-medium uppercase tracking-wider">
              Permission
            </span>
            <Show when={totalPending() > 1}>
              <span class="text-xs text-text-dim">({totalPending()})</span>
            </Show>
            <span class="text-xs font-mono text-accent-info truncate">
              {perm().tool ?? 'unknown'}
            </span>
            <Show when={perm().command}>
              <span class="text-xs text-text-dim font-mono truncate max-w-xs">
                {perm().command}
              </span>
            </Show>
            <div class="ml-auto flex items-center gap-1.5 shrink-0">
              <button
                type="button"
                class="px-2 py-0.5 text-xs font-medium text-green-400 border border-green-400/30 hover:bg-green-400/10 cursor-pointer"
                onClick={() => handleRespond('approve')}
              >
                Approve
              </button>
              <button
                type="button"
                class="px-2 py-0.5 text-xs font-medium text-red-400 border border-red-400/30 hover:bg-red-400/10 cursor-pointer"
                onClick={() => handleRespond('deny')}
              >
                Deny
              </button>
            </div>
          </div>
        </div>
      )}
    </Show>
  );
}

/** Count of pending permissions — use for red dot indicators */
export function usePendingCount() {
  const taskStore = getTaskStore();
  return () => taskStore.state.pendingPermissions.length;
}
