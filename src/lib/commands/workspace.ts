import { apiGet, apiPost } from '../api/client';

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

export type Preset = {
  name: string;
  command: string;
  tabType: 'shell' | 'claude';
  env: Record<string, string>;
};

export async function getAppConfig(): Promise<GlobalConfig> {
  try {
    return await apiGet<GlobalConfig>('/config/app');
  } catch (e) {
    console.error(`getAppConfig failed: ${e}`);
    throw e;
  }
}

export async function saveAppConfig(config: GlobalConfig): Promise<void> {
  try {
    await apiPost('/config/app', config);
  } catch (e) {
    console.error(`saveAppConfig failed: ${e}`);
    throw e;
  }
}

export async function getRepoConfig(repoPath: string): Promise<RepoConfig> {
  try {
    return await apiGet<RepoConfig>(`/config/repo?repoPath=${encodeURIComponent(repoPath)}`);
  } catch (e) {
    console.error(`getRepoConfig failed: ${e}`);
    throw e;
  }
}

export async function saveRepoConfig(repoPath: string, config: RepoConfig): Promise<void> {
  try {
    await apiPost(`/config/repo?repoPath=${encodeURIComponent(repoPath)}`, config);
  } catch (e) {
    console.error(`saveRepoConfig failed: ${e}`);
    throw e;
  }
}

export async function getPresets(repoPath: string): Promise<Preset[]> {
  try {
    return await apiGet<Preset[]>(`/config/presets?repoPath=${encodeURIComponent(repoPath)}`);
  } catch (e) {
    console.error(`getPresets failed: ${e}`);
    throw e;
  }
}

export async function savePresets(repoPath: string, presets: Preset[]): Promise<void> {
  try {
    await apiPost(`/config/presets?repoPath=${encodeURIComponent(repoPath)}`, presets);
  } catch (e) {
    console.error(`savePresets failed: ${e}`);
    throw e;
  }
}
