import { listen } from '@tauri-apps/api/event';
import { PanelLeft, PanelRight } from 'lucide-solid';
import { createSignal, onCleanup, onMount, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import { cancelQueue } from '../../lib/commands/run';
import { getLayoutStore } from '../../lib/stores/layout';
import { getLifecycleStore } from '../../lib/stores/lifecycle';
import { getRepoStore } from '../../lib/stores/repo';
import { getTaskStore } from '../../lib/stores/task';
import type { QueueStatus } from '../../types/github';

export function TopBar() {
  const repoStore = getRepoStore();
  const taskStore = getTaskStore();
  const layout = getLayoutStore();
  const lifecycleStore = getLifecycleStore();
  const [queue, setQueue] = createSignal<QueueStatus | null>(null);

  let unlistenQueue: (() => void) | null = null;
  onMount(() => {
    listen<QueueStatus>('run:queue_status', (e) => {
      const qs = e.payload;
      if (qs.queued.length === 0 && !qs.active) {
        setQueue(null);
      } else {
        setQueue(qs);
      }
    }).then((fn) => {
      unlistenQueue = fn;
    });
  });
  onCleanup(() => unlistenQueue?.());

  const queueBadgeColor = () => {
    const qs = queue();
    if (!qs) return '';
    if (qs.failed > 0) return 'text-accent-error';
    if (qs.active) return 'text-accent-warning';
    return 'text-accent-success';
  };

  return (
    <div class="flex items-center h-11 px-3 bg-bg-sidebar border-b border-border-subtle select-none">
      <Show when={layout.activeView() === 'workspace'}>
        <button
          type="button"
          class="mr-2 p-1.5 text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50 transition-colors duration-150"
          aria-label="Toggle repositories"
          title="Toggle repositories (Ctrl+Shift+B)"
          onClick={() => layout.toggleRepoSidebar()}
        >
          <PanelLeft size={16} />
        </button>
      </Show>
      <span class="text-sm font-bold text-accent-primary mr-4 tracking-tight">Branchdeck</span>
      <Show when={repoStore.getActiveRepo()}>
        <span class="text-base text-text-dim mr-1">{repoStore.getActiveRepo()?.name}</span>
        <Show when={repoStore.getActiveWorktree()}>
          <span class="text-base text-text-dim mr-1">/</span>
          <span class="text-base text-accent-info">{repoStore.getActiveWorktree()?.branch}</span>
        </Show>
      </Show>

      {/* Queue badge */}
      <Show when={queue()}>
        {(qs) => (
          <div class={cn('ml-4 flex items-center gap-2 text-base', queueBadgeColor())}>
            <button
              type="button"
              class="cursor-pointer hover:bg-bg-main/50 px-2 py-1 transition-colors duration-150"
              onClick={() => layout.setActiveView('pr-triage')}
              title="View batch queue"
            >
              <span class={cn(qs().active ? 'animate-pulse' : '')}>
                {qs().active ? '1 running' : ''}
                {qs().queued.length > 0 ? ` · ${qs().queued.length} queued` : ''}
                {qs().completed > 0 ? ` · ${qs().completed} done` : ''}
                {qs().failed > 0 ? ` · ${qs().failed} failed` : ''}
              </span>
            </button>
            <button
              type="button"
              class="text-base text-accent-error hover:text-accent-error/80 cursor-pointer"
              onClick={() => cancelQueue().catch(() => {})}
            >
              Cancel
            </button>
          </div>
        )}
      </Show>

      <div class="ml-auto flex h-full">
        <button
          type="button"
          onClick={() => layout.setActiveView('workspace')}
          class={cn(
            'px-4 h-full text-base font-bold uppercase tracking-wider border-x border-border-subtle transition-colors duration-150 cursor-pointer',
            layout.activeView() === 'workspace'
              ? 'bg-bg-main text-accent-primary'
              : 'text-text-dim hover:text-text-main',
          )}
        >
          Workspace
        </button>
        <button
          type="button"
          onClick={() => layout.setActiveView('pr-triage')}
          class={cn(
            'px-4 h-full text-base font-bold uppercase tracking-wider border-r border-border-subtle transition-colors duration-150 cursor-pointer flex items-center gap-2',
            layout.activeView() === 'pr-triage'
              ? 'bg-bg-main text-accent-primary'
              : 'text-text-dim hover:text-text-main',
          )}
        >
          PR Triage
          <Show when={lifecycleStore.getAttentionCount() > 0}>
            <span class="bg-accent-error text-white text-xs px-1.5 min-w-[18px] text-center">
              {lifecycleStore.getAttentionCount()}
            </span>
          </Show>
          <Show when={taskStore.state.pendingPermissions.length > 0}>
            <span class="relative flex h-2 w-2">
              <span class="absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75 animate-ping" />
              <span class="relative inline-flex rounded-full h-2 w-2 bg-red-400" />
            </span>
          </Show>
        </button>
      </div>

      <div class="flex items-center gap-1 ml-2">
        <button
          type="button"
          class={cn(
            'p-1.5 cursor-pointer hover:bg-bg-main/50 transition-colors duration-150',
            layout.rightPanelContext().kind === 'changes'
              ? 'text-accent-primary'
              : 'text-text-dim hover:text-text-main',
          )}
          aria-label="Toggle changes"
          title="Changes (Ctrl+Shift+L)"
          onClick={() => layout.showRightPanel({ kind: 'changes' })}
        >
          <PanelRight size={16} />
        </button>
      </div>
    </div>
  );
}
