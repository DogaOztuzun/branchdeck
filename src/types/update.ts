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
