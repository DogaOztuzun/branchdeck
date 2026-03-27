export type SetupStep = 'repo' | 'workflows' | 'tokens' | 'review' | 'complete';

export type TokenSourceKind = 'env-var' | 'gh-cli' | 'none';

export interface TokenSource {
  type: TokenSourceKind;
  name?: string;
}

export type Severity = 'critical' | 'high' | 'medium' | 'low';

export interface ProjectSetupConfig {
  'repo-path': string;
  'github-token-source': TokenSource;
  'anthropic-key-source': TokenSource;
  'enabled-workflows': string[];
  'min-severity': Severity;
  'confidence-threshold': number;
}

export interface TokenValidation {
  'github-available': boolean;
  'github-source': string;
  'anthropic-available': boolean;
  'anthropic-source': string;
}

export interface WorkflowOption {
  name: string;
  description: string;
}

export interface SetupStatus {
  configured: boolean;
  'config-path': string;
  config?: ProjectSetupConfig;
}
