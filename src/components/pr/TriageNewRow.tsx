import { createSignal, Show } from 'solid-js';
import { shepherdPr } from '../../lib/commands/lifecycle';
import type { PrSummary } from '../../types/github';
import type { TriagePr } from '../../types/lifecycle';

type TriageNewRowProps = {
  item: TriagePr & { pr: PrSummary };
};

export function TriageNewRow(props: TriageNewRowProps) {
  const [menuOpen, setMenuOpen] = createSignal(false);
  const [menuPos, setMenuPos] = createSignal({ x: 0, y: 0 });
  const [loading, setLoading] = createSignal(false);

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    setMenuPos({ x: e.clientX, y: e.clientY });
    setMenuOpen(true);
  }

  function closeMenu() {
    setMenuOpen(false);
  }

  async function handleStartWorkflow() {
    if (!props.item.repoPath) return;
    closeMenu();
    setLoading(true);
    try {
      await shepherdPr(props.item.repoPath, props.item.pr.number);
    } catch {
      // Error logged by IPC wrapper
    } finally {
      setLoading(false);
    }
  }

  const ciColor = () => {
    const status = props.item.pr.ciStatus;
    if (status === 'FAILURE' || status === 'ERROR') return 'text-accent-error';
    if (status === 'SUCCESS') return 'text-accent-success';
    if (status === 'PENDING') return 'text-accent-warning';
    return 'text-text-dim';
  };

  const ciLabel = () => {
    const status = props.item.pr.ciStatus;
    if (status === 'FAILURE' || status === 'ERROR') return 'FAIL';
    if (status === 'SUCCESS') return 'PASS';
    if (status === 'PENDING') return 'PEND';
    return status ?? '';
  };

  const reviewLabel = () => {
    const decision = props.item.pr.reviewDecision;
    if (decision === 'APPROVED') return 'APPROVED';
    if (decision === 'CHANGES_REQUESTED') return 'CHANGES';
    if (decision === 'REVIEW_REQUIRED') return 'REVIEW';
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
    <>
      <div
        class="px-3 py-1 hover:bg-bg-main/30 flex items-center gap-3 text-base cursor-default"
        tabIndex={0}
        onContextMenu={handleContextMenu}
        onKeyDown={(e) => {
          if (e.key === 'Enter') handleContextMenu(e as unknown as MouseEvent);
        }}
      >
        <span class="truncate flex-1 text-text-main">
          {props.item.pr.branch}
        </span>
        <span class="text-xs text-text-dim shrink-0">
          {props.item.pr.repoName.split('/').pop()}
        </span>
        <span class="text-xs text-text-dim shrink-0">
          #{props.item.pr.number}
        </span>
        <span class={`text-xs font-medium uppercase shrink-0 ${ciColor()}`}>
          {ciLabel()}
        </span>
        <Show when={reviewLabel()}>
          <span class={`text-xs font-medium uppercase shrink-0 ${reviewColor()}`}>
            {reviewLabel()}
          </span>
        </Show>
        <Show when={props.item.pr.additions != null}>
          <span class="text-xs text-accent-success shrink-0">
            +{props.item.pr.additions}
          </span>
          <span class="text-xs text-accent-error shrink-0">
            -{props.item.pr.deletions}
          </span>
        </Show>
        <Show when={age()}>
          <span class="text-xs text-text-dim shrink-0">{age()}</span>
        </Show>
        <Show when={loading()}>
          <span class="text-xs text-accent-warning shrink-0">Starting...</span>
        </Show>
      </div>

      {/* Context menu */}
      <Show when={menuOpen()}>
        <div
          class="fixed inset-0 z-50"
          onClick={closeMenu}
          onContextMenu={(e) => { e.preventDefault(); closeMenu(); }}
        />
        <div
          class="fixed z-50 bg-bg-sidebar border border-border-subtle py-1 min-w-[160px]"
          style={{ left: `${menuPos().x}px`, top: `${menuPos().y}px` }}
        >
          <button
            type="button"
            class="w-full px-3 py-1.5 text-left text-base text-text-main hover:bg-bg-main/50 cursor-pointer"
            onClick={handleStartWorkflow}
          >
            Start Workflow
          </button>
        </div>
      </Show>
    </>
  );
}
