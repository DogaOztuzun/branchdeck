import { For, onCleanup, onMount, Show } from 'solid-js';

export type ContextMenuItem = {
  label: string;
  onClick: () => void;
  variant?: 'default' | 'danger';
};

type ContextMenuProps = {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
};

export function ContextMenu(props: ContextMenuProps) {
  let menuRef: HTMLDivElement | undefined;

  function handleClickOutside(e: MouseEvent) {
    if (menuRef && !menuRef.contains(e.target as Node)) {
      props.onClose();
    }
  }

  onMount(() => {
    document.addEventListener('mousedown', handleClickOutside);
  });

  onCleanup(() => {
    document.removeEventListener('mousedown', handleClickOutside);
  });

  return (
    <Show when={props.items.length > 0}>
      <div
        ref={menuRef}
        class="fixed z-50 bg-surface border border-border rounded shadow-lg py-1 min-w-40"
        style={{ left: `${props.x}px`, top: `${props.y}px` }}
      >
        <For each={props.items}>
          {(item) => (
            <button
              type="button"
              class={`block w-full text-left px-3 py-1.5 text-xs cursor-pointer ${
                item.variant === 'danger'
                  ? 'text-error hover:bg-error/10'
                  : 'text-text hover:bg-bg/50'
              }`}
              onClick={() => {
                item.onClick();
                props.onClose();
              }}
            >
              {item.label}
            </button>
          )}
        </For>
      </div>
    </Show>
  );
}
