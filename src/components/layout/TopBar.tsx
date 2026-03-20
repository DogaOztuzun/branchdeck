import { listen } from '@tauri-apps/api/event';
import { createSignal, onCleanup, onMount, Show } from 'solid-js';
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
    <div class="flex items-center h-11 px-3 bg-surface border-b border-border select-none">
      <button
        type="button"
        class="mr-2 p-1.5 text-text-muted hover:text-text cursor-pointer rounded hover:bg-bg/50"
        aria-label="Toggle repositories"
        title="Toggle repositories (Ctrl+Shift+B)"
        onClick={() => layout.toggleRepoSidebar()}
      >
        <svg
          aria-hidden="true"
          width="18"
          height="18"
          viewBox="0 0 18 18"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <rect x="1.5" y="1.5" width="15" height="15" rx="2" />
          <line x1="6.5" y1="1.5" x2="6.5" y2="16.5" />
        </svg>
      </button>
      <span class="text-sm font-bold text-primary mr-4">Branchdeck</span>
      <Show when={repoStore.getActiveRepo()}>
        <span class="text-xs text-text mr-2">{repoStore.getActiveRepo()?.name}</span>
        <Show when={repoStore.getActiveWorktree()}>
          <span class="text-xs text-text-muted mr-1">/</span>
          <span class="text-xs text-info">{repoStore.getActiveWorktree()?.branch}</span>
        </Show>
      </Show>
      <Show when={queue()}>
        {(qs) => (
          <div class="ml-4 flex items-center gap-2 text-[10px] text-text-muted">
            <span>
              Queue: {qs().active ? '1 running' : ''}
              {qs().queued.length > 0 ? ` · ${qs().queued.length} queued` : ''}
              {qs().completed > 0 ? ` · ${qs().completed} done` : ''}
              {qs().failed > 0 ? ` · ${qs().failed} failed` : ''}
            </span>
            <button
              type="button"
              class="text-[10px] text-red-400 hover:text-red-300 cursor-pointer"
              onClick={() => cancelQueue().catch(() => {})}
            >
              Cancel
            </button>
          </div>
        )}
      </Show>
      <div class="ml-auto flex items-center gap-1">
        <button
          type="button"
          class="p-1.5 text-text-muted hover:text-text cursor-pointer rounded hover:bg-bg/50"
          aria-label="Toggle team"
          title="Toggle team sidebar"
          onClick={() => layout.toggleTeamSidebar()}
        >
          <svg
            aria-hidden="true"
            width="18"
            height="18"
            viewBox="0 0 18 18"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <circle cx="6" cy="6" r="2.5" />
            <circle cx="12" cy="6" r="2.5" />
            <circle cx="9" cy="13" r="2.5" />
          </svg>
        </button>
        <button
          type="button"
          class="p-1.5 text-text-muted hover:text-text cursor-pointer rounded hover:bg-bg/50"
          aria-label="Toggle dashboard"
          title="Toggle task dashboard"
          onClick={() => layout.toggleDashboard()}
        >
          <svg
            aria-hidden="true"
            width="18"
            height="18"
            viewBox="0 0 18 18"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="1.5" y="1.5" width="6" height="6" rx="1" />
            <rect x="10.5" y="1.5" width="6" height="6" rx="1" />
            <rect x="1.5" y="10.5" width="6" height="6" rx="1" />
            <rect x="10.5" y="10.5" width="6" height="6" rx="1" />
          </svg>
        </button>
        <button
          type="button"
          class="p-1.5 text-text-muted hover:text-text cursor-pointer rounded hover:bg-bg/50"
          aria-label="Toggle changes"
          title="Toggle changes (Ctrl+Shift+L)"
          onClick={() => layout.toggleChangesSidebar()}
        >
          <svg
            aria-hidden="true"
            width="18"
            height="18"
            viewBox="0 0 18 18"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="1.5" y="1.5" width="15" height="15" rx="2" />
            <line x1="11.5" y1="1.5" x2="11.5" y2="16.5" />
          </svg>
        </button>
      </div>
    </div>
  );
}
