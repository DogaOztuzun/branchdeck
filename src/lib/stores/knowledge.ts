import { createSignal } from 'solid-js';
import type { KnowledgeStats } from '../../types/knowledge';
import { getKnowledgeStats } from '../commands/knowledge';

const [stats, setStats] = createSignal<KnowledgeStats | null>(null);
const [loading, setLoading] = createSignal(false);

export function getKnowledgeStore() {
  return {
    stats,
    loading,
    loadStats,
    refreshStats,
  };
}

let currentRepoPath: string | null = null;

async function loadStats(repoPath: string): Promise<void> {
  currentRepoPath = repoPath;
  setLoading(true);
  try {
    const result = await getKnowledgeStats(repoPath);
    setStats(result);
  } catch {
    setStats(null);
  } finally {
    setLoading(false);
  }
}

async function refreshStats(): Promise<void> {
  if (currentRepoPath) {
    await loadStats(currentRepoPath);
  }
}
