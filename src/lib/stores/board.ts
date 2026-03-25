import { createSignal } from 'solid-js';
import type { TaskInfo, TaskStatus } from '../../types/task';
import { getKeyboardStore } from './keyboard';

export type BoardViewMode = 'board' | 'list';

export type BoardColumn = {
  label: string;
  statuses: TaskStatus[];
  color: 'muted' | 'warning' | 'primary' | 'success';
};

const columns: BoardColumn[] = [
  { label: 'Backlog', statuses: ['created'], color: 'muted' },
  { label: 'In Progress', statuses: ['running', 'blocked'], color: 'warning' },
  { label: 'Review', statuses: ['succeeded'], color: 'primary' },
  { label: 'Done', statuses: ['failed', 'cancelled'], color: 'success' },
];

const [viewMode, setViewMode] = createSignal<BoardViewMode>('board');
const [selectedColumn, setSelectedColumn] = createSignal(0);
const [selectedCard, setSelectedCard] = createSignal(0);

function groupTasks(tasks: TaskInfo[]): TaskInfo[][] {
  return columns.map((col) => tasks.filter((t) => col.statuses.includes(t.frontmatter.status)));
}

function moveLeft() {
  setSelectedColumn((c) => Math.max(0, c - 1));
  setSelectedCard(0);
}

function moveRight() {
  setSelectedColumn((c) => Math.min(columns.length - 1, c + 1));
  setSelectedCard(0);
}

function moveUp() {
  setSelectedCard((c) => Math.max(0, c - 1));
}

function moveDown(columnLength: number) {
  setSelectedCard((c) => Math.min(columnLength - 1, c + 1));
}

function registerBoardShortcuts(getColumnLength: () => number) {
  const keyboard = getKeyboardStore();
  keyboard.registerShortcut({
    key: 'h',
    handler: moveLeft,
    label: 'Previous column',
    context: 'tasks',
    category: 'Task Actions',
  });
  keyboard.registerShortcut({
    key: 'l',
    handler: moveRight,
    label: 'Next column',
    context: 'tasks',
    category: 'Task Actions',
  });
  keyboard.registerShortcut({
    key: 'k',
    handler: moveUp,
    label: 'Previous card',
    context: 'tasks',
    category: 'Task Actions',
  });
  keyboard.registerShortcut({
    key: 'j',
    handler: () => moveDown(getColumnLength()),
    label: 'Next card',
    context: 'tasks',
    category: 'Task Actions',
  });
  keyboard.registerShortcut({
    key: 'b',
    handler: () => setViewMode('board'),
    label: 'Board view',
    context: 'tasks',
    category: 'Task Actions',
  });
  keyboard.registerShortcut({
    key: 't',
    handler: () => setViewMode('list'),
    label: 'List view',
    context: 'tasks',
    category: 'Task Actions',
  });
}

export function getBoardStore() {
  return {
    columns,
    viewMode,
    setViewMode,
    selectedColumn,
    setSelectedColumn,
    selectedCard,
    setSelectedCard,
    groupTasks,
    registerBoardShortcuts,
  };
}
