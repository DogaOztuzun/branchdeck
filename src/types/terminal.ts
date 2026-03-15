export type PtyEvent =
  | { event: 'output'; data: { bytes: number[] } }
  | { event: 'exit'; data: { code: number | null } };

export type TabInfo = {
  id: string;
  sessionId: string;
  title: string;
  type: 'shell' | 'claude';
  worktreePath: string;
};
