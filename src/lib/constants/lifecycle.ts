import type { LifecycleStatus } from '../../types/lifecycle';

export const LIFECYCLE_STATUS_LABELS: Record<LifecycleStatus, string> = {
  running: 'Analyzing',
  reviewReady: 'Review Ready',
  approved: 'Approved',
  fixing: 'Fixing',
  completed: 'Completed',
  retrying: 'Retrying',
  stale: 'Stale — CI now passing',
  failed: 'Failed — retries exhausted',
};

export const LIFECYCLE_STATUS_COLORS: Record<LifecycleStatus, string> = {
  running: 'text-[var(--color-warning)]',
  reviewReady: 'text-accent-primary',
  approved: 'text-[var(--color-info)]',
  fixing: 'text-[var(--color-warning)]',
  completed: 'text-[var(--color-success)]',
  retrying: 'text-[var(--color-error)]',
  stale: 'text-[var(--color-muted)]',
  failed: 'text-[var(--color-error)]',
};

export const LIFECYCLE_STATUS_ORDER: Record<LifecycleStatus, number> = {
  stale: 0,
  reviewReady: 1,
  running: 2,
  fixing: 3,
  retrying: 4,
  failed: 5,
  approved: 6,
  completed: 7,
};
