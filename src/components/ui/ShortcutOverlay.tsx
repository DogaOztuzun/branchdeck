import { For } from 'solid-js';
import { getKeyboardStore } from '../../lib/stores/keyboard';
import { getLayoutStore } from '../../lib/stores/layout';
import type { ViewName } from '../../types/keyboard';

export function ShortcutOverlay() {
  const keyboard = getKeyboardStore();
  const layout = getLayoutStore();

  const activeView = (): ViewName => {
    const v = layout.activeView();
    return v === 'pr-triage' ? 'inbox' : (v as ViewName);
  };

  const viewLabel = () => {
    const v = activeView();
    return v.charAt(0).toUpperCase() + v.slice(1);
  };

  const shortcuts = () => keyboard.getShortcutsForView(activeView());

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: backdrop dismiss
    <div
      class="fixed inset-0 z-50 flex items-center justify-center"
      onClick={() => keyboard.setIsOverlayOpen(false)}
      onKeyDown={(e) => {
        if (e.key === 'Escape' || e.key === '?') keyboard.setIsOverlayOpen(false);
      }}
      role="presentation"
    >
      <div
        class="w-full max-w-xl bg-bg-sidebar/95 border border-border-subtle p-6"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={() => {}}
        role="dialog"
        aria-label="Keyboard shortcuts"
      >
        <div class="flex items-baseline gap-2 mb-4">
          <span class="text-lg font-semibold text-text-main">Keyboard shortcuts</span>
          <span class="text-base text-text-dim">{viewLabel()}</span>
        </div>
        <div class="grid grid-cols-2 gap-x-8 gap-y-1.5">
          <For each={shortcuts()}>
            {(s) => (
              <div class="flex items-center justify-between py-1">
                <span class="text-sm text-text-main">{s.label}</span>
                <span class="text-sm font-medium text-text-dim bg-bg-main px-1.5 py-px ml-2">
                  {s.key}
                </span>
              </div>
            )}
          </For>
        </div>
        <div class="mt-4 text-[11px] text-text-dim">
          Press <span class="bg-bg-main px-1 py-px">?</span> or{' '}
          <span class="bg-bg-main px-1 py-px">Esc</span> to close
        </div>
      </div>
    </div>
  );
}
