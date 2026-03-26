import type {
  LifecycleEvent,
  LifecycleStatus,
  LifecycleTimelineEntry,
  TriggerSource,
  WorkflowCycle,
  WorkflowType,
} from '../../types/lifecycle';
import { LIFECYCLE_STATUS_LABELS } from '../constants/lifecycle';

/** Infer workflow type from lifecycle context */
export function inferWorkflowType(event: LifecycleEvent): WorkflowType {
  if (event.prKey.includes('#rescore')) return 'verification';
  if (event.prKey.includes('#i')) return 'issue-resolution';
  const segments = event.worktreePath.split('/');
  if (segments.some((s) => s.startsWith('sat-') || s === 'sat')) return 'sat-scoring';
  if (event.status === 'retrying' || event.status === 'fixing') return 'issue-resolution';
  return 'issue-resolution';
}

/** Infer trigger source from lifecycle event */
export function inferTriggerSource(event: LifecycleEvent): TriggerSource {
  if (event.prKey.includes('#rescore')) return 'post-merge';
  if (event.prKey.includes('#i')) return 'issue-detected';
  if (event.attempt > 1) return 'retry';
  return 'pr-poll';
}

/**
 * Resolve the status label for display.
 * Custom displayStatus from workflow definition takes precedence over hardcoded labels.
 */
export function resolveStatusLabel(status: LifecycleStatus, displayStatus?: string): string {
  if (displayStatus) return displayStatus;
  return LIFECYCLE_STATUS_LABELS[status] ?? status;
}

/**
 * Build a LifecycleTimelineEntry from a lifecycle event.
 * Used by the store when recording transitions.
 */
export function buildTimelineEntry(event: LifecycleEvent): LifecycleTimelineEntry {
  return {
    timestamp: Date.now(),
    status: event.status,
    displayStatus: event.displayStatus ?? event.status,
    detail: `${event.displayStatus ?? event.status} (attempt ${event.attempt})`,
  };
}

/** Convert a LifecycleEvent to a WorkflowCycle for display */
export function toCycle(event: LifecycleEvent, timeline: LifecycleTimelineEntry[]): WorkflowCycle {
  return {
    id: `${event.prKey}-${event.attempt}`,
    prKey: event.prKey,
    workflowType: inferWorkflowType(event),
    triggerSource: inferTriggerSource(event),
    status: event.status,
    attempt: event.attempt,
    startedAt: event.startedAt,
    updatedAt: event.startedAt,
    completedAt: event.completedAt ?? null,
    worktreePath: event.worktreePath,
    description: event.worktreePath.split('/').pop() ?? event.prKey,
    workflowName: event.workflowName,
    displayStatus: event.displayStatus,
    timeline,
  };
}
