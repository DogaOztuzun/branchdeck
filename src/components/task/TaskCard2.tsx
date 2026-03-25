import { Show } from 'solid-js';
import { cn } from '../../lib/cn';
import type { TaskInfo } from '../../types/task';
import { InboxBadge } from '../ui/InboxBadge';

type TaskCard2Props = {
  task: TaskInfo;
  selected: boolean;
};

const priorityBorder: Record<string, string> = {
  'issue-fix': 'border-l-2 border-l-accent-warning',
  'pr-shepherd': 'border-l-2 border-l-accent-info',
};

export function TaskCard2(props: TaskCard2Props) {
  const fm = () => props.task.frontmatter;
  const isRunning = () => fm().status === 'running';
  const repo = () => fm().repo?.split('/').pop() ?? '';

  return (
    <div
      class={cn(
        'bg-bg-main border border-border-subtle p-2 mb-1 transition-colors duration-150',
        priorityBorder[fm().type] ?? '',
        props.selected ? 'bg-surface-raised border-l-2 border-l-accent-primary' : '',
      )}
    >
      <div class="text-base font-medium text-text-main line-clamp-2">
        {fm().branch || 'Untitled'}
      </div>
      <div class="flex items-center gap-1.5 mt-1 flex-wrap">
        <InboxBadge
          label={fm().type.replace('-', ' ')}
          structure="outlined"
          color={fm().type === 'issue-fix' ? 'warning' : 'info'}
        />
        <Show when={repo()}>
          <InboxBadge label={repo()} structure="outlined" color="info" />
        </Show>
        <Show when={isRunning()}>
          <span class="text-[11px] text-accent-warning animate-pulse-opacity">&gt;_ running</span>
        </Show>
        <Show when={fm().status === 'succeeded'}>
          <InboxBadge label="completed" structure="outlined" color="success" />
        </Show>
        <Show when={fm().pr}>
          <span class="text-[11px] text-accent-info">#{fm().pr}</span>
        </Show>
      </div>
    </div>
  );
}
