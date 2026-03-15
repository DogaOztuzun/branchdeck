import { invoke } from '@tauri-apps/api/core';
import type { FileStatus, RepoInfo, WorktreeInfo, WorktreePreview } from '../../types/git';

export async function addRepository(): Promise<RepoInfo | null> {
  return await invoke<RepoInfo | null>('add_repository');
}

export async function listRepositories(): Promise<RepoInfo[]> {
  return await invoke<RepoInfo[]>('list_repositories');
}

export async function removeRepository(repoPath: string): Promise<void> {
  await invoke('remove_repository', { repoPath });
}

export async function listWorktrees(repoPath: string): Promise<WorktreeInfo[]> {
  return await invoke<WorktreeInfo[]>('list_worktrees_cmd', { repoPath });
}

export async function createWorktree(
  repoPath: string,
  name: string,
  branch?: string,
): Promise<WorktreeInfo> {
  return await invoke<WorktreeInfo>('create_worktree_cmd', { repoPath, name, branch });
}

export async function removeWorktree(
  repoPath: string,
  worktreeName: string,
  deleteBranch: boolean,
): Promise<void> {
  await invoke('remove_worktree_cmd', { repoPath, worktreeName, deleteBranch });
}

export async function previewWorktree(repoPath: string, name: string): Promise<WorktreePreview> {
  return await invoke<WorktreePreview>('preview_worktree_cmd', { repoPath, name });
}

export async function getRepoStatus(worktreePath: string): Promise<FileStatus[]> {
  return await invoke<FileStatus[]>('get_repo_status', { worktreePath });
}
