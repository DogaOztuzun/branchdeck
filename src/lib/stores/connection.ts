import { createSignal } from 'solid-js';

export type ConnectionStatus = 'connected' | 'reconnecting' | 'disconnected';

const [status, setStatus] = createSignal<ConnectionStatus>('connected');
const [reconnectAttempts, setReconnectAttempts] = createSignal(0);

let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

function startReconnecting() {
  if (status() === 'reconnecting' || status() === 'disconnected') return;
  setStatus('reconnecting');
  setReconnectAttempts(0);

  reconnectTimer = setTimeout(() => {
    if (status() === 'reconnecting') {
      setStatus('disconnected');
    }
  }, 30_000);
}

function markConnected() {
  setStatus('connected');
  setReconnectAttempts(0);
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
}

function retry() {
  setStatus('reconnecting');
  setReconnectAttempts((n) => n + 1);

  if (reconnectTimer) clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => {
    if (status() === 'reconnecting') {
      setStatus('disconnected');
    }
  }, 30_000);
}

export function getConnectionStore() {
  return {
    status,
    reconnectAttempts,
    startReconnecting,
    markConnected,
    retry,
  };
}
