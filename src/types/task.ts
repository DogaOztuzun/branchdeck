export type TaskStatus = 'created' | 'running' | 'blocked' | 'succeeded' | 'failed' | 'cancelled';

export type TaskType = 'issue-fix' | 'pr-shepherd';

export type TaskScope = 'worktree' | 'workspace';

export type TaskFrontmatter = {
  type: TaskType;
  scope: TaskScope;
  status: TaskStatus;
  repo: string;
  branch: string;
  pr: number | null;
  created: string;
  'run-count': number;
};

export type TaskInfo = {
  frontmatter: TaskFrontmatter;
  body: string;
  path: string;
};
