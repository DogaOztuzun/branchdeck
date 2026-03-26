export type FindingCategory = 'app' | 'runner' | 'scenario';

export type FindingSeverity = 'critical' | 'high' | 'medium' | 'low';

export type FindingStatus = 'open' | 'issue-created' | 'fixed' | 'false-positive';

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
};

export type CategoryFilter = 'all' | FindingCategory;

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
