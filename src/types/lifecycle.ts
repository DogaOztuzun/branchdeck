import type { PrSummary } from './github';

export type EpochMs = number;

export type LifecycleStatus =
  | 'running'
  | 'reviewReady'
  | 'approved'
  | 'fixing'
  | 'completed'
  | 'retrying'
  | 'stale'
  | 'failed';

export type LifecycleEvent = {
  prKey: string;
  worktreePath: string;
  status: LifecycleStatus;
  attempt: number;
  startedAt: EpochMs;
  sessionId?: string;
};

export type RunningEntry = {
  prKey: string;
  worktreePath: string;
  tabId: string;
  startedAt: EpochMs;
  attempt: number;
  branch: string;
  baseBranch: string;
};

export type PlanStep = {
  description: string;
  file: string;
  change_type: string;
};

export type FailureInfo = {
  check_name: string;
  error_summary: string;
  root_cause: string;
  fix_approach: string;
};

export type ReviewInfo = {
  reviewer: string;
  comment: string;
  proposed_response: string;
};

export type ApprovedPlan = {
  plan_steps: PlanStep[];
  affected_files: string[];
  summary: string;
};

export type PrContext = {
  repo: string;
  number: number;
  branch: string;
  base_branch: string;
};

export type AnalysisPlan = {
  pr: PrContext;
  confidence: string;
  failures: FailureInfo[];
  reviews: ReviewInfo[];
  plan_steps: PlanStep[];
  affected_files: string[];
  reasoning: string;
  approved: boolean;
  approved_plan: ApprovedPlan | null;
  resolved: boolean;
};

export type TriagePr = {
  prKey: string;
  pr: PrSummary | undefined;
  lifecycle: LifecycleEvent | undefined;
  analysis: AnalysisPlan | undefined;
  currentSessionId: string | undefined;
  repoPath: string | undefined;
};

export type TriageGroups = {
  needsAttention: TriagePr[];
  inProgress: TriagePr[];
  watching: TriagePr[];
  newPrs: TriagePr[];
  done: TriagePr[];
};
