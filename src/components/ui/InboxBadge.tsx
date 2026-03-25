import { cn } from '../../lib/cn';
import type { BadgeColor, BadgeStructure } from '../../types/ui';

type InboxBadgeProps = {
  label: string;
  structure?: BadgeStructure;
  color?: BadgeColor;
  class?: string;
};

const colorMap: Record<BadgeColor, { filled: string; outlined: string }> = {
  primary: {
    filled: 'bg-accent-primary text-bg-main',
    outlined: 'border-accent-primary text-accent-primary',
  },
  success: {
    filled: 'bg-accent-success text-bg-main',
    outlined: 'border-accent-success text-accent-success',
  },
  warning: {
    filled: 'bg-accent-warning text-bg-main',
    outlined: 'border-accent-warning text-accent-warning',
  },
  error: {
    filled: 'bg-accent-error text-bg-main',
    outlined: 'border-accent-error text-accent-error',
  },
  info: {
    filled: 'bg-accent-info text-bg-main',
    outlined: 'border-accent-info text-accent-info',
  },
  muted: {
    filled: 'bg-text-dim text-bg-main',
    outlined: 'border-text-dim text-text-dim',
  },
};

export function InboxBadge(props: InboxBadgeProps) {
  const structure = () => props.structure ?? 'outlined';
  const color = () => props.color ?? 'primary';

  const styles = () => {
    const c = colorMap[color()];
    const s = structure();
    if (s === 'filled') return c.filled;
    return cn(c.outlined, 'border bg-transparent', s === 'pulsing' && 'animate-pulse-opacity');
  };

  return (
    <span
      class={cn(
        'inline-flex items-center px-1.5 py-px text-xs font-medium uppercase tracking-wide',
        styles(),
        props.class,
      )}
    >
      {props.label}
    </span>
  );
}
