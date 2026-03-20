import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { PrFilter, PrInfo, PrSummary } from '../../types/github';

export async function getPrStatus(repoPath: string, branchName: string): Promise<PrInfo | null> {
  try {
    return await invoke<PrInfo | null>('get_pr_status', { repoPath, branchName });
  } catch (e) {
    logError(`getPrStatus failed: ${e}`);
    throw e;
  }
}

export async function checkGithubAvailable(): Promise<boolean> {
  try {
    return await invoke<boolean>('check_github_available');
  } catch (e) {
    logError(`checkGithubAvailable failed: ${e}`);
    return false;
  }
}

export async function listOpenPrs(repoPath: string, filter?: PrFilter): Promise<PrSummary[]> {
  try {
    return await invoke<PrSummary[]>('list_open_prs', { repoPath, filter: filter ?? null });
  } catch (e) {
    logError(`listOpenPrs failed: ${e}`);
    throw e;
  }
}

export async function listAllOpenPrs(repoPaths: string[], filter?: PrFilter): Promise<PrSummary[]> {
  try {
    return await invoke<PrSummary[]>('list_all_open_prs', { repoPaths, filter: filter ?? null });
  } catch (e) {
    logError(`listAllOpenPrs failed: ${e}`);
    throw e;
  }
}

export async function enrichPrSummary(repoPath: string, pr: PrSummary): Promise<PrSummary> {
  try {
    return await invoke<PrSummary>('enrich_pr_summary', { repoPath, pr });
  } catch (e) {
    logError(`enrichPrSummary failed: ${e}`);
    throw e;
  }
}
