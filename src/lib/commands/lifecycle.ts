import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { PrSummary } from '../../types/github';
import type { ApprovedPlan, LifecycleEvent, RunningEntry } from '../../types/lifecycle';

export async function relaunchPr(prKey: string, worktreePath: string): Promise<void> {
  try {
    await invoke('relaunch_pr_cmd', { prKey, worktreePath });
  } catch (e) {
    logError(`relaunchPr failed: ${e}`);
    throw e;
  }
}

export async function skipPr(prKey: string): Promise<void> {
  try {
    await invoke('skip_pr_cmd', { prKey });
  } catch (e) {
    logError(`skipPr failed: ${e}`);
    throw e;
  }
}

export async function getLifecycles(): Promise<LifecycleEvent[]> {
  try {
    return await invoke<LifecycleEvent[]>('get_lifecycles_cmd');
  } catch (e) {
    logError(`getLifecycles failed: ${e}`);
    throw e;
  }
}

export async function toggleOrchestrator(enabled: boolean): Promise<void> {
  try {
    await invoke('toggle_orchestrator_cmd', { enabled });
  } catch (e) {
    logError(`toggleOrchestrator failed: ${e}`);
    throw e;
  }
}

export async function shepherdPr(repoPath: string, prNumber: number): Promise<void> {
  try {
    await invoke('orchestrator_shepherd_pr_cmd', { repoPath, prNumber });
  } catch (e) {
    logError(`shepherdPr failed: ${e}`);
    throw e;
  }
}

export async function readAnalysis(worktreePath: string): Promise<string | null> {
  try {
    return await invoke<string | null>('read_analysis_cmd', { worktreePath });
  } catch (e) {
    logError(`readAnalysis failed: ${e}`);
    throw e;
  }
}

export async function writeApproval(
  worktreePath: string,
  approvedPlan: ApprovedPlan,
): Promise<void> {
  try {
    await invoke('write_approval_cmd', { worktreePath, approvedPlan });
  } catch (e) {
    logError(`writeApproval failed: ${e}`);
    throw e;
  }
}

export async function listDiscoveredPrs(): Promise<PrSummary[]> {
  try {
    return await invoke<PrSummary[]>('list_discovered_prs_cmd');
  } catch (e) {
    logError(`listDiscoveredPrs failed: ${e}`);
    throw e;
  }
}

export async function getRunningEntries(): Promise<RunningEntry[]> {
  try {
    return await invoke<RunningEntry[]>('get_running_entries_cmd');
  } catch (e) {
    logError(`getRunningEntries failed: ${e}`);
    throw e;
  }
}
