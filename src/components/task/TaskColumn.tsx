import { For, Show } from 'solid-js';
import type { BoardColumn } from '../../lib/stores/board';
import type { TaskInfo } from '../../types/task';
import { SectionHeader } from '../ui/SectionHeader';
import { TaskCard2 } from './TaskCard2';

type TaskColumnProps = {
  column: BoardColumn;
  tasks: TaskInfo[];
  selectedCard: number | null;
};

export function TaskColumn(props: TaskColumnProps) {
  return (
    <div class="w-[280px] shrink-0 bg-bg-sidebar border-r border-border-subtle flex flex-col h-full">
      <div class="px-2.5 py-2">
        <SectionHeader
          label={props.column.label}
          count={props.tasks.length}
          class={
            props.column.color === 'warning'
              ? 'text-accent-warning'
              : props.column.color === 'success'
                ? 'text-accent-success'
                : props.column.color === 'primary'
                  ? 'text-accent-primary'
                  : 'text-text-dim'
          }
        />
      </div>
      <div class="flex-1 overflow-y-auto px-2 pb-2">
        <For each={props.tasks}>
          {(task, i) => <TaskCard2 task={task} selected={props.selectedCard === i()} />}
        </For>
        <Show when={props.tasks.length === 0}>
          <div class="text-[11px] text-text-dim text-center py-4">No tasks</div>
        </Show>
      </div>
    </div>
  );
}
