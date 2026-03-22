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
