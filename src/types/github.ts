export type CheckRunInfo = {
  name: string;
  conclusion: string | null;
  status: string;
  detailsUrl: string | null;
};

export type ReviewInfo = {
  user: string;
  state: string;
  submittedAt: string | null;
};

export type PrInfo = {
  number: number;
  title: string;
  state: string;
  isDraft: boolean;
  url: string;
  checks: CheckRunInfo[];
  reviews: ReviewInfo[];
  additions: number | null;
  deletions: number | null;
  reviewDecision: string | null;
};

export type PrSummary = {
  number: number;
  title: string;
  branch: string;
  url: string;
  ciStatus: string | null;
  reviewDecision: string | null;
  repoName: string;
  author: string;
  additions: number | null;
  deletions: number | null;
  changedFiles: number | null;
  createdAt: string | null;
};

export type PrFilter = {
  author?: string | null;
  ciStatus?: string | null;
  label?: string | null;
};

export type ShepherdResult = {
  task: import('./task').TaskInfo;
  worktreePath: string;
  knowledgeRecalled: number;
};

export type QueuedRun = {
  taskPath: string;
  worktreePath: string;
};

export type QueueStatus = {
  active: string | null;
  queued: QueuedRun[];
  completed: number;
  failed: number;
};
