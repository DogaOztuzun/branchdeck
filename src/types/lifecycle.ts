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
  /** The workflow definition name that produced this event */
  workflowName?: string;
  /** Human-readable status label from the workflow definition's lifecycle section */
  displayStatus?: string;
  /** Timestamp when the cycle completed (terminal states) */
  completedAt?: EpochMs;
};

/** A single timestamped entry in a workflow cycle's lifecycle timeline (NFR25) */
export type LifecycleTimelineEntry = {
  timestamp: EpochMs;
  status: string;
  displayStatus: string;
  detail: string;
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

/** Workflow types that the lifecycle view tracks */
export type WorkflowType = 'issue-resolution' | 'sat-scoring' | 'verification' | 'manual';

/** Trigger sources for a workflow cycle */
export type TriggerSource = 'pr-poll' | 'post-merge' | 'issue-detected' | 'retry' | 'manual';

/** A single workflow cycle shown in the lifecycle view */
export type WorkflowCycle = {
  id: string;
  prKey: string;
  workflowType: WorkflowType;
  triggerSource: TriggerSource;
  status: LifecycleStatus;
  attempt: number;
  startedAt: EpochMs;
  updatedAt: EpochMs;
  completedAt: EpochMs | null;
  worktreePath: string;
  description: string;
  /** Workflow definition name (e.g., "pr-shepherd", "sat-scoring") */
  workflowName?: string;
  /** Custom display label for the current status from the workflow definition */
  displayStatus?: string;
  /** Full timeline of lifecycle transitions with timestamps */
  timeline: LifecycleTimelineEntry[];
};
