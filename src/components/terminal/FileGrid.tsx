import { createEffect, createMemo, createSignal, For, Show } from 'solid-js';
import { getRepoStatus } from '../../lib/commands/git';
import { getAgentStore } from '../../lib/stores/agent';
import { shortPath } from '../../lib/utils';
import type { FileAccess } from '../../types/agent';

type FileGridProps = {
  worktreePath: string;
  visible: boolean;
};

type FileState = 'active' | 'modified' | 'read' | 'changed';

function dotColor(state: FileState): string {
  switch (state) {
    case 'active':
      return 'bg-success';
    case 'modified':
      return 'bg-warning';
    case 'read':
      return 'bg-info';
    case 'changed':
      return 'bg-text-muted';
  }
}

function stateLabel(state: FileState): string {
  switch (state) {
    case 'active':
      return 'Agent active';
    case 'modified':
      return 'Modified by agent';
    case 'read':
      return 'Read by agent';
    case 'changed':
      return 'Changed in branch';
  }
}

type FileEntry = {
  path: string;
  state: FileState;
  access: FileAccess | null;
  gitStatus: string | null;
};

export function FileGrid(props: FileGridProps) {
  const agentStore = getAgentStore();
  const [changedFiles, setChangedFiles] = createSignal<Map<string, string>>(new Map());
  const [hoveredFile, setHoveredFile] = createSignal<{
    entry: FileEntry;
    x: number;
    y: number;
  } | null>(null);

  // Load git status (changed files in worktree)
  createEffect(() => {
    if (props.worktreePath && props.visible) {
      getRepoStatus(props.worktreePath)
        .then((statuses) => {
          const map = new Map<string, string>();
          for (const s of statuses) {
            map.set(s.path, s.status);
          }
          setChangedFiles(map);
        })
        .catch(() => setChangedFiles(new Map()));
    }
  });

  // Build file access map from agent events
  const fileAccessMap = createMemo(() => {
    const map = new Map<string, FileAccess>();
    for (const f of agentStore.state.log) {
      if (f.filePath) {
        const rel = f.filePath.startsWith(props.worktreePath)
          ? f.filePath.substring(props.worktreePath.length).replace(/^\//, '')
          : f.filePath;
        if (!map.has(rel)) {
          map.set(rel, {
            path: rel,
            lastTool: f.toolName ?? '',
            lastAgent: f.agentId ?? '',
            lastAccess: f.ts,
            accessCount: 1,
            wasModified: f.kind === 'toolEnd' && (f.toolName === 'Write' || f.toolName === 'Edit'),
          });
        } else {
          const existing = map.get(rel);
          if (existing) {
            existing.accessCount += 1;
            existing.lastAccess = f.ts;
            if (f.toolName) existing.lastTool = f.toolName;
            if (f.kind === 'toolEnd' && (f.toolName === 'Write' || f.toolName === 'Edit')) {
              existing.wasModified = true;
            }
          }
        }
      }
    }
    return map;
  });

  // Currently active files (agent is working on right now)
  const activeFiles = createMemo(() => {
    const active = new Set<string>();
    for (const [, info] of Object.entries(agentStore.state.agentsByTab)) {
      if (info.currentFile) {
        const rel = info.currentFile.startsWith(props.worktreePath)
          ? info.currentFile.substring(props.worktreePath.length).replace(/^\//, '')
          : info.currentFile;
        active.add(rel);
      }
    }
    return active;
  });

  // Merge: git changed files + agent-touched files → deduplicated list
  const entries = createMemo(() => {
    const result = new Map<string, FileEntry>();

    // Git changed files
    for (const [path, status] of changedFiles()) {
      result.set(path, { path, state: 'changed', access: null, gitStatus: status });
    }

    // Agent-touched files (may overlap with git changes)
    for (const [path, access] of fileAccessMap()) {
      const existing = result.get(path);
      const isActive = activeFiles().has(path);
      const state: FileState = isActive ? 'active' : access.wasModified ? 'modified' : 'read';
      if (existing) {
        existing.state = state;
        existing.access = access;
      } else {
        result.set(path, { path, state, access, gitStatus: null });
      }
    }

    return [...result.values()].sort((a, b) => {
      const order: Record<FileState, number> = { active: 0, modified: 1, read: 2, changed: 3 };
      return order[a.state] - order[b.state] || a.path.localeCompare(b.path);
    });
  });

  return (
    <Show when={props.visible}>
      <div class="overflow-y-auto max-h-52 p-2">
        <Show
          when={entries().length > 0}
          fallback={
            <div class="text-[10px] text-text-muted text-center py-2">No file activity</div>
          }
        >
          <div class="flex items-center justify-between mb-1.5 px-0.5">
            <span class="text-[10px] uppercase text-text-muted tracking-wider">Files</span>
            <span class="text-[10px] text-text-muted">{entries().length}</span>
          </div>
          <div class="flex flex-wrap gap-[3px]">
            <For each={entries()}>
              {(entry) => (
                <span
                  role="img"
                  aria-label={entry.path}
                  class={`inline-block w-2.5 h-2.5 rounded-sm ${dotColor(entry.state)}`}
                  title={entry.path}
                  onMouseEnter={(e) => setHoveredFile({ entry, x: e.clientX, y: e.clientY })}
                  onMouseLeave={() => setHoveredFile(null)}
                />
              )}
            </For>
          </div>

          {/* Tooltip */}
          <Show when={hoveredFile()}>
            {(hovered) => (
              <div
                class="fixed z-50 px-2 py-1.5 bg-surface border border-border rounded shadow-lg text-xs max-w-72 pointer-events-none"
                style={{
                  left: `${hovered().x + 12}px`,
                  top: `${hovered().y - 8}px`,
                }}
              >
                <div class="text-text truncate font-mono text-[11px]">
                  {shortPath(hovered().entry.path, 2)}
                </div>
                <div
                  class={`text-[10px] mt-0.5 ${dotColor(hovered().entry.state).replace('bg-', 'text-')}`}
                >
                  {stateLabel(hovered().entry.state)}
                </div>
                <Show when={hovered().entry.gitStatus}>
                  <div class="text-[10px] text-text-muted">Git: {hovered().entry.gitStatus}</div>
                </Show>
                <Show when={hovered().entry.access}>
                  {(access) => (
                    <div class="text-[10px] text-text-muted mt-0.5">
                      {access().lastTool} ({access().accessCount}x)
                    </div>
                  )}
                </Show>
              </div>
            )}
          </Show>
        </Show>
      </div>
    </Show>
  );
}
