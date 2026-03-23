import { listen } from '@tauri-apps/api/event';
import { createMemo } from 'solid-js';
import { batch } from 'solid-js';
import { createStore, produce } from 'solid-js/store';
import type { PrSummary } from '../../types/github';
import type {
  AnalysisPlan,
  LifecycleEvent,
  TriageGroups,
  TriagePr,
} from '../../types/lifecycle';
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
};

function createLifecycleStore() {
  const [state, setState] = createStore<LifecycleStoreState>({
    lifecycles: {},
    analysisPlans: {},
    discoveredPrs: {},
    completedLifecycles: {},
    sessionByPr: {},
    repoNameToPath: {},
  });

  let lifecycleUnlisten: (() => void) | null = null;
  let discoveredUnlisten: (() => void) | null = null;

  function handleLifecycleEvent(event: LifecycleEvent) {
    batch(() => {
      setState(
        produce((s) => {
          // Session tracking: update prKey → sessionId mapping
          if (
            (event.status === 'running' || event.status === 'fixing') &&
            event.sessionId
          ) {
            s.sessionByPr[event.prKey] = event.sessionId;
          }

          // Re-activation: completed PR moving back to active
          if (
            event.status === 'running' &&
            s.completedLifecycles[event.prKey]
          ) {
            delete s.completedLifecycles[event.prKey];
          }

          // Completion transition: move from active to completed
          if (event.status === 'completed') {
            delete s.lifecycles[event.prKey];
            s.completedLifecycles[event.prKey] = event;
          } else {
            s.lifecycles[event.prKey] = event;
          }
        }),
      );

      // Auto-fetch analysis on reviewReady or stale
      if (
        (event.status === 'reviewReady' || event.status === 'stale') &&
        event.worktreePath
      ) {
        loadAnalysis(event.prKey, event.worktreePath);
      }

      // Fallback: fetch session mapping if sessionId missing for running status
      if (
        (event.status === 'running' || event.status === 'fixing') &&
        !event.sessionId
      ) {
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

  async function startListening() {
    if (!lifecycleUnlisten) {
      const unlisten = await listen<LifecycleEvent>(
        'lifecycle:updated',
        (e) => {
          handleLifecycleEvent(e.payload);
        },
      );
      lifecycleUnlisten = unlisten;
    }

    if (!discoveredUnlisten) {
      const unlisten = await listen<PrSummary[]>('pr:discovered', (e) => {
        batch(() => {
          setState(
            produce((s) => {
              // Full snapshot replace — if a PR is gone, it disappears
              const newMap: Record<string, PrSummary> = {};
              for (const pr of e.payload) {
                const key = `${pr.repoName}#${pr.number}`;
                newMap[key] = pr;
              }
              s.discoveredPrs = newMap;
            }),
          );
        });
      });
      discoveredUnlisten = unlisten;
    }
  }

  async function stopListening() {
    if (lifecycleUnlisten) {
      lifecycleUnlisten();
      lifecycleUnlisten = null;
    }
    if (discoveredUnlisten) {
      discoveredUnlisten();
      discoveredUnlisten = null;
    }
  }

  async function loadInitial() {
    // Build repoNameToPath from repo store
    const repoStore = getRepoStore();
    const nameToPath: Record<string, string> = {};
    for (const repo of repoStore.state.repos) {
      // We need the GitHub "owner/repo" name. Since RepoInfo only has local name,
      // we'll build the mapping from discovered PRs (each has repoName + we know repo.path).
      // For now, store local name → path, and also try matching by path suffix.
      nameToPath[repo.name] = repo.path;
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
              // Build repoName → path mapping from discovered PRs
              if (!s.repoNameToPath[pr.repoName]) {
                // Match by repo path ending
                for (const repo of repoStore.state.repos) {
                  const pathParts = repo.path.split('/');
                  const localName = pathParts[pathParts.length - 1];
                  const repoNameParts = pr.repoName.split('/');
                  const ghRepoName =
                    repoNameParts[repoNameParts.length - 1];
                  if (localName === ghRepoName) {
                    s.repoNameToPath[pr.repoName] = repo.path;
                    break;
                  }
                }
              }
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
    for (const [prKey, lifecycle] of Object.entries(
      state.completedLifecycles,
    )) {
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
      (t) =>
        t.lifecycle?.status === 'reviewReady' ||
        t.lifecycle?.status === 'failed',
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
    getTriagePrs,
    getAttentionCount,
  };
}

let store: ReturnType<typeof createLifecycleStore> | undefined;

export function getLifecycleStore() {
  if (!store) store = createLifecycleStore();
  return store;
}

export function groupTriagePrs(items: TriagePr[]): TriageGroups {
  const needsAttention: TriagePr[] = [];
  const inProgress: TriagePr[] = [];
  const watching: TriagePr[] = [];
  const newPrs: TriagePr[] = [];
  const done: TriagePr[] = [];

  for (const item of items) {
    const status = item.lifecycle?.status;

    if (status === 'reviewReady' || status === 'failed') {
      needsAttention.push(item);
    } else if (
      status === 'running' ||
      status === 'fixing' ||
      status === 'approved' ||
      status === 'retrying'
    ) {
      inProgress.push(item);
    } else if (status === 'stale') {
      watching.push(item);
    } else if (status === 'completed') {
      done.push(item);
    } else if (!item.lifecycle && item.pr) {
      newPrs.push(item);
    }
  }

  // Sort newPrs: failing CI first
  newPrs.sort((a, b) => {
    const aFailing = a.pr?.ciStatus === 'FAILURE' || a.pr?.ciStatus === 'ERROR';
    const bFailing = b.pr?.ciStatus === 'FAILURE' || b.pr?.ciStatus === 'ERROR';
    if (aFailing && !bFailing) return -1;
    if (!aFailing && bFailing) return 1;
    return 0;
  });

  return { needsAttention, inProgress, watching, newPrs, done };
}
