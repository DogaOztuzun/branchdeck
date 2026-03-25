import { Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { InboxItem } from '../../types/inbox';
import type { StatusDotStatus } from '../../types/ui';
import { InboxBadge } from '../ui/InboxBadge';
import { StatusDot } from '../ui/StatusDot';
import { InboxRowDetail } from './InboxRowDetail';

type InboxRowProps = {
  item: InboxItem;
  selected: boolean;
  expanded: boolean;
  showRepo: boolean;
  onClick: () => void;
  onMerge: () => void;
  onDismiss: () => void;
};

function mapStatusDot(item: InboxItem): StatusDotStatus {
  if (item.status === 'completed') return 'success';
  if (item.ciStatus === 'failing') return 'error';
  if (item.ciStatus === 'pending') return 'warning';
  if (item.ciStatus === 'passing') return 'success';
  if (item.status === 'needs-attention') return 'warning';
  return 'inactive';
}

function sourceColor(source: string): 'warning' | 'muted' | 'info' {
  if (source === 'sat') return 'warning';
  if (source === 'issue') return 'info';
  return 'muted';
}

function relativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const hours = Math.floor(diff / (1000 * 60 * 60));
  if (hours < 1) return `${Math.floor(diff / (1000 * 60))}m ago`;
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

export function InboxRow(props: InboxRowProps) {
  return (
    <div>
      {/* Collapsed row — 36px */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: keyboard handled by store */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: inbox row click */}
      <div
        class={cn(
          'flex items-center h-9 px-3 gap-2 cursor-pointer border-b border-border-subtle transition-colors duration-150',
          props.selected
            ? 'bg-surface-raised border-l-2 border-l-accent-primary pl-[10px]'
            : 'hover:bg-surface-raised/50',
        )}
        onClick={props.onClick}
      >
        <StatusDot status={mapStatusDot(props.item)} />
        <span class="text-base font-medium text-accent-primary shrink-0">
          {props.item.identifier}
        </span>
        <span class="text-base text-text-main truncate flex-1">{props.item.title}</span>
        <InboxBadge
          label={props.item.source.toUpperCase()}
          structure="outlined"
          color={sourceColor(props.item.source)}
        />
        <Show when={props.showRepo}>
          <span class="text-[11px] text-text-dim shrink-0">{props.item.repo}</span>
        </Show>
        <span class="text-[11px] text-text-dim shrink-0">{relativeTime(props.item.timestamp)}</span>
      </div>

      {/* Expanded detail */}
      <Show when={props.expanded}>
        <InboxRowDetail item={props.item} onMerge={props.onMerge} onDismiss={props.onDismiss} />
      </Show>
    </div>
  );
}
