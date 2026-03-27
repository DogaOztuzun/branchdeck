import type { TaskInfo, TaskType } from '../../types/task';
import { apiGet, apiPost } from '../api/client';

export async function createTask(
  worktreePath: string,
  taskType: TaskType,
  repo: string,
  branch: string,
  pr?: number,
  description?: string,
): Promise<TaskInfo> {
  try {
    return await apiPost<TaskInfo>('/tasks', {
      worktreePath,
      taskType,
      repo,
      branch,
      pr: pr ?? null,
      description: description || null,
    });
  } catch (e) {
    console.error(`createTask failed: ${e}`);
    throw e;
  }
}

export async function getTask(worktreePath: string): Promise<TaskInfo> {
  try {
    return await apiGet<TaskInfo>(`/tasks/detail?worktreePath=${encodeURIComponent(worktreePath)}`);
  } catch (e) {
    console.error(`getTask failed: ${e}`);
    throw e;
  }
}

export async function listTasks(worktreePaths: string[]): Promise<TaskInfo[]> {
  try {
    return await apiPost<TaskInfo[]>('/tasks/list', { worktreePaths });
  } catch (e) {
    console.error(`listTasks failed: ${e}`);
    throw e;
  }
}

export async function startTaskWatcher(worktreePaths: string[]): Promise<void> {
  try {
    await apiPost('/tasks/watcher/start', { worktreePaths });
  } catch (e) {
    console.error(`startTaskWatcher failed: ${e}`);
    throw e;
  }
}

export async function stopTaskWatcher(): Promise<void> {
  try {
    await apiPost('/tasks/watcher/stop');
  } catch (e) {
    console.error(`stopTaskWatcher failed: ${e}`);
    throw e;
  }
}

export async function watchTaskPath(worktreePath: string): Promise<boolean> {
  try {
    return await apiPost<boolean>('/tasks/watcher/add', { worktreePath });
  } catch (e) {
    console.error(`watchTaskPath failed: ${e}`);
    throw e;
  }
}
