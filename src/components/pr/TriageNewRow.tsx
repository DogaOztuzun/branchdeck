import { createSignal, Show } from 'solid-js';
import { shepherdPr } from '../../lib/commands/lifecycle';
import type { PrSummary } from '../../types/github';
import type { TriagePr } from '../../types/lifecycle';

type TriageNewRowProps = {
  item: TriagePr & { pr: PrSummary };
};

function StatusDot(props: { status: string | null }) {
  const color = () => {
    const s = props.status;
    if (s === 'FAILURE' || s === 'ERROR') return 'bg-accent-error';
    if (s === 'SUCCESS') return 'bg-accent-success';
    if (s === 'PENDING') return 'bg-accent-warning';
    return 'bg-text-dim/40';
  };
  return <span class={`inline-block w-2 h-2 shrink-0 ${color()}`} />;
}

export function TriageNewRow(props: TriageNewRowProps) {
  const [expanded, setExpanded] = createSignal(false);
  const [loading, setLoading] = createSignal(false);

  async function handleStartWorkflow() {
    if (!props.item.repoPath) return;
    setLoading(true);
    try {
      await shepherdPr(props.item.repoPath, props.item.pr.number);
    } catch {
      // Error logged by IPC wrapper
    } finally {
      setLoading(false);
    }
  }

  const reviewLabel = () => {
    const decision = props.item.pr.reviewDecision;
    if (decision === 'APPROVED') return 'APPROVED';
    if (decision === 'CHANGES_REQUESTED') return 'CHANGES';
    return '';
  };

  const reviewColor = () => {
    const decision = props.item.pr.reviewDecision;
    if (decision === 'APPROVED') return 'text-accent-success';
    if (decision === 'CHANGES_REQUESTED') return 'text-accent-error';
    return 'text-text-dim';
  };

  const age = () => {
    if (!props.item.pr.createdAt) return '';
    const ms = Date.now() - new Date(props.item.pr.createdAt).getTime();
    const hours = Math.floor(ms / 3_600_000);
    if (hours < 24) return `${hours}h`;
    return `${Math.floor(hours / 24)}d`;
  };

  return (
    <div class="border-b border-border-subtle/50">
      {/* Row — click to expand */}
      <button
        type="button"
        class="w-full text-left px-3 py-1.5 hover:bg-[var(--color-surface-raised)] flex items-center gap-2 text-base cursor-pointer transition-colors duration-150"
        onClick={() => setExpanded(!expanded())}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setExpanded(false);
        }}
      >
        <StatusDot status={props.item.pr.ciStatus} />

        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span class="text-sm text-accent-info shrink-0">#{props.item.pr.number}</span>
            <span class="text-text-main truncate font-medium">{props.item.pr.branch}</span>
          </div>
          <Show when={props.item.pr.title}>
            <div class="text-xs text-text-dim truncate mt-0.5">{props.item.pr.title}</div>
          </Show>
        </div>

        <div class="flex items-center gap-2 shrink-0">
          <Show when={reviewLabel()}>
            <span class={`text-xs font-medium uppercase ${reviewColor()}`}>{reviewLabel()}</span>
          </Show>
          <Show when={props.item.pr.additions != null}>
            <span class="text-xs text-text-dim">
              <span class="text-accent-success">+{props.item.pr.additions}</span>
              <span class="mx-0.5">/</span>
              <span class="text-accent-error">-{props.item.pr.deletions}</span>
            </span>
          </Show>
          <Show when={age()}>
            <span class="text-xs text-text-dim">{age()}</span>
          </Show>
          <Show when={loading()}>
            <span class="text-xs text-accent-warning">Analyzing...</span>
          </Show>
        </div>
      </button>

      {/* Expanded detail — shows action */}
      <Show when={expanded() && !loading()}>
        <div class="px-3 pb-2 pt-1 bg-bg-main/30 flex items-center gap-3">
          <span class="text-xs text-text-dim flex-1">{props.item.pr.url}</span>
          <Show when={props.item.repoPath}>
            <button
              type="button"
              class="text-xs text-accent-primary border border-accent-primary/30 px-3 py-1 hover:bg-accent-primary/10 cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                handleStartWorkflow();
              }}
            >
              Analyze PR
            </button>
          </Show>
        </div>
      </Show>
    </div>
  );
}
