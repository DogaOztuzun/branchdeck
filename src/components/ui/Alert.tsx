import type { JSX } from 'solid-js';
import { cn } from '../../lib/cn';

type AlertVariant = 'success' | 'warning' | 'error' | 'info';

type AlertProps = {
  variant: AlertVariant;
  class?: string;
  children: JSX.Element;
};

const variantStyles: Record<AlertVariant, string> = {
  success: 'border-l-accent-success text-accent-success',
  warning: 'border-l-accent-warning text-accent-warning',
  error: 'border-l-accent-error text-accent-error',
  info: 'border-l-accent-info text-accent-info',
};

export function Alert(props: AlertProps) {
  return (
    <div
      class={cn(
        'px-3 py-2 text-[11px] border-l-2 bg-bg-sidebar',
        variantStyles[props.variant],
        props.class,
      )}
    >
      {props.children}
    </div>
  );
}
