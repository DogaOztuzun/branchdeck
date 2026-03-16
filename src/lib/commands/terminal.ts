import { Channel, invoke } from '@tauri-apps/api/core';
import { error as logError } from '@tauri-apps/plugin-log';
import type { PtyEvent } from '../../types/terminal';

export async function createTerminalSession(
  cwd: string,
  shell: string,
  env: Record<string, string>,
  onEvent: (event: PtyEvent) => void,
): Promise<string> {
  try {
    const channel = new Channel<PtyEvent>();
    channel.onmessage = onEvent;
    return await invoke<string>('create_terminal_session', {
      cwd,
      shell,
      env,
      onOutput: channel,
    });
  } catch (e) {
    logError(`createTerminalSession failed: ${e}`);
    throw e;
  }
}

export async function writeTerminal(sessionId: string, data: Uint8Array): Promise<void> {
  try {
    await invoke('write_terminal', { sessionId, data: Array.from(data) });
  } catch (e) {
    logError(`writeTerminal failed: ${e}`);
    throw e;
  }
}

export async function resizeTerminal(sessionId: string, rows: number, cols: number): Promise<void> {
  try {
    await invoke('resize_terminal', { sessionId, rows, cols });
  } catch (e) {
    logError(`resizeTerminal failed: ${e}`);
    throw e;
  }
}

export async function closeTerminal(sessionId: string): Promise<void> {
  try {
    await invoke('close_terminal', { sessionId });
  } catch (e) {
    logError(`closeTerminal failed: ${e}`);
    throw e;
  }
}
