export type RepoInfo = {
  name: string;
  path: string;
  currentBranch: string;
};

export type WorktreeInfo = {
  name: string;
  path: string;
  branch: string;
  isMain: boolean;
};

export type WorktreePreview = {
  sanitizedName: string;
  branchName: string;
  worktreePath: string;
  baseBranch: string;
  branchExists: boolean;
  pathExists: boolean;
  worktreeExists: boolean;
};

export type BranchInfo = {
  name: string;
  isRemote: boolean;
  isHead: boolean;
  hasWorktree: boolean;
};

export type TrackingInfo = {
  ahead: number;
  behind: number;
  upstreamName: string;
};

export type FileStatus = {
  path: string;
  status: string;
};
