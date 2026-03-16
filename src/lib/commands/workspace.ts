import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';

export type GlobalConfig = {
  window: { width: number; height: number; x: number; y: number };
  defaultShell: string;
  repos: string[];
  lastActiveRepo: string | null;
};

export type RepoConfig = {
  path: string;
  lastWorktree: string | null;
  sidebarCollapsed: boolean;
};

export async function getAppConfig(): Promise<GlobalConfig> {
  try {
    return await invoke<GlobalConfig>('get_app_config');
  } catch (e) {
    logError(`getAppConfig failed: ${e}`);
    throw e;
  }
}

export async function saveAppConfig(config: GlobalConfig): Promise<void> {
  try {
    await invoke('save_app_config', { config });
  } catch (e) {
    logError(`saveAppConfig failed: ${e}`);
    throw e;
  }
}

export async function getRepoConfig(repoPath: string): Promise<RepoConfig> {
  try {
    return await invoke<RepoConfig>('get_repo_config', { repoPath });
  } catch (e) {
    logError(`getRepoConfig failed: ${e}`);
    throw e;
  }
}

export async function saveRepoConfig(repoPath: string, config: RepoConfig): Promise<void> {
  try {
    await invoke('save_repo_config_cmd', { repoPath, config });
  } catch (e) {
    logError(`saveRepoConfig failed: ${e}`);
    throw e;
  }
}
