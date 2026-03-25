import { For, Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { TaskInfo } from '../../types/task';
import type { StatusDotStatus } from '../../types/ui';
import { InboxBadge } from '../ui/InboxBadge';
import { StatusDot } from '../ui/StatusDot';

type TaskListViewProps = {
  tasks: TaskInfo[];
  selectedIndex: number | null;
};

const statusMap: Record<string, StatusDotStatus> = {
  created: 'inactive',
  running: 'warning',
  blocked: 'error',
  succeeded: 'success',
  failed: 'error',
  cancelled: 'inactive',
};

export function TaskListView(props: TaskListViewProps) {
  return (
    <div class="flex-1 overflow-y-auto">
      <For each={props.tasks}>
        {(task, i) => {
          const fm = task.frontmatter;
          return (
            <div
              class={cn(
                'flex items-center h-7 px-3 gap-2 border-b border-border-subtle',
                props.selectedIndex === i() &&
                  'bg-surface-raised border-l-2 border-l-accent-primary pl-[10px]',
              )}
            >
              <StatusDot status={statusMap[fm.status] ?? 'inactive'} />
              <span class="text-base text-text-main truncate flex-1">
                {fm.branch || 'Untitled'}
              </span>
              <InboxBadge label={fm.type.replace('-', ' ')} structure="outlined" color="muted" />
              <Show when={fm.repo}>
                <span class="text-[11px] text-text-dim">{fm.repo?.split('/').pop()}</span>
              </Show>
              <Show when={fm.status === 'running'}>
                <span class="text-[11px] text-accent-warning animate-pulse-opacity">&gt;_</span>
              </Show>
            </div>
          );
        }}
      </For>
    </div>
  );
}
