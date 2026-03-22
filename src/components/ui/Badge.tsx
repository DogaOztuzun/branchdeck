import type { JSX } from 'solid-js';
import { cn } from '../../lib/cn';

type BadgeVariant = 'success' | 'warning' | 'error' | 'info' | 'neutral';

type BadgeProps = {
  variant: BadgeVariant;
  class?: string;
  children: JSX.Element;
};

const variantStyles: Record<BadgeVariant, string> = {
  success: 'z-badge-variant-success',
  warning: 'z-badge-variant-warning',
  error: 'bg-accent-error/10 text-accent-error',
  info: 'bg-accent-info/10 text-accent-info',
  neutral: 'z-badge-variant-outline',
};

export function Badge(props: BadgeProps) {
  return (
    <span class={cn('z-badge', variantStyles[props.variant], props.class)}>{props.children}</span>
  );
}
