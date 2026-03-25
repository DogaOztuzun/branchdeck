import { Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { ActivityEvent, EventType } from '../../types/activity';
import type { BadgeColor } from '../../types/ui';
import { InboxBadge } from '../ui/InboxBadge';
import { EventDetail } from './EventDetail';

type EventRowProps = {
  event: ActivityEvent;
  selected: boolean;
  expanded: boolean;
  onClick: () => void;
};

const typeColor: Record<EventType, BadgeColor> = {
  sat: 'warning',
  orchestrator: 'primary',
  agent: 'info',
  pr: 'success',
  ci: 'muted',
};

function formatTime(timestamp: number): string {
  const d = new Date(timestamp);
  return d
    .toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true })
    .toLowerCase();
}

export function EventRow(props: EventRowProps) {
  return (
    <div>
      {/* biome-ignore lint/a11y/noStaticElementInteractions: event row click */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: keyboard handled by store */}
      <div
        class={cn(
          'flex items-center min-h-7 px-3 gap-2 cursor-pointer border-b border-border-subtle transition-colors duration-150',
          props.selected
            ? 'bg-surface-raised border-l-2 border-l-accent-primary pl-[10px]'
            : 'hover:bg-surface-raised/50',
        )}
        onClick={props.onClick}
      >
        <span class="text-[11px] text-text-dim w-16 shrink-0 tabular-nums">
          {formatTime(props.event.timestamp)}
        </span>
        <InboxBadge
          label={props.event.type.toUpperCase()}
          structure="outlined"
          color={typeColor[props.event.type]}
        />
        <span class="text-base text-text-main truncate flex-1">{props.event.description}</span>
        <Show when={props.event.entityLink}>
          <span class="text-[11px] text-accent-info shrink-0">{props.event.entityLink}</span>
        </Show>
      </div>

      <Show when={props.expanded}>
        <EventDetail event={props.event} />
      </Show>
    </div>
  );
}
