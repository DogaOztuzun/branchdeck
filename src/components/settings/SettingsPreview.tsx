import { Show } from 'solid-js';
import { cn } from '../../lib/cn';
import { getSettingsStore } from '../../lib/stores/settings';

type SettingsPreviewProps = {
  totalFindings: number;
  falsePositives: number;
};

export function SettingsPreview(props: SettingsPreviewProps) {
  const settings = getSettingsStore();

  return (
    <Show when={settings.isDirty()}>
      <div class={cn('text-base py-2 px-3 border border-border-subtle', 'text-accent-warning')}>
        {settings.previewImpact(props.totalFindings, props.falsePositives)}
      </div>
    </Show>
  );
}
