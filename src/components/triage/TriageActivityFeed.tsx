import { createMemo, createSignal, For, onCleanup, onMount, Show } from 'solid-js';
import { getAgentStore, type SessionAgentInfo } from '../../lib/stores/agent';
import { RunActivityRow } from './RunActivityRow';

/**
 * Real-time agent activity feed for the triage view.
 * Shows active and recently completed agent sessions with expandable detail.
 * Follows inbox row pattern (36px collapsed, surface-raised expanded).
 */
export function TriageActivityFeed() {
  const agentStore = getAgentStore();
  const [selectedIndex, setSelectedIndex] = createSignal<number | null>(null);
  const [expandedSessionId, setExpandedSessionId] = createSignal<string | null>(null);

  // Tick to update elapsed times
  const [, setTick] = createSignal(0);
  let tickInterval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    agentStore.startListening();
    tickInterval = setInterval(() => setTick((t) => t + 1), 1000);
  });

  onCleanup(() => {
    if (tickInterval) clearInterval(tickInterval);
  });

  const activeRuns = createMemo((): SessionAgentInfo[] => {
    return agentStore.getActiveRuns().sort((a, b) => b.startedAt - a.startedAt);
  });

  const completedRuns = createMemo((): SessionAgentInfo[] => {
    return Object.values(agentStore.state.agentsBySession)
      .filter((s) => s.status === 'stopped')
      .sort((a, b) => b.lastActivity - a.lastActivity)
      .slice(0, 10); // Show last 10 completed
  });

  const allRuns = createMemo(() => [...activeRuns(), ...completedRuns()]);
  const activeCount = createMemo(() => activeRuns().length);

  function handleClick(sessionId: string, idx: number) {
    if (selectedIndex() === idx && expandedSessionId() === sessionId) {
      setExpandedSessionId(null);
    } else {
      setSelectedIndex(idx);
      setExpandedSessionId(sessionId);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    const runs = allRuns();
    if (runs.length === 0) return;

    if (e.key === 'j') {
      e.preventDefault();
      const cur = selectedIndex();
      if (cur === null) setSelectedIndex(0);
      else if (cur < runs.length - 1) setSelectedIndex(cur + 1);
    } else if (e.key === 'k') {
      e.preventDefault();
      const cur = selectedIndex();
      if (cur !== null && cur > 0) setSelectedIndex(cur - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const idx = selectedIndex();
      if (idx !== null && runs[idx]) {
        const session = runs[idx];
        handleClick(session.sessionId, idx);
      }
    }
  }

  onMount(() => {
    document.addEventListener('keydown', handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener('keydown', handleKeyDown);
  });

  return (
    <div class="mb-4">
      {/* Section header */}
      <Show when={activeCount() > 0}>
        <h3 class="text-[10px] font-medium text-[var(--color-warning)] uppercase tracking-[0.06em] mb-2 px-3">
          ACTIVE AGENTS ({activeCount()})
        </h3>
      </Show>
      <Show when={activeCount() === 0 && completedRuns().length === 0}>
        <h3 class="text-[10px] font-medium text-text-dim uppercase tracking-[0.06em] mb-2 px-3">
          AGENTS
        </h3>
        <div class="px-3 py-3 text-[11px] text-text-dim">
          No active agents. Workflows trigger automatically.
        </div>
      </Show>

      {/* Active runs */}
      <Show when={activeRuns().length > 0}>
        <For each={activeRuns()}>
          {(session, i) => (
            <RunActivityRow
              session={session}
              selected={selectedIndex() === i()}
              expanded={expandedSessionId() === session.sessionId}
              onClick={() => handleClick(session.sessionId, i())}
            />
          )}
        </For>
      </Show>

      {/* Completed runs */}
      <Show when={completedRuns().length > 0}>
        <h3 class="text-[10px] font-medium text-text-dim uppercase tracking-[0.06em] mt-3 mb-1 px-3">
          RECENT ({completedRuns().length})
        </h3>
        <For each={completedRuns()}>
          {(session, i) => {
            const idx = () => activeRuns().length + i();
            return (
              <RunActivityRow
                session={session}
                selected={selectedIndex() === idx()}
                expanded={expandedSessionId() === session.sessionId}
                onClick={() => handleClick(session.sessionId, idx())}
              />
            );
          }}
        </For>
      </Show>
    </div>
  );
}
