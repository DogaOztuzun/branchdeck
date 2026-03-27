import { createEffect, For, Match, on, Show, Switch } from 'solid-js';
import { cn } from '../../lib/cn';
import { getSetupStore } from '../../lib/stores/setup';
import type { SetupStep } from '../../types/setup';
import { Button } from '../ui/Button';

const STEP_LABELS: Record<SetupStep, string> = {
  repo: 'Repository',
  workflows: 'Workflows',
  tokens: 'Credentials',
  review: 'Review',
  complete: 'Complete',
};

const VISIBLE_STEPS: SetupStep[] = ['repo', 'workflows', 'tokens', 'review'];

function StepIndicator() {
  const setup = getSetupStore();

  return (
    <div class="flex items-center gap-1 mb-6">
      <For each={VISIBLE_STEPS}>
        {(step, i) => {
          const isActive = () => setup.currentStep() === step;
          const isPast = () => VISIBLE_STEPS.indexOf(setup.currentStep()) > i();
          return (
            <>
              <Show when={i() > 0}>
                <div
                  class={cn('flex-1 h-px', isPast() ? 'bg-accent-primary' : 'bg-border-subtle')}
                />
              </Show>
              <div class="flex items-center gap-2">
                <div
                  class={cn(
                    'w-2 h-2',
                    isActive()
                      ? 'bg-accent-primary'
                      : isPast()
                        ? 'bg-accent-success'
                        : 'bg-text-dim opacity-40',
                  )}
                />
                <span
                  class={cn(
                    'text-xs font-medium uppercase tracking-wider',
                    isActive() ? 'text-text-main' : 'text-text-dim',
                  )}
                >
                  {STEP_LABELS[step]}
                </span>
              </div>
            </>
          );
        }}
      </For>
    </div>
  );
}

function RepoStep() {
  const setup = getSetupStore();

  return (
    <div>
      <h2 class="text-base font-semibold text-text-main mb-1">Repository Path</h2>
      <p class="text-xs text-text-dim mb-4">
        Point Branchdeck at your project repository. This is the root directory containing your .git
        folder.
      </p>
      <input
        type="text"
        value={setup.repoPath()}
        onInput={(e) => setup.setRepoPath(e.currentTarget.value)}
        placeholder="/home/user/projects/my-app"
        class="w-full bg-bg-input border border-border-subtle text-text-main text-sm px-3 py-2 focus:outline-none focus:border-accent-primary placeholder:text-text-dim/50"
      />
    </div>
  );
}

function WorkflowsStep() {
  const setup = getSetupStore();

  createEffect(
    on(
      () => setup.currentStep(),
      (step) => {
        if (step === 'workflows') {
          setup.loadWorkflows();
        }
      },
    ),
  );

  return (
    <div>
      <h2 class="text-base font-semibold text-text-main mb-1">Workflows</h2>
      <p class="text-xs text-text-dim mb-4">
        Select which autonomous workflows to enable for this project. You can change these later.
      </p>
      <Show
        when={setup.availableWorkflows().length > 0}
        fallback={
          <p class="text-xs text-text-dim">
            No workflows discovered. Workflows will be loaded from the embedded registry,
            ~/.config/branchdeck/workflows/, and .branchdeck/workflows/.
          </p>
        }
      >
        <div class="flex flex-col gap-2">
          <For each={setup.availableWorkflows()}>
            {(wf) => {
              const checked = () => setup.enabledWorkflows().includes(wf.name);
              return (
                <label class="flex items-start gap-3 py-2 px-3 bg-bg-sidebar border border-border-subtle cursor-pointer hover:bg-bg-raised transition-colors duration-150">
                  <input
                    type="checkbox"
                    checked={checked()}
                    onChange={() => setup.toggleWorkflow(wf.name)}
                    class="mt-0.5 accent-accent-primary"
                  />
                  <div class="flex-1 min-w-0">
                    <span class="text-sm font-medium text-text-main">{wf.name}</span>
                    <Show when={wf.description}>
                      <p class="text-xs text-text-dim mt-0.5">{wf.description}</p>
                    </Show>
                  </div>
                </label>
              );
            }}
          </For>
        </div>
      </Show>
      <Show when={setup.availableWorkflows().length > 0 && setup.enabledWorkflows().length === 0}>
        <p class="text-xs text-accent-warning mt-3">
          No workflows selected. Branchdeck won't run any automation until you enable at least one.
        </p>
      </Show>
    </div>
  );
}

