import { createSignal, For, Show } from 'solid-js';
import { relaunchPr, skipPr, writeApproval } from '../../lib/commands/lifecycle';
import type { AnalysisPlan, ApprovedPlan, PlanStep } from '../../types/lifecycle';

type AnalysisCardProps = {
  prKey: string;
  worktreePath: string;
  analysis: AnalysisPlan;
};

const CONFIDENCE_COLORS: Record<string, string> = {
  HIGH: 'text-[var(--color-success)]',
  MEDIUM: 'text-[var(--color-warning)]',
  LOW: 'text-[var(--color-error)]',
};

export function AnalysisCard(props: AnalysisCardProps) {
  const [editedSteps, setEditedSteps] = createSignal<PlanStep[]>([...props.analysis.plan_steps]);
  const [editedFiles, setEditedFiles] = createSignal<string[]>([...props.analysis.affected_files]);
  const [summary, setSummary] = createSignal(
    props.analysis.plan_steps.map((s) => s.description).join(', '),
  );
  const [acting, setActing] = createSignal(false);

  async function handleApprove() {
    if (editedSteps().length === 0) return;
    setActing(true);
    try {
      const plan: ApprovedPlan = {
        plan_steps: editedSteps(),
        affected_files: editedFiles(),
        summary: summary(),
      };
      await writeApproval(props.worktreePath, plan);
      await relaunchPr(props.prKey, props.worktreePath);
    } catch {
      // Error already logged by IPC wrapper
    } finally {
      setActing(false);
    }
  }

  async function handleSkip() {
    setActing(true);
    try {
      await skipPr(props.prKey);
    } finally {
      setActing(false);
    }
  }

  function removeStep(index: number) {
    setEditedSteps((s) => s.filter((_, i) => i !== index));
  }

  function updateStepDescription(index: number, description: string) {
    setEditedSteps((s) => s.map((step, i) => (i === index ? { ...step, description } : step)));
  }

  function moveStep(index: number, direction: -1 | 1) {
    const target = index + direction;
    if (target < 0 || target >= editedSteps().length) return;
    setEditedSteps((s) => {
      const arr = [...s];
      [arr[index], arr[target]] = [arr[target], arr[index]];
      return arr;
    });
  }

  function removeFile(index: number) {
    setEditedFiles((f) => f.filter((_, i) => i !== index));
  }

  return (
    <div class="bg-bg-sidebar border border-border-subtle">
      {/* Header */}
      <div class="px-3 py-2 border-b border-border-subtle flex items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="text-base font-semibold text-text-main">{props.analysis.pr.branch}</span>
          <span
            class={`text-xs font-medium uppercase ${CONFIDENCE_COLORS[props.analysis.confidence] ?? 'text-text-dim'}`}
          >
            {props.analysis.confidence}
          </span>
        </div>
        <span class="text-xs text-text-dim">
          {props.analysis.pr.repo}#{props.analysis.pr.number}
        </span>
      </div>

      <div class="px-3 py-2 space-y-3">
        {/* Failures */}
        <Show when={props.analysis.failures.length > 0}>
          <div>
            <div class="text-xs text-text-dim uppercase mb-1">Failures</div>
            <For each={props.analysis.failures}>
              {(f) => (
                <div class="text-base text-text-main mb-1">
                  <span class="text-[var(--color-error)]">{f.check_name}</span>
                  <span class="text-text-dim"> — {f.error_summary}</span>
                  <div class="text-xs text-text-dim ml-2">
                    Root cause: {f.root_cause}
                    <br />
                    Fix: {f.fix_approach}
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* Reviews */}
        <Show when={props.analysis.reviews.length > 0}>
          <div>
            <div class="text-xs text-text-dim uppercase mb-1">Reviews</div>
            <For each={props.analysis.reviews}>
              {(r) => (
                <div class="text-base text-text-main mb-1">
                  <span class="text-accent-primary">@{r.reviewer}</span>
                  <span class="text-text-dim">: {r.comment}</span>
                  <div class="text-xs text-text-dim ml-2">Response: {r.proposed_response}</div>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* Plan Steps (editable) */}
        <div>
          <div class="text-xs text-text-dim uppercase mb-1">Plan Steps</div>
          <For each={editedSteps()}>
            {(step, index) => (
              <div class="flex items-start gap-1 mb-1 group">
                <span class="text-xs text-text-dim mt-0.5 w-4 shrink-0">{index() + 1}.</span>
                <input
                  type="text"
                  value={step.description}
                  onInput={(e) => updateStepDescription(index(), e.currentTarget.value)}
                  class="flex-1 bg-bg-main border border-border-subtle px-1.5 py-0.5 text-base text-text-main focus:border-accent-primary outline-none"
                />
                <span class="text-xs text-text-dim mt-0.5">{step.file}</span>
                <div class="flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button
                    type="button"
                    onClick={() => moveStep(index(), -1)}
                    class="text-xs text-text-dim hover:text-text-main px-0.5"
                    disabled={index() === 0}
                  >
                    ↑
                  </button>
                  <button
                    type="button"
                    onClick={() => moveStep(index(), 1)}
                    class="text-xs text-text-dim hover:text-text-main px-0.5"
                    disabled={index() === editedSteps().length - 1}
                  >
                    ↓
                  </button>
                  <button
                    type="button"
                    onClick={() => removeStep(index())}
                    class="text-xs text-[var(--color-error)] hover:text-text-main px-0.5"
                  >
                    ×
                  </button>
                </div>
              </div>
            )}
          </For>
        </div>

        {/* Affected Files (editable) */}
        <div>
          <div class="text-xs text-text-dim uppercase mb-1">Affected Files</div>
          <div class="flex flex-wrap gap-1">
            <For each={editedFiles()}>
              {(file, index) => (
                <span class="inline-flex items-center gap-0.5 bg-bg-main border border-border-subtle px-1.5 py-0.5 text-xs text-text-dim">
                  {file}
                  <button
                    type="button"
                    onClick={() => removeFile(index())}
                    class="text-[var(--color-error)] hover:text-text-main"
                  >
                    ×
                  </button>
                </span>
              )}
            </For>
          </div>
        </div>

        {/* Reasoning */}
        <div>
          <div class="text-xs text-text-dim uppercase mb-1">Reasoning</div>
          <div class="text-base text-text-dim">{props.analysis.reasoning}</div>
        </div>

        {/* Commit summary */}
        <div>
          <div class="text-xs text-text-dim uppercase mb-1">Commit Summary</div>
          <input
            type="text"
            value={summary()}
            onInput={(e) => setSummary(e.currentTarget.value)}
            class="w-full bg-bg-main border border-border-subtle px-1.5 py-0.5 text-base text-text-main focus:border-accent-primary outline-none"
          />
        </div>

        {/* Actions */}
        <div class="flex gap-2 pt-1">
          <button
            type="button"
            onClick={handleApprove}
            disabled={acting() || editedSteps().length === 0}
            class="px-3 py-1 text-base font-medium bg-bg-main border border-[var(--color-success)] text-[var(--color-success)] hover:bg-[var(--color-success)] hover:text-bg-main transition-colors duration-150 disabled:opacity-40"
          >
            Approve
          </button>
          <button
            type="button"
            onClick={handleSkip}
            disabled={acting()}
            class="px-3 py-1 text-base font-medium bg-bg-main border border-border-subtle text-text-dim hover:text-text-main transition-colors duration-150 disabled:opacity-40"
          >
            Skip
          </button>
        </div>
      </div>
    </div>
  );
}
