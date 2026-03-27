export type UpdateStatusKind =
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'error';

export interface UpdateStatusPayload {
  status: UpdateStatusKind;
  version?: string;
  error?: string;
}

export type UpdateStatus = 'none' | 'pending-workflow-completion' | 'ready-to-apply';

export type UpdateStatusSummary = {
  'has-update': boolean;
  status: UpdateStatus;
  version: string | null;
  message: string;
};
