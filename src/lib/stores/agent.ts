import { listen } from '@tauri-apps/api/event';
import { batch } from 'solid-js';
import { createStore, produce } from 'solid-js/store';
import type { AgentEvent, AgentStatus } from '../../types/agent';

const MAX_LOG_ENTRIES = 200;

export type AgentLogEntry = {
  id: string;
  kind: AgentEvent['kind'];
  tabId: string;
  sessionId: string;
  toolName: string | null;
  filePath: string | null;
  agentId: string | null;
  message: string | null;
  ts: number;
};

export type TabAgentInfo = {
  status: AgentStatus;
  currentTool: string | null;
  currentFile: string | null;
  subagentCount: number;
};

type AgentStoreState = {
  agentsByTab: Record<string, TabAgentInfo>;
  log: AgentLogEntry[];
};

function createAgentStore() {
  const [state, setState] = createStore<AgentStoreState>({
    agentsByTab: {},
    log: [],
  });

  let logCounter = 0;
  let listenPromise: Promise<() => void> | null = null;

  let isKnownTab: ((tabId: string) => boolean) | null = null;

  function setTabFilter(fn: (tabId: string) => boolean) {
    isKnownTab = fn;
  }

  function handleEvent(event: AgentEvent) {
    if (isKnownTab && !isKnownTab(event.tabId)) return;

    batch(() => {
      const entry: AgentLogEntry = {
        id: `${++logCounter}`,
        kind: event.kind,
        tabId: event.tabId,
        sessionId: event.sessionId,
        toolName: 'toolName' in event ? event.toolName : null,
        filePath: 'filePath' in event ? (event.filePath ?? null) : null,
        agentId: 'agentId' in event ? (event.agentId ?? null) : null,
        message: 'message' in event ? event.message : null,
        ts: event.ts,
      };

      setState(
        produce((s) => {
          s.log.push(entry);
          if (s.log.length > MAX_LOG_ENTRIES) {
            s.log.splice(0, s.log.length - MAX_LOG_ENTRIES);
          }

          // Ensure TabAgentInfo exists (handles events arriving before sessionStart)
          if (!s.agentsByTab[event.tabId]) {
            s.agentsByTab[event.tabId] = {
              status: 'active',
              currentTool: null,
              currentFile: null,
              subagentCount: 0,
            };
          }

          const info = s.agentsByTab[event.tabId];
          switch (event.kind) {
            case 'toolStart':
              info.status = 'active';
              info.currentTool = event.toolName;
              info.currentFile = event.filePath ?? null;
              break;
            case 'toolEnd':
              info.status = 'idle';
              info.currentTool = null;
              info.currentFile = null;
              break;
            case 'subagentStart':
              info.subagentCount += 1;
              break;
            case 'subagentStop':
              if (info.subagentCount > 0) info.subagentCount -= 1;
              break;
            case 'sessionStop':
              info.status = 'stopped';
              info.currentTool = null;
              info.currentFile = null;
              break;
          }
        }),
      );
    });
  }

  async function startListening() {
    if (listenPromise) return;
    listenPromise = listen<AgentEvent>('agent:event', (e) => {
      handleEvent(e.payload);
    });
    try {
      await listenPromise;
    } catch {
      listenPromise = null;
    }
  }

  async function stopListening() {
    if (listenPromise) {
      try {
        const fn = await listenPromise;
        fn();
      } catch {
        // listener never registered
      }
      listenPromise = null;
    }
  }

  function removeTab(tabId: string) {
    setState(
      produce((s) => {
        delete s.agentsByTab[tabId];
      }),
    );
  }

  function getTabAgent(tabId: string): TabAgentInfo | undefined {
    return state.agentsByTab[tabId];
  }

  function getLogForTab(tabId: string): AgentLogEntry[] {
    return state.log.filter((e) => e.tabId === tabId);
  }

  return {
    state,
    startListening,
    stopListening,
    setTabFilter,
    removeTab,
    getTabAgent,
    getLogForTab,
  };
}

let store: ReturnType<typeof createAgentStore> | undefined;

export function getAgentStore() {
  if (!store) store = createAgentStore();
  return store;
}
