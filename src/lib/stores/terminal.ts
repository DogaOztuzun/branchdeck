import { createStore, produce } from 'solid-js/store';
import type { PtyEvent, TabInfo } from '../../types/terminal';
import { closeTerminal, createTerminalSession, writeTerminal } from '../commands/terminal';

type TerminalState = {
  tabs: TabInfo[];
  activeTabId: string | null;
};

function createTerminalStore() {
  const [state, setState] = createStore<TerminalState>({
    tabs: [],
    activeTabId: null,
  });

  const outputHandlers = new Map<string, (data: Uint8Array) => void>();

  function handlePtyEvent(sessionId: string, event: PtyEvent) {
    if (event.event === 'output') {
      const handler = outputHandlers.get(sessionId);
      if (handler) {
        handler(new Uint8Array(event.data.bytes));
      }
    } else if (event.event === 'exit') {
      const tab = state.tabs.find((t) => t.sessionId === sessionId);
      if (tab) {
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
  }

  async function openShellTab(cwd: string) {
    const tabId = crypto.randomUUID();
    let resolvedSessionId = '';

    const sessionId = await createTerminalSession(cwd, '', {}, (event) =>
      handlePtyEvent(resolvedSessionId, event),
    );
    resolvedSessionId = sessionId;

    const tab: TabInfo = {
      id: tabId,
      sessionId,
      title: 'Terminal',
      type: 'shell',
    };

    setState(
      produce((s) => {
        s.tabs.push(tab);
        s.activeTabId = tabId;
      }),
    );
  }

  async function openClaudeTab(cwd: string) {
    const tabId = crypto.randomUUID();
    let resolvedSessionId = '';
    const env = { CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS: '1' };

    const sessionId = await createTerminalSession(cwd, '', env, (event) =>
      handlePtyEvent(resolvedSessionId, event),
    );
    resolvedSessionId = sessionId;

    const tab: TabInfo = {
      id: tabId,
      sessionId,
      title: 'Claude Code',
      type: 'claude',
    };

    setState(
      produce((s) => {
        s.tabs.push(tab);
        s.activeTabId = tabId;
      }),
    );

    const encoder = new TextEncoder();
    await writeTerminal(sessionId, encoder.encode('claude --dangerously-skip-permissions\n'));
  }

  async function closeTab(tabId: string) {
    const tab = state.tabs.find((t) => t.id === tabId);
    if (!tab) return;

    await closeTerminal(tab.sessionId);
    outputHandlers.delete(tab.sessionId);

    setState(
      produce((s) => {
        const index = s.tabs.findIndex((t) => t.id === tabId);
        if (index !== -1) {
          s.tabs.splice(index, 1);
        }
        if (s.activeTabId === tabId) {
          s.activeTabId = s.tabs.length > 0 ? s.tabs[Math.max(0, index - 1)].id : null;
        }
      }),
    );
  }

  function setActiveTab(tabId: string) {
    setState('activeTabId', tabId);
  }

  function registerOutputHandler(sessionId: string, handler: (data: Uint8Array) => void) {
    outputHandlers.set(sessionId, handler);
  }

  function unregisterOutputHandler(sessionId: string) {
    outputHandlers.delete(sessionId);
  }

  return {
    state,
    openShellTab,
    openClaudeTab,
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
