import { createSignal, For, onMount, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import { getKeyboardStore } from '../../lib/stores/keyboard';
import type { CommandItem } from '../../types/keyboard';

export function CommandPalette() {
  const keyboard = getKeyboardStore();
  const [query, setQuery] = createSignal('');
  const [selectedIndex, setSelectedIndex] = createSignal(0);
  let inputRef: HTMLInputElement | undefined;

  onMount(() => {
    inputRef?.focus();
  });

  const filtered = (): CommandItem[] => {
    const q = query().toLowerCase();
    const all = keyboard.getCommands();
    if (!q) return all;
    return all.filter((c) => c.label.toLowerCase().includes(q));
  };

  const execute = (item: CommandItem) => {
    keyboard.setIsPaletteOpen(false);
    item.action();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    const items = filtered();
    if (e.key === 'ArrowDown' || (e.key === 'j' && e.ctrlKey)) {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, items.length - 1));
    } else if (e.key === 'ArrowUp' || (e.key === 'k' && e.ctrlKey)) {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const item = items[selectedIndex()];
      if (item) execute(item);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      keyboard.setIsPaletteOpen(false);
    }
  };

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: backdrop dismiss
    <div
      class="fixed inset-0 z-50 flex justify-center pt-[15vh]"
      onClick={() => keyboard.setIsPaletteOpen(false)}
      onKeyDown={() => {}}
      role="presentation"
    >
      <div
        class="w-full max-w-lg bg-bg-sidebar border border-border-subtle shadow-lg"
        style={{ 'max-height': '400px' }}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={() => {}}
        role="dialog"
        aria-label="Command palette"
      >
        <input
          ref={inputRef}
          type="text"
          placeholder="Type a command..."
          class="w-full h-10 px-4 bg-transparent text-lg text-text-main placeholder:text-text-dim outline-none border-b border-border-subtle"
          value={query()}
          onInput={(e) => {
            setQuery(e.currentTarget.value);
            setSelectedIndex(0);
          }}
          onKeyDown={handleKeyDown}
        />
        <div class="overflow-y-auto" style={{ 'max-height': '352px' }}>
          <For each={filtered()}>
            {(item, i) => (
              <button
                type="button"
                class={cn(
                  'w-full flex items-center justify-between px-4 py-2 text-base text-text-main cursor-pointer',
                  i() === selectedIndex() ? 'bg-surface-raised' : 'hover:bg-surface-raised/50',
                )}
                onClick={() => execute(item)}
                onMouseEnter={() => setSelectedIndex(i())}
              >
                <span>{item.label}</span>
                <Show when={item.shortcut}>
                  <span class="text-sm text-text-dim bg-bg-main px-1.5 py-px">{item.shortcut}</span>
                </Show>
              </button>
            )}
          </For>
          <Show when={filtered().length === 0}>
            <div class="px-4 py-4 text-sm text-text-dim text-center">No matching commands</div>
          </Show>
        </div>
      </div>
    </div>
  );
}
