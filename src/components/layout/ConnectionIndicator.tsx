import { Show } from 'solid-js';
import { getConnectionStore } from '../../lib/stores/connection';
import { ActionButton } from '../ui/ActionButton';

export function ConnectionIndicator() {
  const connection = getConnectionStore();

  return (
    <>
      {/* Reconnecting: subtle text in TopBar */}
      <Show when={connection.status() === 'reconnecting'}>
        <span class="text-[11px] text-text-dim animate-pulse-opacity ml-2">Reconnecting...</span>
      </Show>
    </>
  );
}

export function DisconnectedBanner() {
  const connection = getConnectionStore();

  return (
    <Show when={connection.status() === 'disconnected'}>
      <div class="flex items-center justify-center gap-2 h-8 bg-accent-error/10 border-b border-border-subtle">
        <span class="text-sm text-accent-error">Disconnected from daemon</span>
        <ActionButton
          label="Retry"
          variant="secondary"
          size="compact"
          onClick={() => connection.retry()}
        />
      </div>
    </Show>
  );
}
