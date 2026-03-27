import { PanelLeft, PanelRight } from 'lucide-solid';
import { createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { onEvent } from '../../lib/api/events';
import { cn } from '../../lib/cn';
import { cancelQueue } from '../../lib/commands/run';
import type { AppView } from '../../lib/stores/layout';
import { getLayoutStore } from '../../lib/stores/layout';
import { getLifecycleStore } from '../../lib/stores/lifecycle';
import { getRepoStore } from '../../lib/stores/repo';
import { getTaskStore } from '../../lib/stores/task';
import type { QueueStatus } from '../../types/github';
import { UpdateIndicator } from './UpdateIndicator';

const navTabs: { label: string; view: AppView }[] = [
  { label: 'Workspace', view: 'workspace' },
  { label: 'Inbox', view: 'inbox' },
  { label: 'SAT', view: 'sat' },
  { label: 'Tasks', view: 'tasks' },
  { label: 'Lifecycle', view: 'lifecycle' },
];

export function TopBar() {
  const repoStore = getRepoStore();
  const taskStore = getTaskStore();
  const layout = getLayoutStore();
  const lifecycleStore = getLifecycleStore();
  const [queue, setQueue] = createSignal<QueueStatus | null>(null);

  let unsubQueue: (() => void) | null = null;
  onMount(() => {
    unsubQueue = onEvent<QueueStatus>('run:queue_status', (envelope) => {
      const qs = envelope.data;
      if (qs.queued.length === 0 && !qs.active) {
        setQueue(null);
      } else {
        setQueue(qs);
      }
    });
  });
  onCleanup(() => unsubQueue?.());

  const queueBadgeColor = () => {
    const qs = queue();
    if (!qs) return '';
    if (qs.failed > 0) return 'text-accent-error';
    if (qs.active) return 'text-accent-warning';
    return 'text-accent-success';
  };

  const isActive = (view: AppView) => {
    const current = layout.activeView();
    // inbox absorbs legacy pr-triage
    if (view === 'inbox' && current === 'pr-triage') return true;
    return current === view;
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
      <Show when={repoStore.getActiveRepo() && layout.activeView() === 'workspace'}>
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
              onClick={() => layout.setActiveView('inbox')}
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

      {/* Update status */}
      <UpdateIndicator />

      {/* Navigation tabs */}
      <div class="ml-auto flex h-full items-center">
        <For each={navTabs}>
          {(tab) => (
            <button
              type="button"
              onClick={() => layout.setActiveView(tab.view)}
              class={cn(
                'relative px-4 h-full text-sm font-medium transition-colors duration-150 cursor-pointer',
                isActive(tab.view) ? 'text-text-main' : 'text-text-dim hover:text-text-main',
              )}
            >
              {tab.label}
              <Show when={tab.view === 'inbox' && lifecycleStore.getAttentionCount() > 0}>
                <span class="ml-1.5 bg-accent-error text-bg-main text-xs px-1.5 min-w-[18px] text-center">
                  {lifecycleStore.getAttentionCount()}
                </span>
              </Show>
              <Show when={tab.view === 'inbox' && taskStore.state.pendingPermissions.length > 0}>
                <span class="relative ml-1 flex h-2 w-2 inline-flex">
                  <span class="absolute inline-flex h-full w-full bg-accent-error opacity-75 animate-ping" />
                  <span class="relative inline-flex h-2 w-2 bg-accent-error" />
                </span>
              </Show>
              {/* Active indicator: 2px bottom border */}
              <div
                class={cn(
                  'absolute bottom-0 left-0 right-0 h-0.5',
                  isActive(tab.view) ? 'bg-accent-primary' : 'bg-transparent',
                )}
              />
            </button>
          )}
        </For>
      </div>

      <div class="flex items-center gap-1 ml-2">
        <Show when={layout.activeView() === 'workspace'}>
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
        </Show>
      </div>
    </div>
  );
}
