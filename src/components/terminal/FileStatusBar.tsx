import { createMemo, createSignal, Show } from 'solid-js';
import { getAgentStore } from '../../lib/stores/agent';
import { getRepoStore } from '../../lib/stores/repo';
import { FileGrid } from '../layout/FileGrid';

export function FileStatusBar() {
  const repoStore = getRepoStore();
  const agentStore = getAgentStore();
  const [expanded, setExpanded] = createSignal(false);

  const worktreePath = () => repoStore.state.activeWorktreePath ?? '';

  const fileCount = createMemo(() => {
    const wt = worktreePath();
    if (!wt) return { total: 0, active: 0, modified: 0 };
    let total = 0;
    let active = 0;
    let modified = 0;

    for (const f of agentStore.state.log) {
      if (f.filePath?.startsWith(wt)) {
        total++;
        if (f.kind === 'toolEnd' && (f.toolName === 'Write' || f.toolName === 'Edit')) {
          modified++;
        }
      }
    }

    for (const [, info] of Object.entries(agentStore.state.agentsByTab)) {
      if (info.currentFile?.startsWith(wt)) {
        active++;
      }
    }

    return { total: Math.min(total, 999), active, modified };
  });

  return (
    <Show when={worktreePath() && fileCount().total > 0}>
      <div class="border-t border-border-subtle bg-bg-sidebar">
        <button
          type="button"
          class="flex items-center justify-between w-full px-3 py-0.5 text-xs text-text-dim hover:text-text-main cursor-pointer hover:bg-bg-main/30 transition-colors"
          onClick={() => setExpanded((v) => !v)}
        >
          <span>
            {fileCount().total} files
            <Show when={fileCount().active > 0}>
              <span class="text-accent-success"> ({fileCount().active} active)</span>
            </Show>
            <Show when={fileCount().modified > 0}>
              <span class="text-accent-warning"> ({fileCount().modified} modified)</span>
            </Show>
          </span>
          <span>{expanded() ? '\u25BC' : '\u25B6'}</span>
        </button>
        <Show when={expanded()}>
          <FileGrid worktreePath={worktreePath()} />
        </Show>
      </div>
    </Show>
  );
}
