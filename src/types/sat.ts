export type FindingCategory = 'app' | 'runner' | 'scenario';

export type FindingSeverity = 'critical' | 'high' | 'medium' | 'low';

export type FindingStatus = 'open' | 'issue-created' | 'fixed' | 'false-positive';

export type ConfidenceLevel = 'high' | 'medium' | 'low';

export type SATFinding = {
  id: string;
  title: string;
  category: FindingCategory;
  severity: FindingSeverity;
  status: FindingStatus;
  persona: string;
  cycle: number;
  confidence: number;
  evidence?: string;
};

export type SATCycle = {
  cycle: number;
  score: number;
  date: string;
  findingsCount: number;
  /** Number of findings that were false positives in this cycle */
  falsePositives: number;
  /** Number of findings that were confirmed fixed in this cycle */
  issuesFixed: number;
  /** Total issues found in this cycle */
  issuesFound: number;
};

export type CategoryFilter = 'all' | FindingCategory;

/** Classification accuracy metrics computed from historical cycle data (FR27, NFR24) */
export type ClassificationAccuracy = {
  totalClassifications: number;
  truePositives: number;
  falsePositives: number;
  /** Accuracy as 0-100 percentage, or null if no data */
  accuracy: number | null;
  cyclesCounted: number;
};

// Pipeline types (Story 3.5)

export type SatPipelineStage = 'generate' | 'execute' | 'score' | 'create_issues';

export type SatPipelineStatus =
  | { status: 'running'; stage: SatPipelineStage }
  | { status: 'completed' }
  | { status: 'failed'; stage: SatPipelineStage; error: string };

export type SatStageResult = {
  stage: SatPipelineStage;
  success: boolean;
  duration_ms: number;
  error?: string;
};

export type SatPipelineResult = {
  status: SatPipelineStatus;
  stages: SatStageResult[];
  total_duration_ms: number;
  run_id?: string;
  aggregate_score?: number;
  issues_created?: number;
};
