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
const registeredTypes = new Set<string>();
let eventSource: EventSource | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectAttempt = 0;

const RECONNECT_BASE_MS = 1000;
const RECONNECT_MAX_MS = 30000;

function ensureConnection() {
  if (eventSource?.readyState !== EventSource.CLOSED && eventSource !== null) {
    return;
  }

  // Clear pending reconnect timer to prevent duplicate EventSource
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }

  const url = `${getBaseUrl()}/events`;
  registeredTypes.clear();
  eventSource = new EventSource(url);

  eventSource.onopen = () => {
    reconnectAttempt = 0;
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  };

  eventSource.onerror = () => {
    eventSource?.close();
    eventSource = null;
    if (!reconnectTimer) {
      const delay = Math.min(RECONNECT_BASE_MS * 2 ** reconnectAttempt, RECONNECT_MAX_MS);
      reconnectAttempt++;
      reconnectTimer = setTimeout(() => {
        reconnectTimer = null;
        if (listeners.size > 0) {
          ensureConnection();
        }
      }, delay);
    }
  };

  // Register handlers for all subscribed event types
  for (const eventType of listeners.keys()) {
    registerSseHandler(eventType);
  }
}

function registerSseHandler(eventType: string) {
  if (!eventSource) return;
  if (registeredTypes.has(eventType)) return;
  registeredTypes.add(eventType);
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
  registeredTypes.clear();
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
}
