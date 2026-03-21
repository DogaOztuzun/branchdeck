import type { JSX } from 'solid-js';
import { cn } from '../../lib/cn';

type BadgeVariant = 'success' | 'warning' | 'error' | 'info' | 'neutral';

type BadgeProps = {
  variant: BadgeVariant;
  class?: string;
  children: JSX.Element;
};

const variantStyles: Record<BadgeVariant, string> = {
  success: 'bg-accent-success/15 text-accent-success',
  warning: 'bg-accent-warning/15 text-accent-warning',
  error: 'bg-accent-error/15 text-accent-error',
  info: 'bg-accent-info/15 text-accent-info',
  neutral: 'bg-bg-sidebar text-text-dim border border-border-subtle',
};

export function Badge(props: BadgeProps) {
  return (
    <span
      class={cn(
        'inline-block text-[10px] font-medium px-2 py-0.5',
        variantStyles[props.variant],
        props.class,
      )}
    >
      {props.children}
    </span>
  );
}
