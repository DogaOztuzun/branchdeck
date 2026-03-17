import { invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { AgentDefinition, AgentState, FileAccess } from '../../types/agent';

export async function getAgents(tabId: string): Promise<AgentState[]> {
  try {
    return await invoke<AgentState[]>('get_agents', { tabId });
  } catch (e) {
    logError(`getAgents failed: ${e}`);
    throw e;
  }
}

export async function getFileActivity(): Promise<FileAccess[]> {
  try {
    return await invoke<FileAccess[]>('get_file_activity');
  } catch (e) {
    logError(`getFileActivity failed: ${e}`);
    throw e;
  }
}

export async function listAgentDefinitions(repoPath: string): Promise<AgentDefinition[]> {
  try {
    return await invoke<AgentDefinition[]>('list_agent_definitions', { repoPath });
  } catch (e) {
    logError(`listAgentDefinitions failed: ${e}`);
    throw e;
  }
}

export async function installAgentHooks(repoPath: string): Promise<void> {
  try {
    await invoke('install_agent_hooks', { repoPath });
  } catch (e) {
    logError(`installAgentHooks failed: ${e}`);
    throw e;
  }
}

export async function removeAgentHooks(repoPath: string): Promise<void> {
  try {
    await invoke('remove_agent_hooks', { repoPath });
  } catch (e) {
    logError(`removeAgentHooks failed: ${e}`);
    throw e;
  }
}
