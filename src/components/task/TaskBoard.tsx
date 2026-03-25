import { createMemo, For, Match, onMount, Show, Switch } from 'solid-js';
import { getBoardStore } from '../../lib/stores/board';
import { getTaskStore } from '../../lib/stores/task';
import type { TaskInfo } from '../../types/task';
import { ActionButton } from '../ui/ActionButton';
import { EmptyState } from '../ui/EmptyState';
import { TaskColumn } from './TaskColumn';
import { TaskListView } from './TaskListView';

export function TaskBoard() {
  const taskStore = getTaskStore();
  const board = getBoardStore();

  const allTasks = createMemo((): TaskInfo[] => Object.values(taskStore.state.tasksByWorktree));
  const grouped = createMemo(() => board.groupTasks(allTasks()));

  onMount(() => {
    const getColLength = () => {
      const g = grouped();
      const col = board.selectedColumn();
      return g[col]?.length ?? 0;
    };
    board.registerBoardShortcuts(getColLength);
  });

  const totalTasks = () => allTasks().length;
  const activeAgents = () => allTasks().filter((t) => t.frontmatter.status === 'running').length;

  return (
    <div class="flex-1 flex flex-col overflow-hidden">
      {/* Header */}
      <div class="flex items-center h-9 px-3 gap-3 border-b border-border-subtle shrink-0">
        <span class="text-lg font-semibold text-text-main">Tasks</span>
        <div class="flex items-center gap-1">
          <ActionButton
            label="Board"
            variant={board.viewMode() === 'board' ? 'primary' : 'secondary'}
            size="compact"
            shortcutHint="b"
            onClick={() => board.setViewMode('board')}
          />
          <ActionButton
            label="List"
            variant={board.viewMode() === 'list' ? 'primary' : 'secondary'}
            size="compact"
            shortcutHint="t"
            onClick={() => board.setViewMode('list')}
          />
        </div>
        <span class="text-[11px] text-text-dim ml-auto">
          {totalTasks()} tasks
          <Show when={activeAgents() > 0}>
            <span class="text-accent-warning animate-pulse-opacity ml-1">
              · {activeAgents()} active agents
            </span>
          </Show>
        </span>
      </div>

      {/* Content */}
      <Show
        when={totalTasks() > 0}
        fallback={
          <EmptyState
            message="Nothing in progress"
            detail="Tasks appear when workflows run or you create them manually"
          />
        }
      >
        <Switch>
          <Match when={board.viewMode() === 'board'}>
            <div class="flex-1 flex overflow-x-auto">
              <For each={board.columns}>
                {(column, colIdx) => (
                  <TaskColumn
                    column={column}
                    tasks={grouped()[colIdx()] ?? []}
                    selectedCard={board.selectedColumn() === colIdx() ? board.selectedCard() : null}
                  />
                )}
              </For>
            </div>
          </Match>
          <Match when={board.viewMode() === 'list'}>
            <TaskListView tasks={allTasks()} selectedIndex={board.selectedCard()} />
          </Match>
        </Switch>
      </Show>
    </div>
  );
}
