import { Show } from 'solid-js';
import { getLayoutStore } from '../../lib/stores/layout';
import { getRepoStore } from '../../lib/stores/repo';

export function TopBar() {
  const repoStore = getRepoStore();
  const layout = getLayoutStore();

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
