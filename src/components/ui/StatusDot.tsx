import { cn } from '../../lib/cn';
import type { StatusDotStatus } from '../../types/ui';

type StatusDotProps = {
  status: StatusDotStatus;
  class?: string;
};

const statusColors: Record<StatusDotStatus, string> = {
  error: 'bg-accent-error',
  success: 'bg-accent-success',
  warning: 'bg-accent-warning',
  info: 'bg-accent-info',
  inactive: 'bg-text-dim opacity-40',
};

export function StatusDot(props: StatusDotProps) {
  const colorClass = () => statusColors[props.status] ?? statusColors.inactive;

  return (
    <div
      class={cn('size-2 shrink-0', colorClass(), props.class)}
      role="status"
      aria-label={props.status}
    />
  );
}
