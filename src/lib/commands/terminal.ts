import { Channel, invoke } from '@tauri-apps/api/core';
import type { PtyEvent } from '../../types/terminal';

export async function createTerminalSession(
  cwd: string,
  shell: string,
  env: Record<string, string>,
  onEvent: (event: PtyEvent) => void,
): Promise<string> {
  const channel = new Channel<PtyEvent>();
  channel.onmessage = onEvent;
  return await invoke<string>('create_terminal_session', {
    cwd,
    shell,
    env,
    onOutput: channel,
  });
}

export async function writeTerminal(sessionId: string, data: Uint8Array): Promise<void> {
  await invoke('write_terminal', { sessionId, data: Array.from(data) });
}

export async function resizeTerminal(sessionId: string, rows: number, cols: number): Promise<void> {
  await invoke('resize_terminal', { sessionId, rows, cols });
}

export async function closeTerminal(sessionId: string): Promise<void> {
  await invoke('close_terminal', { sessionId });
}
