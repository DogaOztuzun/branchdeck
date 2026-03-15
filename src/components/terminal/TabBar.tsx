import { createSignal, For, Show } from 'solid-js';
import type { TabInfo } from '../../types/terminal';

type TabBarProps = {
  tabs: TabInfo[];
  activeTabId: string | null;
  onSelectTab: (tabId: string) => void;
  onCloseTab: (tabId: string) => void;
  onNewShell: () => void;
  onNewClaude: () => void;
};

export function TabBar(props: TabBarProps) {
  const [menuOpen, setMenuOpen] = createSignal(false);

  return (
    <div class="flex items-center bg-surface border-b border-border h-9 select-none">
      <div class="flex flex-1 overflow-x-auto">
        <For each={props.tabs}>
          {(tab) => (
            <div
              class={`flex items-center gap-1.5 px-3 py-1.5 text-xs border-r border-border whitespace-nowrap ${
                props.activeTabId === tab.id
                  ? 'bg-bg text-primary'
                  : 'text-text-muted hover:text-text hover:bg-bg/50'
              }`}
            >
              <button
                type="button"
                class="flex items-center"
                onClick={() => props.onSelectTab(tab.id)}
              >
                <span>{tab.title}</span>
              </button>
              <button
                type="button"
                class="ml-1 opacity-50 hover:opacity-100 hover:text-error cursor-pointer"
                aria-label="Close tab"
                onClick={(e) => {
                  e.stopPropagation();
                  props.onCloseTab(tab.id);
                }}
              >
                {'\u00D7'}
              </button>
            </div>
          )}
        </For>
      </div>
      <div class="relative">
        <button
          type="button"
          class="px-3 py-1.5 text-xs text-text-muted hover:text-text cursor-pointer"
          onClick={() => setMenuOpen(!menuOpen())}
        >
          +
        </button>
        <Show when={menuOpen()}>
          <div class="absolute right-0 top-full z-50 bg-surface border border-border rounded shadow-lg min-w-40">
            <button
              type="button"
              class="block w-full text-left px-3 py-2 text-xs text-text hover:bg-bg cursor-pointer"
              onClick={() => {
                props.onNewShell();
                setMenuOpen(false);
              }}
            >
              New Terminal
              <span class="ml-2 text-text-muted">Ctrl+Shift+T</span>
            </button>
            <button
              type="button"
              class="block w-full text-left px-3 py-2 text-xs text-text hover:bg-bg cursor-pointer"
              onClick={() => {
                props.onNewClaude();
                setMenuOpen(false);
              }}
            >
              New Claude Code
              <span class="ml-2 text-text-muted">Ctrl+Shift+A</span>
            </button>
          </div>
        </Show>
      </div>
    </div>
  );
}
