import { invoke } from '@tauri-apps/api/core';

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
  return await invoke<GlobalConfig>('get_app_config');
}

export async function saveAppConfig(config: GlobalConfig): Promise<void> {
  await invoke('save_app_config', { config });
}

export async function getRepoConfig(repoPath: string): Promise<RepoConfig> {
  return await invoke<RepoConfig>('get_repo_config', { repoPath });
}

export async function saveRepoConfig(repoPath: string, config: RepoConfig): Promise<void> {
  await invoke('save_repo_config_cmd', { repoPath, config });
}
