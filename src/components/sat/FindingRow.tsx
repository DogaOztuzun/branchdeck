import { Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { SATFinding } from '../../types/sat';
import type { BadgeColor } from '../../types/ui';
import { InboxBadge } from '../ui/InboxBadge';

type FindingRowProps = {
  finding: SATFinding;
  selected: boolean;
  expanded: boolean;
  onClick: () => void;
};

const severityColor: Record<string, BadgeColor> = {
  critical: 'error',
  high: 'warning',
  medium: 'primary',
  low: 'muted',
};

const categoryColor: Record<string, BadgeColor> = {
  app: 'primary',
  runner: 'muted',
  scenario: 'muted',
};

const statusColor: Record<string, BadgeColor> = {
  open: 'warning',
  'issue-created': 'info',
  fixed: 'success',
  'false-positive': 'muted',
};

export function FindingRow(props: FindingRowProps) {
  const isFalsePositive = () => props.finding.status === 'false-positive';

  return (
    <div>
      {/* biome-ignore lint/a11y/noStaticElementInteractions: finding row click */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: keyboard handled by store */}
      <div
        class={cn(
          'flex items-center h-9 px-3 gap-2 cursor-pointer border-b border-border-subtle transition-colors duration-150',
          props.selected
            ? 'bg-surface-raised border-l-2 border-l-accent-primary pl-[10px]'
            : 'hover:bg-surface-raised/50',
          isFalsePositive() && 'opacity-40',
        )}
        onClick={props.onClick}
      >
        <InboxBadge
          label={props.finding.severity}
          structure="filled"
          color={severityColor[props.finding.severity] ?? 'muted'}
        />
        <InboxBadge
          label={props.finding.category}
          structure="outlined"
          color={categoryColor[props.finding.category] ?? 'muted'}
        />
        <span class="text-base text-text-main truncate flex-1">{props.finding.title}</span>
        <span class="text-[11px] text-text-dim shrink-0">{props.finding.persona}</span>
        <span class="text-[11px] text-text-dim shrink-0">C{props.finding.cycle}</span>
        <InboxBadge
          label={props.finding.status.replace('-', ' ')}
          structure="outlined"
          color={statusColor[props.finding.status] ?? 'muted'}
        />
      </div>

      <Show when={props.expanded}>
        <div class="bg-surface-raised px-3 py-2 border-b border-border-subtle">
          <div class="flex items-center gap-3 text-[11px]">
            <span class="text-text-dim">Confidence: {props.finding.confidence}%</span>
            <span class="text-text-dim">Persona: {props.finding.persona}</span>
            <span class="text-text-dim">Cycle: {props.finding.cycle}</span>
          </div>
          <Show when={props.finding.evidence}>
            <p class="mt-1 text-sm text-text-dim">{props.finding.evidence}</p>
          </Show>
        </div>
      </Show>
    </div>
  );
}
