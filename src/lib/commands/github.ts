import type { PrFilter, PrInfo, PrSummary } from '../../types/github';
import { apiGet, apiPost } from '../api/client';

export async function getPrStatus(repoPath: string, branchName: string): Promise<PrInfo | null> {
  try {
    return await apiGet<PrInfo | null>(
      `/github/pr-status?repoPath=${encodeURIComponent(repoPath)}&branchName=${encodeURIComponent(branchName)}`,
    );
  } catch (e) {
    console.error(`getPrStatus failed: ${e}`);
    throw e;
  }
}

export async function checkGithubAvailable(): Promise<boolean> {
  try {
    return await apiGet<boolean>('/github/available');
  } catch (e) {
    console.error(`checkGithubAvailable failed: ${e}`);
    return false;
  }
}

export async function listOpenPrs(repoPath: string, filter?: PrFilter): Promise<PrSummary[]> {
  try {
    return await apiPost<PrSummary[]>('/github/open-prs', {
      repoPath,
      filter: filter ?? null,
    });
  } catch (e) {
    console.error(`listOpenPrs failed: ${e}`);
    throw e;
  }
}

export async function listAllOpenPrs(repoPaths: string[], filter?: PrFilter): Promise<PrSummary[]> {
  try {
    return await apiPost<PrSummary[]>('/github/all-open-prs', {
      repoPaths,
      filter: filter ?? null,
    });
  } catch (e) {
    console.error(`listAllOpenPrs failed: ${e}`);
    throw e;
  }
}

export async function enrichPrSummary(repoPath: string, pr: PrSummary): Promise<PrSummary> {
  try {
    return await apiPost<PrSummary>('/github/enrich-pr', { repoPath, pr });
  } catch (e) {
    console.error(`enrichPrSummary failed: ${e}`);
    throw e;
  }
}
