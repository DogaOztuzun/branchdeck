import { For, Show } from 'solid-js';
import { getRepoStore } from '../../lib/stores/repo';
import { getTerminalStore } from '../../lib/stores/terminal';
import { TabBar } from './TabBar';
import { TerminalView } from './TerminalView';

export function TerminalArea() {
  const terminalStore = getTerminalStore();
  const repoStore = getRepoStore();

  function getCwd(): string {
    return repoStore.state.activeWorktree?.path ?? repoStore.state.activeRepo?.path ?? '.';
  }

  return (
    <div class="flex flex-col h-full">
      <TabBar
        tabs={terminalStore.state.tabs}
        activeTabId={terminalStore.state.activeTabId}
        onSelectTab={(id) => terminalStore.setActiveTab(id)}
        onCloseTab={(id) => terminalStore.closeTab(id)}
        onNewShell={() => terminalStore.openShellTab(getCwd())}
        onNewClaude={() => terminalStore.openClaudeTab(getCwd())}
      />
      <div class="flex-1 relative">
        <Show
          when={terminalStore.state.tabs.length > 0}
          fallback={
            <div class="flex flex-col items-center justify-center h-full text-text-muted">
              <div class="text-sm mb-4">No terminal open</div>
              <div class="flex gap-4">
                <button
                  type="button"
                  class="px-4 py-2 text-xs border border-border rounded hover:border-primary hover:text-text cursor-pointer"
                  onClick={() => terminalStore.openShellTab(getCwd())}
                >
                  Open Terminal
                  <span class="ml-2 text-text-muted">Ctrl+Shift+T</span>
                </button>
                <button
                  type="button"
                  class="px-4 py-2 text-xs border border-border rounded hover:border-primary hover:text-text cursor-pointer"
                  onClick={() => terminalStore.openClaudeTab(getCwd())}
                >
                  Start Claude Code
                  <span class="ml-2 text-text-muted">Ctrl+Shift+A</span>
                </button>
              </div>
            </div>
          }
        >
          <For each={terminalStore.state.tabs}>
            {(tab) => (
              <TerminalView
                sessionId={tab.sessionId}
                visible={terminalStore.state.activeTabId === tab.id}
              />
            )}
          </For>
        </Show>
      </div>
    </div>
  );
}
