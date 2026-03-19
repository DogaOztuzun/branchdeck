import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { KnowledgeStats, QueryResult } from '../../types/knowledge';

export async function queryKnowledge(
  repoPath: string,
  worktreeId: string | null,
  query: string,
  topK?: number,
): Promise<QueryResult[]> {
  try {
    return await invoke<QueryResult[]>('query_knowledge', {
      repoPath,
      worktreeId,
      query,
      topK: topK ?? null,
    });
  } catch (e) {
    logError(`queryKnowledge failed: ${e}`);
    throw e;
  }
}

export async function ingestKnowledge(
  repoPath: string,
  worktreeId: string | null,
  content: string,
  entryType: string,
): Promise<number> {
  try {
    return await invoke<number>('ingest_knowledge', {
      repoPath,
      worktreeId,
      content,
      entryType,
    });
  } catch (e) {
    logError(`ingestKnowledge failed: ${e}`);
    throw e;
  }
}

export async function getKnowledgeStats(repoPath: string): Promise<KnowledgeStats> {
  try {
    return await invoke<KnowledgeStats>('get_knowledge_stats', {
      repoPath,
    });
  } catch (e) {
    logError(`getKnowledgeStats failed: ${e}`);
    throw e;
  }
}
