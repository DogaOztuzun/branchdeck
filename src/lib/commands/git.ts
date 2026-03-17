import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type {
  BranchInfo,
  FileStatus,
  RepoInfo,
  TrackingInfo,
  WorktreeInfo,
  WorktreePreview,
} from '../../types/git';

export async function addRepository(): Promise<RepoInfo | null> {
  try {
    return await invoke<RepoInfo | null>('add_repository');
  } catch (e) {
    logError(`addRepository failed: ${e}`);
    throw e;
  }
}

export async function listRepositories(): Promise<RepoInfo[]> {
  try {
    return await invoke<RepoInfo[]>('list_repositories');
  } catch (e) {
    logError(`listRepositories failed: ${e}`);
    throw e;
  }
}

export async function removeRepository(repoPath: string): Promise<void> {
  try {
    await invoke('remove_repository', { repoPath });
  } catch (e) {
    logError(`removeRepository failed: ${e}`);
    throw e;
  }
}

export async function listWorktrees(repoPath: string): Promise<WorktreeInfo[]> {
  try {
    return await invoke<WorktreeInfo[]>('list_worktrees_cmd', { repoPath });
  } catch (e) {
    logError(`listWorktrees failed: ${e}`);
    throw e;
  }
}

export async function createWorktree(
  repoPath: string,
  name: string,
  branch?: string,
  baseBranch?: string,
): Promise<WorktreeInfo> {
  try {
    return await invoke<WorktreeInfo>('create_worktree_cmd', {
      repoPath,
      name,
      branch,
      baseBranch,
    });
  } catch (e) {
    logError(`createWorktree failed: ${e}`);
    throw e;
  }
}

export async function removeWorktree(
  repoPath: string,
  worktreeName: string,
  deleteBranch: boolean,
): Promise<void> {
  try {
    await invoke('remove_worktree_cmd', { repoPath, worktreeName, deleteBranch });
  } catch (e) {
    logError(`removeWorktree failed: ${e}`);
    throw e;
  }
}

export async function previewWorktree(repoPath: string, name: string): Promise<WorktreePreview> {
  try {
    return await invoke<WorktreePreview>('preview_worktree_cmd', { repoPath, name });
  } catch (e) {
    logError(`previewWorktree failed: ${e}`);
    throw e;
  }
}

export async function listBranches(repoPath: string): Promise<BranchInfo[]> {
  try {
    return await invoke<BranchInfo[]>('list_branches_cmd', { repoPath });
  } catch (e) {
    logError(`listBranches failed: ${e}`);
    throw e;
  }
}

export async function getBranchTracking(
  repoPath: string,
  branchName: string,
): Promise<TrackingInfo | null> {
  try {
    return await invoke<TrackingInfo | null>('get_branch_tracking_cmd', { repoPath, branchName });
  } catch (e) {
    logError(`getBranchTracking failed: ${e}`);
    throw e;
  }
}

export async function getRepoStatus(worktreePath: string): Promise<FileStatus[]> {
  try {
    return await invoke<FileStatus[]>('get_repo_status', { worktreePath });
  } catch (e) {
    logError(`getRepoStatus failed: ${e}`);
    throw e;
  }
}

export async function listRepoFiles(worktreePath: string): Promise<string[]> {
  try {
    return await invoke<string[]>('list_repo_files_cmd', { worktreePath });
  } catch (e) {
    logError(`listRepoFiles failed: ${e}`);
    throw e;
  }
}
