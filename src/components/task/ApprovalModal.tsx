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
        <div class="fixed bottom-0 left-0 right-0 z-50 border-t border-accent-warning/40 bg-bg-sidebar shadow-lg">
          <div class="flex items-center gap-3 px-4 py-2 max-w-screen-xl mx-auto">
            <span class="w-2 h-2 rounded-full bg-accent-warning animate-pulse shrink-0" />
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 text-xs">
                <span class="text-accent-warning font-medium uppercase tracking-wider text-[10px]">
                  Permission
                </span>
                <Show when={totalPending() > 1}>
                  <span class="text-[10px] text-text-dim">({totalPending()} pending)</span>
                </Show>
              </div>
              <div class="flex items-center gap-2 text-xs text-text-main mt-0.5">
                <span class="font-mono text-accent-info">{perm().tool ?? 'unknown'}</span>
                <Show when={perm().command}>
                  <span class="text-text-dim font-mono truncate max-w-md">{perm().command}</span>
                </Show>
              </div>
            </div>
            <div class="flex items-center gap-2 shrink-0">
              <button
                type="button"
                class="px-3 py-1.5 text-xs font-medium text-green-400 border border-green-400/40 hover:bg-green-400/10 cursor-pointer transition-colors"
                onClick={() => handleRespond('approve')}
              >
                Approve
              </button>
              <button
                type="button"
                class="px-3 py-1.5 text-xs font-medium text-red-400 border border-red-400/40 hover:bg-red-400/10 cursor-pointer transition-colors"
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
