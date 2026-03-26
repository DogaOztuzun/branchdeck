import { batch } from 'solid-js';
import { createStore, produce } from 'solid-js/store';
import type { AgentEvent, AgentStatus } from '../../types/agent';
import { onEvent } from '../api/events';

const MAX_LOG_ENTRIES = 1000;
const MAX_LOG_PER_SESSION = 50;

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

export type SessionAgentInfo = {
  sessionId: string;
  tabId: string;
  status: AgentStatus;
  currentTool: string | null;
  currentFile: string | null;
  subagentCount: number;
  startedAt: number;
  lastActivity: number;
};

type AgentStoreState = {
  agentsByTab: Record<string, TabAgentInfo>;
  agentsBySession: Record<string, SessionAgentInfo>;
  log: AgentLogEntry[];
};

function createAgentStore() {
  const [state, setState] = createStore<AgentStoreState>({
    agentsByTab: {},
    agentsBySession: {},
    log: [],
  });

  let logCounter = 0;
  let tauriUnlisten: (() => void) | null = null;
  const sseUnsubscribes: (() => void)[] = [];

  let isKnownTab: ((tabId: string) => boolean) | null = null;
  const extraTabs = new Set<string>();
  const extraSessions = new Set<string>();

  // rAF batching: queue events and flush once per frame
  let pendingEvents: AgentEvent[] = [];
  let rafHandle: number | null = null;

  function setTabFilter(fn: (tabId: string) => boolean) {
    isKnownTab = fn;
  }

  function includeTab(tabId: string) {
    extraTabs.add(tabId);
  }

  function includeSession(sessionId: string) {
    extraSessions.add(sessionId);
  }

  function shouldAcceptEvent(event: AgentEvent): boolean {
    // Always accept if session is explicitly tracked
    if (extraSessions.has(event.sessionId)) return true;
    // Accept if tab is known
    if (extraTabs.has(event.tabId)) return true;
    if (isKnownTab?.(event.tabId)) return true;
    // Always accept session-level events (start/stop) for discovery
    if (event.kind === 'sessionStart' || event.kind === 'sessionStop') return true;
    // Accept if we already track this session
    if (state.agentsBySession[event.sessionId]) return true;
    return false;
  }

  function processEvents(events: AgentEvent[]) {
    batch(() => {
      setState(
        produce((s) => {
          for (const event of events) {
            // Add log entry
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

            s.log.push(entry);

            // Ensure TabAgentInfo exists
            if (!s.agentsByTab[event.tabId]) {
              s.agentsByTab[event.tabId] = {
                status: 'active',
                currentTool: null,
                currentFile: null,
                subagentCount: 0,
              };
            }

            // Ensure SessionAgentInfo exists
            if (!s.agentsBySession[event.sessionId]) {
              s.agentsBySession[event.sessionId] = {
                sessionId: event.sessionId,
                tabId: event.tabId,
                status: 'active',
                currentTool: null,
                currentFile: null,
                subagentCount: 0,
                startedAt: event.ts,
                lastActivity: event.ts,
              };
            }

            const tabInfo = s.agentsByTab[event.tabId];
            const sessionInfo = s.agentsBySession[event.sessionId];

            switch (event.kind) {
              case 'sessionStart':
                tabInfo.status = 'active';
                sessionInfo.status = 'active';
                sessionInfo.startedAt = event.ts;
                sessionInfo.lastActivity = event.ts;
                break;
              case 'toolStart':
                tabInfo.status = 'active';
                tabInfo.currentTool = event.toolName;
                tabInfo.currentFile = event.filePath ?? null;
                sessionInfo.status = 'active';
                sessionInfo.currentTool = event.toolName;
                sessionInfo.currentFile = event.filePath ?? null;
                sessionInfo.lastActivity = event.ts;
                break;
              case 'toolEnd':
                tabInfo.status = 'idle';
                tabInfo.currentTool = null;
                tabInfo.currentFile = null;
                sessionInfo.status = 'idle';
                sessionInfo.currentTool = null;
                sessionInfo.currentFile = null;
                sessionInfo.lastActivity = event.ts;
                break;
              case 'subagentStart':
                tabInfo.subagentCount += 1;
                sessionInfo.subagentCount += 1;
                sessionInfo.lastActivity = event.ts;
                break;
              case 'subagentStop':
                if (tabInfo.subagentCount > 0) tabInfo.subagentCount -= 1;
                if (sessionInfo.subagentCount > 0) sessionInfo.subagentCount -= 1;
                sessionInfo.lastActivity = event.ts;
                break;
              case 'sessionStop':
                tabInfo.status = 'stopped';
                tabInfo.currentTool = null;
                tabInfo.currentFile = null;
                sessionInfo.status = 'stopped';
                sessionInfo.currentTool = null;
                sessionInfo.currentFile = null;
                sessionInfo.lastActivity = event.ts;
                break;
            }
          }

          // Ring buffer eviction: global cap
          if (s.log.length > MAX_LOG_ENTRIES) {
            s.log.splice(0, s.log.length - MAX_LOG_ENTRIES);
          }
        }),
      );
    });
  }

  function queueEvent(event: AgentEvent) {
    if (!shouldAcceptEvent(event)) return;

    pendingEvents.push(event);
    if (rafHandle === null) {
      rafHandle = requestAnimationFrame(() => {
        rafHandle = null;
        const events = pendingEvents;
        pendingEvents = [];
        processEvents(events);
      });
    }
  }

  function handleEvent(event: AgentEvent) {
    queueEvent(event);
  }

  async function startListening() {
    // Re-entry guard: prevent duplicate listeners on remount
    if (tauriUnlisten || sseUnsubscribes.length > 0) return;

    // Try Tauri listen first (desktop mode)
    let tauriAvailable = false;
    try {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<AgentEvent>('agent:event', (e) => {
        handleEvent(e.payload);
      });
      tauriUnlisten = unlisten;
      tauriAvailable = true;
    } catch {
      // Not in Tauri — use SSE
    }

    // Only subscribe via SSE if Tauri is unavailable (avoid duplicate events)
    if (!tauriAvailable) {
      const agentEventTypes = [
        'agent:session_start',
        'agent:tool_start',
        'agent:tool_end',
        'agent:subagent_start',
        'agent:subagent_stop',
        'agent:session_stop',
        'agent:notification',
      ];

      for (const eventType of agentEventTypes) {
        const unsub = onEvent<AgentEvent>(eventType, (envelope) => {
          handleEvent(envelope.data);
        });
        sseUnsubscribes.push(unsub);
      }
    }
  }

  async function stopListening() {
    if (tauriUnlisten) {
      tauriUnlisten();
      tauriUnlisten = null;
    }
    for (const unsub of sseUnsubscribes) {
      unsub();
    }
    sseUnsubscribes.length = 0;

    if (rafHandle !== null) {
      cancelAnimationFrame(rafHandle);
      rafHandle = null;
      pendingEvents = [];
    }
  }

  function removeTab(tabId: string) {
    extraTabs.delete(tabId);
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

  function getLogForSession(sessionId: string): AgentLogEntry[] {
    return state.log.filter((e) => e.sessionId === sessionId).slice(-MAX_LOG_PER_SESSION);
  }

  function getSessionAgent(sessionId: string): SessionAgentInfo | undefined {
    return state.agentsBySession[sessionId];
  }

  function getActiveRuns(): SessionAgentInfo[] {
    return Object.values(state.agentsBySession).filter(
      (s) => s.status === 'active' || s.status === 'idle',
    );
  }

  return {
    state,
    startListening,
    stopListening,
    setTabFilter,
    includeTab,
    includeSession,
    removeTab,
    getTabAgent,
    getLogForTab,
    getLogForSession,
    getSessionAgent,
    getActiveRuns,
  };
}

let store: ReturnType<typeof createAgentStore> | undefined;

export function getAgentStore() {
  if (!store) store = createAgentStore();
  return store;
}
