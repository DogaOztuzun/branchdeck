import type { JSX } from 'solid-js';
import { Show } from 'solid-js';
import { cn } from '../../lib/cn';

type SidebarItemProps = {
  icon?: (props: { size: number; class?: string }) => JSX.Element;
  label: string;
  active?: boolean;
  onClick?: () => void;
  onContextMenu?: (e: MouseEvent) => void;
  badge?: string | number;
  branch?: string;
  class?: string;
  children?: JSX.Element;
};

export function SidebarItem(props: SidebarItemProps) {
  const iconClass = () =>
    props.active ? 'text-accent-primary' : 'text-text-dim group-hover:text-accent-primary';

  return (
    <button
      type="button"
      onClick={props.onClick}
      onContextMenu={props.onContextMenu}
      class={cn(
        'w-full flex items-center justify-between px-3 py-1 transition-colors duration-150 group cursor-pointer',
        props.active
          ? 'bg-bg-main text-accent-primary'
          : 'text-text-main hover:bg-bg-main hover:text-accent-primary',
        props.class,
      )}
    >
      <div class="flex items-center gap-2 overflow-hidden min-w-0">
        <Show when={props.icon}>
          {(getIcon) => {
            const Icon = getIcon();
            return <Icon size={14} class={iconClass()} />;
          }}
        </Show>
        <span class="text-xs font-medium truncate">{props.label}</span>
        {props.children}
      </div>
      <div class="flex items-center gap-2 shrink-0">
        <Show when={props.branch}>
          <span class="text-[10px] text-accent-info font-mono">{props.branch}</span>
        </Show>
        <Show when={props.badge != null}>
          <span class="text-[10px] bg-bg-sidebar px-1.5 py-0.5 border border-border-subtle font-mono text-text-dim">
            {props.badge}
          </span>
        </Show>
      </div>
    </button>
  );
}
