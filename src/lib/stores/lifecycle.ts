import { listen } from '@tauri-apps/api/event';
import { batch } from 'solid-js';
import { createStore, produce } from 'solid-js/store';
import type { AnalysisPlan, LifecycleEvent } from '../../types/lifecycle';
import { getLifecycles, readAnalysis } from '../commands/lifecycle';

type LifecycleStoreState = {
  lifecycles: Record<string, LifecycleEvent>;
  analysisPlans: Record<string, AnalysisPlan>;
};

function createLifecycleStore() {
  const [state, setState] = createStore<LifecycleStoreState>({
    lifecycles: {},
    analysisPlans: {},
  });

  let listenPromise: Promise<() => void> | null = null;

  function handleLifecycleEvent(event: LifecycleEvent) {
    batch(() => {
      setState(
        produce((s) => {
          s.lifecycles[event.prKey] = event;
        }),
      );

      if ((event.status === 'reviewReady' || event.status === 'stale') && event.worktreePath) {
        loadAnalysis(event.prKey, event.worktreePath);
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
    if (listenPromise) return;
    listenPromise = listen<LifecycleEvent>('lifecycle:updated', (e) => {
      handleLifecycleEvent(e.payload);
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

  async function loadInitial() {
    try {
      const events = await getLifecycles();
      batch(() => {
        setState(
          produce((s) => {
            for (const event of events) {
              s.lifecycles[event.prKey] = event;
            }
          }),
        );
      });
    } catch {
      // No lifecycles available
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

  return {
    state,
    startListening,
    stopListening,
    loadInitial,
    loadAnalysis,
    getLifecycleForPr,
    getAnalysisPlan,
    getAllLifecycles,
  };
}

let store: ReturnType<typeof createLifecycleStore> | undefined;

export function getLifecycleStore() {
  if (!store) store = createLifecycleStore();
  return store;
}
