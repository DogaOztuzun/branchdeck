export type UpdateStatus = 'none' | 'pending-workflow-completion' | 'ready-to-apply';

export type UpdateStatusSummary = {
  'has-update': boolean;
  status: UpdateStatus;
  version: string | null;
  message: string;
};
