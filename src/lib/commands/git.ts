import type {
  BranchInfo,
  FileStatus,
  RepoInfo,
  TrackingInfo,
  WorktreeInfo,
  WorktreePreview,
} from '../../types/git';
import { apiDelete, apiGet, apiPost } from '../api/client';

export async function addRepository(): Promise<RepoInfo | null> {
  try {
    return await apiPost<RepoInfo | null>('/repos');
  } catch (e) {
    console.error(`addRepository failed: ${e}`);
    throw e;
  }
}

export async function listRepositories(): Promise<RepoInfo[]> {
  try {
    const detail = await apiGet<{ repo: RepoInfo; worktrees: WorktreeInfo[] }>('/repos');
    return [detail.repo];
  } catch (e) {
    console.error(`listRepositories failed: ${e}`);
    throw e;
  }
}

export async function removeRepository(repoPath: string): Promise<void> {
  try {
    await apiDelete(`/repos?repoPath=${encodeURIComponent(repoPath)}`);
  } catch (e) {
    console.error(`removeRepository failed: ${e}`);
    throw e;
  }
}

export async function listWorktrees(_repoPath: string): Promise<WorktreeInfo[]> {
  try {
    const detail = await apiGet<{ repo: RepoInfo; worktrees: WorktreeInfo[] }>('/repos');
    return detail.worktrees;
  } catch (e) {
    console.error(`listWorktrees failed: ${e}`);
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
    return await apiPost<WorktreeInfo>('/repos/worktrees', {
      repoPath,
      name,
      branch,
      baseBranch,
    });
  } catch (e) {
    console.error(`createWorktree failed: ${e}`);
    throw e;
  }
}

export async function removeWorktree(
  repoPath: string,
  worktreeName: string,
  deleteBranch: boolean,
): Promise<void> {
  try {
    await apiDelete(
      `/repos/worktrees/${encodeURIComponent(worktreeName)}?repoPath=${encodeURIComponent(repoPath)}&deleteBranch=${deleteBranch}`,
    );
  } catch (e) {
    console.error(`removeWorktree failed: ${e}`);
    throw e;
  }
}

export async function previewWorktree(repoPath: string, name: string): Promise<WorktreePreview> {
  try {
    return await apiGet<WorktreePreview>(
      `/repos/worktrees/${encodeURIComponent(name)}/preview?repoPath=${encodeURIComponent(repoPath)}`,
    );
  } catch (e) {
    console.error(`previewWorktree failed: ${e}`);
    throw e;
  }
}

export async function listBranches(repoPath: string): Promise<BranchInfo[]> {
  try {
    return await apiGet<BranchInfo[]>(`/repos/branches?repoPath=${encodeURIComponent(repoPath)}`);
  } catch (e) {
    console.error(`listBranches failed: ${e}`);
    throw e;
  }
}

export async function getBranchTracking(
  repoPath: string,
  branchName: string,
): Promise<TrackingInfo | null> {
  try {
    return await apiGet<TrackingInfo | null>(
      `/repos/branches/${encodeURIComponent(branchName)}/tracking?repoPath=${encodeURIComponent(repoPath)}`,
    );
  } catch (e) {
    console.error(`getBranchTracking failed: ${e}`);
    throw e;
  }
}

export async function getRepoStatus(worktreePath: string): Promise<FileStatus[]> {
  try {
    return await apiGet<FileStatus[]>(
      `/repos/status?worktreePath=${encodeURIComponent(worktreePath)}`,
    );
  } catch (e) {
    console.error(`getRepoStatus failed: ${e}`);
    throw e;
  }
}
