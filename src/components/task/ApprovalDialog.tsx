import { Show } from 'solid-js';
import type { PermissionRequestEvent } from '../../types/run';

type ApprovalDialogProps = {
  permission: PermissionRequestEvent;
  onRespond: (decision: 'approve' | 'deny') => void;
};

export function ApprovalDialog(props: ApprovalDialogProps) {
  return (
    <div class="mx-2 my-1.5 p-2 rounded border border-warning/40 bg-accent-warning/5">
      <div class="text-[10px] uppercase text-accent-warning tracking-wider mb-1">
        Permission Required
      </div>
      <div class="text-xs text-text-main mb-1">
        Tool: <span class="font-mono text-accent-info">{props.permission.tool ?? 'unknown'}</span>
      </div>
      <Show when={props.permission.command}>
        <div class="text-[10px] text-text-dim font-mono bg-bg-main/50 rounded px-1.5 py-1 mb-1.5 break-all max-h-16 overflow-y-auto">
          {props.permission.command}
        </div>
      </Show>
      <div class="flex gap-1.5">
        <button
          type="button"
          class="flex-1 px-2 py-1 text-[10px] font-medium text-green-400 border border-green-400/40 rounded hover:bg-green-400/10 cursor-pointer"
          onClick={() => props.onRespond('approve')}
        >
          Approve
        </button>
        <button
          type="button"
          class="flex-1 px-2 py-1 text-[10px] font-medium text-red-400 border border-red-400/40 rounded hover:bg-red-400/10 cursor-pointer"
          onClick={() => props.onRespond('deny')}
        >
          Deny
        </button>
      </div>
    </div>
  );
}
