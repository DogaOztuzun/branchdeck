import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { TaskInfo, TaskType } from '../../types/task';

export async function createTask(
  worktreePath: string,
  taskType: TaskType,
  repo: string,
  branch: string,
  pr?: number,
  description?: string,
): Promise<TaskInfo> {
  try {
    return await invoke<TaskInfo>('create_task_cmd', {
      worktreePath,
      taskType,
      repo,
      branch,
      pr: pr ?? null,
      description: description || null,
    });
  } catch (e) {
    logError(`createTask failed: ${e}`);
    throw e;
  }
}

export async function getTask(worktreePath: string): Promise<TaskInfo> {
  try {
    return await invoke<TaskInfo>('get_task_cmd', { worktreePath });
  } catch (e) {
    logError(`getTask failed: ${e}`);
    throw e;
  }
}

export async function listTasks(worktreePaths: string[]): Promise<TaskInfo[]> {
  try {
    return await invoke<TaskInfo[]>('list_tasks_cmd', { worktreePaths });
  } catch (e) {
    logError(`listTasks failed: ${e}`);
    throw e;
  }
}

export async function startTaskWatcher(worktreePaths: string[]): Promise<void> {
  try {
    await invoke('start_task_watcher', { worktreePaths });
  } catch (e) {
    logError(`startTaskWatcher failed: ${e}`);
    throw e;
  }
}

export async function stopTaskWatcher(): Promise<void> {
  try {
    await invoke('stop_task_watcher');
  } catch (e) {
    logError(`stopTaskWatcher failed: ${e}`);
    throw e;
  }
}

export async function watchTaskPath(worktreePath: string): Promise<boolean> {
  try {
    return await invoke<boolean>('watch_task_path', { worktreePath });
  } catch (e) {
    logError(`watchTaskPath failed: ${e}`);
    throw e;
  }
}
