import type { QueueStatus, ShepherdResult } from '../../types/github';
import type { RunInfo } from '../../types/run';
import { apiGet, apiPost } from '../api/client';

export async function launchRun(
  taskPath: string,
  worktreePath: string,
  maxTurns?: number,
  maxBudgetUsd?: number,
): Promise<RunInfo> {
  try {
    return await apiPost<RunInfo>('/runs', {
      taskPath,
      worktreePath,
      maxTurns: maxTurns ?? null,
      maxBudgetUsd: maxBudgetUsd ?? null,
    });
  } catch (e) {
    console.error(`launchRun failed: ${e}`);
    throw e;
  }
}

export async function cancelRun(): Promise<void> {
  try {
    await apiPost('/runs/cancel');
  } catch (e) {
    console.error(`cancelRun failed: ${e}`);
    throw e;
  }
}

export async function getRunStatus(): Promise<RunInfo | null> {
  try {
    return await apiGet<RunInfo | null>('/runs/status');
  } catch (e) {
    console.error(`getRunStatus failed: ${e}`);
    throw e;
  }
}

export async function recoverRuns(worktreePaths: string[]): Promise<RunInfo[]> {
  try {
    return await apiPost<RunInfo[]>('/runs/recover', { worktreePaths });
  } catch (e) {
    console.error(`recoverRuns failed: ${e}`);
    throw e;
  }
}

export async function retryRun(taskPath: string, worktreePath: string): Promise<RunInfo> {
  try {
    return await apiPost<RunInfo>('/runs/retry', { taskPath, worktreePath });
  } catch (e) {
    console.error(`retryRun failed: ${e}`);
    throw e;
  }
}

export async function resumeRun(taskPath: string, worktreePath: string): Promise<RunInfo> {
  try {
    return await apiPost<RunInfo>('/runs/resume', { taskPath, worktreePath });
  } catch (e) {
    console.error(`resumeRun failed: ${e}`);
    throw e;
  }
}

export async function shepherdPr(
  repoPath: string,
  prNumber: number,
  autoLaunch?: boolean,
): Promise<ShepherdResult> {
  try {
    return await apiPost<ShepherdResult>('/runs/shepherd', {
      repoPath,
      prNumber,
      autoLaunch: autoLaunch ?? null,
    });
  } catch (e) {
    console.error(`shepherdPr failed: ${e}`);
    throw e;
  }
}

export async function batchLaunch(pairs: [string, string][]): Promise<QueueStatus> {
  try {
    return await apiPost<QueueStatus>('/runs/batch', { pairs });
  } catch (e) {
    console.error(`batchLaunch failed: ${e}`);
    throw e;
  }
}

export async function cancelQueue(): Promise<void> {
  try {
    await apiPost('/runs/queue/cancel');
  } catch (e) {
    console.error(`cancelQueue failed: ${e}`);
    throw e;
  }
}

export async function getQueueStatus(): Promise<QueueStatus> {
  try {
    return await apiGet<QueueStatus>('/runs/queue/status');
  } catch (e) {
    console.error(`getQueueStatus failed: ${e}`);
    throw e;
  }
}

export async function respondToPermission(
  toolUseId: string,
  decision: string,
  reason?: string,
): Promise<void> {
  try {
    await apiPost('/runs/permission', {
      toolUseId,
      decision,
      reason: reason ?? null,
    });
  } catch (e) {
    console.error(`respondToPermission failed: ${e}`);
    throw e;
  }
}
