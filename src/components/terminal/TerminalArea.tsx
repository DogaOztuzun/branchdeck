import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import type { Preset } from '../../lib/commands/workspace';
import { getPresets } from '../../lib/commands/workspace';
import { getAgentStore } from '../../lib/stores/agent';
import { getRepoStore } from '../../lib/stores/repo';
import { getTerminalStore } from '../../lib/stores/terminal';
import { Button } from '../ui/Button';
import { AgentActivity } from './AgentActivity';
import { FileStatusBar } from './FileStatusBar';
import { PresetManager } from './PresetManager';
import { TabBar } from './TabBar';
import { TerminalView } from './TerminalView';

export function TerminalArea() {
  const terminalStore = getTerminalStore();
  const repoStore = getRepoStore();
  const agentStore = getAgentStore();

  const worktreePath = () => repoStore.state.activeWorktreePath ?? '';
  const repoPath = () => repoStore.state.activeRepoPath ?? '';

  const visibleTabs = createMemo(() =>
    terminalStore.state.tabs.filter((t) => t.worktreePath === worktreePath()),
  );

  const activeTabId = createMemo(
    () => terminalStore.state.activeTabByWorktree[worktreePath()] ?? null,
  );

  const [presets, setPresets] = createSignal<Preset[]>([]);
  const [presetManagerOpen, setPresetManagerOpen] = createSignal(false);
  const [presetVersion, setPresetVersion] = createSignal(0);
  const [activityVisible, setActivityVisible] = createSignal(false);

  const hasClaudeTab = createMemo(() => visibleTabs().some((t) => t.type === 'claude'));

  const agentLog = createMemo(() => {
    const claudeTabIds = new Set(
      visibleTabs()
        .filter((t) => t.type === 'claude')
        .map((t) => t.id),
    );
    if (claudeTabIds.size === 0) return [];
    return agentStore.state.log.filter((e) => claudeTabIds.has(e.tabId));
  });

  onMount(() => {
    agentStore.setTabFilter((tabId) => terminalStore.state.tabs.some((t) => t.id === tabId));
    agentStore.startListening();
  });

  onCleanup(() => {
    agentStore.stopListening();
  });

  createEffect(() => {
    const repo = repoPath();
    presetVersion();
    if (repo) {
      getPresets(repo)
        .then((result) => setPresets(result))
        .catch(() => setPresets([]));
    } else {
      setPresets([]);
    }
  });

  return (
    <div class="flex flex-col h-full">
      <TabBar
        tabs={visibleTabs()}
        activeTabId={activeTabId()}
        onSelectTab={(id) => terminalStore.setActiveTab(worktreePath(), id)}
        onCloseTab={(id) => terminalStore.closeTab(id)}
        onNewShell={() => terminalStore.openShellTab(worktreePath())}
        onNewClaude={() => terminalStore.openClaudeTab(worktreePath())}
        presets={presets()}
        onRunPreset={(preset) => terminalStore.runPreset(worktreePath(), preset)}
        onManagePresets={() => setPresetManagerOpen(true)}
        getTabAgent={(tabId) => agentStore.getTabAgent(tabId)}
      />
      <div class="flex-1 relative">
        <Show when={visibleTabs().length === 0}>
          <div class="absolute inset-0 flex flex-col items-center justify-center text-text-dim z-10">
            <div class="text-sm mb-4">
              {worktreePath() ? 'No terminal open' : 'Select a repository to start'}
            </div>
            <Show when={worktreePath()}>
              <div class="flex gap-3">
                <Button variant="ghost" onClick={() => terminalStore.openShellTab(worktreePath())}>
                  Open Terminal
                  <span class="ml-2 text-text-dim text-xs">Ctrl+Shift+T</span>
                </Button>
                <Button variant="ghost" onClick={() => terminalStore.openClaudeTab(worktreePath())}>
                  Start Claude Code
                  <span class="ml-2 text-text-dim text-xs">Ctrl+Shift+A</span>
                </Button>
              </div>
            </Show>
          </div>
        </Show>
        <For each={terminalStore.state.tabs}>
          {(tab) => (
            <TerminalView
              sessionId={tab.sessionId}
              visible={tab.worktreePath === worktreePath() && activeTabId() === tab.id}
            />
          )}
        </For>
      </div>
      <Show when={hasClaudeTab()}>
        <div class="flex items-center border-t border-border-subtle bg-bg-sidebar px-2">
          <button
            type="button"
            class="px-2 py-0.5 text-xs text-text-dim hover:text-text-main cursor-pointer"
            onClick={() => setActivityVisible((v) => !v)}
          >
            {activityVisible() ? '\u25BC' : '\u25B6'} Activity
          </button>
        </div>
      </Show>
      <FileStatusBar />
      <AgentActivity entries={agentLog()} visible={activityVisible() && hasClaudeTab()} />
      <PresetManager
        open={presetManagerOpen()}
        repoPath={repoPath()}
        onClose={() => setPresetManagerOpen(false)}
        onPresetsChanged={() => setPresetVersion((v) => v + 1)}
      />
    </div>
  );
}
