export type KnowledgeType = 'trajectory' | 'commit' | 'explicit' | 'errorResolution' | 'pattern';

export type KnowledgeMetadata = {
  sessionId: string | null;
  toolNames: string[];
  filePaths: string[];
  runStatus: string | null;
  costUsd: number | null;
  qualityScore: number;
};

export type KnowledgeStats = {
  totalEntries: number;
  trajectoryCount: number;
  explicitCount: number;
  avgQuality: number;
  lastIngested: number | null;
};

export type QueryResult = {
  id: number;
  content: string;
  entryType: KnowledgeType;
  distance: number;
  metadata: KnowledgeMetadata;
};

export type MergeResult = {
  promoted: number;
  discarded: number;
  deleted: number;
};
