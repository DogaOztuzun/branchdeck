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
