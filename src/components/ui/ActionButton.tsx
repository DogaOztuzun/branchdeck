import { Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { ActionButtonSize, ActionButtonVariant } from '../../types/ui';

type ActionButtonProps = {
  label: string;
  variant?: ActionButtonVariant;
  size?: ActionButtonSize;
  shortcutHint?: string;
  disabled?: boolean;
  onClick?: () => void;
  class?: string;
  type?: 'button' | 'submit';
};

const variantStyles: Record<ActionButtonVariant, string> = {
  primary:
    'border border-accent-primary text-accent-primary hover:bg-accent-primary hover:text-bg-main',
  secondary: 'border border-text-dim text-text-dim hover:text-text-main hover:border-text-main',
};

const sizeStyles: Record<ActionButtonSize, string> = {
  default: 'h-8 px-3 text-sm font-medium',
  compact: 'h-6 px-2 text-[11px] font-medium',
};

export function ActionButton(props: ActionButtonProps) {
  return (
    <button
      type={props.type ?? 'button'}
      disabled={props.disabled}
      onClick={() => props.onClick?.()}
      class={cn(
        'inline-flex items-center gap-1 transition-colors duration-150 cursor-pointer',
        'disabled:opacity-50 disabled:pointer-events-none',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-primary',
        variantStyles[props.variant ?? 'secondary'],
        sizeStyles[props.size ?? 'default'],
        props.class,
      )}
      aria-disabled={props.disabled}
    >
      {props.label}
      <Show when={props.shortcutHint}>
        <span class="text-xs text-text-dim">{props.shortcutHint}</span>
      </Show>
    </button>
  );
}
