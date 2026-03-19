import { listen } from '@tauri-apps/api/event';
import { batch } from 'solid-js';
import { createStore, produce } from 'solid-js/store';
import type {
  AssistantTextEvent,
  PermissionRequestEvent,
  RunInfo,
  RunStepEvent,
  ToolCallEvent,
} from '../../types/run';
import type { TaskInfo } from '../../types/task';
import { getRunStatus } from '../commands/run';
import { listTasks } from '../commands/task';

const MAX_LOG_ENTRIES = 200;

export type RunLogEntry = {
  id: string;
  type: 'run_step' | 'assistant_text' | 'tool_call' | 'status_change';
  detail: string;
  sessionId: string | null;
  ts: number;
};

type TaskStoreState = {
  tasksByWorktree: Record<string, TaskInfo>;
  activeRun: RunInfo | null;
  runLog: RunLogEntry[];
  pendingPermissions: PermissionRequestEvent[];
};

function normalizePath(p: string): string {
  return p.endsWith('/') ? p : `${p}/`;
}

function worktreePathFromTaskPath(taskPath: string): string {
  // Task path is like /foo/bar/.branchdeck/task.md — strip the suffix
  // Normalize to trailing slash to match worktree paths from git
  const suffix = '.branchdeck/task.md';
  if (taskPath.endsWith(suffix)) {
    return normalizePath(taskPath.slice(0, -suffix.length));
  }
  return normalizePath(taskPath);
}

function createTaskStore() {
  const [state, setState] = createStore<TaskStoreState>({
    tasksByWorktree: {},
    activeRun: null,
    runLog: [],
    pendingPermissions: [],
  });

  let logCounter = 0;
  const listenPromises: Promise<() => void>[] = [];
  let listening = false;

  async function loadTasks(worktreePaths: string[]) {
    try {
      const tasks = await listTasks(worktreePaths);
      setState(
        produce((s) => {
          for (const task of tasks) {
            const wtPath = worktreePathFromTaskPath(task.path);
            s.tasksByWorktree[wtPath] = task;
          }
        }),
      );
    } catch {
      // Tasks that fail to load are silently skipped
    }

    // Check if there is an active run that was recovered on startup
    try {
      const runStatus = await getRunStatus();
      if (runStatus) {
        setState('activeRun', runStatus);
      }
    } catch {
      // Run status check is best-effort
    }
  }

  function handleTaskUpdated(task: TaskInfo) {
    const wtPath = worktreePathFromTaskPath(task.path);
    setState('tasksByWorktree', wtPath, task);
  }

  function handlePermissionRequest(perm: PermissionRequestEvent) {
    setState(
      produce((s) => {
        // Avoid duplicates
        if (!s.pendingPermissions.some((p) => p.toolUseId === perm.toolUseId)) {
          s.pendingPermissions.push(perm);
        }
      }),
    );
  }

  function removePermission(toolUseId: string) {
    setState(
      produce((s) => {
        const idx = s.pendingPermissions.findIndex((p) => p.toolUseId === toolUseId);
        if (idx !== -1) s.pendingPermissions.splice(idx, 1);
      }),
    );
  }

  function handleRunStatusChanged(run: RunInfo) {
    batch(() => {
      setState('activeRun', run);
      // Clear all pending permissions when run completes
      if (run.status !== 'blocked' && run.status !== 'running') {
        setState('pendingPermissions', []);
      }
      setState(
        produce((s) => {
          const costSuffix =
            run.status === 'succeeded' || run.status === 'failed'
              ? ` ($${run.costUsd.toFixed(4)})`
              : '';
          const entry: RunLogEntry = {
            id: `${++logCounter}`,
            type: 'status_change',
            detail: `Run ${run.status}${costSuffix}`,
            sessionId: run.sessionId,
            ts: Date.now(),
          };
          s.runLog.push(entry);
          if (s.runLog.length > MAX_LOG_ENTRIES) {
            s.runLog.splice(0, s.runLog.length - MAX_LOG_ENTRIES);
          }
        }),
      );
    });
  }

  function handleRunStep(event: RunStepEvent | AssistantTextEvent | ToolCallEvent) {
    let detail: string;
    let type: RunLogEntry['type'];

    switch (event.type) {
      case 'run_step':
        type = 'run_step';
        detail = `${event.step}: ${event.detail}`;
        break;
      case 'assistant_text':
        type = 'assistant_text';
        detail = event.text;
        break;
      case 'tool_call':
        type = 'tool_call';
        detail = event.filePath ? `${event.tool} ${event.filePath}` : event.tool;
        break;
    }

    const truncated = detail.length > 500 ? `${detail.slice(0, 497)}...` : detail;

    setState(
      produce((s) => {
        const entry: RunLogEntry = {
          id: `${++logCounter}`,
          type,
          detail: truncated,
          sessionId: event.sessionId,
          ts: Date.now(),
        };
        s.runLog.push(entry);
        if (s.runLog.length > MAX_LOG_ENTRIES) {
          s.runLog.splice(0, s.runLog.length - MAX_LOG_ENTRIES);
        }
      }),
    );
  }

  async function startListening() {
    if (listening) return;
    listening = true;

    try {
      listenPromises.push(
        listen<TaskInfo>('task:updated', (e) => {
          handleTaskUpdated(e.payload);
        }),
      );
      listenPromises.push(
        listen<RunInfo>('run:status_changed', (e) => {
          handleRunStatusChanged(e.payload);
        }),
      );
      listenPromises.push(
        listen<RunStepEvent | AssistantTextEvent | ToolCallEvent>('run:step', (e) => {
          handleRunStep(e.payload);
        }),
      );
      listenPromises.push(
        listen<PermissionRequestEvent>('run:permission_request', (e) => {
          handlePermissionRequest(e.payload);
        }),
      );
      await Promise.all(listenPromises);
    } catch {
      listening = false;
    }
  }

  async function stopListening() {
    if (!listening) return;
    for (const promise of listenPromises) {
      try {
        const unlisten = await promise;
        unlisten();
      } catch {
        // listener never registered
      }
    }
    listenPromises.length = 0;
    listening = false;
  }

  function clearAll() {
    setState(
      produce((s) => {
        s.tasksByWorktree = {};
        s.activeRun = null;
        s.runLog = [];
        s.pendingPermissions = [];
      }),
    );
    logCounter = 0;
  }

  function getRunDuration(): number {
    const run = state.activeRun;
    if (!run?.startedAt) return 0;
    const startMs = new Date(run.startedAt).getTime();
    if (Number.isNaN(startMs)) return 0;
    return Math.max(0, Math.floor((Date.now() - startMs) / 1000));
  }

  function hasTaskForWorktree(wtPath: string): boolean {
    const key = normalizePath(wtPath);
    return key in state.tasksByWorktree;
  }

  return {
    state,
    loadTasks,
    startListening,
    stopListening,
    clearAll,
    getRunDuration,
    hasTaskForWorktree,
    removePermission,
  };
}

let store: ReturnType<typeof createTaskStore> | undefined;

export function getTaskStore() {
  if (!store) store = createTaskStore();
  return store;
}
