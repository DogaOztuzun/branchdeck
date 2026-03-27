import { listen } from '@tauri-apps/api/event';
import { createSignal } from 'solid-js';
import type { UpdateStatusKind, UpdateStatusPayload } from '../../types/update';

const [status, setStatus] = createSignal<UpdateStatusKind>('idle');
const [version, setVersion] = createSignal<string | undefined>();
const [error, setError] = createSignal<string | undefined>();

let initialized = false;
let unlisten: (() => void) | null = null;

function init() {
  if (initialized) return;
  initialized = true;

  listen<string | UpdateStatusPayload>('update:status', (event) => {
    const payload = event.payload;

    if (typeof payload === 'string') {
      // Simple status strings: "checking", "idle"
      setStatus(payload as UpdateStatusKind);
      if (payload === 'idle' || payload === 'checking') {
        setError(undefined);
      }
    } else {
      // Structured payload with version info
      setStatus(payload.status);
      setVersion(payload.version);
      if (payload.error) {
        setError(payload.error);
      } else {
        setError(undefined);
      }
    }
  }).then((fn) => {
    unlisten = fn;
  });
}

export function getUpdateStore() {
  init();

  return {
    status,
    version,
    error,
    cleanup() {
      unlisten?.();
      initialized = false;
      unlisten = null;
    },
  };
}