function TokensStep() {
  const setup = getSetupStore();

  createEffect(
    on(
      () => setup.currentStep(),
      (step) => {
        if (step === 'tokens') {
          setup.validateTokens();
        }
      },
    ),
  );

  const tv = () => setup.tokenValidation();

  return (
    <div>
      <h2 class="text-base font-semibold text-text-main mb-1">Credentials</h2>
      <p class="text-xs text-text-dim mb-4">
        Branchdeck needs a GitHub token and an Anthropic API key. These are detected automatically
        from environment variables and the gh CLI.
      </p>

      <Show
        when={!setup.isValidating()}
        fallback={<p class="text-xs text-text-dim animate-pulse-slow">Checking credentials...</p>}
      >
        <Show when={tv()}>
          {(validation) => (
            <div class="flex flex-col gap-3">
              {/* GitHub */}
              <div class="flex items-center gap-3 py-2 px-3 bg-bg-sidebar border border-border-subtle">
                <div
                  class={cn(
                    'w-2 h-2',
                    validation()['github-available'] ? 'bg-accent-success' : 'bg-accent-error',
                  )}
                />
                <div class="flex-1">
                  <span class="text-sm text-text-main">GitHub Token</span>
                  <p class="text-xs text-text-dim">
                    {validation()['github-available']
                      ? `Found via ${validation()['github-source']}`
                      : 'Not found. Set GITHUB_TOKEN env var or run `gh auth login`.'}
                  </p>
                </div>
              </div>

              {/* Anthropic */}
              <div class="flex items-center gap-3 py-2 px-3 bg-bg-sidebar border border-border-subtle">
                <div
                  class={cn(
                    'w-2 h-2',
                    validation()['anthropic-available'] ? 'bg-accent-success' : 'bg-accent-error',
                  )}
                />
                <div class="flex-1">
                  <span class="text-sm text-text-main">Anthropic API Key</span>
                  <p class="text-xs text-text-dim">
                    {validation()['anthropic-available']
                      ? `Found via ${validation()['anthropic-source']}`
                      : 'Not found. Set ANTHROPIC_API_KEY env var.'}
                  </p>
                </div>
              </div>

              <Show
                when={!validation()['github-available'] || !validation()['anthropic-available']}
              >
                <p class="text-xs text-accent-warning">
                  Missing credentials will limit functionality. You can continue setup and add them
                  later.
                </p>
              </Show>
            </div>
          )}
        </Show>
      </Show>
    </div>
  );
}

function ReviewStep() {
  const setup = getSetupStore();
  const tv = () => setup.tokenValidation();

  return (
    <div>
      <h2 class="text-base font-semibold text-text-main mb-1">Review Configuration</h2>
      <p class="text-xs text-text-dim mb-4">Confirm your project settings before saving.</p>

      <div class="flex flex-col gap-3">
        <div class="py-2 px-3 bg-bg-sidebar border border-border-subtle">
          <span class="text-xs text-text-dim uppercase tracking-wider">Repository</span>
          <p class="text-sm text-text-main mt-1">{setup.repoPath()}</p>
        </div>

        <div class="py-2 px-3 bg-bg-sidebar border border-border-subtle">
          <span class="text-xs text-text-dim uppercase tracking-wider">Workflows</span>
          <p class="text-sm text-text-main mt-1">
            {setup.enabledWorkflows().length > 0
              ? setup.enabledWorkflows().join(', ')
              : 'None selected'}
          </p>
        </div>

        <div class="py-2 px-3 bg-bg-sidebar border border-border-subtle">
          <span class="text-xs text-text-dim uppercase tracking-wider">GitHub</span>
          <p class="text-sm text-text-main mt-1">
            {tv()?.['github-available']
              ? `Available (${tv()?.['github-source']})`
              : 'Not configured'}
          </p>
        </div>

        <div class="py-2 px-3 bg-bg-sidebar border border-border-subtle">
          <span class="text-xs text-text-dim uppercase tracking-wider">Anthropic</span>
          <p class="text-sm text-text-main mt-1">
            {tv()?.['anthropic-available']
              ? `Available (${tv()?.['anthropic-source']})`
              : 'Not configured'}
          </p>
        </div>
      </div>
    </div>
  );
}

function CompleteStep() {
  return (
    <div class="flex flex-col items-center justify-center py-8">
      <span class="text-3xl font-light text-accent-success mb-4">&#10003;</span>
      <h2 class="text-base font-semibold text-text-main mb-2">Setup Complete</h2>
      <p class="text-xs text-text-dim text-center max-w-sm">
        Your project is configured. Branchdeck will use .branchdeck/config.yaml for project-specific
        settings. You can re-run setup from the Settings view.
      </p>
    </div>
  );
}

export function ProjectSetupFlow(props: { onComplete?: () => void }) {
  const setup = getSetupStore();

  async function handleNext() {
    const step = setup.currentStep();
    if (step === 'review') {
      const ok = await setup.saveConfig();
      if (ok) {
        setup.goNext();
        props.onComplete?.();
      }
    } else {
      setup.goNext();
    }
  }

  return (
    <div class="flex-1 flex flex-col overflow-hidden">
      <div class="flex-1 overflow-y-auto">
        <div class="mx-auto max-w-[540px] pt-6 pb-16 px-4">
          <h1 class="text-lg font-semibold text-text-main mb-1">Project Setup</h1>
          <p class="text-xs text-text-dim mb-6">
            Configure Branchdeck for your project in a few steps.
          </p>

          <StepIndicator />

          <Switch>
            <Match when={setup.currentStep() === 'repo'}>
              <RepoStep />
            </Match>
            <Match when={setup.currentStep() === 'workflows'}>
              <WorkflowsStep />
            </Match>
            <Match when={setup.currentStep() === 'tokens'}>
              <TokensStep />
            </Match>
            <Match when={setup.currentStep() === 'review'}>
              <ReviewStep />
            </Match>
            <Match when={setup.currentStep() === 'complete'}>
              <CompleteStep />
            </Match>
          </Switch>

          <Show when={setup.error()}>
            <p class="text-xs text-accent-error mt-4">{setup.error()}</p>
          </Show>

          <Show when={setup.currentStep() !== 'complete'}>
            <div class="flex items-center justify-between mt-6 pt-4 border-t border-border-subtle">
              <Show when={setup.canGoBack()} fallback={<div />}>
                <Button variant="secondary" onClick={() => setup.goBack()}>
                  Back
                </Button>
              </Show>
              <Button
                variant="primary"
                disabled={!setup.canGoNext() || setup.isSaving()}
                onClick={handleNext}
              >
                {setup.currentStep() === 'review'
                  ? setup.isSaving()
                    ? 'Saving...'
                    : 'Save Configuration'
                  : 'Next'}
              </Button>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}
