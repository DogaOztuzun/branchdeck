export type InboxItemType = 'pr' | 'issue' | 'sat-finding';

export type InboxItemStatus = 'needs-attention' | 'ready-to-merge' | 'completed';

export type InboxSource = 'sat' | 'manual' | 'issue';

export type CiStatus = 'passing' | 'failing' | 'pending';

export type InboxItem = {
  id: string;
  type: InboxItemType;
  title: string;
  /** Display identifier like "#42" or "ISS-15" */
  identifier: string;
  branch?: string;
  status: InboxItemStatus;
  source: InboxSource;
  repo: string;
  /** Unix ms timestamp */
  timestamp: number;
  satDelta?: number;
  persona?: string;
  ciStatus?: CiStatus;
  filesChanged?: number;
  agentDuration?: string;
  /** True when SAT found > agent fixed > SAT verified improvement */
  loopComplete?: boolean;
};

export type InboxGroup = {
  label: string;
  status: InboxItemStatus;
  color: 'error' | 'success' | 'muted';
  items: InboxItem[];
};
