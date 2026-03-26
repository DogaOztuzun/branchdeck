/** Status dot states — maps to DESIGN.md semantic colors (UX-DR2) */
export type StatusDotStatus = 'error' | 'success' | 'warning' | 'info' | 'inactive';

/** Badge structural variants (UX-DR3) */
export type BadgeStructure = 'filled' | 'outlined' | 'pulsing';

/** Badge semantic color */
export type BadgeColor = 'primary' | 'success' | 'warning' | 'error' | 'info' | 'muted';

/** Action button size (UX-DR8) */
export type ActionButtonSize = 'default' | 'compact';

/** Action button variant (UX-DR8) */
export type ActionButtonVariant = 'primary' | 'secondary';

/** Summary stat item for SummaryStatsBar (UX-DR5) */
export type SummaryStatItem = {
  label: string;
  value: string;
  color?: 'primary' | 'success' | 'warning' | 'error' | 'info';
  /** Optional sparkline data points (rendered as 80x24px inline SVG) */
  sparkline?: { data: number[]; color: string };
};

/** Empty state contextual variant (UX-DR9) */
export type EmptyStateVariant = 'inbox' | 'sat' | 'tasks' | 'generic';
