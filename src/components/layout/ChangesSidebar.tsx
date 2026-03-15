import { For, Show } from 'solid-js';
import { getRepoStore } from '../../lib/stores/repo';

function statusColor(status: string): string {
  switch (status) {
    case 'modified':
      return 'text-warning';
    case 'new':
      return 'text-success';
    case 'deleted':
      return 'text-error';
    case 'renamed':
      return 'text-info';
    case 'conflicted':
      return 'text-error';
    default:
      return 'text-text-muted';
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case 'modified':
      return 'M';
    case 'new':
      return 'A';
    case 'deleted':
      return 'D';
    case 'renamed':
      return 'R';
    case 'conflicted':
      return 'C';
    default:
      return '?';
  }
}

function statusTooltip(status: string): string {
  switch (status) {
    case 'modified':
      return 'Modified';
    case 'new':
      return 'Added';
    case 'deleted':
      return 'Deleted';
    case 'renamed':
      return 'Renamed';
    case 'conflicted':
      return 'Conflicted';
    default:
      return 'Unknown';
  }
}

export function ChangesSidebar() {
  const repoStore = getRepoStore();

  return (
    <div class="flex flex-col h-full bg-surface">
      <div class="px-3 py-2 text-xs font-bold text-text-muted uppercase tracking-wider border-b border-border">
        Changes
        <Show when={repoStore.state.statuses.length > 0}>
          <span class="ml-1.5 text-text-muted font-normal">
            ({repoStore.state.statuses.length})
          </span>
        </Show>
      </div>
      <div class="flex-1 overflow-y-auto">
        <Show
          when={repoStore.state.statuses.length > 0}
          fallback={<div class="px-3 py-4 text-xs text-text-muted">No changes detected</div>}
        >
          <For each={repoStore.state.statuses}>
            {(file) => (
              <div class="flex items-center px-3 py-1 text-xs hover:bg-bg/50 cursor-default">
                <span
                  class={`w-4 font-bold ${statusColor(file.status)}`}
                  title={statusTooltip(file.status)}
                >
                  {statusLabel(file.status)}
                </span>
                <span class="ml-2 truncate text-text" title={file.path}>
                  {file.path}
                </span>
              </div>
            )}
          </For>
        </Show>
      </div>
    </div>
  );
}
