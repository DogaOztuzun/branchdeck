/** App view names for navigation */
export type ViewName = 'workspace' | 'inbox' | 'sat' | 'tasks' | 'lifecycle';

/** A registered keyboard shortcut */
export type ShortcutAction = {
  key: string;
  handler: () => void;
  label: string;
  /** Which view this shortcut is active in, or 'global' for everywhere */
  context: ViewName | 'global';
  /** Category for command palette and shortcut overlay grouping */
  category: 'Navigation' | 'Inbox Actions' | 'SAT Actions' | 'Task Actions' | 'General';
};

/** An item in the command palette */
export type CommandItem = {
  label: string;
  category: string;
  action: () => void;
  shortcut?: string;
};
