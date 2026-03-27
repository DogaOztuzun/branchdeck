import type { PtyEvent } from '../../types/terminal';
import { getBaseUrl } from '../api/client';

// Terminal uses WebSocket for PTY streaming. The daemon WS route is not yet
// implemented — terminal is desktop-only via Tauri PTY. In web/Docker mode,
// these functions throw with a clear message.

/** Returns true when running inside the Tauri desktop shell. */
function isTauriEnv(): boolean {
  return '__TAURI__' in window;
}

export async function createTerminalSession(
  cwd: string,
  shell: string,
  env: Record<string, string>,
  onEvent: (event: PtyEvent) => void,
): Promise<string> {
  if (!isTauriEnv()) {
    throw new Error('Terminal is not available in web mode. PTY requires the desktop app.');
  }

  const baseUrl = getBaseUrl();
  const wsUrl = baseUrl.replace(/^http/, 'ws');
  const sessionId = crypto.randomUUID();

  try {
    const ws = new WebSocket(
      `${wsUrl}/terminal/${sessionId}?cwd=${encodeURIComponent(cwd)}&shell=${encodeURIComponent(shell)}`,
    );

    ws.onmessage = (e) => {
      try {
        const event = JSON.parse(e.data) as PtyEvent;
        onEvent(event);
      } catch {
        // Malformed WS message
      }
    };

    ws.onerror = () => {
      console.error('Terminal WebSocket error');
    };

    // Store the WebSocket for later use by writeTerminal/resizeTerminal/closeTerminal
    terminalSockets.set(sessionId, ws);

    // Wait for connection to open
    await new Promise<void>((resolve, reject) => {
      ws.onopen = () => resolve();
      ws.onerror = () => reject(new Error('Terminal WebSocket connection failed'));
    });

    // Send initial config (env vars etc)
    ws.send(JSON.stringify({ type: 'init', env }));

    return sessionId;
  } catch (e) {
    console.error(`createTerminalSession failed: ${e}`);
    throw e;
  }
}

const terminalSockets = new Map<string, WebSocket>();

export async function writeTerminal(sessionId: string, data: Uint8Array): Promise<void> {
  const ws = terminalSockets.get(sessionId);
  if (!ws || ws.readyState !== WebSocket.OPEN) {
    console.error(`writeTerminal: no open WebSocket for session ${sessionId}`);
    return;
  }
  ws.send(data);
}

export async function resizeTerminal(sessionId: string, rows: number, cols: number): Promise<void> {
  const ws = terminalSockets.get(sessionId);
  if (!ws || ws.readyState !== WebSocket.OPEN) {
    console.error(`resizeTerminal: no open WebSocket for session ${sessionId}`);
    return;
  }
  ws.send(JSON.stringify({ type: 'resize', rows, cols }));
}

export async function closeTerminal(sessionId: string): Promise<void> {
  const ws = terminalSockets.get(sessionId);
  if (ws) {
    ws.close();
    terminalSockets.delete(sessionId);
  }
}
