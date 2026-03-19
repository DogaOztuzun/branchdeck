export type RunStatus =
  | 'created'
  | 'starting'
  | 'running'
  | 'blocked'
  | 'succeeded'
  | 'failed'
  | 'cancelled';

export type RunInfo = {
  sessionId: string | null;
  taskPath: string;
  status: RunStatus;
  startedAt: string;
  costUsd: number;
  lastHeartbeat: string | null;
  elapsedSecs: number;
};

export type RunStepEvent = {
  type: 'run_step';
  step: string;
  detail: string;
  sessionId: string | null;
};

export type AssistantTextEvent = {
  type: 'assistant_text';
  text: string;
  sessionId: string | null;
};

export type ToolCallEvent = {
  type: 'tool_call';
  tool: string;
  filePath: string | null;
  sessionId: string | null;
};

export type RunStatusEvent = {
  type: 'run_complete' | 'run_error';
  status: string;
  error?: string;
  costUsd?: number;
  sessionId: string | null;
};
