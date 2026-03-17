import { createStore, produce } from 'solid-js/store';
import type { PtyEvent, TabInfo } from '../../types/terminal';
import { closeTerminal, createTerminalSession, writeTerminal } from '../commands/terminal';
import type { Preset } from '../commands/workspace';

type TerminalState = {
  tabs: TabInfo[];
  activeTabByWorktree: Record<string, string | null>;
};

function createTerminalStore() {
  const [state, setState] = createStore<TerminalState>({
    tabs: [],
    activeTabByWorktree: {},
  });

  const outputHandlers = new Map<string, (data: Uint8Array) => void>();

  function handlePtyEvent(sessionId: string, event: PtyEvent) {
    if (event.event === 'output') {
      const handler = outputHandlers.get(sessionId);
      if (handler) {
        handler(new Uint8Array(event.data.bytes));
      }
    } else if (event.event === 'exit') {
      setState(
        produce((s) => {
          const t = s.tabs.find((t) => t.sessionId === sessionId);
          if (t) {
            t.title = `${t.title} (exited)`;
          }
        }),
      );
    }
  }

  function getTabsForWorktree(worktreePath: string): TabInfo[] {
    return state.tabs.filter((t) => t.worktreePath === worktreePath);
  }

  function getActiveTabId(worktreePath: string): string | null {
    return state.activeTabByWorktree[worktreePath] ?? null;
  }

  async function openShellTab(worktreePath: string) {
    const tabId = crypto.randomUUID();
    let resolvedSessionId = '';

    const sessionId = await createTerminalSession(worktreePath, '', {}, (event) =>
      handlePtyEvent(resolvedSessionId, event),
    );
    resolvedSessionId = sessionId;

    const tab: TabInfo = {
      id: tabId,
      sessionId,
      title: 'Terminal',
      type: 'shell',
      worktreePath,
    };

    setState(
      produce((s) => {
        s.tabs.push(tab);
        s.activeTabByWorktree[worktreePath] = tabId;
      }),
    );
  }

  async function openClaudeTab(worktreePath: string, tabId?: string) {
    const id = tabId ?? crypto.randomUUID();
    let resolvedSessionId = '';
    const env: Record<string, string> = {
      CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS: '1',
      BRANCHDECK_PORT: '13370',
      BRANCHDECK_TAB_ID: id,
      BRANCHDECK_SESSION_ID: crypto.randomUUID(),
    };

    const sessionId = await createTerminalSession(worktreePath, '', env, (event) =>
      handlePtyEvent(resolvedSessionId, event),
    );
    resolvedSessionId = sessionId;

    const tab: TabInfo = {
      id,
      sessionId,
      title: 'Claude Code',
      type: 'claude',
      worktreePath,
    };

    setState(
      produce((s) => {
        s.tabs.push(tab);
        s.activeTabByWorktree[worktreePath] = id;
      }),
    );

    const encoder = new TextEncoder();
    await writeTerminal(sessionId, encoder.encode('claude --dangerously-skip-permissions\n'));
  }

  async function runPreset(worktreePath: string, preset: Preset) {
    if (preset.tabType === 'claude') {
      await openClaudeTab(worktreePath);
      // The claude tab auto-writes the claude command. Now write the preset command after a short delay.
      const tabs = getTabsForWorktree(worktreePath);
      const lastTab = tabs[tabs.length - 1];
      if (lastTab && preset.command) {
        const encoder = new TextEncoder();
        // Small delay to let claude code start
        setTimeout(async () => {
          await writeTerminal(lastTab.sessionId, encoder.encode(`${preset.command}\n`));
        }, 500);
      }
    } else {
      const tabId = crypto.randomUUID();
      let resolvedSessionId = '';
      const sessionId = await createTerminalSession(worktreePath, '', preset.env, (event) =>
        handlePtyEvent(resolvedSessionId, event),
      );
      resolvedSessionId = sessionId;

      const tab: TabInfo = {
        id: tabId,
        sessionId,
        title: preset.name,
        type: 'shell',
        worktreePath,
      };

      setState(
        produce((s) => {
          s.tabs.push(tab);
          s.activeTabByWorktree[worktreePath] = tabId;
        }),
      );

      if (preset.command) {
        const encoder = new TextEncoder();
        await writeTerminal(sessionId, encoder.encode(`${preset.command}\n`));
      }
    }
  }

  async function closeTab(tabId: string) {
    const tab = state.tabs.find((t) => t.id === tabId);
    if (!tab) return;

    const { worktreePath } = tab;

    await closeTerminal(tab.sessionId);
    outputHandlers.delete(tab.sessionId);

    setState(
      produce((s) => {
        const index = s.tabs.findIndex((t) => t.id === tabId);
        if (index !== -1) {
          s.tabs.splice(index, 1);
        }
        if (s.activeTabByWorktree[worktreePath] === tabId) {
          const remaining = s.tabs.filter((t) => t.worktreePath === worktreePath);
          s.activeTabByWorktree[worktreePath] =
            remaining.length > 0
              ? (remaining[Math.max(0, index - 1)]?.id ?? remaining[0]?.id ?? null)
              : null;
        }
      }),
    );
  }

  function setActiveTab(worktreePath: string, tabId: string) {
    setState('activeTabByWorktree', worktreePath, tabId);
  }

  function registerOutputHandler(sessionId: string, handler: (data: Uint8Array) => void) {
    outputHandlers.set(sessionId, handler);
  }

  function unregisterOutputHandler(sessionId: string) {
    outputHandlers.delete(sessionId);
  }

  return {
    state,
    getTabsForWorktree,
    getActiveTabId,
    openShellTab,
    openClaudeTab,
    runPreset,
    closeTab,
    setActiveTab,
    registerOutputHandler,
    unregisterOutputHandler,
  };
}

let store: ReturnType<typeof createTerminalStore> | undefined;

export function getTerminalStore() {
  if (!store) store = createTerminalStore();
  return store;
}
