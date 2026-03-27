import { batch, createMemo } from 'solid-js';
import { createStore, produce } from 'solid-js/store';
import type { PrSummary } from '../../types/github';
import type {
  AnalysisPlan,
  LifecycleEvent,
  LifecycleTimelineEntry,
  TriagePr,
} from '../../types/lifecycle';
import { onEvent } from '../api/events';
import { listOpenPrs } from '../commands/github';
import {
  getLifecycles,
  getRunningEntries,
  listDiscoveredPrs,
  readAnalysis,
} from '../commands/lifecycle';
import { getRepoStore } from './repo';

type LifecycleStoreState = {
  lifecycles: Record<string, LifecycleEvent>;
  analysisPlans: Record<string, AnalysisPlan>;
  discoveredPrs: Record<string, PrSummary>;
  completedLifecycles: Record<string, LifecycleEvent>;
  sessionByPr: Record<string, string>;
  repoNameToPath: Record<string, string>;
  /** Timeline entries per prKey: records every lifecycle transition for full lifecycle view */
  timelines: Record<string, LifecycleTimelineEntry[]>;
};

function createLifecycleStore() {
  const [state, setState] = createStore<LifecycleStoreState>({
    lifecycles: {},
    analysisPlans: {},
    discoveredPrs: {},
    completedLifecycles: {},
    sessionByPr: {},
    repoNameToPath: {},
    timelines: {},
  });

  const sseUnsubscribes: (() => void)[] = [];

  function handleLifecycleEvent(event: LifecycleEvent) {
    batch(() => {
      setState(
        produce((s) => {
          // Session tracking: update prKey → sessionId mapping
          if ((event.status === 'running' || event.status === 'fixing') && event.sessionId) {
            s.sessionByPr[event.prKey] = event.sessionId;
          }

          // Re-activation: completed PR moving back to active
          if (event.status === 'running' && s.completedLifecycles[event.prKey]) {
            delete s.completedLifecycles[event.prKey];
          }

          // Completion transition: move from active to completed
          if (event.status === 'completed') {
            delete s.lifecycles[event.prKey];
            s.completedLifecycles[event.prKey] = event;
          } else {
            s.lifecycles[event.prKey] = event;
          }

          // Record timeline entry for full lifecycle view (NFR25)
          if (!s.timelines[event.prKey]) {
            s.timelines[event.prKey] = [];
          }
          s.timelines[event.prKey].push({
            timestamp: Date.now(),
            status: event.status,
            displayStatus: event.displayStatus ?? event.status,
            detail: `${event.displayStatus ?? event.status} (attempt ${event.attempt})`,
          });
        }),
      );

      // Auto-fetch analysis on reviewReady or stale
      if ((event.status === 'reviewReady' || event.status === 'stale') && event.worktreePath) {
        loadAnalysis(event.prKey, event.worktreePath);
      }

      // Fallback: fetch session mapping if sessionId missing for running status
      if ((event.status === 'running' || event.status === 'fixing') && !event.sessionId) {
        getRunningEntries()
          .then((entries) => {
            setState(
              produce((s) => {
                for (const entry of entries) {
                  s.sessionByPr[entry.prKey] = entry.tabId;
                }
              }),
            );
          })
          .catch(() => {});
      }
    });
  }

  async function loadAnalysis(prKey: string, worktreePath: string) {
    try {
      const raw = await readAnalysis(worktreePath);
      if (raw) {
        const plan: AnalysisPlan = JSON.parse(raw);
        setState(
          produce((s) => {
            s.analysisPlans[prKey] = plan;
          }),
        );
      }
    } catch {
      // Analysis not available yet
    }
  }

  function startListening() {
    // Re-entry guard
    if (sseUnsubscribes.length > 0) return;

    sseUnsubscribes.push(
      onEvent<LifecycleEvent>('workflow:lifecycle_updated', (envelope) => {
        handleLifecycleEvent(envelope.data);
      }),
    );

    sseUnsubscribes.push(
      onEvent<PrSummary[]>('workflow:pr_discovered', (envelope) => {
        batch(() => {
          setState(
            produce((s) => {
              // Full snapshot replace — if a PR is gone, it disappears
              const newMap: Record<string, PrSummary> = {};
              for (const pr of envelope.data) {
                const key = `${pr.repoName}#${pr.number}`;
                newMap[key] = pr;
              }
              s.discoveredPrs = newMap;
            }),
          );
        });
      }),
    );
  }

  function stopListening() {
    for (const unsub of sseUnsubscribes) {
      unsub();
    }
    sseUnsubscribes.length = 0;
  }

  async function loadInitial() {
    const repoStore = getRepoStore();

    // Build repoName → path by calling listOpenPrs per repo.
    // Each returned PR carries the GitHub repoName from the remote,
    // which tells us the mapping from GitHub name → local path.
    const nameToPath: Record<string, string> = {};
    for (const repo of repoStore.state.repos) {
      try {
        const repoPrs = await listOpenPrs(repo.path);
        if (repoPrs.length > 0) {
          nameToPath[repoPrs[0].repoName] = repo.path;
        }
      } catch {
        // Skip unreachable repos
      }
    }

    try {
      const [events, prs, entries] = await Promise.all([
        getLifecycles(),
        listDiscoveredPrs(),
        getRunningEntries(),
      ]);

      batch(() => {
        setState(
          produce((s) => {
            // Store resolved repo mappings
            Object.assign(s.repoNameToPath, nameToPath);

            // Process lifecycles: separate active from completed
            for (const event of events) {
              if (event.status === 'completed') {
                s.completedLifecycles[event.prKey] = event;
              } else {
                s.lifecycles[event.prKey] = event;
              }
            }

            // Process discovered PRs
            for (const pr of prs) {
              const key = `${pr.repoName}#${pr.number}`;
              s.discoveredPrs[key] = pr;
            }

            // Process running entries for session mapping
            for (const entry of entries) {
              s.sessionByPr[entry.prKey] = entry.tabId;
            }
          }),
        );
      });
    } catch {
      // Initial load failed
    }
  }

  function getLifecycleForPr(prKey: string): LifecycleEvent | undefined {
    return state.lifecycles[prKey];
  }

  function getAnalysisPlan(prKey: string): AnalysisPlan | undefined {
    return state.analysisPlans[prKey];
  }

  function getAllLifecycles(): LifecycleEvent[] {
    return Object.values(state.lifecycles);
  }

  function getTimeline(prKey: string): LifecycleTimelineEntry[] {
    return state.timelines[prKey] ?? [];
  }

  const triagePrs = createMemo((): TriagePr[] => {
    const result: TriagePr[] = [];
    const seen = new Set<string>();

    // Merge from discovered PRs
    for (const [prKey, pr] of Object.entries(state.discoveredPrs)) {
      seen.add(prKey);
      result.push({
        prKey,
        pr,
        lifecycle: state.lifecycles[prKey] ?? state.completedLifecycles[prKey],
        analysis: state.analysisPlans[prKey],
        currentSessionId: state.sessionByPr[prKey],
        repoPath: state.repoNameToPath[pr.repoName],
      });
    }

    // Add lifecycle entries without discovery (race window)
    for (const [prKey, lifecycle] of Object.entries(state.lifecycles)) {
      if (!seen.has(prKey)) {
        seen.add(prKey);
        result.push({
          prKey,
          pr: undefined,
          lifecycle,
          analysis: state.analysisPlans[prKey],
          currentSessionId: state.sessionByPr[prKey],
          repoPath: undefined,
        });
      }
    }

    // Add completed entries without discovery
    for (const [prKey, lifecycle] of Object.entries(state.completedLifecycles)) {
      if (!seen.has(prKey)) {
        seen.add(prKey);
        result.push({
          prKey,
          pr: undefined,
          lifecycle,
          analysis: state.analysisPlans[prKey],
          currentSessionId: undefined,
          repoPath: undefined,
        });
      }
    }

    return result;
  });

  function getTriagePrs(): TriagePr[] {
    return triagePrs();
  }

  function getAttentionCount(): number {
    return triagePrs().filter(
      (t) => t.lifecycle?.status === 'reviewReady' || t.lifecycle?.status === 'failed',
    ).length;
  }

  return {
    state,
    startListening,
    stopListening,
    loadInitial,
    loadAnalysis,
    getLifecycleForPr,
    getAnalysisPlan,
    getAllLifecycles,
    getTimeline,
    getTriagePrs,
    getAttentionCount,
  };
}

let store: ReturnType<typeof createLifecycleStore> | undefined;

export function getLifecycleStore() {
  if (!store) store = createLifecycleStore();
  return store;
}

export { groupTriagePrs } from '../utils/triage';
