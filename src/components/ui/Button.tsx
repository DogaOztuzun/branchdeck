import type { JSX } from 'solid-js';
import { cn } from '../../lib/cn';

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
type ButtonSize = 'default' | 'compact';

type ButtonProps = {
  variant?: ButtonVariant;
  size?: ButtonSize;
  class?: string;
  children: JSX.Element;
} & Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, 'class'>;

const variantStyles: Record<ButtonVariant, string> = {
  primary: 'bg-accent-primary text-bg-main hover:opacity-90',
  secondary: 'bg-bg-sidebar text-text-main border border-border-subtle hover:bg-surface-raised',
  ghost:
    'bg-transparent text-text-dim border border-border-subtle hover:text-text-main hover:bg-bg-main/50',
  danger: 'bg-transparent text-accent-error border border-accent-error/30 hover:bg-accent-error/10',
};

const sizeStyles: Record<ButtonSize, string> = {
  default: 'h-8 px-4 text-xs',
  compact: 'h-6 px-3 text-[10px]',
};

export function Button(props: ButtonProps) {
  return (
    <button
      {...props}
      type={props.type ?? 'button'}
      class={cn(
        'inline-flex items-center justify-center font-medium transition-colors duration-150 cursor-pointer select-none',
        variantStyles[props.variant ?? 'secondary'],
        sizeStyles[props.size ?? 'default'],
        props.class,
      )}
    >
      {props.children}
    </button>
  );
}
