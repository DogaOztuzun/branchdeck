import { createMemo, For, Show } from 'solid-js';
import { getRepoStore } from '../../lib/stores/repo';
import { getTerminalStore } from '../../lib/stores/terminal';
import { TabBar } from './TabBar';
import { TerminalView } from './TerminalView';

export function TerminalArea() {
  const terminalStore = getTerminalStore();
  const repoStore = getRepoStore();

  const worktreePath = () => repoStore.state.activeWorktreePath ?? '';

  const visibleTabs = createMemo(() =>
    terminalStore.state.tabs.filter((t) => t.worktreePath === worktreePath()),
  );

  const activeTabId = createMemo(
    () => terminalStore.state.activeTabByWorktree[worktreePath()] ?? null,
  );

  return (
    <div class="flex flex-col h-full">
      <TabBar
        tabs={visibleTabs()}
        activeTabId={activeTabId()}
        onSelectTab={(id) => terminalStore.setActiveTab(worktreePath(), id)}
        onCloseTab={(id) => terminalStore.closeTab(id)}
        onNewShell={() => terminalStore.openShellTab(worktreePath())}
        onNewClaude={() => terminalStore.openClaudeTab(worktreePath())}
      />
      <div class="flex-1 relative">
        {/* Empty state — shown above terminals, does not unmount them */}
        <Show when={visibleTabs().length === 0}>
          <div class="absolute inset-0 flex flex-col items-center justify-center text-text-muted z-10">
            <div class="text-sm mb-4">
              {worktreePath() ? 'No terminal open' : 'Select a repository to start'}
            </div>
            <Show when={worktreePath()}>
              <div class="flex gap-4">
                <button
                  type="button"
                  class="px-4 py-2 text-xs border border-border rounded hover:border-primary hover:text-text cursor-pointer"
                  onClick={() => terminalStore.openShellTab(worktreePath())}
                >
                  Open Terminal
                  <span class="ml-2 text-text-muted">Ctrl+Shift+T</span>
                </button>
                <button
                  type="button"
                  class="px-4 py-2 text-xs border border-border rounded hover:border-primary hover:text-text cursor-pointer"
                  onClick={() => terminalStore.openClaudeTab(worktreePath())}
                >
                  Start Claude Code
                  <span class="ml-2 text-text-muted">Ctrl+Shift+A</span>
                </button>
              </div>
            </Show>
          </div>
        </Show>
        {/* All terminals always mounted — visibility controlled per-terminal */}
        <For each={terminalStore.state.tabs}>
          {(tab) => (
            <TerminalView
              sessionId={tab.sessionId}
              visible={tab.worktreePath === worktreePath() && activeTabId() === tab.id}
            />
          )}
        </For>
      </div>
    </div>
  );
}
