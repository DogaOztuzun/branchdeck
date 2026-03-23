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
    <>
      <div
        class="px-3 py-1.5 hover:bg-bg-main/50 flex items-center gap-2 text-base cursor-pointer border-b border-border-subtle/50 transition-colors duration-150"
        tabIndex={0}
        onClick={handleStartWorkflow}
        onContextMenu={handleContextMenu}
        onKeyDown={(e) => {
          if (e.key === 'Enter') handleStartWorkflow();
        }}
      >
        {/* Status dot */}
        <StatusDot status={props.item.pr.ciStatus} />

        {/* Left: branch + title */}
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span class="text-sm text-accent-info shrink-0">
              #{props.item.pr.number}
            </span>
            <span class="text-text-main truncate font-medium">
              {props.item.pr.branch}
            </span>
            <span class="text-xs text-text-dim shrink-0">
              {props.item.pr.repoName.split('/').pop()}
            </span>
          </div>
          <Show when={props.item.pr.title}>
            <div class="text-xs text-text-dim truncate mt-0.5">
              {props.item.pr.title}
            </div>
          </Show>
        </div>

        {/* Right: badges + action */}
        <div class="flex items-center gap-2 shrink-0">
          <Show when={reviewLabel()}>
            <span class={`text-xs font-medium uppercase ${reviewColor()}`}>
              {reviewLabel()}
            </span>
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
