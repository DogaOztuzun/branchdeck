import type { KnowledgeStats, QueryResult, Suggestion } from '../../types/knowledge';
import { apiDelete, apiGet, apiPost } from '../api/client';

export async function queryKnowledge(
  repoPath: string,
  worktreeId: string | null,
  query: string,
  topK?: number,
): Promise<QueryResult[]> {
  try {
    return await apiPost<QueryResult[]>('/knowledge/query', {
      repoPath,
      worktreeId,
      query,
      topK: topK ?? null,
    });
  } catch (e) {
    console.error(`queryKnowledge failed: ${e}`);
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
    return await apiPost<number>('/knowledge/ingest', {
      repoPath,
      worktreeId,
      content,
      entryType,
    });
  } catch (e) {
    console.error(`ingestKnowledge failed: ${e}`);
    throw e;
  }
}

export async function getKnowledgeStats(repoPath: string): Promise<KnowledgeStats> {
  try {
    return await apiGet<KnowledgeStats>(
      `/knowledge/stats?repoPath=${encodeURIComponent(repoPath)}`,
    );
  } catch (e) {
    console.error(`getKnowledgeStats failed: ${e}`);
    throw e;
  }
}

export async function forgetKnowledge(entryId: number): Promise<void> {
  try {
    await apiDelete(`/knowledge/entries/${entryId}`);
  } catch (e) {
    console.error(`forgetKnowledge failed: ${e}`);
    throw e;
  }
}

export async function suggestNext(
  repoPath: string,
  context: string,
  topK?: number,
): Promise<Suggestion[]> {
  try {
    return await apiPost<Suggestion[]>('/knowledge/suggest', {
      repoPath,
      context,
      topK: topK ?? null,
    });
  } catch (e) {
    console.error(`suggestNext failed: ${e}`);
    throw e;
  }
}
