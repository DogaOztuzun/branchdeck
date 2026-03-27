import type { PrSummary } from '../../types/github';
import type { ApprovedPlan, LifecycleEvent, RunningEntry } from '../../types/lifecycle';
import { apiGet, apiPost } from '../api/client';

export async function relaunchPr(prKey: string, worktreePath: string): Promise<void> {
  try {
    await apiPost('/lifecycle/relaunch', { prKey, worktreePath });
  } catch (e) {
    console.error(`relaunchPr failed: ${e}`);
    throw e;
  }
}

export async function skipPr(prKey: string): Promise<void> {
  try {
    await apiPost('/lifecycle/skip', { prKey });
  } catch (e) {
    console.error(`skipPr failed: ${e}`);
    throw e;
  }
}

export async function getLifecycles(): Promise<LifecycleEvent[]> {
  try {
    return await apiGet<LifecycleEvent[]>('/lifecycle/events');
  } catch (e) {
    console.error(`getLifecycles failed: ${e}`);
    throw e;
  }
}

export async function toggleOrchestrator(enabled: boolean): Promise<void> {
  try {
    await apiPost('/lifecycle/orchestrator/toggle', { enabled });
  } catch (e) {
    console.error(`toggleOrchestrator failed: ${e}`);
    throw e;
  }
}

export async function shepherdPr(repoPath: string, prNumber: number): Promise<void> {
  try {
    await apiPost('/lifecycle/shepherd', { repoPath, prNumber });
  } catch (e) {
    console.error(`shepherdPr failed: ${e}`);
    throw e;
  }
}

export async function readAnalysis(worktreePath: string): Promise<string | null> {
  try {
    return await apiGet<string | null>(
      `/lifecycle/analysis?worktreePath=${encodeURIComponent(worktreePath)}`,
    );
  } catch (e) {
    console.error(`readAnalysis failed: ${e}`);
    throw e;
  }
}

export async function writeApproval(
  worktreePath: string,
  approvedPlan: ApprovedPlan,
): Promise<void> {
  try {
    await apiPost('/lifecycle/approval', { worktreePath, approvedPlan });
  } catch (e) {
    console.error(`writeApproval failed: ${e}`);
    throw e;
  }
}

export async function listDiscoveredPrs(): Promise<PrSummary[]> {
  try {
    return await apiGet<PrSummary[]>('/lifecycle/discovered-prs');
  } catch (e) {
    console.error(`listDiscoveredPrs failed: ${e}`);
    throw e;
  }
}

export async function getRunningEntries(): Promise<RunningEntry[]> {
  try {
    return await apiGet<RunningEntry[]>('/lifecycle/running');
  } catch (e) {
    console.error(`getRunningEntries failed: ${e}`);
    throw e;
  }
}
