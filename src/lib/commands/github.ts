import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { PrInfo } from '../../types/github';

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
