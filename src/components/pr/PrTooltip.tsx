import { openUrl } from '@tauri-apps/plugin-opener';
import { createSignal, onCleanup, onMount } from 'solid-js';
import { Portal } from 'solid-js/web';
import type { PrInfo } from '../../types/github';

type PrTooltipProps = {
  pr: PrInfo;
  anchorEl: HTMLElement;
  onClose: () => void;
  onHover?: () => void;
};

function stateColor(state: string, isDraft: boolean): string {
  if (isDraft) return '#565f89';
  switch (state.toLowerCase()) {
    case 'open':
      return '#7aa2f7';
    case 'merged':
      return '#bb9af7';
    case 'closed':
      return '#f7768e';
    default:
      return '#565f89';
  }
}

function stateLabel(state: string, isDraft: boolean): string {
  if (isDraft) return 'Draft';
  return state.charAt(0).toUpperCase() + state.slice(1).toLowerCase();
}

function reviewColor(state: string): string {
  switch (state.toLowerCase()) {
    case 'approved':
      return '#9ece6a';
    case 'changes_requested':
      return '#f7768e';
    case 'commented':
      return '#e0af68';
    default:
      return '#565f89';
  }
}

function reviewLabel(state: string): string {
  switch (state.toLowerCase()) {
    case 'approved':
      return 'Approved';
    case 'changes_requested':
      return 'Changes requested';
    case 'commented':
      return 'Commented';
    case 'pending':
      return 'Pending';
    case 'dismissed':
      return 'Dismissed';
    default:
      return state;
  }
}

function checkIcon(
  status: string,
  conclusion: string | null,
): {
  char: string;
  color: string;
} {
  if (status === 'completed') {
    if (conclusion === 'success') return { char: '\u2713', color: '#9ece6a' };
    if (conclusion === 'skipped') return { char: '\u2192', color: '#565f89' };
    if (conclusion === 'neutral') return { char: '\u2014', color: '#565f89' };
    if (conclusion === 'cancelled') return { char: '\u25CB', color: '#565f89' };
    return { char: '\u2715', color: '#f7768e' };
  }
  if (status === 'in_progress') return { char: '\u2022', color: '#e0af68' };
  return { char: '\u25CB', color: '#565f89' };
}

