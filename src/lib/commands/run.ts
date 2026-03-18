import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { RunInfo } from '../../types/run';

export async function launchRun(
  taskPath: string,
  worktreePath: string,
  maxTurns?: number,
  maxBudgetUsd?: number,
): Promise<RunInfo> {
  try {
    return await invoke<RunInfo>('launch_run_cmd', {
      taskPath,
      worktreePath,
      maxTurns: maxTurns ?? null,
      maxBudgetUsd: maxBudgetUsd ?? null,
    });
  } catch (e) {
    logError(`launchRun failed: ${e}`);
    throw e;
  }
}

export async function cancelRun(): Promise<void> {
  try {
    await invoke('cancel_run_cmd');
  } catch (e) {
    logError(`cancelRun failed: ${e}`);
    throw e;
  }
}

export async function getRunStatus(): Promise<RunInfo | null> {
  try {
    return await invoke<RunInfo | null>('get_run_status_cmd');
  } catch (e) {
    logError(`getRunStatus failed: ${e}`);
    throw e;
  }
}
