import { Show } from 'solid-js';
import type { InboxItem } from '../../types/inbox';
import { ActionButton } from '../ui/ActionButton';
import { InboxBadge } from '../ui/InboxBadge';

type InboxRowDetailProps = {
  item: InboxItem;
  onMerge: () => void;
  onDismiss: () => void;
};

export function InboxRowDetail(props: InboxRowDetailProps) {
  const canMerge = () => props.item.type === 'pr' && props.item.ciStatus === 'passing';

  return (
    <div class="bg-surface-raised px-3 py-2 border-b border-border-subtle">
      {/* Detail info */}
      <div class="flex flex-wrap items-center gap-3 mb-2">
        <Show when={props.item.satDelta != null}>
          <InboxBadge
            label={`${(props.item.satDelta ?? 0) > 0 ? '+' : ''}${props.item.satDelta} pts`}
            structure="filled"
            color={(props.item.satDelta ?? 0) > 0 ? 'success' : 'error'}
          />
        </Show>
        <Show when={props.item.persona}>
          <span class="text-[11px] text-text-dim">{props.item.persona}</span>
        </Show>
        <Show when={props.item.ciStatus}>
          <InboxBadge
            label={props.item.ciStatus ?? ''}
            structure="outlined"
            color={
              props.item.ciStatus === 'passing'
                ? 'success'
                : props.item.ciStatus === 'failing'
                  ? 'error'
                  : 'warning'
            }
          />
        </Show>
        <Show when={props.item.filesChanged}>
          <span class="text-[11px] text-accent-info">{props.item.filesChanged} files</span>
        </Show>
        <Show when={props.item.agentDuration}>
          <span class="text-[11px] text-text-dim">{props.item.agentDuration}</span>
        </Show>
      </div>

      {/* Loop complete label */}
      <Show when={props.item.loopComplete}>
        <div class="text-[11px] text-accent-success mb-2">found &gt; fixed &gt; verified</div>
      </Show>

      {/* Action buttons */}
      <div class="flex items-center gap-2">
        <ActionButton label="Review" variant="secondary" size="compact" shortcutHint="r" />
        <Show when={props.item.type === 'pr'}>
          <ActionButton
            label="Merge"
            variant="primary"
            size="compact"
            shortcutHint="m"
            disabled={!canMerge()}
            onClick={props.onMerge}
          />
        </Show>
        <ActionButton label="View diff" variant="secondary" size="compact" shortcutHint="d" />
        <ActionButton
          label="Dismiss"
          variant="secondary"
          size="compact"
          shortcutHint="x"
          onClick={props.onDismiss}
        />
      </div>
    </div>
  );
}