export function PrTooltip(props: PrTooltipProps) {
  const [position, setPosition] = createSignal({ top: 0, left: 0 });
  const [checksExpanded, setChecksExpanded] = createSignal(false);

  let leaveTimer: ReturnType<typeof setTimeout> | undefined;
  let mounted = true;

  function startLeaveTimer() {
    if (leaveTimer !== undefined) clearTimeout(leaveTimer);
    leaveTimer = setTimeout(() => {
      if (mounted) props.onClose();
    }, 200);
  }

  function cancelLeaveTimer() {
    if (leaveTimer !== undefined) {
      clearTimeout(leaveTimer);
      leaveTimer = undefined;
    }
    if (mounted) props.onHover?.();
  }

  onMount(() => {
    const rect = props.anchorEl.getBoundingClientRect();
    const maxWidth = 320;
    let top = rect.bottom + 4;
    let left = rect.right + 4;

    if (left + maxWidth > window.innerWidth) {
      left = Math.max(4, window.innerWidth - maxWidth - 4);
    }
    if (top + 200 > window.innerHeight) {
      top = Math.max(4, rect.top - 200 - 4);
    }

    setPosition({ top, left });

    props.anchorEl.addEventListener('mouseenter', cancelLeaveTimer);
    props.anchorEl.addEventListener('mouseleave', startLeaveTimer);
  });

  onCleanup(() => {
    mounted = false;
    if (leaveTimer !== undefined) {
      clearTimeout(leaveTimer);
      leaveTimer = undefined;
    }
    props.anchorEl.removeEventListener('mouseenter', cancelLeaveTimer);
    props.anchorEl.removeEventListener('mouseleave', startLeaveTimer);
  });

  const skippedConclusions = new Set(['skipped', 'neutral', 'cancelled']);
  const activeChecks = () =>
    props.pr.checks.filter((c) => !skippedConclusions.has(c.conclusion ?? ''));
  const passedChecks = () =>
    activeChecks().filter((c) => c.status === 'completed' && c.conclusion === 'success').length;

  const diffStats = () => {
    if (props.pr.additions === null && props.pr.deletions === null) return null;
    return `+${props.pr.additions ?? 0} -${props.pr.deletions ?? 0}`;
  };

  return (
    <Portal>
      {/* biome-ignore lint/a11y/noStaticElementInteractions: tooltip hover behavior, not interactive content */}
      <div
        class="fixed z-50 max-w-[320px] border border-border-subtle bg-bg-sidebar p-3 text-xs shadow-lg space-y-2"
        style={{
          top: `${position().top}px`,
          left: `${position().left}px`,
        }}
        onMouseEnter={cancelLeaveTimer}
        onMouseLeave={startLeaveTimer}
      >
        {/* Header row */}
        <div class="flex items-center gap-2">
          {props.pr.url ? (
            <button
              type="button"
              class="font-semibold text-blue-400 hover:underline cursor-pointer"
              onClick={() => openUrl(props.pr.url)}
            >
              PR #{props.pr.number}
            </button>
          ) : (
            <span class="font-semibold">PR #{props.pr.number}</span>
          )}
          <span
            class="rounded-full px-1.5 py-0.5 text-xs font-medium leading-none"
            style={{
              'background-color': stateColor(props.pr.state, props.pr.isDraft),
              color: '#1a1b26',
            }}
          >
            {stateLabel(props.pr.state, props.pr.isDraft)}
          </span>
          <span class="ml-auto text-muted">{diffStats() ?? '\u2014'}</span>
        </div>

        {/* Title */}
        <p class="line-clamp-2 leading-snug text-foreground">{props.pr.title}</p>

        {/* Reviews section */}
        <div>
          <h4 class="mb-1 font-semibold text-text-dim">Reviews</h4>
          {props.pr.reviewDecision && (
            <span
              class="mb-1 inline-block px-1.5 py-0.5 text-xs font-medium leading-none"
              style={{
                'background-color': reviewColor(props.pr.reviewDecision),
                color: '#1a1b26',
              }}
            >
              {reviewLabel(props.pr.reviewDecision)}
            </span>
          )}
          {props.pr.reviews.length === 0 ? (
            <p class="text-text-dim">No reviews</p>
          ) : (
            <ul class="space-y-0.5">
              {(() => {
                const grouped = new Map<string, { user: string; state: string; count: number }>();
                for (const review of props.pr.reviews) {
                  const key = `${review.user}:${review.state}`;
                  const existing = grouped.get(key);
                  if (existing) {
                    existing.count++;
                  } else {
                    grouped.set(key, { user: review.user, state: review.state, count: 1 });
                  }
                }
                return [...grouped.values()].map((entry) => (
                  <li class="flex items-center gap-1.5">
                    <span
                      class="inline-block h-2 w-2 rounded-full shrink-0"
                      style={{ 'background-color': reviewColor(entry.state) }}
                    />
                    <span class="text-text-main">{entry.user}</span>
                    <span class="text-text-dim">
                      {reviewLabel(entry.state)}
                      {entry.count > 1 ? ` \u00d7${entry.count}` : ''}
                    </span>
                  </li>
                ));
              })()}
            </ul>
          )}
        </div>

        {/* Checks section */}
        <div>
          <h4 class="mb-1 font-semibold text-muted">Checks</h4>
          {props.pr.checks.length === 0 ? (
            <p class="text-muted">No checks</p>
          ) : (
            <>
              <button
                type="button"
                class="cursor-pointer text-muted hover:text-foreground"
                onClick={() => setChecksExpanded((v) => !v)}
              >
                {passedChecks()}/{activeChecks().length} checks passed{' '}
                {checksExpanded() ? '\u25B4' : '\u25BE'}
              </button>
              {checksExpanded() && (
                <ul class="mt-1 space-y-0.5">
                  {props.pr.checks.map((check) => {
                    const icon = checkIcon(check.status, check.conclusion);
                    return (
                      <li class="flex items-center gap-1.5">
                        <span style={{ color: icon.color }}>{icon.char}</span>
                        {check.detailsUrl ? (
                          <button
                            type="button"
                            class="cursor-pointer text-blue-400 hover:underline"
                            onClick={() => openUrl(check.detailsUrl as string)}
                          >
                            {check.name}
                          </button>
                        ) : (
                          <span class="text-foreground">{check.name}</span>
                        )}
                      </li>
                    );
                  })}
                </ul>
              )}
            </>
          )}
        </div>

        {/* Footer */}
        {props.pr.url && (
          <div class="border-t border-border-subtle pt-2">
            <button
              type="button"
              class="cursor-pointer text-blue-400 hover:underline"
              onClick={() => openUrl(props.pr.url)}
            >
              View on GitHub
            </button>
          </div>
        )}
      </div>
    </Portal>
  );
}
