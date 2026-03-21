import { listen } from '@tauri-apps/api/event';
import { LayoutGrid, PanelLeft, PanelRight, Users } from 'lucide-solid';
import { createSignal, onCleanup, onMount, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import { cancelQueue } from '../../lib/commands/run';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';
import type { QueueStatus } from '../../types/github';

export function TopBar() {
  const repoStore = getRepoStore();
  const layout = getLayoutStore();
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

  return (
    <div class="flex items-center h-11 px-3 bg-bg-sidebar border-b border-border-subtle select-none">
      <button
        type="button"
        class="mr-2 p-1.5 text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50 transition-colors duration-150"
        aria-label="Toggle repositories"
        title="Toggle repositories (Ctrl+Shift+B)"
        onClick={() => layout.toggleRepoSidebar()}
      >
        <PanelLeft size={16} />
      </button>
      <span class="text-sm font-bold text-accent-primary mr-4 tracking-tight">Branchdeck</span>
      <Show when={repoStore.getActiveRepo()}>
        <span class="text-xs text-text-dim mr-1">{repoStore.getActiveRepo()?.name}</span>
        <Show when={repoStore.getActiveWorktree()}>
          <span class="text-xs text-text-dim mr-1">/</span>
          <span class="text-xs text-accent-info">{repoStore.getActiveWorktree()?.branch}</span>
        </Show>
      </Show>
      <Show when={queue()}>
        {(qs) => (
          <div class="ml-4 flex items-center gap-2 text-[10px] text-text-dim">
            <span>
              Queue: {qs().active ? '1 running' : ''}
              {qs().queued.length > 0 ? ` · ${qs().queued.length} queued` : ''}
              {qs().completed > 0 ? ` · ${qs().completed} done` : ''}
              {qs().failed > 0 ? ` · ${qs().failed} failed` : ''}
            </span>
            <button
              type="button"
              class="text-[10px] text-accent-error hover:text-accent-error/80 cursor-pointer"
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
            'px-4 h-full text-[11px] font-bold uppercase tracking-wider border-x border-border-subtle transition-colors duration-150 cursor-pointer',
            layout.activeView() === 'workspace'
              ? 'bg-bg-main text-accent-primary'
              : 'text-text-dim hover:text-text-main',
          )}
        >
          Workspace
        </button>
        <button
          type="button"
          onClick={() => layout.setActiveView('orchestrations')}
          class={cn(
            'px-4 h-full text-[11px] font-bold uppercase tracking-wider border-r border-border-subtle transition-colors duration-150 cursor-pointer flex items-center gap-2',
            layout.activeView() === 'orchestrations'
              ? 'bg-bg-main text-accent-primary'
              : 'text-text-dim hover:text-text-main',
          )}
        >
          Orchestrations
          <Show when={queue()}>
            <span class="bg-accent-warning/20 text-accent-warning px-1 text-[9px]">
              {queue()?.active ? 1 : 0}
            </span>
          </Show>
        </button>
      </div>

      <div class="flex items-center gap-1 ml-2">
        <button
          type="button"
          class="p-1.5 text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50 transition-colors duration-150"
          aria-label="Toggle team"
          title="Toggle team sidebar"
          onClick={() => layout.toggleTeamSidebar()}
        >
          <Users size={16} />
        </button>
        <button
          type="button"
          class="p-1.5 text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50 transition-colors duration-150"
          aria-label="Toggle dashboard"
          title="Toggle task dashboard"
          onClick={() => layout.toggleDashboard()}
        >
          <LayoutGrid size={16} />
        </button>
        <button
          type="button"
          class="p-1.5 text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/50 transition-colors duration-150"
          aria-label="Toggle changes"
          title="Toggle changes (Ctrl+Shift+L)"
          onClick={() => layout.toggleChangesSidebar()}
        >
          <PanelRight size={16} />
        </button>
      </div>
    </div>
  );
}
