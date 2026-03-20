import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { QueueStatus, ShepherdResult } from '../../types/github';
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

export async function recoverRuns(worktreePaths: string[]): Promise<RunInfo[]> {
  try {
    return await invoke<RunInfo[]>('recover_runs_cmd', { worktreePaths });
  } catch (e) {
    logError(`recoverRuns failed: ${e}`);
    throw e;
  }
}

export async function retryRun(taskPath: string, worktreePath: string): Promise<RunInfo> {
  try {
    return await invoke<RunInfo>('retry_run_cmd', { taskPath, worktreePath });
  } catch (e) {
    logError(`retryRun failed: ${e}`);
    throw e;
  }
}

export async function resumeRun(taskPath: string, worktreePath: string): Promise<RunInfo> {
  try {
    return await invoke<RunInfo>('resume_run_cmd', { taskPath, worktreePath });
  } catch (e) {
    logError(`resumeRun failed: ${e}`);
    throw e;
  }
}

export async function shepherdPr(
  repoPath: string,
  prNumber: number,
  autoLaunch?: boolean,
): Promise<ShepherdResult> {
  try {
    return await invoke<ShepherdResult>('shepherd_pr_cmd', {
      repoPath,
      prNumber,
      autoLaunch: autoLaunch ?? null,
    });
  } catch (e) {
    logError(`shepherdPr failed: ${e}`);
    throw e;
  }
}

export async function batchLaunch(pairs: [string, string][]): Promise<QueueStatus> {
  try {
    return await invoke<QueueStatus>('batch_launch_cmd', { pairs });
  } catch (e) {
    logError(`batchLaunch failed: ${e}`);
    throw e;
  }
}

export async function cancelQueue(): Promise<void> {
  try {
    await invoke('cancel_queue_cmd');
  } catch (e) {
    logError(`cancelQueue failed: ${e}`);
    throw e;
  }
}

export async function getQueueStatus(): Promise<QueueStatus> {
  try {
    return await invoke<QueueStatus>('queue_status_cmd');
  } catch (e) {
    logError(`getQueueStatus failed: ${e}`);
    throw e;
  }
}

export async function respondToPermission(
  toolUseId: string,
  decision: string,
  reason?: string,
): Promise<void> {
  try {
    await invoke('respond_to_permission_cmd', {
      toolUseId,
      decision,
      reason: reason ?? null,
    });
  } catch (e) {
    logError(`respondToPermission failed: ${e}`);
    throw e;
  }
}
