import type { PtyEvent } from '../../types/terminal';

// Terminal PTY requires the daemon WebSocket route which is not yet implemented.
// Desktop mode previously used Tauri IPC for PTY; that was removed in the daemon
// migration (Epic 8). These stubs throw with a clear message until the WS route
// lands (tracked for a future story).

const NOT_AVAILABLE = 'Terminal is not available — daemon WebSocket PTY route not yet implemented.';

export async function createTerminalSession(
  _cwd: string,
  _shell: string,
  _env: Record<string, string>,
  _onEvent: (event: PtyEvent) => void,
): Promise<string> {
  throw new Error(NOT_AVAILABLE);
}

export async function writeTerminal(_sessionId: string, _data: Uint8Array): Promise<void> {
  throw new Error(NOT_AVAILABLE);
}

export async function resizeTerminal(
  _sessionId: string,
  _rows: number,
  _cols: number,
): Promise<void> {
  throw new Error(NOT_AVAILABLE);
}

export async function closeTerminal(_sessionId: string): Promise<void> {
  // No-op: nothing to close when terminal is not available
}
