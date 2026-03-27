/**
 * SSE event subscription for the branchdeck daemon.
 * Stores use onEvent<T>() to subscribe to typed daemon events.
 */

import { getBaseUrl } from './client';

export type SseEnvelope<T = unknown> = {
  id: string;
  type: string;
  timestamp: number;
  run_id?: string;
  data: T;
};

type EventCallback<T> = (envelope: SseEnvelope<T>) => void;
type Unsubscribe = () => void;

const listeners = new Map<string, Set<EventCallback<unknown>>>();
let eventSource: EventSource | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

const RECONNECT_DELAY_MS = 3000;

function ensureConnection() {
  if (eventSource?.readyState !== EventSource.CLOSED && eventSource !== null) {
    return;
  }

  const url = `${getBaseUrl()}/events`;
  eventSource = new EventSource(url);

  eventSource.onopen = () => {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  };

  eventSource.onerror = () => {
    eventSource?.close();
    eventSource = null;
    if (!reconnectTimer) {
      reconnectTimer = setTimeout(() => {
        reconnectTimer = null;
        if (listeners.size > 0) {
          ensureConnection();
        }
      }, RECONNECT_DELAY_MS);
    }
  };

  // Register handlers for all subscribed event types
  for (const eventType of listeners.keys()) {
    registerSseHandler(eventType);
  }
}

function registerSseHandler(eventType: string) {
  if (!eventSource) return;
  eventSource.addEventListener(eventType, (e: Event) => {
    const messageEvent = e as MessageEvent;
    const callbacks = listeners.get(eventType);
    if (!callbacks || callbacks.size === 0) return;

    try {
      const envelope = JSON.parse(messageEvent.data) as SseEnvelope;
      for (const cb of callbacks) {
        cb(envelope);
      }
    } catch {
      // Malformed SSE data — skip
    }
  });
}

/**
 * Subscribe to a typed SSE event from the daemon.
 * Returns an unsubscribe function.
 *
 * @example
 * const unsub = onEvent<AgentEvent>('agent:tool_start', (envelope) => {
 *   handleToolStart(envelope.data);
 * });
 */
export function onEvent<T = unknown>(eventType: string, callback: EventCallback<T>): Unsubscribe {
  let callbacks = listeners.get(eventType);
  if (!callbacks) {
    callbacks = new Set();
    listeners.set(eventType, callbacks);
    // Register on existing EventSource if connected
    if (eventSource) {
      registerSseHandler(eventType);
    }
  }
  callbacks.add(callback as EventCallback<unknown>);

  ensureConnection();

  return () => {
    callbacks?.delete(callback as EventCallback<unknown>);
    if (callbacks?.size === 0) {
      listeners.delete(eventType);
    }
    // Close connection if no listeners remain
    if (listeners.size === 0) {
      eventSource?.close();
      eventSource = null;
    }
  };
}

/**
 * Close the SSE connection and clear all listeners.
 */
export function disconnectEvents() {
  eventSource?.close();
  eventSource = null;
  listeners.clear();
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
}
