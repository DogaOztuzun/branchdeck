export type EventType = 'sat' | 'orchestrator' | 'agent' | 'pr' | 'ci';

export type ActivityEvent = {
  id: string;
  type: EventType;
  /** Unix ms timestamp */
  timestamp: number;
  description: string;
  entityLink?: string;
  detail?: Record<string, string>;
};

export type TimeRange = '8h' | '24h' | 'all';

export type EventFilter = {
  activeTypes: Set<EventType>;
  timeRange: TimeRange;
};
