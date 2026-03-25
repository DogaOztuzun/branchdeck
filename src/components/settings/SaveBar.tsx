import { Show } from 'solid-js';
import { getSettingsStore } from '../../lib/stores/settings';
import { ActionButton } from '../ui/ActionButton';

export function SaveBar() {
  const settings = getSettingsStore();

  return (
    <Show when={settings.isDirty() || settings.saveStatus() !== 'idle'}>
      <div class="sticky bottom-0 flex items-center gap-2 px-3 py-2 bg-bg-sidebar border-t border-border-subtle">
        <Show when={settings.isDirty()}>
          <ActionButton
            label="Save"
            variant="primary"
            shortcutHint="Ctrl+S"
            onClick={() => settings.save()}
          />
          <ActionButton label="Discard" variant="secondary" onClick={() => settings.discard()} />
        </Show>
        <Show when={settings.saveStatus() === 'saved'}>
          <span class="text-sm text-accent-success">Settings saved</span>
        </Show>
        <Show when={settings.saveStatus() === 'error'}>
          <span class="text-sm text-accent-error">Failed to save settings</span>
        </Show>
      </div>
    </Show>
  );
}
