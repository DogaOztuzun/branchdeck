import type { AgentDefinition, AgentState, FileAccess } from '../../types/agent';
import { apiGet, apiPost } from '../api/client';

export async function getAgents(tabId: string): Promise<AgentState[]> {
  try {
    return await apiGet<AgentState[]>(`/activity/sessions/${encodeURIComponent(tabId)}/agents`);
  } catch (e) {
    console.error(`getAgents failed: ${e}`);
    throw e;
  }
}

export async function getFileActivity(): Promise<FileAccess[]> {
  try {
    return await apiGet<FileAccess[]>('/activity/files');
  } catch (e) {
    console.error(`getFileActivity failed: ${e}`);
    throw e;
  }
}

export async function listAgentDefinitions(repoPath: string): Promise<AgentDefinition[]> {
  try {
    return await apiGet<AgentDefinition[]>(
      `/agents/definitions?repoPath=${encodeURIComponent(repoPath)}`,
    );
  } catch (e) {
    console.error(`listAgentDefinitions failed: ${e}`);
    throw e;
  }
}

export async function installAgentHooks(repoPath: string): Promise<void> {
  try {
    await apiPost('/agents/hooks/install', { repoPath });
  } catch (e) {
    console.error(`installAgentHooks failed: ${e}`);
    throw e;
  }
}

export async function removeAgentHooks(repoPath: string): Promise<void> {
  try {
    await apiPost('/agents/hooks/remove', { repoPath });
  } catch (e) {
    console.error(`removeAgentHooks failed: ${e}`);
    throw e;
  }
}
