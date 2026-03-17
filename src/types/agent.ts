export type EpochMs = number;

export type AgentStatus = 'active' | 'idle' | 'stopped';

export type AgentEvent =
  | { kind: 'sessionStart'; sessionId: string; tabId: string; model: string | null; ts: EpochMs }
  | {
      kind: 'toolStart';
      sessionId: string;
      agentId: string | null;
      tabId: string;
      toolName: string;
      toolUseId: string;
      filePath: string | null;
      ts: EpochMs;
    }
  | {
      kind: 'toolEnd';
      sessionId: string;
      agentId: string | null;
      tabId: string;
      toolName: string;
      toolUseId: string;
      filePath: string | null;
      ts: EpochMs;
    }
  | {
      kind: 'subagentStart';
      sessionId: string;
      agentId: string;
      agentType: string;
      tabId: string;
      ts: EpochMs;
    }
  | {
      kind: 'subagentStop';
      sessionId: string;
      agentId: string;
      agentType: string;
      tabId: string;
      ts: EpochMs;
    }
  | { kind: 'sessionStop'; sessionId: string; tabId: string; ts: EpochMs }
  | {
      kind: 'notification';
      sessionId: string;
      tabId: string;
      title: string | null;
      message: string;
      ts: EpochMs;
    };

export type AgentState = {
  sessionId: string;
  agentId: string | null;
  agentType: string | null;
  tabId: string;
  status: AgentStatus;
  currentTool: string | null;
  currentFile: string | null;
  startedAt: EpochMs;
  lastActivity: EpochMs;
};

export type FileAccess = {
  path: string;
  lastTool: string;
  lastAgent: string;
  lastAccess: EpochMs;
  accessCount: number;
  wasModified: boolean;
};

export type AgentDefinition = {
  name: string;
  description: string;
  model: string | null;
  tools: string[];
  permissionMode: string | null;
  filePath: string;
};
