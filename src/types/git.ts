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

export type FileStatus = {
  path: string;
  status: string;
};
