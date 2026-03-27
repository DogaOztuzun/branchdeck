import { createSignal } from 'solid-js';
import type {
  ProjectSetupConfig,
  SetupStatus,
  SetupStep,
  TokenSource,
  TokenValidation,
  WorkflowOption,
} from '../../types/setup';
import { apiGet, apiPost } from '../api/client';

const STEPS: SetupStep[] = ['repo', 'workflows', 'tokens', 'review'];

/** Map a validated token source string to the correct TokenSource shape. */
function mapTokenSource(available?: boolean, source?: string): TokenSource {
  if (!available || !source) return { type: 'none' };
  if (source === 'gh-cli') return { type: 'gh-cli' };
  if (source.startsWith('env:')) return { type: 'env-var', name: source.slice(4) };
  return { type: 'none' };
}

function createSetupStore() {
  const [currentStep, setCurrentStep] = createSignal<SetupStep>('repo');
  const [repoPath, setRepoPath] = createSignal('');
  const [enabledWorkflows, setEnabledWorkflows] = createSignal<string[]>([]);
  const [availableWorkflows, setAvailableWorkflows] = createSignal<WorkflowOption[]>([]);
  const [tokenValidation, setTokenValidation] = createSignal<TokenValidation | null>(null);
  const [isValidating, setIsValidating] = createSignal(false);
  const [isSaving, setIsSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [setupComplete, setSetupComplete] = createSignal(false);
  const [needsSetup, setNeedsSetup] = createSignal(false);

  function stepIndex(): number {
    return STEPS.indexOf(currentStep());
  }

  function canGoNext(): boolean {
    const step = currentStep();
    if (step === 'repo') return repoPath().trim().length > 0;
    if (step === 'workflows') return true;
    if (step === 'tokens') return tokenValidation() !== null;
    if (step === 'review') return true;
    return false;
  }

  function canGoBack(): boolean {
    return stepIndex() > 0;
  }

  function goNext() {
    const idx = stepIndex();
    if (idx < STEPS.length - 1) {
      setCurrentStep(STEPS[idx + 1]);
      setError(null);
    }
  }

  function goBack() {
    const idx = stepIndex();
    if (idx > 0) {
      setCurrentStep(STEPS[idx - 1]);
      setError(null);
    }
  }

  function toggleWorkflow(name: string) {
    const current = enabledWorkflows();
    if (current.includes(name)) {
      setEnabledWorkflows(current.filter((w) => w !== name));
    } else {
      setEnabledWorkflows([...current, name]);
    }
  }

  async function checkSetupStatus(path: string) {
    try {
      const status = await apiGet<SetupStatus>(
        `/setup/status?repo_path=${encodeURIComponent(path)}`,
      );
      if (status.configured && status.config) {
        // Pre-fill from existing config for re-run
        setRepoPath(status.config['repo-path']);
        setEnabledWorkflows(status.config['enabled-workflows']);
        setNeedsSetup(false);
      } else {
        setNeedsSetup(true);
      }
      return status;
    } catch {
      setNeedsSetup(true);
      return null;
    }
  }

  async function loadWorkflows() {
    const path = repoPath();
    if (!path) return;
    try {
      const workflows = await apiGet<WorkflowOption[]>(
        `/setup/workflows?repo_path=${encodeURIComponent(path)}`,
      );
      setAvailableWorkflows(workflows);
      // Select all by default
      if (enabledWorkflows().length === 0) {
        setEnabledWorkflows(workflows.map((w) => w.name));
      }
    } catch {
      setAvailableWorkflows([]);
    }
  }

  async function validateTokens() {
    setIsValidating(true);
    setError(null);
    try {
      const result = await apiGet<TokenValidation>('/setup/validate');
      setTokenValidation(result);
    } catch (e) {
      setError(`Failed to validate tokens: ${e}`);
    } finally {
      setIsValidating(false);
    }
  }

  async function saveConfig(): Promise<boolean> {
    setIsSaving(true);
    setError(null);
    try {
      const tv = tokenValidation();
      const config: ProjectSetupConfig = {
        'repo-path': repoPath(),
        'github-token-source': mapTokenSource(tv?.['github-available'], tv?.['github-source']),
        'anthropic-key-source': mapTokenSource(
          tv?.['anthropic-available'],
          tv?.['anthropic-source'],
        ),
        'enabled-workflows': enabledWorkflows(),
        'min-severity': 'high',
        'confidence-threshold': 70,
      };
      await apiPost('/setup/save', config);
      setSetupComplete(true);
      setNeedsSetup(false);
      return true;
    } catch (e) {
      setError(`Failed to save configuration: ${e}`);
      return false;
    } finally {
      setIsSaving(false);
    }
  }

  function reset() {
    setCurrentStep('repo');
    setRepoPath('');
    setEnabledWorkflows([]);
    setAvailableWorkflows([]);
    setTokenValidation(null);
    setError(null);
    setSetupComplete(false);
    setIsValidating(false);
    setIsSaving(false);
  }

  /** Start setup for a given repo (re-run or first-time). */
  function startSetup(path: string) {
    reset();
    setRepoPath(path);
    setNeedsSetup(true);
  }

  return {
    currentStep,
    repoPath,
    setRepoPath,
    enabledWorkflows,
    availableWorkflows,
    tokenValidation,
    isValidating,
    isSaving,
    error,
    setupComplete,
    needsSetup,
    canGoNext,
    canGoBack,
    goNext,
    goBack,
    toggleWorkflow,
    checkSetupStatus,
    loadWorkflows,
    validateTokens,
    saveConfig,
    reset,
    startSetup,
  };
}

let store: ReturnType<typeof createSetupStore> | undefined;

export function getSetupStore() {
  if (!store) store = createSetupStore();
  return store;
}
